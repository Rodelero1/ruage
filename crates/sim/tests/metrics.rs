//! Metrics for the balance engine (item 8). A batch run is one row per (script, seed): timings, resources delivered
//! per minute, villagers lost, the first military unit, the outcome.
//!
//! Written before the implementation (D7). The numbers are **derived from what the world already reports** - stock,
//! entities, researched techs, the outcome - so the runner never becomes a second simulation, and nothing new is
//! hashed.

use std::path::PathBuf;

use ruage_sim::{Command, EntityId, EntityKind, Metrics, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
    combat: ruage_data::Combat,
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
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
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
    world.load_combat(&fx.combat);
    world
}

fn military_roles(fx: &Fixture) -> Vec<String> {
    fx.economy.metrics.military_roles.clone()
}

/// A world that gathers for a minute reports what it delivered, and nothing else: no age, no soldier, no losses.
#[test]
fn a_gathering_minute_reports_delivered_resources_and_nothing_else() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let wood = world.slot_of("wood").expect("the pack has wood");
    let mut metrics = Metrics::new(&world, 1, &military_roles(&fx));

    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    let node = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if slot == wood))
        .map(|entity| entity.id)
        .next()
        .expect("a tree");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("a villager can be sent at a tree");

    let ticks = u64::from(world.tick_rate_hz()) * 60;
    for _ in 0..ticks {
        world.step().expect("stepping");
        metrics.observe(&world);
    }

    assert!(
        metrics.delivered(wood) > 0,
        "a villager on a tree for a minute delivers wood: {}",
        metrics.delivered(wood)
    );
    assert_eq!(
        metrics.per_minute(wood),
        metrics.delivered(wood),
        "sixty seconds of a one-minute run is the same number"
    );
    assert_eq!(metrics.feudal_at(), None, "no age was researched");
    assert_eq!(metrics.castle_at(), None);
    assert_eq!(metrics.first_military_at(), None, "nobody was trained");
    assert_eq!(metrics.villagers_lost(), 0, "nobody died");
}

/// A villager that dies is a loss, counted once - and the metric has to see it without the engine keeping a tally.
#[test]
fn a_dead_villager_is_counted_as_a_loss_once() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let mut metrics = Metrics::new(&world, 1, &military_roles(&fx));
    metrics.observe(&world);
    assert_eq!(metrics.villagers_lost(), 0);

    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: villager,
        })
        .expect("a villager can be deleted");

    for _ in 0..40 {
        world.step().expect("stepping");
        metrics.observe(&world);
    }
    assert_eq!(
        metrics.villagers_lost(),
        1,
        "one villager died, and a metric that counts it twice is worse than none"
    );
}

/// The same script on the same seed reports the same numbers: a balance table nobody can reproduce is opinion.
#[test]
fn the_same_run_reports_the_same_numbers() {
    let Some(fx) = fixture() else {
        return;
    };
    let wood = seeded(&fx).slot_of("wood").expect("wood");
    let play = |fx: &Fixture| {
        let mut world = seeded(fx);
        let mut metrics = Metrics::new(&world, 1, &military_roles(fx));
        let villager: EntityId = world
            .entities()
            .iter()
            .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
            .expect("a villager")
            .id;
        let node = world
            .entities()
            .iter()
            .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if slot == wood))
            .map(|entity| entity.id)
            .next()
            .expect("a tree");
        world
            .submit_checked(Command::Gather { villager, node })
            .expect("a gather order");
        for _ in 0..600 {
            world.step().expect("stepping");
            metrics.observe(&world);
        }
        (
            metrics.delivered(wood),
            metrics.per_minute(wood),
            world.state_hash(),
        )
    };
    assert_eq!(
        play(&fx),
        play(&fx),
        "a balance number has to be reproducible"
    );
}

/// **Delivered and spent are different things.** A bank that paid for a villager has not gathered less food, and a
/// metric that adds the price to the delivery report makes an economy look twice as fast as it is. Written after the
/// mutation that folded spending into delivery passed the gathering test untouched - nothing was bought in it.
#[test]
fn buying_something_is_not_a_delivery() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let food = world.slot_of("food").expect("the pack has food");
    let wood = world.slot_of("wood").expect("the pack has wood");
    let mut metrics = Metrics::new(&world, 1, &military_roles(&fx));
    let opening_food = world.stock(1, food);
    let opening_wood = world.stock(1, wood);

    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    let node = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if slot == wood))
        .map(|entity| entity.id)
        .next()
        .expect("a tree");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("a gather order");
    for _ in 0..600 {
        world.step().expect("stepping");
        metrics.observe(&world);
    }

    // A villager costs 50 food: the town centre can make one, and the queue is the cheapest way to spend.
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .id;
    let villager_unit = world.role_unit("villager").expect("the villager role");
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager_unit,
        })
        .expect("a villager can be queued");
    for _ in 0..40 {
        world.step().expect("stepping");
        metrics.observe(&world);
    }

    assert_eq!(
        metrics.spent(food),
        i64::from(opening_food) - i64::from(world.stock(1, food)),
        "what the price took out of the bank is spending, and it is counted as spending"
    );
    assert_eq!(
        metrics.delivered(food),
        0,
        "nothing gathered food, so nothing was delivered - a purchase is not income"
    );
    // Wood: deposits are the only source, and the bank's gain is exactly what arrived minus what was paid out. What
    // is still in the villager's hands never reached the bank, so it is not part of either number.
    assert_eq!(
        metrics.delivered(wood) - metrics.spent(wood),
        i64::from(world.stock(1, wood)) - i64::from(opening_wood),
        "delivered minus spent is exactly what the bank gained (delivered {}, spent {}, carry {})",
        metrics.delivered(wood),
        metrics.spent(wood),
        world.carry(villager).raw() / world.scale()
    );
}
