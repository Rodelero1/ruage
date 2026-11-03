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
///
/// **And they must not overlap**, because both orders are checked against a world in which neither has landed yet: a
/// house owns its whole footprint, so a second anchor one tile from the first is accepted and can never start. That is a
/// real shape - the queue check runs before the first site exists - and the engine reports the failure rather than
/// swallowing it (`a_queued_site_that_cannot_start_is_reported`).
fn place_twice(world: &mut World, builder: ruage_sim::EntityId, unit: u32) {
    let mut first: Option<(u16, u16)> = None;
    let mut placed = 0;
    for y in 4..120u16 {
        for x in 4..120u16 {
            if let Some((fx, fy)) = first
                && x < fx.saturating_add(3)
                && y < fy.saturating_add(3)
            {
                continue;
            }
            if world
                .submit_checked(Command::Build {
                    villager: builder,
                    unit,
                    at: (x, y),
                })
                .is_ok()
            {
                placed += 1;
                if placed == 1 {
                    first = Some((x, y));
                }
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

/// **A second site that overlaps the first is refused when it lands, and the refusal names the builder.** Two orders
/// given in one tick are both validated against a world in which neither site exists yet, so the second passes
/// `submit_checked` and fails at apply time; that is why `place_twice` keeps the two anchors apart. The refusal used to
/// read `entity 0` for every command, because the actor was a stub - a refusal that cannot name its actor cannot be
/// diagnosed.
#[test]
fn a_second_site_that_overlaps_the_first_names_its_builder() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let house = world.role_unit("house").expect("house");
    let builder = a_villager(&world);
    for at in [(4u16, 4u16), (5, 5)] {
        world
            .submit_checked(Command::Build {
                villager: builder,
                unit: house,
                at,
            })
            .unwrap_or_else(|err| panic!("{at:?} refused before it was even queued: {err:?}"));
    }
    world
        .step()
        .expect("one tick: the first site lands, the second is refused");
    match world.last_refusal() {
        Some((id, why, tick)) => {
            println!("refused for {id:?} at tick {tick}: {why}");
            assert_eq!(
                id, builder,
                "the refusal names the villager whose order could not start"
            );
            assert!(
                matches!(why, ruage_sim::SimError::SiteOccupied { .. }),
                "and it says the ground is taken: {why}"
            );
        }
        None => panic!("the second site's refusal must be in state, not swallowed"),
    }
}

/// **A queued site whose ground is taken before it starts leaves a refusal.** The queue pops the next site on the tick the
/// last one finishes, and `start_build` can fail there - somebody else built on the ground, a tree grew back, the villager
/// was pushed out of reach. The first version wrote `let _ = self.start_build(...)` and the queued order vanished with no
/// record; it now goes into `last_refusal` like any self-issued order.
#[test]
fn a_queued_site_whose_ground_is_taken_is_reported() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let house = world.role_unit("house").expect("house");
    let builder = a_villager(&world);
    // Two legal sites, far enough apart that neither footprint touches the other.
    let mut first: Option<(u16, u16)> = None;
    let mut second: Option<(u16, u16)> = None;
    'search: for y in 4..120u16 {
        for x in 4..120u16 {
            if !world.site_fits(1, house, (x, y)) {
                continue;
            }
            match first {
                None => first = Some((x, y)),
                Some((fx, fy)) => {
                    if x >= fx.saturating_add(3) || y >= fy.saturating_add(3) {
                        second = Some((x, y));
                        break 'search;
                    }
                }
            }
        }
    }
    let (a, b) = (first.expect("a site"), second.expect("a second site"));
    for at in [a, b] {
        world
            .submit_checked(Command::Build {
                villager: builder,
                unit: house,
                at,
            })
            .expect("both sites are legal when they are ordered");
    }
    world.step().expect("the first site lands");
    // Take the second site's ground before the first is finished, with one of our own buildings so it is certainly seen.
    world.step().expect("the first build starts");
    let blocker = ruage_sim::EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: blocker,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (
                ruage_fixed::Fixed::from_raw(i64::from(b.0) * scale),
                ruage_fixed::Fixed::from_raw(i64::from(b.1) * scale),
            ),
            speed: ruage_fixed::Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("the blocker is placed");
    let mut reported = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if let Some((id, why, _)) = world.last_refusal()
            && id == builder
        {
            println!("the queued site was refused: {why}");
            reported = true;
            break;
        }
    }
    assert!(
        reported,
        "a queued site that cannot start must leave a refusal naming its builder"
    );
}
