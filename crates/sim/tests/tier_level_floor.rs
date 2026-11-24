//! **The level floor: a cluttered start still gets a chain** (D303, D344).
//!
//! `route_with` used to ask the hierarchy only when `descent(start).min(descent(goal)) >= 2` - the start's *4-tile* cell
//! had to be passable. On the acceptance map that refused 127 of 185 long walks (D338), so the tier answered 0 of 61,276
//! routes and everything landed on top of it (D341, D342, D343) was unread. The binary refuses nothing: `FUN_007ee980`
//! walks the levels down and floors at 0, and a failed mid-level attempt *falls through* to the low-level path
//! (`0xf98`: "Looking for a LOW LEVEL path (mid-level path failed)") - a fall-through, not a gate.
//!
//! **What this fixture pins.** `descent` is the coarsest **passable** cell (D297, re-read 2026-11-22 - see
//! `docs/archive/research/pathfinding-levels-evidence.md`). The scene puts one blocker in the start's 4-tile cell but outside its 2-tile
//! cell, so `descent(start) == 1`: under the old floor the flat search answers, under the new one the search runs at level
//! 1 and the chain's waypoints are crossings of **2-tile** boundaries (`n * 2 * scale`), which no tile-centre route can
//! produce.
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

/// 72 x 72 grass, a wood wall at x = 36 down to y = 47 (so the walk must go round its foot), and **one tree at (11, 43)**:
/// inside the start's level-2 cell (x 8..12, y 40..44) and outside its level-1 cell (x 8..10, y 40..42).
fn cluttered_start_map() -> String {
    let (width, height) = (72usize, 72usize);
    let mut text = format!("schema = 1\n[map]\nwidth = {width}\nheight = {height}\nterrain = [\n");
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            let forest = (x == 36 && y <= 47) || (x == 11 && y == 43);
            row.push(if forest { 'F' } else { 'G' });
        }
        text.push_str(&format!("  \"{row}\",\n"));
    }
    text.push_str("]\n[[map.resource]]\nkind = \"wood\"\namount = 100\nhp = 20\ntiles = [");
    for y in 0..=47 {
        text.push_str(&format!("[36,{y}],"));
    }
    // The single tree that clutters the start's 4-tile cell.
    text.push_str("[11,43],");
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
        ruage_data::load_map_str(text, "tier_level_floor.toml", &resources),
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

/// The same map as a bare ground, so the fixture can assert the descent it depends on without reaching into `World`.
fn ground_of(text: &str) -> Ground {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "tier_level_floor.toml", &resources),
    );
    let mut ground = Ground::from_map(&map);
    // What the world's seeding does to the same map: each resource stamps the tile it stands on. `Ground::from_map` carries
    // terrain only - a forest's *floor* is walkable and the tree is the obstacle - so a premise about the pyramid's cells
    // has to stamp them here.
    for node in &map.resources {
        ground.block(node.x, node.y);
    }
    ground
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

/// **A long walk whose 4-tile cell is cluttered is answered at level 1.**
#[test]
fn a_walk_from_a_cluttered_cell_is_answered_two_tiles_at_a_time() {
    // **The generated pack may be absent** (D8): `common::present` panics on a load error, and the pack is absent
    // until `cargo xtask gamedata` has run - so the fixture skips, as `tests/common/mod.rs`'s first rule says.
    if !repo("config/gamedata/units.toml").exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let mut world = world_of(&cluttered_start_map());
    let walker = a_villager(&mut world, 900, (8, 40));

    // The descent the gate reads: level 2 is blocked (the tree), level 1 is passable. This is the whole premise, so assert
    // it rather than trusting the scene. Reach 0 here: a villager's radius grows the blocked set by a fraction of a tile,
    // and the tree is 1.5 tiles from the nearest tile of the start's 2-tile cell, so it cannot move this answer.
    let ground = ground_of(&cluttered_start_map());
    let levels = Levels::of(&ground);
    assert_eq!(
        (levels.descent((8, 40)), levels.descent((64, 40))),
        (1, 3),
        "the start must descend to level 1 and the goal stay at 3, or this scene tests another rule"
    );

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (64, 40),
            target: None,
        })
        .expect("a walk round the forest");
    world.step().expect("the route is planned");

    let route = world.route_points(walker);
    assert!(!route.is_empty(), "the walker was given no route at all");
    let points: Vec<(i64, i64)> = route.iter().map(|(x, y)| (x.raw(), y.raw())).collect();
    assert!(
        points.len() > 1,
        "the wall forces a detour, so the answer must be a chain: {points:?}"
    );

    // Interior waypoints only: the last point is the exact place the order pointed, which is not a boundary.
    let interiors = &points[..points.len().saturating_sub(1)];
    let on_a_two_tile_boundary = interiors
        .iter()
        .any(|(x, y)| x % (2 * SCALE) == 0 || y % (2 * SCALE) == 0);
    assert!(
        on_a_two_tile_boundary,
        "the start descends to level 1, so the chain's crossings sit on 2-tile boundaries; every interior waypoint \
         here is a tile centre, which is the flat search answering: {points:?}"
    );
    assert!(
        points.iter().any(|(_, y)| *y >= 48 * SCALE),
        "the route must go south round the forest's foot: {points:?}"
    );
}
