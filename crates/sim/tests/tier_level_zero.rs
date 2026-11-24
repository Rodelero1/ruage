//! **The game's search at level 0** (D347): a walk whose 2-tile cell is culled still comes from the hierarchy.
//!
//! `FUN_007ee980` floors the descent at **0** and then *searches* there - `FUN_007ee760(0, ...)` opens the tile's own cell
//! and `FUN_007ed280` expands it four-directionally, the same coarse machine over 1-tile cells. Ours refuses `descent == 0`
//! and hands the walk to the octile flat search instead, which is a *different search with a different cost model* (D346's
//! named remainder). Following the game (owner, 2026-11-22) means level 0 is answered by the levelled search too.
//!
//! **How the shape says which tier answered.** The flat search's interior waypoints are tile **centres** (`n * scale +
//! scale/2`); the levelled answer's are crossings of cell boundaries, which sit on `n * scale` - at level 0 that is the tile
//! line - and `Ground::smooth` only removes them, never moves them. So one interior waypoint whose coordinate is an exact
//! multiple of the tile is an answer from the pyramid, and no tile-centre route can produce one.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Ground, Levels, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const SCALE: i64 = 20_000;

/// The walled scene of the other tier fixtures, with **one tree at (8, 41)**: inside the start's 2-tile cell (x 8..10,
/// y 40..42) and far enough from the mover's centre (one tile down) that he does not start inside its grown box.
fn cluttered_cell_map() -> String {
    let (width, height) = (72usize, 72usize);
    let mut text = format!("schema = 1\n[map]\nwidth = {width}\nheight = {height}\nterrain = [\n");
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let forest = (x == 36 && y <= 47) || (x == 8 && y == 41);
            row.push(if forest { 'F' } else { 'G' });
        }
        text.push_str(&format!("  \"{row}\",\n"));
    }
    text.push_str("]\n[[map.resource]]\nkind = \"wood\"\namount = 100\nhp = 20\ntiles = [");
    for y in 0..=47 {
        text.push_str(&format!("[36,{y}],"));
    }
    text.push_str("[8,41],");
    text.push_str("]\n");
    text.push_str("[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 70\ny = 70\n");
    text
}

fn resources() -> ruage_data::Resources {
    common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    )
}

fn world_of(text: &str) -> World {
    let resources = resources();
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "tier_level_zero.toml", &resources),
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

/// **A walk from a cluttered 2-tile cell is answered at level 0, not by the flat search.**
#[test]
fn a_walk_from_a_cluttered_cell_is_answered_at_level_zero() {
    if !repo("config/gamedata/units.toml").exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    // The premise, on a ground stamped the way seeding stamps it: the start's 2-tile cell holds the tree, so the descent
    // floors at 0; the goal is in the open and stays at 3.
    let resources = resources();
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(&cluttered_cell_map(), "tier_level_zero.toml", &resources),
    );
    let mut ground = Ground::from_map(&map);
    for node in &map.resources {
        ground.block(node.x, node.y);
    }
    let levels = Levels::of(&ground);
    assert_eq!(
        (levels.descent((8, 40)), levels.descent((64, 40))),
        (0, 3),
        "the start must floor at level 0 and the goal stay at 3, or this scene tests another rule"
    );

    let mut world = world_of(&cluttered_cell_map());
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
    let interiors = &points[..points.len().saturating_sub(1)];
    assert!(
        interiors
            .iter()
            .any(|(x, y)| x % SCALE == 0 || y % SCALE == 0),
        "the descent floors at 0 and the binary searches there, so the chain's crossings sit on tile lines; every \
         interior waypoint here is a tile centre, which is the flat search answering: {points:?}"
    );
    assert!(
        points.iter().any(|(_, y)| *y >= 48 * SCALE),
        "the route must still go south round the forest's foot: {points:?}"
    );
}
