//! Training: pay the price, wait out the create time, get a unit.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, SimError, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project pack's column for a resource, so a test says "wood" the way content does and the
/// pack is what turns that into a stockpile column (D17).
fn slot(name: &str) -> usize {
    slot_in(
        &ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml"),
        name,
    )
}

fn slot_in(pack: &ruage_data::Resources, name: &str) -> usize {
    pack.slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
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
    // The pack first: the map names its resources and only the pack turns a name into a column.
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
    let mut world = World::new(&fx.settings, 5).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
}

/// A world on content of the test's choosing, with food in the bank: the tests that need a hundred
/// orders should not spend their time gathering them.
fn seeded_rich(fx: &Fixture, economy: &Economy) -> World {
    let mut economy = economy.clone();
    economy.start.stock = [("food".to_owned(), 2_000)].into_iter().collect();
    let mut world = World::new(&fx.settings, 5).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
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

/// Gather until the bank holds at least `want` in a stockpile column, from the nearest node of it.
fn bank(world: &mut World, slot: usize, want: u32) {
    let villager = first_villager(world);
    let here = world.position(villager).expect("position");
    let node = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(slot))
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
        if world.stock(1, slot) >= want {
            return;
        }
    }
    panic!(
        "could not bank {want} in column {slot}; the bank holds {}",
        world.stock(1, slot)
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
        .filter(|e| e.kind == EntityKind::Resource(slot("wood")))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a tree")
        .id;
    bank(&mut world, slot("wood"), 25);

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

/// One slot per building was a slice, not the game: a real barracks queues. The order is paid when it
/// is placed - that is what makes a queue a commitment - and the second villager arrives one create
/// time after the first, not immediately.
#[test]
fn a_second_order_joins_the_queue_and_is_paid_at_once() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    bank(&mut world, slot("food"), 150);
    let centre = town_centre(&world);
    let order = |world: &mut World| {
        world
            .submit_checked(Command::Train {
                building: centre,
                unit: VILLAGER,
            })
            .expect("the order is accepted while one is training")
    };

    order(&mut world);
    world.step().expect("step");
    let after_first = world.stock(1, slot("food"));
    order(&mut world);
    // The order is priced when it is applied, one tick later, and the tick after that begins work on
    // it; the price is the same either way.
    world.step().expect("step");
    assert_eq!(
        world.stock(1, slot("food")),
        after_first - 50,
        "the queued order is paid when it is placed"
    );

    // Both arrive, one create time apart.
    let create_ticks = u64::from(world.tick_rate_hz()) * 25;
    let mut arrivals = Vec::new();
    let mut seen = villager_count(&world);
    for _ in 0..(create_ticks * 4) {
        world.step().expect("step");
        let now = villager_count(&world);
        while seen < now {
            arrivals.push(world.tick());
            seen += 1;
        }
        if arrivals.len() == 2 {
            break;
        }
    }
    assert_eq!(arrivals.len(), 2, "both villagers arrived");
    let gap = arrivals[1] - arrivals[0];
    assert!(
        gap.abs_diff(create_ticks) <= 2,
        "the second waits its own create time: {gap} ticks against {create_ticks}"
    );
}

/// The queue has a length, and content owns it.
#[test]
fn the_queue_stops_at_the_length_content_allows() {
    let Some(fx) = fixture() else { return };
    // The population ceiling is not what this test is about, so it is lifted out of the way: a queue
    // of five villagers needs five population and the town centre alone has that.
    let mut economy = fx.economy.clone();
    economy.population.cap_default = 50;
    economy.population.town_centre_population = 50;
    let mut world = seeded_rich(&fx, &economy);
    let centre = town_centre(&world);
    let length = economy.train.queue_length;
    for placed in 0..length {
        world
            .submit_checked(Command::Train {
                building: centre,
                unit: VILLAGER,
            })
            .unwrap_or_else(|err| panic!("order {placed} of {length} was refused: {err:?}"));
    }
    match world.submit_checked(Command::Train {
        building: centre,
        unit: VILLAGER,
    }) {
        Err(SimError::QueueFull {
            id,
            length: refused,
        }) => {
            assert_eq!((id, refused), (centre, length));
        }
        other => panic!("expected QueueFull, got {other:?}"),
    }
}

fn villager_count(world: &World) -> usize {
    world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .count()
}

const VILLAGER: u32 = 83;

#[test]
fn a_villager_costs_fifty_food_and_appears_after_its_create_time() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    bank(&mut world, slot("food"), 50);
    let centre = town_centre(&world);
    let before = world.stock(1, slot("food"));
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
        world.stock(1, slot("food")),
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
        new_villager.speed,
        speed.resolve(world.scale(), "speed").expect("resolved"),
        "speed comes from the unit table"
    );
    assert_eq!(
        new_villager.velocity.0.raw(),
        0,
        "a freshly trained villager stands still: speed is a capability, not a motion"
    );
}
