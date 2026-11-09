//! **The picture's footprint is the collision footprint** (D158): if a building's diamond covers tiles it does not block, a villager
//! walking legally *beside* it is drawn on top of it - and if it blocks tiles its diamond does not cover, a villager disappears
//! "through" a wall. The owner's report (*"villager can pass through the built house"*) is exactly what either offset looks like.
//!
//! Written before any change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Snapshot, footprint_tiles};

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
    for y in 30..56i64 {
        for x in 8..56i64 {
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

/// **Every tile the diamond covers is solid, and every solid tile is covered** - for a house (2x2), a barracks (3x3) and a wall
/// (1x1), on the same ground, so an offset in either direction cannot hide.
#[test]
fn the_drawn_footprint_is_the_blocked_footprint() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    for (index, role) in ["house", "barracks", "palisade_wall"].iter().enumerate() {
        let unit = world.role_unit(role).expect("a role from the pack");
        let Some(at) = open_patch(&world, 8, 8) else {
            return;
        };
        let id = EntityId(9_500 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Building,
                unit: Some(unit),
                position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
                speed: Fixed::ZERO,
                hp: 600,
                amount: 0,
            })
            .expect("a building");
        world.step().expect("step");

        let snapshot = Snapshot::of(&world, &fx.map, 1);
        let view = snapshot
            .entities
            .iter()
            .find(|entity| entity.id == id.0)
            .expect("the building is in the snapshot");
        let anchor = (
            view.position.0.floor() as i32,
            view.position.1.floor() as i32,
        );
        let (min_x, min_y, max_x, max_y) = footprint_tiles(anchor, view.footprint);
        let mut covered = 0;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (x, y) = (x as u16, y as u16);
                assert!(
                    !world.tile_walkable(x, y),
                    "{role}: the diamond covers {x},{y} but the engine does not block it"
                );
                covered += 1;
            }
        }
        assert_eq!(
            covered,
            (view.footprint.0 * view.footprint.1) as i32,
            "{role}: the diamond covers its footprint"
        );
        // And the ring around it is still walkable: a 1-tile mistake would show here rather than as "a villager inside a house".
        for y in (min_y - 1)..=(max_y + 1) {
            for x in (min_x - 1)..=(max_x + 1) {
                let inside = x >= min_x && x <= max_x && y >= min_y && y <= max_y;
                if inside || x < 0 || y < 0 {
                    continue;
                }
                let (x, y) = (x as u16, y as u16);
                // The ring may hold a tree or another building of the map - only *this* building's own overflow is a bug, so a
                // tile that has nothing standing on it must be walkable.
                if !world.is_free_ground(x, y) {
                    continue;
                }
                assert!(
                    world.tile_walkable(x, y),
                    "{role}: {x},{y} is beside the diamond and must stay walkable"
                );
            }
        }
    }
}
