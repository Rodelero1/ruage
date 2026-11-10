//! **No unit's body may overlap a blocked tile** (D176).
//!
//! The owner: *"units still collide with the impassable building diamonds (the ellipse cannot overlap with it at all, not just center of it)."*
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
use ruage_sim::{EntityId, World};

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
        ruage_sim::load_scenario_file(&repo("config/skirmish.toml")),
    );
    let mut runner = common::must(
        "Runner::new",
        ruage_sim::Runner::new(&scenario, world.tick_rate_hz()),
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
        "unit {} stands with its body {:.2} tiles inside a blocked tile - the ellipse overlaps the diamond",
        -(clearance as f64) / scale as f64,
        id.0
    );
    let _ = Fixed::ZERO;
}
