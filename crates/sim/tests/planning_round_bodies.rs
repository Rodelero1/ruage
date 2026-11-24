//! **A body the mover already stands inside is still planned round** (P7, `docs/archive/research/parity-plan.md`).
//!
//! The original's search starts on the tile the mover stands on and puts **every stander the mover does not ignore** into it
//! (`FUN_007f9d80`): that bodies touch is the step pass's problem, not the plan's. Ours drops every body within a quarter tile of the mover in
//! both axes, so two bodies that overlap - which D290 allows, there is no press floor - are invisible to each other's routes, and the plan
//! certifies a leg the step then refuses.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const MAP: &str = "schema = 1\n[map]\nwidth = 48\nheight = 32\n";

/// The map with its terrain rows: `[[map.town_center]]` blocks must come **after** `terrain`, or TOML reads them as fields of the map.
fn open_map() -> String {
    let mut text = String::from(MAP);
    text.push_str("terrain = [\n");
    for _ in 0..32 {
        text.push_str("  \"");
        text.push_str(&"G".repeat(48));
        text.push_str("\",\n");
    }
    text.push_str("]\n                   [[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n                   [[map.town_center]]\nplayer = 2\nx = 46\ny = 30\n");
    text
}

fn world_of(text: &str) -> World {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "round_bodies.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must(
        "load_units_file",
        ruage_data::load_units_file(&repo("config/gamedata/units.toml")),
    );
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    world
}

/// A villager standing still at a raw position: no order, so his route is empty and the plan must treat him as an obstacle.
fn a_stander(world: &mut World, id: u32, at: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(at.0), Fixed::from_raw(at.1)),
            speed: Fixed::parse("0.8", world.scale()).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// **Two bodies that overlap are still two bodies.** Spawning them on top of each other is not the way this engine reaches that state -
/// `Command::Spawn`'s placement separates them (measured: a spawn at `410400` landed at `419600`), so the overlap has to be *walked into*,
/// as the game does it: both are sent to the same point (D289's rule - every man goes to the tile he was ordered to), where they press
/// together, and only then is one of them given a new walk. Its plan must bend round the other, not straight through him.
#[test]
fn a_body_the_mover_overlaps_is_still_planned_round() {
    let scale = 20_000i64;
    let at = |x: i64, y: i64| (x * scale + scale / 2, y * scale + scale / 2);
    let mut world = world_of(&open_map());
    let walker = a_stander(&mut world, 1_001, at(20, 20));
    let friend = a_stander(&mut world, 1_002, at(21, 20));

    // One point for both, which is how a crowd ends up pressed (D289).
    let meeting = (Fixed::from_raw(at(22, 20).0), Fixed::from_raw(at(22, 20).1));
    for unit in [walker, friend] {
        world
            .submit_checked(Command::WalkTo {
                unit,
                to: meeting,
                target: None,
            })
            .expect("a walk is ordered");
    }
    for _ in 0..400 {
        world.step().expect("a tick");
    }
    let (a, b) = (
        world.entity(walker).expect("the walker").position,
        world.entity(friend).expect("the friend").position,
    );
    let gap = ((a.0.raw() - b.0.raw()).abs()).max((a.1.raw() - b.1.raw()).abs());
    assert!(
        gap < 2_000,
        "the pair did not press together ({gap} raw apart), so the fixture measures nothing"
    );

    // Now send the walker past the friend's body and read what the plan did with him.
    world
        .submit_checked(Command::WalkTo {
            unit: walker,
            to: (
                Fixed::from_raw(a.0.raw() + 6 * scale),
                Fixed::from_raw(a.1.raw()),
            ),
            target: None,
        })
        .expect("a second walk is ordered");
    world.step().expect("the route is planned");

    let route = world.route_points(walker);
    assert!(!route.is_empty(), "the walker was given no route at all");
    assert!(
        route.len() > 1,
        "the route past a body the mover is standing inside is one straight leg ({} point(s), the friend {gap} raw away): a plan that \
         ignores him is a leg the step refuses",
        route.len()
    );
}
