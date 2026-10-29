//! Build queues: a villager that is given a second site does not abandon the first (D66).
//!
//! Written before the implementation (D7). The owner's question, in his words: "is build order queuing
//! possible? That means that builder never stops building until done and then moves to the next".
//!
//! The rule the game uses, and the one this asks for: an order given to a busy builder is the *next*
//! thing, not a replacement. Two consequences worth testing separately, because they can break apart:
//! the queue starts the next site on the tick the last one finished, and it pays for that site then
//! rather than when it was ordered - otherwise ordering five houses with the money for one would be
//! accepted and the world would go into debt.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
}

/// Two sites with one builder: the `Build` order is the authority on which tiles are free, so the grid
/// is walked until it accepts two. Both must be accepted - a refused second one would leave no queue.
fn place_twice(world: &mut World, builder: ruage_sim::EntityId, unit: u32) {
    let mut placed = 0;
    for y in 4..120u16 {
        for x in 4..120u16 {
            if world
                .submit_checked(Command::Build {
                    villager: builder,
                    unit,
                    at: (x, y),
                })
                .is_ok()
            {
                placed += 1;
                if placed == 2 {
                    return;
                }
            }
        }
    }
    panic!("the world accepted {placed} of the two sites a queue needs");
}

/// The first villager of player 1.
fn a_villager(world: &World) -> ruage_sim::EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

/// Everything player 1 has in the stockpile, summed: a house costs 25 wood and nothing else moves.
fn total(world: &World) -> i64 {
    (0..4).map(|slot| world.stock(1, slot) as i64).sum()
}

fn count_of(world: &World, unit: u32) -> usize {
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.unit == Some(unit))
        .count()
}

/// Step until the condition holds, or give up: a walk plus a house is a few hundred ticks.
fn step_until(world: &mut World, ticks: usize, done: impl Fn(&World) -> bool) -> bool {
    for _ in 0..ticks {
        if let Err(err) = world.step() {
            panic!("stepping: {err:?}");
        }
        if done(world) {
            return true;
        }
    }
    false
}

#[test]
fn a_second_site_waits_for_the_first_and_then_starts() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let house = world.role_unit("house").expect("house");
    let builder = a_villager(&world);

    place_twice(&mut world, builder, house);
    // Orders apply on the next tick (`command_apply = "next_tick"`), so the first site only exists after
    // one step - and the second order, applied after it, is what queues behind it.
    world.step().expect("one tick");

    // Queued, not started: there is one foundation, not two.
    assert_eq!(
        count_of(&world, house),
        1,
        "a queued site must not appear before the one in front of it is finished"
    );

    // Run long enough for the first house (and its walk) and no longer than the second.
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if count_of(&world, house) == 2 {
            break;
        }
    }

    assert_eq!(
        count_of(&world, house),
        2,
        "the queued site starts on its own once the first is finished, without another order"
    );
    // With nothing left in the queue the builder has no site, no path and no queue: that is what
    // `bookkeeping_touches` refusing it means, once both houses are up.
    let done = step_until(&mut world, 4_000, |world| {
        world
            .entities()
            .iter()
            .filter(|entity| {
                entity.player == 1 && entity.unit == Some(house) && entity.construction.is_none()
            })
            .count()
            == 2
    });
    assert!(done, "both houses finish without any further order");
    assert_eq!(
        count_of(&world, house),
        2,
        "a queue of two sites builds two sites, not three"
    );
    assert!(
        !world.bookkeeping_touches(builder),
        "and the builder is left with nothing to do"
    );
}

#[test]
fn a_queued_site_is_paid_for_when_it_starts() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let house = world.role_unit("house").expect("house");
    let builder = a_villager(&world);
    let before = total(&world);

    place_twice(&mut world, builder, house);
    world
        .step()
        .expect("one tick, so the first site is placed and paid for");

    let after_ordering = total(&world);
    let spent = before - after_ordering;
    assert!(
        spent > 0 && spent < 60,
        "only the site that started is paid for, not the one waiting behind it (spent {spent})"
    );

    for _ in 0..2_000 {
        world.step().expect("stepping");
        if count_of(&world, house) == 2 {
            break;
        }
    }
    let when_second_starts = total(&world);
    assert!(
        when_second_starts < after_ordering,
        "the second site is paid for on the tick it starts"
    );
}
