//! **The original smooths the levelled route before the unit walks it** (D346).
//!
//! `move_obj.cpp` prints the mid-level path (`"        Found a valid mid-level path with %d waypoints"`, `0xf56`), calls
//! `FUN_008acf20`, and prints it again (`"          After smoothing, path has %d waypoints"`, `0xf69`). That function walks
//! the polyline with two indices - the **last kept** waypoint and a candidate one further on - and asks `FUN_00840c60`
//! (the radius-gown segment test, the query `+0x268` inset belongs to) whether the straight segment between the two is
//! walkable. Clear: the waypoint in between is deleted (`FUN_007feb80(param_1 + 1)`) and the same candidate is tried again.
//! Blocked: the waypoint is kept and both indices advance. So what the unit is handed is the polyline's **corners**, not
//! every cell-edge crossing.
//!
//! Ours hands over every crossing: measured on the acceptance map, 12.3 waypoints a walk against the flat search's 4.8 and
//! 1.6 tiles of extra walking (D345). This fixture pins the missing pass on a scene where the raw chain is long and mostly
//! straight.
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

const SCALE: i64 = 20_000;

/// 72 x 72 grass with a wood wall at x = 36 down to y = 47: the walk from (8, 40) to (64, 40) has to go south round its
/// foot, so the flat search cannot answer with one point and the chain is a real one - and both ends are in the open, so
/// the search runs at level 3 and the crossings are 8 tiles apart.
fn walled_map() -> String {
    let (width, height) = (72usize, 72usize);
    let mut text = format!("schema = 1\n[map]\nwidth = {width}\nheight = {height}\nterrain = [\n");
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            row.push(if x == 36 && y <= 47 { 'F' } else { 'G' });
        }
        text.push_str(&format!("  \"{row}\",\n"));
    }
    text.push_str("]\n[[map.resource]]\nkind = \"wood\"\namount = 100\nhp = 20\ntiles = [");
    for y in 0..=47 {
        text.push_str(&format!("[36,{y}],"));
    }
    text.push_str("]\n");
    text.push_str("[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 70\ny = 70\n");
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
        ruage_data::load_map_str(text, "tier_smoothing.toml", &resources),
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
    let at = (
        Fixed::from_raw(tile.0 * SCALE + SCALE / 2),
        Fixed::from_raw(tile.1 * SCALE + SCALE / 2),
    );
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: at,
            speed: Fixed::parse("0.8", world.scale()).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// **The chain is handed over smoothed: only its corners keep a waypoint.**
#[test]
fn the_levelled_route_is_smoothed_before_it_is_handed_over() {
    if !repo("config/gamedata/units.toml").exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let mut world = world_of(&walled_map());
    let walker = a_villager(&mut world, 900, (8, 40));

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (64, 40),
            target: None,
        })
        .expect("a walk round the forest");
    world.step().expect("the route is planned");

    let points: Vec<(i64, i64)> = world
        .route_points(walker)
        .iter()
        .map(|(x, y)| (x.raw(), y.raw()))
        .collect();
    assert!(!points.is_empty(), "the walker was given no route at all");
    // The route is the hierarchy's (the wall is in the way, the walk is 56 tiles), and smoothing keeps only what the
    // straight-line test refuses to skip: the two corners round the forest's foot plus the ends. The raw chain this scene
    // produces is the level-3 crossings, one every eight tiles each way.
    assert!(
        points.len() <= 6,
        "the hierarchy's polyline must be smoothed to its corners - the raw chain here is 10+ crossings; got {}: {points:?}",
        points.len()
    );
    assert!(
        points.iter().any(|(_, y)| *y >= 48 * SCALE),
        "the route must still go south round the forest's foot: {points:?}"
    );
}
