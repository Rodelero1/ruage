//! A decision probe, not a rule: how much more does a **radius** collision bind than a **tile** one in a real match?
//!
//! D98a adopts the original's collision, which is radius-based at the new position. The engine's first slice is
//! tile-based because the pather is tile-based and the test is exact. The question this probe answers is whether that
//! staging is worth it: if a radius rule binds only slightly more often than a tile rule, the two are nearly the same
//! work and going straight to radii costs little; if it binds far more often, the deadlock question is real and the
//! staged path buys a diagnosis. Ignored: run with `--release -- --ignored --nocapture`.

use std::path::PathBuf;

use ruage_sim::{EntityKind, Runner, Task, World, load_scenario_file};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "measurement, not a rule"]
fn what_a_radius_rule_binds_that_a_tile_rule_does_not() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs");
    let effects =
        ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).expect("effects");
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs");
    let scenario = load_scenario_file(&repo("config/acceptance.toml")).expect("scenario");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.apply_policy(&scenario.policy);
    world.load_combat(&combat);
    world.load_research(&techs, &effects).expect("research");
    world.load_civs(&civs).expect("civs");
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runner");

    let scale = world.scale();
    let radius_of = |unit: Option<u32>| {
        unit.and_then(|id| units.unit(id))
            .map(|unit| {
                unit.radius[0]
                    .resolve(scale, "radius[0]")
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    let ticks = 36_000u32;
    let mut tile_pairs: u64 = 0;
    let mut radius_pairs: u64 = 0;
    // The discriminating cells of the 2x2: a radius rule that bites where a tile rule would not, and the reverse.
    let mut radius_only: u64 = 0;
    let mut tile_only: u64 = 0;
    let mut ticks_with_radius_binding: u64 = 0;
    let mut max_radius_pairs = 0u64;
    let mut unit_ticks: u64 = 0;

    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
        let bodies: Vec<(i64, i64, i64, i64)> = world
            .entities()
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Unit
                    && !matches!(entity.task, Task::Garrisoned { .. })
                    && entity.hp > 0
            })
            .map(|entity| {
                let (x, y) = (entity.position.0.raw(), entity.position.1.raw());
                let r = radius_of(entity.variant.or(entity.unit)).raw();
                (x, y, r, 0)
            })
            .collect();
        unit_ticks += bodies.len() as u64;
        let mut this_tick_radius = 0u64;
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let (ax, ay, ar, _) = bodies[i];
                let (bx, by, br, _) = bodies[j];
                let tile = (ax.div_euclid(scale), ay.div_euclid(scale))
                    == (bx.div_euclid(scale), by.div_euclid(scale));
                if tile {
                    tile_pairs += 1;
                }
                let reach = ar + br;
                let (dx, dy) = (ax - bx, ay - by);
                let overlaps = reach > 0 && dx * dx + dy * dy < reach * reach;
                if overlaps {
                    radius_pairs += 1;
                    this_tick_radius += 1;
                }
                match (tile, overlaps) {
                    (true, false) => tile_only += 1,
                    (false, true) => radius_only += 1,
                    _ => {}
                }
            }
        }
        if this_tick_radius > 0 {
            ticks_with_radius_binding += 1;
        }
        max_radius_pairs = max_radius_pairs.max(this_tick_radius);
    }

    println!(
        "{ticks} ticks, mean {:.1} bodies/tick",
        unit_ticks as f64 / f64::from(ticks)
    );
    println!(
        "pairs in the same tile (a tile rule binds): {tile_pairs} pair-ticks, {} ticks had any",
        tile_pairs.min(u64::from(ticks))
    );
    println!(
        "pairs within both radii (a radius rule binds): {radius_pairs} pair-ticks on {ticks_with_radius_binding} ticks, \
         max {max_radius_pairs} in one tick"
    );
    println!(
        "disagreements: a radius rule alone binds {radius_only} pair-ticks; a tile rule alone binds {tile_only}"
    );
    if tile_pairs > 0 {
        println!(
            "radius binds {:.1}x as often as tile",
            radius_pairs as f64 / tile_pairs as f64
        );
    }
}
