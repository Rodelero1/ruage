//! **No unit's body may overlap a blocked tile** (D176).
//!
//! The owner, twice. First: *"units still collide with the impassable building diamonds (the ellipse cannot overlap with it at all, not just
//! center of it)"*. Then, after the ring had been shrunk to the body, with a screenshot of a villager on the town centre's light yard whose
//! ellipse crosses the dark core's edge: *"I see clear overlap."*
//!
//! Our ground test asks only whether the unit's **centre** lands on a blocked tile, so a body of radius 0.2 can sit half inside a wall.
//!
//! The original's rule, from the Forgotten Empires pathfinding talk, is *"shrink the unit to a point, expand every other obstruction"* - a disc
//! of radius `r` against a rectangle is that rectangle grown by `r`. Stated for our engine: **the distance from a unit's centre to the nearest
//! blocked tile is never less than its own radius.**
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

/// **Every unit, after a busy run, has its body clear of every blocked tile.** The village's own map, the skirmish script for two thousand
/// ticks: villagers crowd round trees and mines, soldiers walk past the town centre, and nothing may end up with its disc inside a wall.
#[test]
fn no_unit_ever_stands_with_its_body_inside_a_wall() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::present(&repo("config/map.toml"), |path| {
        ruage_data::load_map_file(path, &resources)
    });
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_path));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    let scenario = common::must(
        "load_skirmish",
        ruage_script::load_scenario_file(&repo("config/skirmish.toml")),
    );
    let mut runner = common::must(
        "Runner::new",
        ruage_script::Runner::new(&scenario, world.tick_rate_hz()),
    );

    for _ in 0..2_000 {
        runner.due(&mut world);
        world.step().expect("a tick");
    }

    let scale = world.scale();
    let mut worst: Option<(i64, EntityId)> = None;
    for entity in world.entities() {
        if !matches!(entity.kind, ruage_sim::EntityKind::Unit) || entity.hp <= 0 {
            continue;
        }
        let clearance = world.body_clearance(entity.id);
        if worst.is_none_or(|(seen, _)| clearance < seen) {
            worst = Some((clearance, entity.id));
        }
    }
    let Some((clearance, id)) = worst else {
        return;
    };
    assert!(
        clearance >= 0,
        // **The arguments were swapped**, so the message read "unit 0.00005 ... 9 tiles inside": the fraction went where the
        // id belongs. It said the opposite of the truth for as long as it existed (found 2026-11-13 while measuring a
        // 1-raw-unit penetration that the sentence hid).
        "unit {} stands with its body {:.5} tiles inside a blocked tile - the ellipse overlaps the diamond",
        id.0,
        -(clearance as f64) / scale as f64
    );
    let _ = Fixed::ZERO;
}

/// **Ordered to the tile touching the town centre's solid core, the villager's body stays clear of it - on every tick.**
///
/// The owner's screenshot is this exact spot: standing on the light yard, ellipse across the dark edge.
#[test]
fn a_villager_walking_beside_a_building_keeps_its_body_clear() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::present(&repo("config/map.toml"), |path| {
        ruage_data::load_map_file(path, &resources)
    });
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_path));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));

    let scale = world.scale();
    // The town centre's **solid core**, not its walkable yard: the first blocked tile of a building's footprint.
    let Some(at) = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.unit.is_some())
        .and_then(|entity| {
            let (x, y) = world.tile_of_position(entity.position);
            (x..x + 4)
                .flat_map(|tx| (y..y + 4).map(move |ty| (tx, ty)))
                .find(|(tx, ty)| !world.tile_walkable(*tx, *ty))
        })
    else {
        return;
    };
    let villager = world.role_unit("villager").expect("villager");
    let start = (i64::from(at.0) - 3, i64::from(at.1));
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_800),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(start.0 * scale + scale / 2),
                Fixed::from_raw(start.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Move {
            unit: EntityId(9_800),
            to: (at.0, at.1),
        })
        .expect("a move at the core");

    // **Every tick**, not just the last: an overlap for one frame is an overlap the player sees.
    let mut worst = i64::MAX;
    for _ in 0..300 {
        world.step().expect("a tick");
        worst = worst.min(world.body_clearance(EntityId(9_800)));
    }
    assert!(
        worst >= 0,
        "the villager's body was {:.2} tiles inside the town centre's blocked tile at some point - the ellipse overlaps the diamond",
        -(worst as f64) / scale as f64
    );
}
