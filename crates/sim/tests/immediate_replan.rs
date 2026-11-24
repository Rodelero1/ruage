//! **Ground that goes stale is re-planned on the tick it is discovered** (P4, `docs/archive/research/parity-plan.md`).
//!
//! The original's epilogue re-plans on **every** terrain collision (`0x122f`, `0x128b`: *"We have %d non-past collisions and %d terrain
//! collisions, repathing"*). Ours throttles it to once every `REPLAN_TICKS` (20) after the route was written, so a wall that appears in a
//! walker's lane leaves him standing for up to a second before anything looks for another way.
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

fn map_text() -> String {
    let mut text = String::from("schema = 1\n[map]\nwidth = 40\nheight = 24\nterrain = [\n");
    for _ in 0..24 {
        text.push_str("  \"");
        text.push_str(&"G".repeat(40));
        text.push_str("\",\n");
    }
    text.push_str("]\n[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 38\ny = 22\n");
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
        ruage_data::load_map_str(text, "immediate_replan.toml", &resources),
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

fn a_villager(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(tile.0 * scale + scale / 2),
                Fixed::from_raw(tile.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// **A wall that appears in a walker's lane costs him one tick, not twenty.** He is ordered ten tiles east on open ground, the route is one
/// straight leg, and two ticks into the walk a palisade segment lands squarely in the lane. The original re-plans on that very collision; the
/// claim here is that the route is rebuilt and bends round the wall within **three** ticks of it appearing.
#[test]
fn a_wall_in_the_lane_is_replanned_the_tick_it_is_seen() {
    let scale = 20_000i64;
    let mut world = world_of(&map_text());
    let walker = a_villager(&mut world, 1_001, (10, 12));
    let goal = (
        Fixed::from_raw(20 * scale + scale / 2),
        Fixed::from_raw(12 * scale + scale / 2),
    );
    world
        .submit_checked(Command::WalkTo {
            unit: walker,
            to: goal,
            target: None,
        })
        .expect("a walk is ordered");
    world.step().expect("the walk starts");
    assert_eq!(
        world.route_points(walker).len(),
        1,
        "open ground must answer with one leg, or this fixture is not testing a stale route"
    );

    // Two ticks in, put a wall across the lane.
    for tick in 0..2 {
        world.step().expect("a tick");
        assert!(tick < 2);
    }
    let palisade = world.role_unit("palisade_wall").expect("a palisade role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_001),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(palisade),
            position: (Fixed::from_raw(11 * scale), Fixed::from_raw(12 * scale)),
            speed: Fixed::ZERO,
            hp: 9_900,
            amount: 0,
        })
        .expect("a wall segment lands");
    // **Walk until he is refused, one tick at a time**, and ask what the plan did on the tick of the refusal.
    let mut stopped = None;
    let mut previous = world.entity(walker).expect("the walker").position;
    for tick in 0..40 {
        let before = world.repaths();
        world.step().expect("a tick");
        let now = world.entity(walker).expect("the walker").position;
        if now == previous {
            stopped = Some((tick, before, world.repaths()));
            break;
        }
        previous = now;
    }
    let Some((tick, before, after)) = stopped else {
        panic!("the wall never stopped him: the fixture measures nothing");
    };
    assert!(
        after > before,
        "on tick {tick} ground refused him and the route was not re-planned ({before} -> {after} re-paths): a stale route is a man \
         standing still while the search he needs waits out a timer"
    );
}
