//! **The levelled tier runs when bodies are about, and ignores them** (P14, sequence step 3).
//!
//! The binary's coarse search is the land unit's default and its body holds **no stander insertion** (`FUN_007ee980`
//! calls `FUN_007fd880`, `FUN_007ed5c0`, `FUN_007f0be0`, `FUN_007ede40`, `FUN_007edb80`, `FUN_007eeda0`, `FUN_007ee760`,
//! `FUN_007ee600`, and never `FUN_007f9d80`); openage's note says the same in words - the mip-map *"ignores moving objects,
//! e.g. units or wildlife"*. Ours did the opposite: `route_with` only consulted the pyramid when `avoid.is_empty()`, and every
//! long walk in a busy match carries a stander, so the tier answered **0 of 121** long walks (D334).
//!
//! This fixture is the witness the register owed and the conformance row could not be: it goes through `World::plan_path`, so
//! the gate is on its path, and the route it asks for is long and carries a body in the plan's own avoid list.
//!
//! **How the shape says which tier answered.** The flat search's interior waypoints are **tile centres** (`n * scale + scale/2`,
//! by construction in `route_with`); the hierarchy's are the **crossings of cell edges**, which sit on a tile boundary
//! (`n * scale`) - the refinement in `levels.rs` puts them there deliberately (D322). So one waypoint with a coordinate that is
//! an exact multiple of the scale is a route that came out of the pyramid, and no tile-centre route can produce one.
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

/// 72 x 72 open grass with a **wood at x = 36 down to y = 47**: the walk from (8, 40) to (64, 40) must go south round its
/// foot, which is a detour of eleven tiles - long enough for the hierarchy, and with an obstacle the coarse cells see (a tree
/// blocks every cell that holds it, `FUN_007f1100`) so the chain is a real one and not the straight line. The wood is a
/// `[[map.resource]]` because the terrain under a tree is walkable: what blocks is the tree.
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
        ruage_data::load_map_str(text, "tier_ignores_bodies.toml", &resources),
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

fn tile_centre(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

/// A villager at a tile centre: `reach`-sized, standing unless it is ordered to walk.
fn a_villager(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let at = tile_centre(tile.0, tile.1);
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

/// **A long walk with a stander near it is answered by the hierarchy.** The stander stands off the corridor - far enough
/// that the coarse chain's legs are clear of him, near enough that the plan's own avoid list is not empty, which is exactly
/// the state the `avoid.is_empty()` gate refused.
#[test]
fn a_long_walk_past_a_stander_comes_from_the_hierarchy() {
    let mut world = world_of(&walled_map());
    let idle = a_villager(&mut world, 900, (20, 41));
    assert_eq!(
        world.path_length(idle),
        0,
        "the stander must have no route, or he is not a stander"
    );
    let walker = a_villager(&mut world, 901, (8, 40));

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
    // A one-point route is the straight-line answer, not a tier's: say that rather than blaming the flat search for an
    // interior that does not exist.
    assert!(
        points.len() > 1,
        "the corridor is blocked, so the answer must be a chain, not one waypoint: {points:?}"
    );
    // Every interior waypoint but the last, which is the exact place the order pointed.
    let interior = &points[..points.len().saturating_sub(1)];
    let on_a_boundary = interior
        .iter()
        .any(|(x, y)| x % SCALE == 0 || y % SCALE == 0);
    assert!(
        on_a_boundary,
        "a long walk with one stander near it must come from the hierarchy, whose waypoints are cell-edge crossings; \
         every interior waypoint here is a tile centre, so the flat search answered: {points:?}"
    );
    assert!(
        points.iter().any(|(_, y)| *y >= 48 * SCALE),
        "the route must go south round the forest's foot: {points:?}"
    );
}
