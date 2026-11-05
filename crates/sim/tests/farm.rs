//! **A farm takes one farmer, and an empty one leaves a husk** (owner, 2026-11-04):
//!
//! *"Only 1 farmer per farm (can be constructed by multiple, but only 1 can farm once construction is finished). Once farm
//! expires, it decays and can/needs to be replanted. It will cost 60 wood"* ... *"Husk is effect only ... It obstructs nothing as
//! if it was not there"* ... *"if there is no empty farm nearby, villager becomes idle"*.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
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
    let id = EntityId(next_id(world));
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: world.role_unit("farm"),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
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
