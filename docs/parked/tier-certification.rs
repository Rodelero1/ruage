// PARKED, NOT IN THE SUITE (D345). Copy back to `crates/sim/tests/` only after the tier answers with a route the flat
// search does not beat - see `docs/next-step.md`, "Make the chain competitive". While the level floor is 2 this scene is
// answered by the flat search and the fixture fails, which is correct: it asserts a route the engine does not produce.
//! **A body does not refuse the chain** (D345) - the game's coarse search is blind to bodies.
//!
//! `plan_path_with` hands the planner every body near the line (`World::standing_bodies`: the bodies that are *not* walking),
//! and the tier certified each leg of its chain against each of their grown boxes. But the engine walks through a body that
//! has stood still since the route was stamped (D320, *past*), so the veto refused exactly the bodies the step resolves -
//! and the binary's own coarse search inserts no body at all (`FUN_007ee980` calls no stander inserter; D338's crosscheck).
//! Measured on the acceptance map with the level floor at 1: of the 31 long walks that reached the coarse search, **17** had
//! a leg refused by a body and **none** answered (D344's funnel).
//!
//! The scene is the floor fixture's - a walk whose 4-tile cell holds a tree outside its 2-tile cell, so the search runs at
//! level 1 and its line runs along y = 40.5 - with one villager standing **in the corridor**, on the tile centre at
//! (20.5, 40.5) that the chain crosses. The chain must answer; today it is refused and the flat search goes round him.
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

/// 72 x 72 grass, a wood wall at x = 36 down to y = 47, and one tree at (11, 43) that puts the start at level 1 - the same
/// scene as `tier_level_floor`, so the chain's line is the straight run along y = 40.5 where the stander waits.
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
    text.push_str("[11,43],");
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
        ruage_data::load_map_str(text, "tier_certification.toml", &resources),
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

/// **A stander in the corridor does not refuse the chain.**
#[test]
fn a_stander_in_the_corridor_does_not_refuse_the_chain() {
    // **The generated pack may be absent** (D8): `common::present` panics on a load error, and the pack is absent
    // until `cargo xtask gamedata` has run - so the fixture skips, as `tests/common/mod.rs`'s first rule says.
    if !repo("config/gamedata/units.toml").exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    // The premise, on a ground stamped the way the world's seeding stamps it: the start descends to level 1, the goal to 3.
    let resources = resources();
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(
            &cluttered_start_map(),
            "tier_certification.toml",
            &resources,
        ),
    );
    let mut ground = Ground::from_map(&map);
    for node in &map.resources {
        ground.block(node.x, node.y);
    }
    let levels = Levels::of(&ground);
    assert_eq!(
        (levels.descent((8, 40)), levels.descent((64, 40))),
        (1, 3),
        "the scene must put the search at level 1, or its line is not the run at y = 40.5"
    );

    let mut world = world_of(&cluttered_start_map());
    // The stander: dead centre of tile (20, 40), which is where the chain's line at y = 40.5 crosses that tile.
    let villager = world.role_unit("villager").expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(901),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(410_000), Fixed::from_raw(810_000)),
            speed: Fixed::parse("0.8", world.scale()).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("the stander");
    world.step().expect("the stander lands");
    assert_eq!(
        world.path_length(EntityId(901)),
        0,
        "the stander must have no route, or he is not a stander"
    );

    let walker = a_villager(&mut world, 902, (8, 40));
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
            .any(|(x, y)| x % (2 * SCALE) == 0 || y % (2 * SCALE) == 0),
        "a body in the corridor must not veto a chain - the game's coarse search is blind to bodies, and the step walks \
         through a stander once he is past (D320); every interior waypoint here is a tile centre, which is the flat \
         search answering: {points:?}"
    );
}
