//! Gathering: walk to a node, take from it at the variant's rate, deplete it.
//!
//! Written before the implementation (D7). Returning to a drop site and depositing is the next
//! increment; these tests stop at "carry is full".

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

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
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
}

/// A 6x6 map with one five-wood tree, so depletion can happen inside a test's patience.
const TINY_MAP: &str = r#"
schema = 1
[map]
width = 6
height = 6
terrain = [
  "GGGGGG",
  "GGGFGG",
  "GGGGGG",
  "GGGGGG",
  "GGGGGG",
  "GGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 5
tiles = [[3, 1]]
[[map.town_centre]]
player = 1
x = 1
y = 1
[[map.town_centre]]
player = 2
x = 5
y = 5
"#;

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Unit && e.player == 1)
        .expect("player 1 has a villager")
        .id
}

/// The nearest node of `kind` to `from`: squared distance, ties broken by id, so the choice is a
/// function of state rather than of iteration order.
fn nearest(world: &World, slot: usize, from: (Fixed, Fixed)) -> EntityId {
    world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(slot))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - from.0.raw();
            let dy = e.position.1.raw() - from.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a node of that kind exists")
        .id
}

fn amount_of(world: &World, node: EntityId) -> Fixed {
    world.entity(node).expect("node").amount
}

#[test]
fn a_gather_order_does_not_teleport_anyone() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);
    let node_position = world.position(node).expect("node position");

    world.submit(Command::Gather { villager, node });
    world.step().expect("step");

    let after = world.position(villager).expect("position");
    assert_ne!(after, here, "the villager must start walking");
    assert_ne!(after, node_position, "and must not arrive in one tick");
}

#[test]
fn the_villager_walks_to_the_node_and_then_takes_from_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);
    let node_position = world.position(node).expect("node position");
    let before = amount_of(&world, node).raw();

    world.submit(Command::Gather { villager, node });
    for _ in 0..600 {
        world.step().expect("step");
    }

    assert_eq!(
        world.position(villager),
        Some(node_position),
        "the villager should be standing on the node"
    );
    let taken = before - amount_of(&world, node).raw();
    assert!(taken > 0, "the node must have been worked");
    assert_eq!(
        world.carry(villager).raw(),
        taken,
        "what the villager carries must equal what the node lost"
    );
}

#[test]
fn the_rate_is_the_variants_work_rate_per_tick() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);

    // VMLUM gathers wood at 0.390 per second: 7800 at scale 20000, so 390 a tick at 20 Hz.
    let variant = fx
        .economy
        .gather
        .variant
        .get("wood")
        .copied()
        .expect("wood variant");
    let rate = fx.units.unit(variant).expect("VMLUM").work_rate;
    let per_second = rate.resolve(world.scale(), "work_rate").expect("resolved");
    assert_eq!(per_second.raw(), 7_800, "0.390 at scale 20000");

    world.submit(Command::Gather { villager, node });
    let mut arrived = None;
    for tick in 0..2_000 {
        world.step().expect("step");
        if world.position(villager) == world.position(node) {
            arrived = Some(tick);
            break;
        }
    }
    let arrived = arrived.expect("the villager must arrive");
    let at_arrival = world.carry(villager).raw();

    for _ in 0..20 {
        world.step().expect("step");
    }
    let gained = world.carry(villager).raw() - at_arrival;
    assert_eq!(
        gained,
        per_second.raw(),
        "one second of gathering (arrived at tick {arrived}) must be 7800 units, not {gained}"
    );
}

#[test]
fn the_carry_cap_comes_from_the_variant_and_bounds_every_trip() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);

    let variant = fx
        .economy
        .gather
        .variant
        .get("wood")
        .copied()
        .expect("wood variant");
    let cap = fx.units.unit(variant).expect("VMLUM").carry;

    let cap_raw = i64::from(cap) * world.scale();
    world.submit(Command::Gather { villager, node });
    for tick in 0..4_000 {
        world.step().expect("step");
        assert!(
            world.carry(villager).raw() <= cap_raw,
            "at tick {tick} the villager carried {} units, past the variant's cap of {cap}",
            world.carry(villager).raw() / world.scale()
        );
    }

    // And it kept working rather than standing there full: a full load is taken home.
    assert!(
        world.stock(1, slot("wood")) >= u32::from(cap),
        "a villager that fills up must deliver, not idle at the tree"
    );
}

#[test]
fn a_depleted_node_stops_yielding() {
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    let map = ruage_data::load_map_str(TINY_MAP, "tiny.toml", &fx.resources).expect("tiny map");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // The pack opens with a stockpile, so every count below is measured from here.
    let opening = world.stock(1, slot("wood"));
    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .expect("the tree")
        .id;
    assert_eq!(
        amount_of(&world, node).raw(),
        5 * world.scale(),
        "five wood"
    );

    let villager = first_villager(&world);
    world.submit(Command::Gather { villager, node });
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    assert_eq!(amount_of(&world, node), Fixed::ZERO, "the tree is empty");
    // The load never filled, so the villager takes the partial load home rather than standing on
    // a dead tree with five wood in hand. Counted from the opening stockpile, because the pack now
    // opens with one.
    assert_eq!(
        world.stock(1, slot("wood")) - opening,
        5,
        "everything the tree held reaches the stockpile"
    );
    assert_eq!(
        world.carry(villager),
        Fixed::ZERO,
        "and nothing is left in hand"
    );
}

#[test]
fn a_full_villager_carries_the_wood_home() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);
    let before = amount_of(&world, node).raw();

    // Not "starts with nothing": the pack opens with the standard stockpile. What the test is about
    // is a load arriving, so everything is counted from here.
    let opening = world.stock(1, slot("wood"));

    world.submit(Command::Gather { villager, node });
    let mut delivered = 0;
    for _ in 0..6_000 {
        world.step().expect("step");
        delivered = world.stock(1, slot("wood")) - opening;
        if delivered > 0 {
            break;
        }
    }

    assert!(delivered > 0, "a full villager must deliver its load");
    let taken = before - amount_of(&world, node).raw();
    assert_eq!(
        i64::from(delivered) * world.scale() + world.carry(villager).raw(),
        taken,
        "everything the node lost is either in the stockpile or still in hand"
    );
    assert_eq!(delivered, 10, "one trip is one full VMLUM load");
}

#[test]
fn a_partial_load_comes_home_when_the_node_runs_out() {
    // A three-wood tree against a ten-wood cap: the load never fills, so the "carry is full"
    // rule never fires. Without a second rule the villager stands on a dead tree holding wood
    // for the rest of the match - the resource is neither in the stockpile nor working.
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    let map = ruage_data::load_map_str(
        &TINY_MAP.replace("amount = 5", "amount = 3"),
        "tiny.toml",
        &fx.resources,
    )
    .expect("tiny map");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // The pack opens with a stockpile, so every count below is measured from here.
    let opening = world.stock(1, slot("wood"));
    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .expect("the tree")
        .id;
    let villager = first_villager(&world);
    world.submit(Command::Gather { villager, node });

    for _ in 0..2_000 {
        world.step().expect("step");
    }

    assert_eq!(amount_of(&world, node), Fixed::ZERO, "the tree is empty");
    assert_eq!(
        world.stock(1, slot("wood")) - opening,
        3,
        "the partial load must be delivered, not stranded in the villager's hands"
    );
    assert_eq!(
        world.carry(villager),
        Fixed::ZERO,
        "and nothing is left in hand afterwards"
    );
}

#[test]
fn a_closer_lumber_camp_wins_over_the_town_centre() {
    // VMLUM's own drop list is [109 town centre, 562 lumber camp]. With a camp standing next to
    // the tree, a woodcutter must use it: that is the whole point of the campaign economy.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, slot("wood"), here);
    let node_position = world.position(node).expect("node position");

    let camp_unit = 562;
    let camp_position = (node_position.0, node_position.1);
    let camp = EntityId(9_000);
    world
        .submit_checked(Command::Spawn {
            id: camp,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(camp_unit),
            position: camp_position,
            speed: Fixed::ZERO,
            hp: 600,
            amount: 0,
        })
        .expect("spawn a lumber camp");

    world.submit(Command::Gather { villager, node });
    // Run until the villager fills up and chooses a destination.
    let mut drop = None;
    for _ in 0..4_000 {
        world.step().expect("step");
        let returning = world.entity(villager).map(|entity| entity.task);
        if let Some(ruage_sim::Task::Return { drop: target, .. }) = returning {
            drop = Some(target);
            break;
        }
    }

    assert_eq!(
        drop,
        Some(camp),
        "the woodcutter must head for the camp it is standing next to"
    );
}

#[test]
fn two_villagers_on_one_node_cannot_take_more_than_it_holds() {
    // Each take is bounded by the node's amount *as read at the start of the tick*, so two
    // villagers working a one-wood tree can each be allowed to take it. That would mint a
    // resource out of nothing - the node clamps at zero and the villagers keep the difference.
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    let map = ruage_data::load_map_str(
        &TINY_MAP.replace("amount = 5", "amount = 1"),
        "tiny.toml",
        &fx.resources,
    )
    .expect("tiny map");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .expect("the tree")
        .id;
    let villagers: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Unit && e.player == 1)
        .map(|e| e.id)
        .take(2)
        .collect();
    assert_eq!(villagers.len(), 2, "the fixture has two workers");

    for villager in &villagers {
        world.submit(Command::Gather {
            villager: *villager,
            node,
        });
    }
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    let carried: i64 = villagers.iter().map(|v| world.carry(*v).raw()).sum();
    assert_eq!(amount_of(&world, node), Fixed::ZERO, "the tree is empty");
    assert_eq!(
        carried,
        world.scale(),
        "one wood was taken in total, not two: {} units came out of a one-unit tree",
        carried / world.scale()
    );
}

#[test]
fn ordering_a_building_to_gather_is_an_error() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let town_centre = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Building)
        .expect("TC")
        .id;

    match world.submit_checked(Command::Gather {
        villager,
        node: town_centre,
    }) {
        Err(SimError::NotAResource { id }) => assert_eq!(id, town_centre),
        other => panic!("expected NotAResource, got {other:?}"),
    }
}

#[test]
fn ordering_a_node_to_gather_is_an_error() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let here = world.position(first_villager(&world)).expect("position");
    let node = nearest(&world, slot("wood"), here);

    match world.submit_checked(Command::Gather {
        villager: node,
        node,
    }) {
        Err(SimError::NotAUnit { id }) => assert_eq!(id, node),
        other => panic!("expected NotAUnit, got {other:?}"),
    }
}

#[test]
fn the_same_orders_produce_the_same_hash() {
    let Some(fx) = fixture() else { return };
    let run = |fx: &Fixture| {
        let mut world = seeded(fx);
        let villager = first_villager(&world);
        let here = world.position(villager).expect("position");
        let node = nearest(&world, slot("wood"), here);
        world.submit(Command::Gather { villager, node });
        for _ in 0..500 {
            world.step().expect("step");
        }
        world.state_hash()
    };
    assert_eq!(run(&fx), run(&fx));
}
