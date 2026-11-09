//! **A building is drawn on its own tiles and no others** (D159): the owner's hypothesis that the window and the headless run disagree.
//!
//! The headless tests prove the engine keeps a villager off a house's tiles (D158). If the *picture* paints the house's grey over a
//! neighbouring tile, then a villager standing legally beside it is drawn **inside** it - and the two runs "disagree" while the
//! simulation is identical. This test asks the only question that settles it: is the pixel at a neighbouring tile's centre still the
//! ground, or is it the building?
//!
//! Written before any change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render};

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

fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    // **Inside player 1's sight**: a tile in the fog is drawn as `UNSEEN`, and a test that asked about it would be asking about
    // the fog rather than about the house.
    for y in 10..26i64 {
        for x in 5..30i64 {
            let clear = (0..height).all(|dy| {
                (0..width).all(|dx| {
                    let (tx, ty) = ((x + dx) as u16, (y + dy) as u16);
                    world.tile_walkable(tx, ty) && world.is_free_ground(tx, ty)
                })
            });
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

/// A pixel that belongs to **that tile and no other**, and only while the tile is drawn on its own point.
///
/// The tile's diamond is centred on `project(x, y)` and its edge is where `|dx|/rx + |dy|/ry == 1`. So a point **45% of the way
/// to the upper-left vertex** is inside the tile (0.9 of the way to the edge) and *outside* it if the whole thing is drawn half a
/// tile down-and-right - which is exactly the offset this test exists to catch. It is also far from a role glyph, which sits on
/// the block's middle.
fn centre_of(camera: &Camera, tile: (u16, u16)) -> (u32, u32) {
    let (rx, ry) = camera.radii();
    let (x, y) = camera.project((f32::from(tile.0), f32::from(tile.1)));
    (
        (x as f32 - rx * 0.45).max(0.0) as u32,
        (y as f32 - ry * 0.45).max(0.0) as u32,
    )
}

/// **No neighbour of a house is painted as the house.** Every tile around a 2x2 house must still show ground at its centre - if the
/// diamond is drawn too wide, the tile in front of the house is grey and a villager standing there reads as "inside the house".
#[test]
fn a_house_paints_only_its_own_tiles() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let house_role = world.role_unit("house").expect("house");
    let Some(at) = open_patch(&world, 6, 4) else {
        return;
    };
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_600),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house_role),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = Camera::new((1280, 720)).centred_on((at.0 as f32 + 1.0, at.1 as f32 + 1.0));
    let frame = render(&snapshot, &camera);

    // The house covers the tiles its footprint names.
    let own: Vec<(u16, u16)> = (0..2)
        .flat_map(|dy| (0..2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| ((at.0 + dx) as u16, (at.1 + dy) as u16))
        .collect();
    for tile in &own {
        let (x, y) = centre_of(&camera, *tile);
        assert_eq!(
            frame.pixel(x, y),
            palette::BUILDING,
            "the house's own tile {tile:?} is painted as the house"
        );
    }

    // And **no** tile just outside it is.
    for dy in -1..=2i64 {
        for dx in -1..=2i64 {
            let (tx, ty) = (at.0 + dx, at.1 + dy);
            if tx < 0 || ty < 0 {
                continue;
            }
            let tile = (tx as u16, ty as u16);
            if own.contains(&tile) {
                continue;
            }
            let (x, y) = centre_of(&camera, tile);
            if x >= 1280 || y >= 720 {
                continue;
            }
            assert_ne!(
                frame.pixel(x, y),
                palette::BUILDING,
                "tile {tile:?} is beside the house but painted as the house - a villager there would look inside it"
            );
        }
    }
}
