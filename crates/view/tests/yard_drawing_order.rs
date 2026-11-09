//! **A unit in the yard is drawn on the yard** (D163): the walkable part of a building's footprint is ground, so it is painted with
//! the ground and before the units. Painting it in the entity pass meant the building covered every unit north of it, and a villager
//! in the yard came out hidden or half-visible - which reads as "standing on the structure".
//!
//! Written after the owner's third screenshot of the same complaint (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render_with_solids, solids_of};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn seeded() -> Option<(World, ruage_data::Map)> {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");
    Some((world, map))
}

#[test]
fn a_unit_standing_in_the_yard_is_painted_over_the_yard() {
    let Some((mut world, map)) = seeded() else {
        return;
    };
    let scale = world.scale();
    let town = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| (entity.id, world.tile_of_position(entity.position)))
        .expect("a town centre");
    let (_, at) = town;
    // A **yard** tile of this town centre, on its north-east side: walkable, and at a smaller depth than the building - which is the
    // case the entity pass painted over.
    let yard = (at.0 + 2, at.1);
    assert!(
        world.tile_walkable(yard.0, yard.1),
        "the tile {yard:?} is yard, not core"
    );
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(9_800);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            // **The tile's centre**, where a settled unit really stands - and where the marker is now drawn (D168).
            position: (
                Fixed::from_raw(i64::from(yard.0) * scale + scale / 2),
                Fixed::from_raw(i64::from(yard.1) * scale + scale / 2),
            ),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &map, 1);
    let solids = solids_of(&world, &snapshot);
    let camera = Camera::new((900, 700)).centred_on((f32::from(at.0) + 2.0, f32::from(at.1) + 2.0));
    let frame = render_with_solids(&snapshot, &camera, &solids);
    let (px, py) = camera.project((f32::from(yard.0), f32::from(yard.1)));
    let painted = frame.pixel(px.max(0) as u32, py.max(0) as u32);
    assert_ne!(
        painted,
        palette::BUILDING,
        "the unit in the yard is covered by the structure"
    );
    assert_ne!(
        painted,
        palette::BUILDING_YARD,
        "the unit in the yard is not drawn at all: the yard was painted over it"
    );
}
