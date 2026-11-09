//! **The virtual fence**: the impassable part of a building is visible, and no unit's centre is ever on it (D162).
//!
//! The owner, with a screenshot of villagers standing on a town centre: *"Is it possible to have a virtual border around each building and
//! no unit['s] center … can cross it? Except for Farm and Town Center (for TC the virtual fence would be around the impassable structure
//! of the building in the top quarter of the diamond - highlighted in lighter gray)."*
//!
//! The engine has enforced exactly that since D115 - the town centre is a **4-tile core plus a walkable yard**, and the walk grid refuses
//! the core - so what was missing was the *picture*: the core and the yard were both light grey, which made a villager in the yard look
//! like a villager in the masonry. Written before the colour change it justifies (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render_with_solids, solids_of};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy"),
        units: ruage_data::load_units_file(&units).expect("units"),
        resources,
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("stockpile");
    world
}

fn luma(pixel: [u8; 4]) -> u16 {
    u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2])
}

#[test]
fn the_impassable_core_is_dark_and_the_yard_is_light() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let (across, down) = town.footprint;
    let solids = solids_of(&world, &snapshot);
    let mask = solids.get(&town.id).expect("a mask");
    let camera = Camera::new((900, 700)).centred_on((
        town.position.0 + across as f32 / 2.0,
        town.position.1 + down as f32 / 2.0,
    ));
    let frame = render_with_solids(&snapshot, &camera, &solids);

    let wall = luma(palette::BUILDING);
    let yard = luma(palette::BUILDING_YARD);
    assert!(
        wall + 60 <= yard,
        "the core and the yard are too close in tone to read apart: {wall} against {yard}"
    );

    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    let (min_x, min_y, _, _) = ruage_view::footprint_tiles(anchor, town.footprint);
    let (rx, ry) = camera.radii();
    let mut core = 0;
    let mut yard_seen = 0;
    for dy in 0..down as i32 {
        for dx in 0..across as i32 {
            let (x, y) = (min_x + dx, min_y + dy);
            // **Pixels, not tiles**: `rx`/`ry` are the diamond's radii in *screen* space, so the offset is applied after the
            // projection. Subtracting them from a tile coordinate put the sample tiles away (my first version of this test did
            // exactly that and found nothing solid at all).
            let (px, py) = camera.project((x as f32, y as f32));
            let at = (
                (px as f32 - rx * 0.45).max(0.0) as i32,
                (py as f32 + ry * 0.45).max(0.0) as i32,
            );
            if at.0 < 0 || at.1 < 0 || at.0 as u32 >= 900 || at.1 as u32 >= 700 {
                continue;
            }
            let painted = frame.pixel(at.0 as u32, at.1 as u32);
            match mask[(dy * across as i32 + dx) as usize] {
                true => {
                    assert_eq!(
                        painted,
                        palette::BUILDING,
                        "the core tile ({x},{y}) is not structure"
                    );
                    core += 1;
                }
                false => {
                    assert_eq!(
                        painted,
                        palette::BUILDING_YARD,
                        "the yard tile ({x},{y}) is not yard"
                    );
                    yard_seen += 1;
                }
            }
        }
    }
    assert_eq!(core, 4, "the town centre's core is four tiles (D115)");
    assert!(
        yard_seen >= 8,
        "the yard is the rest of the footprint: {yard_seen}"
    );
}

/// **No unit's centre is ever on a solid tile** - the owner's rule, and the engine's: a villager walking about the town centre may use the
/// yard and never the core.
#[test]
fn no_unit_centre_crosses_the_core() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let town = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| (entity.id, world.tile_of_position(entity.position)))
        .expect("a town centre");
    let (_, at) = town;
    let villager_role = world.role_unit("villager").expect("villager");
    // A villager on the far side of the town centre, ordered to walk to the other far side: the straight line crosses the core.
    let start = (i64::from(at.0) - 1, i64::from(at.1) + 4);
    let id = EntityId(9_700);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager_role),
            position: (
                Fixed::from_raw(start.0 * scale),
                Fixed::from_raw(start.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (at.0 + 4, at.1 - 1),
        })
        .expect("a walk across");

    let mut core_tiles = Vec::new();
    for dy in 0..4i64 {
        for dx in 0..4i64 {
            let tile = ((at.0 as i64 + dx) as u16, (at.1 as i64 + dy) as u16);
            if !world.tile_walkable(tile.0, tile.1) {
                core_tiles.push(tile);
            }
        }
    }
    assert_eq!(core_tiles.len(), 4, "four core tiles: {core_tiles:?}");
    for step in 0..1_200 {
        world.step().expect("step");
        let here = world.tile_of_position(world.position(id).expect("position"));
        assert!(
            !core_tiles.contains(&here),
            "step {step}: the unit's centre crossed the core at {here:?}"
        );
    }
}
