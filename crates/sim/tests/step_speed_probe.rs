//! **Probe, not a rule** (D7/D209): buckets every villager's per-tick displacement, to answer one question before a fixture is
//! written for it - does this engine move a villager in whole steps, or in fractions of one?
//!
//! The owner's rule (2026-11-17): *"villagers never slow down ... they may stop or stop to repath ... so either they walk, or
//! stand."* A step is `speed / tick_rate` raw units. Buckets, per tick, per villager:
//!
//! * **stand** - displacement 0;
//! * **walk** - displacement within one raw unit of the step;
//! * **partial** - 0 < displacement < step - 1: a slow-down;
//! * **over** - displacement > step + 1: faster than the unit's own speed.
//!
//! `--ignored --nocapture`, prints one table and asserts nothing.

use std::collections::BTreeMap;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn pack() -> Option<ruage_data::ContentPack> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(ruage_data::ContentPack::load(&repo("config")).expect("the pack"))
}

fn scene(pack: &ruage_data::ContentPack) -> Option<(World, Vec<EntityId>)> {
    let mut economy = pack.economy.clone();
    economy.start.civs = vec!["British".to_owned()];
    economy.start.stock = [("food".to_owned(), 2_000), ("wood".to_owned(), 2_000)]
        .into_iter()
        .collect();
    let mut world = World::new(&pack.sim, 7).expect("world");
    world
        .seed_from(&pack.map, &economy, &pack.units)
        .expect("seeding");
    world
        .load_resources(&pack.resources)
        .expect("the pack fits");
    world
        .load_research(&pack.techs, &pack.effects)
        .expect("research");
    world.load_civs(&pack.civs).expect("civs");
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    let nodes: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .take(4)
        .collect();
    let mut workers = Vec::new();
    for (nth, node) in nodes.iter().enumerate() {
        let id = EntityId(9_000 + u32::try_from(nth).unwrap_or(0));
        let (tx, ty) = world.resource_tiles(*node)[0];
        let stand = (1u16..=3)
            .flat_map(|d| {
                [
                    (tx + d, ty),
                    (tx, ty + d),
                    (tx.saturating_sub(d), ty),
                    (tx, ty.saturating_sub(d)),
                ]
            })
            .find(|(x, y)| world.is_free_ground(*x, *y))?;
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(i64::from(stand.0) * scale + scale / 2),
                    Fixed::from_raw(i64::from(stand.1) * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("lands");
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: *node,
            })
            .expect("a gather order");
        workers.push(id);
    }
    // **One long walk as well**, so a villager with nothing to do but travel is in the sample.
    let walker = EntityId(9_600);
    let (wx, wy) = (30u16, 30u16);
    world
        .submit_checked(Command::Spawn {
            id: walker,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(5 * scale + scale / 2),
                Fixed::from_raw(5 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a walker");
    world.step().expect("lands");
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (wx, wy),
        })
        .expect("a walk order");
    workers.push(walker);
    Some((world, workers))
}

#[test]
#[ignore = "probe: buckets per-tick displacement, asserts nothing"]
fn how_a_villager_moves() {
    let Some(pack) = pack() else { return };
    let Some((mut world, workers)) = scene(&pack) else {
        return;
    };
    world.trace_steps();
    let scale = world.scale();
    let mut last: BTreeMap<EntityId, (Fixed, Fixed)> = BTreeMap::new();
    let mut buckets: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut partial_sizes: BTreeMap<i64, u64> = BTreeMap::new();
    let mut partial_task: BTreeMap<String, u64> = BTreeMap::new();
    let mut walk_dirs: BTreeMap<&'static str, u64> = BTreeMap::new();
    for id in &workers {
        if let Some(pos) = world.position(*id) {
            last.insert(*id, pos);
        }
    }
    for _ in 0..4_000 {
        world.step().expect("a tick");
        for id in &workers {
            let Some(here) = world.position(*id) else {
                continue;
            };
            let Some(previous) = last.insert(*id, here) else {
                continue;
            };
            let dx = here.0.raw() - previous.0.raw();
            let dy = here.1.raw() - previous.1.raw();
            let distance = ((dx * dx + dy * dy) as f64).sqrt();
            let step = world.entity(*id).map_or(0.0, |entity| {
                entity.speed.raw() as f64 / f64::from(world.tick_rate_hz().max(1))
            });
            if step == 0.0 {
                continue;
            }
            let budget = if distance == 0.0 {
                *buckets.entry("stand").or_default() += 1;
                continue;
            } else if (distance - step).abs() <= 1.0 {
                *buckets.entry("walk").or_default() += 1;
                // A whole step, but is it one step or a slide with both components?
                if dx != 0 && dy != 0 {
                    *walk_dirs.entry("diagonal").or_default() += 1;
                } else {
                    *walk_dirs.entry("axis").or_default() += 1;
                }
                continue;
            } else if distance < step {
                "partial"
            } else {
                "over"
            };
            *buckets.entry(budget).or_default() += 1;
            if budget == "partial" {
                if *buckets.get("partial").unwrap_or(&0) <= 20 {
                    let (task, decision) =
                        world
                            .entity(*id)
                            .map_or(("gone".to_owned(), "?".to_owned()), |entity| {
                                (
                                    format!("{:?}", entity.task),
                                    format!("{:?}", entity.last_step),
                                )
                            });
                    // **Did he land on a tile centre?** A waypoint is a tile's middle, so a position whose raw coordinates
                    // are `k*scale + scale/2` on both axes is where a leg ended - the "land exactly" rule, not a slide.
                    let half = i64::from(world.scale()) / 2;
                    let on_centre = |v: i64| (v - half).rem_euclid(i64::from(world.scale())) == 0;
                    println!(
                        "    partial at tick {} id {} d=({dx}, {dy}) step={step:.0} n={:.3} at=({}, {}) mods=({}, {}) centre={} {task} {decision} path_len={}",
                        world.tick(),
                        id.0,
                        distance / step,
                        here.0.raw(),
                        here.1.raw(),
                        here.0.raw().rem_euclid(i64::from(world.scale())),
                        here.1.raw().rem_euclid(i64::from(world.scale())),
                        on_centre(here.0.raw()) && on_centre(here.1.raw()),
                        world.path_length(*id)
                    );
                }
                *partial_sizes
                    .entry((distance / step * 1000.0).round() as i64)
                    .or_default() += 1;
                let task =
                    world
                        .entity(*id)
                        .map_or("gone".to_owned(), |entity| match entity.task {
                            Task::Gather { progress, .. } if progress.raw() > 0 => {
                                "Gather(working)".to_owned()
                            }
                            Task::Gather { .. } => "Gather(walking)".to_owned(),
                            Task::Return { .. } => "Return".to_owned(),
                            Task::Build { .. } => "Build".to_owned(),
                            Task::Move { .. } => "Move".to_owned(),
                            Task::Idle => "Idle".to_owned(),
                            _ => format!("{:?}", std::mem::discriminant(&entity.task)),
                        });
                *partial_task.entry(task).or_default() += 1;
            }
        }
    }
    let total: u64 = buckets.values().sum();
    println!(
        "\nper-tick displacement, {} villagers, 4,000 ticks ({total} body-ticks)",
        workers.len()
    );
    for (name, count) in &buckets {
        println!(
            "  {name:>8}: {count:>7}  ({:.2}%)",
            100.0 * *count as f64 / total as f64
        );
    }
    println!("  step shapes of a walk: {walk_dirs:?}");
    if !partial_sizes.is_empty() {
        let mut sizes: Vec<_> = partial_sizes.iter().collect();
        sizes.sort_by_key(|(ratio, _)| **ratio);
        println!("  partial steps, as thousandths of one step: {sizes:?}");
        println!("  partial steps by task: {partial_task:?}");
    }
    let _ = scale;
}
