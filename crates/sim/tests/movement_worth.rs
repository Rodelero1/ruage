//! Two probes that decide whether **B** (string-pull a route) and **C** (radius collision, distance arrival) are worth
//! writing. Ignored by default: they print numbers and assert nothing.
//!
//! ```text
//! cargo test --release -p ruage-sim --test movement_worth -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use ruage_sim::{EntityKind, Runner, Task, World, find_path, line_clear, load_scenario_file};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn project_map() -> Option<(ruage_data::Map, ruage_data::Resources)> {
    if !repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    Some((map, resources))
}

/// **B's worth**: how much walking distance would string-pulling save? A route is pulled greedily - from each node,
/// extend to the furthest node a clear line reaches - and the two lengths are compared. Slack the pather leaves on the
/// table is exactly what B would remove.
#[test]
#[ignore = "measurement, not a rule"]
fn how_much_would_string_pulling_save() {
    let Some((map, _)) = project_map() else {
        return;
    };
    let walk = ruage_sim::Walk::from_map(&map);

    let length = |nodes: &[(u16, u16)]| -> f64 {
        nodes
            .windows(2)
            .map(|pair| {
                let (dx, dy) = (
                    f64::from(pair[1].0) - f64::from(pair[0].0),
                    f64::from(pair[1].1) - f64::from(pair[0].1),
                );
                dx.hypot(dy)
            })
            .sum()
    };

    let mut rng: u64 = 0x5eed_1234_abcd_0001;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let mut pairs = 0u32;
    let mut raw_total = 0.0;
    let mut pulled_total = 0.0;
    let mut worst = 0.0f64;
    while pairs < 200 {
        let a = (
            u16::try_from(next() % u64::from(map.width)).unwrap_or(0),
            u16::try_from(next() % u64::from(map.height)).unwrap_or(0),
        );
        let b = (
            u16::try_from(next() % u64::from(map.width)).unwrap_or(0),
            u16::try_from(next() % u64::from(map.height)).unwrap_or(0),
        );
        if a == b || walk.blocked(a.0, a.1) || walk.blocked(b.0, b.1) {
            continue;
        }
        let Some(steps) = find_path(&walk, a, b) else {
            continue;
        };
        let mut route = vec![a];
        route.extend(steps.iter().copied());
        // Greedy pull: from the current node, jump to the furthest one a clear line reaches.
        let mut pulled = vec![a];
        let mut at = 0usize;
        while at + 1 < route.len() {
            let mut jump = route.len() - 1;
            while jump > at + 1 && !line_clear(&walk, route[at], route[jump]) {
                jump -= 1;
            }
            pulled.push(route[jump]);
            at = jump;
        }
        let (raw, tight) = (length(&route), length(&pulled));
        if tight > 0.0 {
            worst = worst.max((raw - tight) / tight);
        }
        raw_total += raw;
        pulled_total += tight;
        pairs += 1;
    }
    println!(
        "B: {pairs} routes - pather {raw_total:.1} tiles, string-pulled {pulled_total:.1} tiles, \
         saving {:.2}% (worst single route {:.1}%)",
        (raw_total - pulled_total) / raw_total * 100.0,
        worst * 100.0
    );
}

/// **C's worth**: bodies that are closer than the sum of their radii yet stand on **different tiles** - the overlap the
/// tile predicate cannot see - and same-tile pairs that do not actually touch, which the tile predicate refuses anyway.
#[test]
#[ignore = "measurement, not a rule"]
fn how_much_overlap_the_tile_rule_misses() {
    let Some((map, resources)) = project_map() else {
        return;
    };
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs");
    let effects =
        ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).expect("effects");
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs");
    let scenario = load_scenario_file(&repo("config/acceptance.toml")).expect("scenario");
    let _ = &resources;

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
                    .raw()
            })
            .unwrap_or(0)
    };

    let mut missed = 0u64; // touching, different tiles: the tile rule lets it through
    let mut refused = 0u64; // same tile, not touching: the tile rule refuses it anyway
    let mut touching = 0u64; // all touching pairs, for a denominator
    let mut ticks = 0u32;
    let mut worst_gap = f64::MAX;
    for _ in 0..36_000 {
        runner.due(&mut world);
        world.step().expect("step");
        ticks += 1;
        let bodies: Vec<(i64, i64, i64)> = world
            .entities()
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Unit
                    && !matches!(entity.task, Task::Garrisoned { .. })
                    && entity.hp > 0
            })
            .map(|entity| {
                (
                    entity.position.0.raw(),
                    entity.position.1.raw(),
                    radius_of(entity.variant.or(entity.unit)),
                )
            })
            .collect();
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let (ax, ay, ar) = bodies[i];
                let (bx, by, br) = bodies[j];
                let reach = ar + br;
                if reach <= 0 {
                    continue;
                }
                let (dx, dy) = (ax - bx, ay - by);
                let squared = dx * dx + dy * dy;
                let overlaps = squared < reach * reach;
                let same_tile = ax.div_euclid(scale) == bx.div_euclid(scale)
                    && ay.div_euclid(scale) == by.div_euclid(scale);
                if overlaps {
                    touching += 1;
                    if !same_tile {
                        missed += 1;
                        let gap = (reach as f64 - (squared as f64).sqrt()) / scale as f64;
                        worst_gap = worst_gap.min(gap);
                    }
                } else if same_tile {
                    refused += 1;
                }
            }
        }
    }
    println!(
        "C: over {ticks} ticks - {touching} touching pair-ticks; {missed} of them on different tiles \
         (the tile rule misses {:.1}%, deepest overlap {:.3} tiles); {refused} same-tile pair-ticks that do not touch \
         (the tile rule refuses those anyway)",
        if touching > 0 {
            missed as f64 / touching as f64 * 100.0
        } else {
            0.0
        },
        if worst_gap == f64::MAX {
            0.0
        } else {
            worst_gap
        }
    );
}
