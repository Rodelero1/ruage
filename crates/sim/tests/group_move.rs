//! Group movement: an order to one tile is an order to a *place*, and a crowd spreads out.
//!
//! Written before the implementation (D7). Two units ordered to the same tile cannot both stand on it, and
//! the naive answer - all of them walk to it and queue - reads as a broken army: they bunch on one approach
//! and the ones behind never arrive. The order is resolved when it is given: each unit of a crowd is given
//! its own tile near the one that was clicked.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

fn place(world: &mut World, fx: &Fixture, player: u8, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let spearman = fx.economy.role["spearman"];
    let speed = fx
        .units
        .unit(spearman)
        .expect("in the table")
        .speed
        .resolve(scale, "speed")
        .expect("a walker");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(spearman),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed,
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity exists");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

#[test]
fn a_crowd_ordered_to_one_tile_all_arrive_and_spread() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let mut squad = Vec::new();
    for (n, tile) in [(16i64, 20i64), (16, 22), (16, 24), (18, 20), (18, 22)]
        .into_iter()
        .enumerate()
    {
        squad.push(place(&mut world, &fx, 1, 9_800 + n as u32, tile));
    }

    for unit in &squad {
        world
            .submit_checked(Command::Move {
                unit: *unit,
                to: (22, 22),
            })
            .expect("move");
    }
    for _ in 0..800 {
        world.step().expect("step");
    }

    let mut tiles: Vec<(i64, i64)> = squad.iter().map(|id| tile_of(&world, *id)).collect();
    tiles.sort();
    let before = tiles.len();
    tiles.dedup();
    assert_eq!(
        tiles.len(),
        before,
        "two units of one order share a tile: {tiles:?}"
    );

    for tile in &tiles {
        let distance = (tile.0 - 22).abs().max((tile.1 - 22).abs());
        assert!(
            distance <= 2,
            "a unit of the order ended {distance} tiles from the place it was sent: {tile:?}"
        );
    }
}
