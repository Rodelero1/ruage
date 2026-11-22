//! **Why do bodies end up in one spot?** A decision probe, not a rule.
//!
//! The owner, 2026-11-22: *"villagers stack on each other a lot (like 5 in the same spot)"*. `collision_probe` sizes the pile
//! (11 bodies in one group, 317,041 pairs closer than a fifth of a tile) and shows only **877** of 349,824 overlapping
//! pair-ticks are *new* on a tick - so the pile is a state the engine settles into, entered rarely. This probe asks the next
//! question: **which exemption let the mover land there**, read at the tick the overlap appears, and not at the end of the
//! tick like the census (an exemption can lapse inside a tick).
//!
//! The arms are the ones the movement pass actually has (`motion.rs`): a **moving mate** on both routes' lists (the game's
//! path-ignore list, `FUN_007fe010`), a **past** collider (`path->createdAt >= other->lastMoveTime`), and **neither** - which
//! would be a defect, since the step should have refused. Measured 2026-11-22 at `ed064fb`: **mate 0, past 500, neither
//! 164**, and every offender is a `Gather`/`Return` pair - men walking the same route to the same place, arriving one after
//! another.
//!
//! Ignored: `--release -- --ignored --nocapture`.

use std::path::PathBuf;

use ruage_script::{Runner, load_scenario_file};
use ruage_sim::{EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "measurement, not a rule"]
fn which_exemption_lets_a_man_land_on_a_friend() {
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

    // Per pair: was it apart last tick? A pair that becomes overlapping is one the rule let through.
    let mut was_apart: std::collections::BTreeMap<(u32, u32), bool> =
        std::collections::BTreeMap::new();
    let mut before: std::collections::BTreeMap<u32, (i64, i64)> = std::collections::BTreeMap::new();
    let (mut landed_mate, mut landed_past, mut landed_neither) = (0u64, 0u64, 0u64);
    let mut printed = 0u64;
    let mut by_task: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();

    let say = |task: Task| {
        let text = format!("{task:?}");
        text.split([' ', '{'])
            .next()
            .unwrap_or("?")
            .trim()
            .to_owned()
    };

    for _ in 0..36_000u32 {
        runner.due(&mut world);
        world.step().expect("step");
        let bodies: Vec<(u32, i64, i64, i64)> = world
            .entities()
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Unit
                    && !matches!(entity.task, Task::Garrisoned { .. })
                    && entity.hp > 0
            })
            .map(|entity| {
                (
                    entity.id.0,
                    entity.position.0.raw(),
                    entity.position.1.raw(),
                    radius_of(entity.variant.or(entity.unit)).raw(),
                )
            })
            .collect();
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                let (a_id, ax, ay, ar) = bodies[i];
                let (b_id, bx, by, br) = bodies[j];
                let reach = if ar + br > 0 { ar + br } else { scale };
                let (dx, dy) = (ax - bx, ay - by);
                let apart = (dx * dx + dy * dy).isqrt();
                let key = (a_id.min(b_id), a_id.max(b_id));
                let newly = apart < reach && was_apart.get(&key).copied().unwrap_or(true);
                was_apart.insert(key, apart >= reach);
                if !newly {
                    continue;
                }
                // **Who moved into whom?** The mover is the one whose position changed this tick; when both did, the
                // exemption is read in the mover's direction and the other direction is printed beside it.
                let moved =
                    |id: u32,
                     x: i64,
                     y: i64,
                     before: &std::collections::BTreeMap<u32, (i64, i64)>| {
                        before.get(&id).is_some_and(|(px, py)| (*px, *py) != (x, y))
                    };
                let choice = if moved(a_id, ax, ay, &before) && !moved(b_id, bx, by, &before) {
                    Some((a_id, b_id))
                } else if moved(b_id, bx, by, &before) && !moved(a_id, ax, ay, &before) {
                    Some((b_id, a_id))
                } else {
                    None
                };
                if let Some((mover, other)) = choice {
                    let (one, two) = (EntityId(mover), EntityId(other));
                    let arm = if world.is_mate(one, two) {
                        landed_mate += 1;
                        "mate"
                    } else if world.is_past(one, two) {
                        landed_past += 1;
                        "past"
                    } else {
                        landed_neither += 1;
                        "NEITHER"
                    };
                    if arm == "NEITHER" {
                        let tasks = |id: EntityId| {
                            world
                                .entity(id)
                                .map(|entity| say(entity.task))
                                .unwrap_or_else(|| "gone".to_owned())
                        };
                        *by_task
                            .entry(format!("{} into {}", tasks(one), tasks(two)))
                            .or_default() += 1;
                    }
                    if printed < 24 {
                        printed += 1;
                        let tell = |id: EntityId| {
                            let entity = world.entity(id).expect("a body");
                            format!(
                                "{} {:?} route={} pause={} last_moved={}",
                                if entity.kind == EntityKind::Unit {
                                    "unit"
                                } else {
                                    "?"
                                },
                                say(entity.task),
                                world.route_points(id).len(),
                                entity.pause_remaining,
                                entity.last_moved
                            )
                        };
                        let (mx, my) = (
                            world.position(one).expect("a"),
                            world.position(two).expect("b"),
                        );
                        println!(
                            "LANDING {arm}: {mover} into {other}, apart={apart} reach={reach} dx={} dy={} \
                             at=({},{}) past(mover->other)={} past(other->mover)={} mate={} reach_between={}\n  mover: {}\n  other: {}",
                            mx.0.raw() - my.0.raw(),
                            mx.1.raw() - my.1.raw(),
                            mx.0.raw(),
                            mx.1.raw(),
                            world.is_past(one, two),
                            world.is_past(two, one),
                            world.is_mate(one, two),
                            world.reach_between(one, two).raw(),
                            tell(one),
                            tell(two)
                        );
                    }
                }
            }
        }
        before = bodies.iter().map(|(id, x, y, _)| (*id, (*x, *y))).collect();
    }

    println!(
        "36,000 ticks: **new overlaps whose mover can be named** - mate {landed_mate}, past {landed_past}, \
         **neither {landed_neither}**"
    );
    println!("the NEITHER landings by task pair:");
    for (pair, count) in by_task {
        println!("  {count:>6}  {pair}");
    }
}
