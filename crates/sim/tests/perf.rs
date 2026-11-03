//! Performance probes. Ignored by default: they print timings rather than assert behaviour, and a
//! test that only prints is not a test. Run them deliberately:
//!
//! ```text
//! cargo test --release -p ruage-sim --test perf -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use ruage_data::{ContentPack, Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World, load_personality_file};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fixture() -> Option<(Settings, Map, Economy, Units, ruage_data::Resources)> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return None;
    }
    // The pack first: the map names its resources and only the pack turns a name into a column.
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some((
        ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        ruage_data::load_units_file(&units).expect("units.toml"),
        resources,
    ))
}

/// Idle units at rest: the cheapest possible world, so whatever cost shows up is per-entity
/// bookkeeping rather than pathfinding.
fn world_with(
    fx: &(Settings, Map, Economy, Units, ruage_data::Resources),
    extra_units: usize,
) -> World {
    let (settings, map, economy, units, resources) = fx;
    let mut world = World::new(settings, 11).expect("world");
    world.seed_from(map, economy, units).expect("seeding");
    world
        .load_resources(resources)
        .expect("the pack fits the stockpile");
    let scale = world.scale();
    for (next, i) in (10_000u32..).zip(0..extra_units) {
        let id = EntityId(next);
        let x = 2 + (i as i64 % 60);
        let y = 2 + (i as i64 / 60) % 60;
        world
            .submit_checked(Command::Spawn {
                id,
                player: u8::try_from(1 + (i % 8)).expect("1..=8"),
                kind: EntityKind::Unit,
                unit: Some(83),
                position: (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale)),
                speed: Fixed::ZERO,
                hp: 25,
                amount: 0,
            })
            .expect("spawn");
    }
    world.step().expect("one step to apply the queued spawns");
    world
}

#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn idle_world_scales_with_entities() {
    let Some(fx) = fixture() else { return };
    let ticks = 2_000u32;
    for extra in [0usize, 250, 500, 1000, 2000] {
        let mut world = world_with(&fx, extra);
        let entities = world.entities().len();
        let start = Instant::now();
        for _ in 0..ticks {
            world.step().expect("step");
        }
        let elapsed = start.elapsed();
        let per_tick = elapsed.as_secs_f64() / f64::from(ticks);
        let per_tick_us = per_tick * 1e6;
        let per_entity_ns = per_tick_us * 1e3 / entities as f64;
        println!(
            "{entities:>5} entities  {per_tick_us:>9.2} us/tick  {per_entity_ns:>7.1} ns/entity/tick"
        );
    }
}

#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn a_ten_minute_match_in_release() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, 0);
    let entities = world.entities().len();
    let ticks = 12_000u32;
    let start = Instant::now();
    for _ in 0..ticks {
        world.step().expect("step");
    }
    let elapsed = start.elapsed();
    println!(
        "10 min at 20 Hz ({ticks} ticks, {entities} entities): {:.1} ms, {:.2} us/tick",
        elapsed.as_secs_f64() * 1e3,
        elapsed.as_secs_f64() / f64::from(ticks) * 1e6
    );
}

/// What a **blocked** production queue costs per tick (D85). A building with nowhere to put its finished unit retries
/// every tick until a tile frees, so this is the shape that repeats; the probe prints the price with the yard full and
/// the same world without a queue, which is the difference the pause actually costs.
#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn a_blocked_queue_costs_this_much_per_tick() {
    let Some(fx) = fixture() else { return };
    let (settings, map, economy, units, resources) = &fx;
    let mut economy = economy.clone();
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 1_000),
    ]
    .into_iter()
    .collect();
    economy.population.cap_default = 1_000;
    economy.population.town_center_population = 500;
    economy.decay.enabled = false;
    economy.train.by_building.insert(84, vec![83]);
    let mut world = World::new(settings, 11).expect("world");
    world.seed_from(map, &economy, units).expect("seeding");
    world
        .load_resources(resources)
        .expect("the pack fits the stockpile");
    let scale = world.scale();
    let market = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    world
        .submit_checked(Command::Spawn {
            id: market,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(84),
            position: (Fixed::from_raw(20 * scale), Fixed::from_raw(20 * scale)),
            speed: Fixed::ZERO,
            hp: 1_800,
            amount: 0,
        })
        .expect("a market");
    world.step().expect("step");
    // Every tile a recruit could be placed on, filled by a unit of the same player.
    let mut id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    let mut blocked = 0;
    for dy in -2i64..=2 {
        for dx in -2i64..=2 {
            let (x, y) = (20 + dx, 20 + dy);
            let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            if !world.tile_walkable(x, y) {
                continue;
            }
            let taken = world.entities().iter().any(|e| {
                (e.position.0.raw() / scale, e.position.1.raw() / scale)
                    == (i64::from(x), i64::from(y))
            });
            if taken {
                continue;
            }
            world
                .submit_checked(Command::Spawn {
                    id,
                    player: 1,
                    kind: EntityKind::Unit,
                    unit: Some(83),
                    position: (
                        Fixed::from_raw(i64::from(x) * scale),
                        Fixed::from_raw(i64::from(y) * scale),
                    ),
                    speed: Fixed::ZERO,
                    hp: 25,
                    amount: 0,
                })
                .expect("a blocker");
            id = EntityId(id.0 + 1);
            blocked += 1;
        }
    }
    world.step().expect("step");
    // Best of five samples of 500 ticks in each half, and a warmup before the first: this machine's timings are bimodal
    // (a shared 2-CPU box), and a single sample of a probe that compares two halves is measuring the noise.
    let sample = |world: &mut World, ticks: u32| {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..ticks {
                world.step().expect("step");
            }
            best = best.min(start.elapsed().as_secs_f64() / f64::from(ticks) * 1e6);
        }
        best
    };
    for _ in 0..500 {
        world.step().expect("warmup");
    }
    let quiet = sample(&mut world, 500);
    world
        .submit_checked(Command::Train {
            building: market,
            unit: 83,
        })
        .expect("train");
    for _ in 0..500 {
        world.step().expect("warmup");
    }
    let paused = sample(&mut world, 500);
    println!(
        "{} entities, {blocked} blockers, queue {:?}: quiet {quiet:.2} us/tick, blocked {paused:.2} us/tick",
        world.entities().len(),
        world.training_pause(market)
    );
}

/// The same idle world with a **memory switched on** for every player, which is what a match with rules for all eight
/// seats pays (D92). Sightings are taken once a game second, so ten of these two hundred ticks do the mask work.
#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn idle_world_with_a_memory_costs_this_much() {
    let Some(fx) = fixture() else { return };
    let ticks = 200u32;
    for extra in [0usize, 250, 500, 1000, 2000] {
        let mut world = world_with(&fx, extra);
        for player in 1..=8u8 {
            world.observe_for(player);
        }
        let entities = world.entities().len();
        let start = Instant::now();
        for _ in 0..ticks {
            world.step().expect("step");
        }
        let elapsed = start.elapsed();
        let per_tick = elapsed.as_secs_f64() / f64::from(ticks);
        let per_tick_us = per_tick * 1e6;
        let per_entity_ns = per_tick_us * 1e3 / entities as f64;
        println!(
            "{entities:>5} entities  {per_tick_us:>9.2} us/tick  {per_entity_ns:>7.1} ns/entity/tick  (memory on)"
        );
    }
}

/// What a match pays **once**, at load, now that the villager shares travel with the personality (D94): the pack's
/// identity walk (which hashes `economy.toml` and every file under `ai/`), the personality parse, and installing the
/// 19-row table into a world. None of it is per tick - the point of the probe is that the number stays at load time so
/// nobody "optimises" a table that costs microseconds per match.
#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn what_a_match_loads_and_installs_costs() {
    let Some(fx) = fixture() else { return };
    let reps = 200u32;

    let start = Instant::now();
    for _ in 0..reps {
        let pack = ContentPack::load(&repo("config")).expect("the pack loads");
        std::hint::black_box(pack.identity());
    }
    println!(
        "pack load + identity ({} files hashed): {:.2} us each",
        ContentPack::load(&repo("config"))
            .expect("the pack loads")
            .parts
            .len(),
        start.elapsed().as_secs_f64() / f64::from(reps) * 1e6
    );

    let path = repo("config/ai/base.toml");
    let start = Instant::now();
    for _ in 0..reps {
        let personality = load_personality_file(&path).expect("the base personality");
        std::hint::black_box(personality.policy.share.len());
    }
    println!(
        "personality parse ({} rules, {} share rows): {:.2} us each",
        load_personality_file(&path)
            .expect("the base personality")
            .rule
            .len(),
        load_personality_file(&path)
            .expect("the base personality")
            .policy
            .share
            .len(),
        start.elapsed().as_secs_f64() / f64::from(reps) * 1e6
    );

    let policy = load_personality_file(&path)
        .expect("the base personality")
        .policy;
    let mut world = world_with(&fx, 0);
    let reps = 1_000u32;
    let start = Instant::now();
    for _ in 0..reps {
        world.apply_policy(&policy);
        std::hint::black_box(world.entities().len());
    }
    println!(
        "apply_policy ({} rows): {:.2} us each",
        policy.share.len(),
        start.elapsed().as_secs_f64() / f64::from(reps) * 1e6
    );
}
