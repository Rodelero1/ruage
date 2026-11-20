//! **A farm takes one farmer, and an empty one leaves a husk** (owner, 2026-11-04):
//!
//! *"Only 1 farmer per farm (can be constructed by multiple, but only 1 can farm once construction is finished). Once farm
//! expires, it decays and can/needs to be replanted. It will cost 60 wood"* ... *"Husk is effect only ... It obstructs nothing as
//! if it was not there"* ... *"if there is no empty farm nearby, villager becomes idle"*.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_plain()
}

fn next_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// A finished farm on clear ground, holding `food` **whole** units.
fn a_farm(world: &mut World, at: (i64, i64), food: i64) -> EntityId {
    let scale = world.scale();
    a_farm_at(world, (at.0 * scale, at.1 * scale), food)
}

/// The same plot anchored at an arbitrary **point**, which is what `Command::Spawn` is handed: the middle of a tile is as
/// legal as its corner, and a scenario or a saved game may carry either.
fn a_farm_at(world: &mut World, at: (i64, i64), food: i64) -> EntityId {
    let id = EntityId(next_id(world));
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: world.role_unit("farm"),
            position: (Fixed::from_raw(at.0), Fixed::from_raw(at.1)),
            speed: Fixed::ZERO,
            hp: 480,
            // `amount` in a Spawn is **whole units**: the engine scales it.
            amount: u32::try_from(food).expect("in range"),
        })
        .expect("a farm on clear ground");
    world.step().expect("one tick");
    id
}

fn a_villager(world: &mut World, at: (i64, i64)) -> EntityId {
    let id = EntityId(next_id(world));
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: world.role_unit("villager"),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::from_raw(scale * 8 / 10),
            hp: 25,
            amount: 0,
        })
        .expect("a villager on clear ground");
    world.step().expect("one tick");
    id
}

/// Three clear tiles in a row, away from the seeded base.
fn clear_ground(world: &World) -> (i64, i64) {
    for y in 40..50u16 {
        for x in 40..50u16 {
            if world.is_free_ground(x, y)
                && world.is_free_ground(x + 3, y)
                && world.is_free_ground(x, y + 3)
                && world.is_free_ground(x + 1, y + 3)
                && world.is_free_ground(x + 2, y + 3)
                && world.is_free_ground(x + 3, y + 3)
            {
                return (i64::from(x), i64::from(y));
            }
        }
    }
    panic!("no clear ground on the map");
}

fn working_on(world: &World, who: EntityId) -> Option<EntityId> {
    match world.entity(who).map(|entity| entity.task) {
        Some(Task::Gather { node, .. }) => Some(node),
        _ => None,
    }
}

#[test]
fn a_second_farmer_goes_to_a_free_farm() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let farm_a = a_farm(&mut world, (x, y), 50);
    let farm_b = a_farm(&mut world, (x + 3, y), 50);
    let first = a_villager(&mut world, (x, y + 3));
    let second = a_villager(&mut world, (x + 1, y + 3));

    world
        .submit_checked(Command::Gather {
            villager: first,
            node: farm_a,
        })
        .expect("the first farmer takes his farm");
    world
        .submit_checked(Command::Gather {
            villager: second,
            node: farm_a,
        })
        .expect("a second order is taken, and redirected");
    world.step().expect("stepping");

    assert_eq!(
        working_on(&world, first),
        Some(farm_a),
        "the first farmer keeps the farm he was sent to"
    );
    assert_eq!(
        working_on(&world, second),
        Some(farm_b),
        "the second goes to the free farm instead of standing on the first man's crop"
    );
}

/// **A plot's reach is its ground, not the point it is anchored at.** `Command::Spawn` puts a building wherever it is told,
/// so a plot may stand anchored at the **middle** of a tile; its nearest free ring tile is then one and a half tiles away on
/// both axes, and a reach measured to the anchor *point* refused the order outright - a man could not be sent to a plot
/// standing in the open three tiles away. The measure is the gap to the **footprint**, which is the shape of the game's own
/// arrival test (a box gap): a stand that touches the plot's ground is a stand.
#[test]
fn a_man_sent_to_a_plot_anchored_at_a_tiles_middle_works_it() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let scale = world.scale();
    // The middle of the anchor tile: half a tile east and south of the corner `a_farm` uses.
    let farm = a_farm_at(
        &mut world,
        (x * scale + scale / 2, y * scale + scale / 2),
        50,
    );
    let farmer = a_villager(&mut world, (x + 3, y + 3));
    world
        .submit_checked(Command::Gather {
            villager: farmer,
            node: farm,
        })
        .expect("a plot three tiles away on open ground: the order must be taken");

    let before = world.entity(farm).expect("the plot").amount.raw();
    for _ in 0..400 {
        world.step().expect("stepping");
    }
    let after = world.entity(farm).expect("the plot").amount.raw();
    assert!(
        after < before,
        "the man walks over and takes food out of it: {before} -> {after}"
    );
}

#[test]
fn a_second_farmer_with_no_free_farm_becomes_idle() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let farm = a_farm(&mut world, (x, y), 50);
    let first = a_villager(&mut world, (x, y + 3));
    let second = a_villager(&mut world, (x + 1, y + 3));

    world
        .submit_checked(Command::Gather {
            villager: first,
            node: farm,
        })
        .expect("the first farmer takes the farm");
    world
        .submit_checked(Command::Gather {
            villager: second,
            node: farm,
        })
        .expect("the order is taken and answered with an idle villager");
    world.step().expect("stepping");

    assert_eq!(working_on(&world, first), Some(farm));
    assert_eq!(
        world.entity(second).map(|entity| entity.task),
        Some(Task::Idle),
        "with no free farm anywhere, the second villager is idle rather than a second man on one crop"
    );
}

/// **"Nearby" means nearby** (the owner's word). A free farm on the other side of the map is not offered to a second farmer:
/// he stands idle, which is what "2 C" says, rather than walking 40 tiles to a field nobody sent him to.
#[test]
fn a_free_farm_far_away_does_not_take_the_second_farmer() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let here = a_farm(&mut world, (x, y), 50);
    // A free plot far from the working one: beyond `[farm] free_farm_radius`.
    let far = a_farm(&mut world, (x + 40, y), 50);
    let first = a_villager(&mut world, (x, y + 3));
    let second = a_villager(&mut world, (x + 1, y + 3));

    world
        .submit_checked(Command::Gather {
            villager: first,
            node: here,
        })
        .expect("the first farmer takes the near farm");
    world
        .submit_checked(Command::Gather {
            villager: second,
            node: here,
        })
        .expect("the order is taken");
    world.step().expect("stepping");

    assert_eq!(working_on(&world, first), Some(here));
    assert_ne!(
        working_on(&world, second),
        Some(far),
        "a plot 40 tiles away is not nearby: the villager idles instead"
    );
    assert_eq!(
        world.entity(second).map(|entity| entity.task),
        Some(Task::Idle),
        "with no free farm nearby, the second villager is idle"
    );
}

/// **Both orders in the same tick**, which is what a box selection sends: the second must not slip in before the first has a
/// task. This is the case a check that runs only "when the order is applied" can miss, so it is tested separately.
#[test]
fn two_orders_in_one_tick_still_take_two_farms() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let a = a_farm(&mut world, (x, y), 50);
    let b = a_farm(&mut world, (x + 3, y), 50);
    let first = a_villager(&mut world, (x, y + 3));
    let second = a_villager(&mut world, (x + 1, y + 3));

    // Both submitted before any tick runs.
    world
        .submit_checked(Command::Gather {
            villager: first,
            node: a,
        })
        .expect("first order");
    world
        .submit_checked(Command::Gather {
            villager: second,
            node: a,
        })
        .expect("second order, same tick");
    world.step().expect("stepping");

    let one = working_on(&world, first);
    let two = working_on(&world, second);
    assert_eq!(one, Some(a), "the first keeps the farm he named");
    assert_eq!(
        two,
        Some(b),
        "and the second gets the other plot, not the same one"
    );
}

/// **A villager whose farm turns into a husk and then vanishes is not left holding a dead id.** He takes the last load home,
/// the husk's while passes, and the entity goes: the villager must end up doing something - idle or at another node - rather
/// than standing with a task that names a thing which no longer exists.
#[test]
fn a_villager_survives_his_farm_vanishing() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let farm = a_farm(&mut world, (x, y), 1);
    let farmer = a_villager(&mut world, (x, y + 3));
    world
        .submit_checked(Command::Gather {
            villager: farmer,
            node: farm,
        })
        .expect("a farm with one unit of food");

    // Past the husk's whole life and well beyond: the entity is gone, and the villager has had time to settle.
    for _ in 0..30 * 40 + 1200 {
        world.step().expect("stepping");
    }
    assert!(world.entity(farm).is_none(), "the husk is gone");
    let task = world.entity(farmer).map(|entity| entity.task);
    assert!(
        !matches!(task, Some(Task::Gather { node, .. }) if node == farm),
        "the villager still holds the vanished farm as his node: {task:?}"
    );
    assert!(
        world.entity(farmer).is_some(),
        "and he is still on the map, not removed with the husk"
    );
}

#[test]
fn an_emptied_farm_leaves_a_husk_that_nothing_blocks_and_then_goes() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (x, y) = clear_ground(&world);
    let scale = world.scale();
    // One whole unit of food: a single take empties it.
    let farm = a_farm(&mut world, (x, y), 1);
    let farmer = a_villager(&mut world, (x, y + 3));
    world
        .submit_checked(Command::Gather {
            villager: farmer,
            node: farm,
        })
        .expect("a farm with food is workable");

    // Long enough to walk over and take the one unit; well inside the husk's while.
    for _ in 0..300 {
        world.step().expect("stepping");
    }
    let husk = world
        .entity(farm)
        .expect("the husk is still on the map: vanishing takes some time");
    assert!(
        world.is_expired_farm(farm),
        "a farm with nothing left is a husk: the panel and the plot both read it that way"
    );
    assert!(
        husk.amount.raw() <= 0,
        "and it holds no food, so nothing can be gathered from it"
    );

    // **It obstructs nothing**: a villager walks over the husk's own ground.
    let over = (husk.position.0, husk.position.1);
    world.submit(Command::Move {
        target: None,
        unit: farmer,
        to: (
            u16::try_from(over.0.raw() / scale).expect("in range"),
            u16::try_from(over.1.raw() / scale).expect("in range"),
        ),
    });
    let mut reached = false;
    for _ in 0..600 {
        world.step().expect("stepping");
        if let Some(at) = world.position(farmer) {
            let dx = at.0.raw() - over.0.raw();
            let dy = at.1.raw() - over.1.raw();
            if dx * dx + dy * dy < scale * scale {
                reached = true;
                break;
            }
        }
    }
    assert!(
        reached,
        "the villager cannot stand where the husk is: a husk is effect only and obstructs nothing"
    );

    // **And then it goes**, after its own while (30 s of content, 40 ticks a second).
    for _ in 0..30 * 40 + 400 {
        world.step().expect("stepping");
    }
    assert!(
        world.entity(farm).is_none(),
        "a husk is a picture, not a building: it vanishes after its while"
    );
}
