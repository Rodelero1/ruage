//! Gathering: walk to a node, take from it at the variant's rate, deplete it.
//!
//! Written before the implementation (D7). Returning to a drop site and depositing is the next
//! increment; these tests stop at "carry is full".

use std::path::PathBuf;

use ruage_data::{Economy, Map, ResourceKind, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

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
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
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
fn nearest(world: &World, kind: ResourceKind, from: (Fixed, Fixed)) -> EntityId {
    world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(kind))
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
    let node = nearest(&world, ResourceKind::Wood, here);
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
    let node = nearest(&world, ResourceKind::Wood, here);
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
    let node = nearest(&world, ResourceKind::Wood, here);

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
fn the_carry_cap_comes_from_the_variant_and_stops_the_work() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = nearest(&world, ResourceKind::Wood, here);

    let variant = fx
        .economy
        .gather
        .variant
        .get("wood")
        .copied()
        .expect("wood variant");
    let cap = fx.units.unit(variant).expect("VMLUM").carry;

    world.submit(Command::Gather { villager, node });
    for _ in 0..4_000 {
        world.step().expect("step");
    }

    assert_eq!(
        world.carry(villager).raw(),
        i64::from(cap) * world.scale(),
        "a full villager holds exactly the variant's carry ({cap})"
    );
}

#[test]
fn a_depleted_node_stops_yielding() {
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    let map = ruage_data::load_map_str(TINY_MAP, "tiny.toml").expect("tiny map");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");

    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
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
    assert_eq!(
        world.carry(villager).raw(),
        5 * world.scale(),
        "the villager took exactly what the tree held, and stopped"
    );
}

#[test]
fn two_villagers_on_one_node_cannot_take_more_than_it_holds() {
    // Each take is bounded by the node's amount *as read at the start of the tick*, so two
    // villagers working a one-wood tree can each be allowed to take it. That would mint a
    // resource out of nothing - the node clamps at zero and the villagers keep the difference.
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    let map = ruage_data::load_map_str(&TINY_MAP.replace("amount = 5", "amount = 1"), "tiny.toml")
        .expect("tiny map");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");

    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
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
    let node = nearest(&world, ResourceKind::Wood, here);

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
        let node = nearest(&world, ResourceKind::Wood, here);
        world.submit(Command::Gather { villager, node });
        for _ in 0..500 {
            world.step().expect("step");
        }
        world.state_hash()
    };
    assert_eq!(run(&fx), run(&fx));
}
