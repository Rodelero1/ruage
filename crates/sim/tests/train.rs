//! Training: pay the price, wait out the create time, get a unit.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, ResourceKind, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, SimError, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    units: Units,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml")).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 5).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
}

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Building && e.player == 1 && e.unit == Some(109))
        .expect("the town centre")
        .id
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Unit && e.player == 1)
        .expect("a villager")
        .id
}

/// Gather until the bank holds at least `want` of a kind, from the nearest node of that kind.
fn bank(world: &mut World, kind: ResourceKind, want: u32) {
    let villager = first_villager(world);
    let here = world.position(villager).expect("position");
    let node = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(kind))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a node of that kind")
        .id;
    world.submit(Command::Gather { villager, node });
    for _ in 0..200_000 {
        world.step().expect("step");
        if world.stock(1, kind) >= want {
            return;
        }
    }
    panic!(
        "could not bank {want} {kind:?}; the bank holds {}",
        world.stock(1, kind)
    );
}

#[test]
fn training_a_unit_this_building_does_not_make_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    // A spearman is a barracks unit (12), not a town-centre one.
    match world.submit_checked(Command::Train {
        building: centre,
        unit: 74,
    }) {
        Err(SimError::NotTrainableHere { building, unit }) => {
            assert_eq!(building, 109);
            assert_eq!(unit, 74);
        }
        other => panic!("expected NotTrainableHere, got {other:?}"),
    }
}

#[test]
fn a_building_that_is_still_going_up_cannot_train() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(ResourceKind::Wood))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a tree")
        .id;
    bank(&mut world, ResourceKind::Wood, 25);

    let scale = world.scale();
    let x = u16::try_from(world.position(villager).expect("p").0.raw() / scale).unwrap();
    let y = u16::try_from(world.position(villager).expect("p").1.raw() / scale).unwrap();
    let site = (x + 2, y + 2);
    world
        .submit_checked(Command::Build {
            villager,
            unit: 70,
            at: site,
        })
        .expect("a house order");
    world.step().expect("step");
    let house = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Building && e.unit == Some(70) && e.player == 1)
        .expect("the site")
        .id;

    match world.submit_checked(Command::Train {
        building: house,
        unit: 83,
    }) {
        Err(SimError::BuildingNotReady { id }) => assert_eq!(id, house),
        other => panic!("expected BuildingNotReady, got {other:?}"),
    }
    let _ = node;
}

#[test]
fn a_second_order_cannot_replace_a_running_one() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Enough for two villagers on purpose: with only 50 food the second order would be refused
    // for being unaffordable, and the test would pass without proving anything about the slot.
    bank(&mut world, ResourceKind::Food, 100);
    let centre = town_centre(&world);
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: 83,
        })
        .expect("the first order");
    world.step().expect("step");
    let after_first = world.stock(1, ResourceKind::Food);
    assert!(
        after_first >= 50,
        "the second order must be affordable, or this test proves nothing"
    );

    // The food for the second villager would be taken and the first job thrown away with it.
    match world.submit_checked(Command::Train {
        building: centre,
        unit: 83,
    }) {
        Err(SimError::AlreadyTraining { id }) => assert_eq!(id, centre),
        other => panic!("expected AlreadyTraining, got {other:?}"),
    }
    assert_eq!(
        world.stock(1, ResourceKind::Food),
        after_first,
        "a refused order must cost nothing"
    );
}

#[test]
fn a_villager_costs_fifty_food_and_appears_after_its_create_time() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    bank(&mut world, ResourceKind::Food, 50);
    let centre = town_centre(&world);
    let before = world.stock(1, ResourceKind::Food);
    let villagers = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Unit && e.player == 1)
        .count();

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: 83,
        })
        .expect("the order must be accepted");
    world.step().expect("step");
    assert_eq!(
        world.stock(1, ResourceKind::Food),
        before - 50,
        "a villager costs 50 food, paid when the order lands"
    );

    let started = world.tick();
    let create_time = u64::try_from(fx.units.unit(83).expect("VMBAS").train_time).unwrap();
    let expected = create_time * u64::from(world.tick_rate_hz());
    let mut spawned_at = None;
    for _ in 0..(expected * 3) {
        world.step().expect("step");
        let now = world
            .entities()
            .iter()
            .filter(|e| e.kind == EntityKind::Unit && e.player == 1)
            .count();
        if now > villagers {
            spawned_at = Some(world.tick() - started);
            break;
        }
    }
    assert_eq!(
        spawned_at,
        Some(expected),
        "a {create_time} s villager at {} Hz must appear {expected} ticks after the order lands",
        world.tick_rate_hz()
    );

    let new_villager = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Unit && e.player == 1)
        .max_by_key(|e| e.id.0)
        .expect("the new villager");
    assert_eq!(new_villager.hp, 25, "hp comes from the unit table");
    assert_eq!(new_villager.task, Task::Idle);
    let speed = fx.units.unit(83).expect("VMBAS").speed;
    assert_eq!(
        new_villager.velocity.0,
        speed.resolve(world.scale(), "speed").expect("resolved"),
        "speed comes from the unit table"
    );
}
