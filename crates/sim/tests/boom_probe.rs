//! A throwaway probe: what does a plain Dark Age opening actually buy us?
//!
//! Run with `cargo test --test boom_probe -- --nocapture`. It is a measurement, not an assertion: it plays a
//! simple economy - villagers from tick 0, a mill on the berries, a barracks, the two ages - and prints the
//! in-game clock whenever something lands. The acceptance scenario will be written from these numbers rather
//! than from arithmetic.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fx {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
    combat: ruage_data::Combat,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fx> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fx {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs"),
    })
}

fn clock(tick: u64, rate: u32) -> String {
    let seconds = tick / u64::from(rate);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn tile_of(world: &World, position: (Fixed, Fixed)) -> (i64, i64) {
    (
        position.0.raw() / world.scale(),
        position.1.raw() / world.scale(),
    )
}

/// The nearest resource node in stockpile column `slot`.
fn near_node(world: &World, from: (Fixed, Fixed), slot: usize) -> Option<EntityId> {
    world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .filter(|entity| entity.amount.raw() > 0)
        .min_by_key(|entity| {
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            dx * dx + dy * dy
        })
        .map(|entity| entity.id)
}

/// Tiles near `centre` with nothing on them, nearest first. Walkability is left to the order: a blocked
/// tile is refused by `Build`, and trying the next candidate is cheaper than asking first.
fn candidates(world: &World, centre: (i64, i64)) -> Vec<(u16, u16)> {
    let mut found = Vec::new();
    for radius in 2..9i64 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (x, y) = (centre.0 + dx, centre.1 + dy);
                if x < 0 || y < 0 || x > 60 || y > 60 {
                    continue;
                }
                let taken = world
                    .entities()
                    .iter()
                    .any(|entity| tile_of(world, entity.position) == (x, y));
                if !taken {
                    found.push((x as u16, y as u16));
                }
            }
        }
    }
    found
}

/// Place `unit` beside the town centre with any villager: the first candidate tile the order accepts.
///
/// `reserved` holds tiles already spoken for this pass. Orders are queued, not applied, so two builds in one
/// pass would otherwise pick the same tile and the second would fail at apply time - which surfaces as a
/// `step` error rather than as a refusal here.
fn try_build(
    world: &mut World,
    unit: u32,
    centre: (i64, i64),
    reserved: &mut Vec<(u16, u16)>,
    busy: &mut std::collections::BTreeSet<u32>,
) -> bool {
    // A *different* villager for every site: one builder given a second order abandons the first site, and
    // a mill foundation then sits unfinished for the rest of the match. That cost a probe run to learn.
    let Some(builder) = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .filter(|entity| matches!(entity.task, Task::Idle))
        .filter(|entity| !busy.contains(&entity.id.0))
        .map(|entity| entity.id)
        .next()
    else {
        return false;
    };
    busy.insert(builder.0);
    for at in candidates(world, centre).into_iter().take(24) {
        if reserved.contains(&at) {
            continue;
        }
        if world
            .submit_checked(Command::Build {
                villager: builder,
                unit,
                at,
            })
            .is_ok()
        {
            reserved.push(at);
            return true;
        }
    }
    false
}

/// Ignored by default: it is a measurement, not a claim, and it prints a timeline instead of asserting one.
/// Run it with `cargo test --test boom_probe -- --include-ignored --nocapture`.
#[test]
#[ignore = "measurement, not an assertion"]
fn what_a_dark_age_opening_buys() {
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seed");
    world.load_resources(&fx.resources).expect("stockpile fits");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research");
    world.load_civs(&fx.civs).expect("civs");

    let rate = world.tick_rate_hz();
    let food = world.slot_of("food").expect("food");
    let wood = world.slot_of("wood").expect("wood");
    let gold = world.slot_of("gold").expect("gold");
    let villager = world.role_unit("villager").expect("villager");
    let house = world.role_unit("house").expect("house");
    let mill = world.role_unit("mill").expect("mill");
    let lumber_camp = world.role_unit("lumber_camp").expect("lumber camp");
    let farm_unit = world.role_unit("farm").expect("farm");
    let blacksmith = world.role_unit("blacksmith").expect("blacksmith");
    let market = world.role_unit("market").expect("market");
    let mining_camp = world.role_unit("mining_camp").expect("mining camp");

    let town_centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| (entity.id, entity.position))
        .expect("the town centre");
    let (tc, tc_at) = town_centre;
    let tc_tile = tile_of(&world, tc_at);

    let mut by_slot: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for entity in world.entities() {
        if let EntityKind::Resource(slot) = entity.kind {
            *by_slot.entry(slot).or_default() += 1;
        }
    }
    println!(
        "slots: food={food} wood={wood} gold={gold} | nodes by slot={by_slot:?} | nearest wood={:?}",
        near_node(&world, tc_at, wood)
    );
    let mut built: Vec<&str> = Vec::new();
    let mut feudal_at: Option<u64> = None;
    let mut castle_at: Option<u64> = None;
    let mut last_report = 0u64;
    let mut assigned: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let mut next_train_at = 0u64;
    let mut busy: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();

    for tick in 0..24_000u64 {
        // --- economy -------------------------------------------------------------------------
        if tick % 10 == 0 {
            let pop = world.population(1);
            let cap = world.population_cap(1);
            let mut reserved: Vec<(u16, u16)> = Vec::new();
            // Houses: keep two headroom ahead of the population.
            if pop + 2 >= cap {
                let _ = try_build(&mut world, house, tc_tile, &mut reserved, &mut busy);
            }

            // Villagers, one order at a time so the queue is not spammed.
            if pop < 14
                && tick >= next_train_at
                && world.stock(1, food) >= 50
                && world
                    .submit_checked(Command::Train {
                        building: tc,
                        unit: villager,
                    })
                    .is_ok()
            {
                next_train_at = tick + 40;
            }

            // The two Dark Age buildings the Feudal Age wants.
            if !started(&world, "mill")
                && world.stock(1, wood) >= 100
                && try_build(&mut world, mill, tc_tile, &mut reserved, &mut busy)
            {
                built.push("mill");
            }
            // The second Dark Age building a fast castle actually builds: a lumber camp, not a barracks.
            // The researched orders skip the barracks, and the 75 wood is wanted for the market.
            if have(&world, "mill")
                && !started(&world, "lumber_camp")
                && world.stock(1, wood) >= 100
                && try_build(&mut world, lumber_camp, tc_tile, &mut reserved, &mut busy)
            {
                built.push("lumber_camp");
            }

            // Farms: the researched orders' answer to "the berries ran out". Four at a time, 60 wood each,
            // beside the town centre; a farmer works the plot and deposits at the TC.
            if have(&world, "mill") && world.stock(1, wood) >= 60 {
                let alive = world
                    .entities()
                    .iter()
                    .filter(|entity| entity.player == 1 && entity.unit == Some(farm_unit))
                    .count();
                if alive < 4 {
                    let _ = try_build(&mut world, farm_unit, tc_tile, &mut reserved, &mut busy);
                }
            }

            // After the Feudal Age: the cheapest pair of Feudal buildings for the Castle Age, and a camp
            // for the gold the Castle Age costs.
            if feudal_at.is_some() {
                if !started(&world, "blacksmith") && world.stock(1, wood) >= 150 {
                    let _ = try_build(&mut world, blacksmith, tc_tile, &mut reserved, &mut busy);
                }
                if !started(&world, "market") && world.stock(1, wood) >= 175 {
                    let _ = try_build(&mut world, market, tc_tile, &mut reserved, &mut busy);
                }
                if !started(&world, "mining_camp") && world.stock(1, wood) >= 100 {
                    let _ = try_build(&mut world, mining_camp, tc_tile, &mut reserved, &mut busy);
                }
            }

            // Assign idle villagers: 3 on berries, 3 on wood, 2 on gold once a camp exists.
            let counts = gather_counts(&world, &fx, food, wood, gold);
            let workers: Vec<EntityId> = world
                .entities()
                .iter()
                .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
                .filter(|entity| matches!(entity.task, Task::Idle))
                .filter(|entity| !busy.contains(&entity.id.0))
                .map(|entity| entity.id)
                .collect();
            for worker in workers {
                // Three on food, the rest on wood: wood pays for the houses, the mill, the barracks and
                // everything after them. Filling food first is what starved the first probe run of wood.
                let (want_food, want_wood, want_gold) = if started(&world, "mining_camp") {
                    (4, 5, 2)
                } else {
                    (3, 6, 0)
                };
                let ripe_farm = world
                    .entities()
                    .iter()
                    .filter(|entity| entity.player == 1 && entity.unit == Some(farm_unit))
                    .filter(|entity| entity.construction.is_none() && entity.amount.raw() > 0)
                    .map(|entity| entity.id)
                    .next();
                let target = if counts.0 < want_food {
                    ripe_farm.or_else(|| near_node(&world, tc_at, food))
                } else if counts.1 < want_wood {
                    near_node(&world, tc_at, wood)
                } else if counts.2 < want_gold {
                    near_node(&world, tc_at, gold)
                } else {
                    near_node(&world, tc_at, wood)
                };
                if let Some(node) = target {
                    let outcome = world.submit_checked(Command::Gather {
                        villager: worker,
                        node,
                    });
                    if let Err(err) = outcome
                        && tick < 400
                    {
                        println!("  gather refused at {}: {err}", clock(tick, rate));
                    }
                }
            }

            // --- ages ------------------------------------------------------------------------
            if feudal_at.is_none()
                && have(&world, "mill")
                && have(&world, "mill")
                && have(&world, "lumber_camp")
                && world.stock(1, food) >= 500
                && world
                    .submit_checked(Command::Research {
                        building: tc,
                        tech: 101,
                    })
                    .is_ok()
            {
                feudal_at = Some(tick);
                built.push("feudal-research-started");
            }
            if castle_at.is_none()
                && world.researched(1).contains(&101)
                && have(&world, "blacksmith")
                && have(&world, "market")
                && world.stock(1, food) >= 800
                && world.stock(1, gold) >= 200
                && world
                    .submit_checked(Command::Research {
                        building: tc,
                        tech: 102,
                    })
                    .is_ok()
            {
                castle_at = Some(tick);
            }
            let _ = assigned.insert(0);
        }

        world.step().expect("step");

        if tick / 600 != last_report / 600 {
            last_report = tick;
            println!(
                "{} pop={} food={} wood={} gold={} mill={}/{} barr={}/{} feudal={} castle={} buildings={}",
                clock(tick, rate),
                world.population(1),
                world.stock(1, food),
                world.stock(1, wood),
                world.stock(1, gold),
                started(&world, "mill"),
                have(&world, "mill"),
                started(&world, "lumber_camp"),
                have(&world, "lumber_camp"),
                feudal_at.is_some(),
                castle_at.is_some(),
                world
                    .entities()
                    .iter()
                    .filter(|e| e.player == 1 && e.kind == EntityKind::Building)
                    .count(),
            );
        }
    }
    println!(
        "done: feudal_started={:?} castle_started={:?} researched={:?}",
        feudal_at.map(|t| clock(t, rate)),
        castle_at.map(|t| clock(t, rate)),
        world.researched(1),
    );
}

/// Does `player` 1 have a finished building of this role?
fn have(world: &World, role: &str) -> bool {
    let Some(unit) = world.role_unit(role) else {
        return false;
    };
    world
        .entities()
        .iter()
        .any(|entity| entity.unit == Some(unit) && entity.construction.is_none())
}

/// Has player 1 started (or finished) a building of this role?
fn started(world: &World, role: &str) -> bool {
    let Some(unit) = world.role_unit(role) else {
        return false;
    };
    world
        .entities()
        .iter()
        .any(|entity| entity.unit == Some(unit))
}

fn gather_counts(
    world: &World,
    fx: &Fx,
    food: usize,
    wood: usize,
    gold: usize,
) -> (usize, usize, usize) {
    let mut counts = (0usize, 0usize, 0usize);
    for entity in world.entities() {
        if entity.player != 1 || entity.kind != EntityKind::Unit {
            continue;
        }
        if let Task::Gather { node, .. } = entity.task
            && let Some(target) = world.entity(node)
            && let EntityKind::Resource(slot) = target.kind
        {
            if slot == food {
                counts.0 += 1;
            } else if slot == wood {
                counts.1 += 1;
            } else if slot == gold {
                counts.2 += 1;
            }
        }
    }
    let _ = fx;
    counts
}
