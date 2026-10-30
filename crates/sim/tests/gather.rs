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

    // On the node, or beside it: a forest tree stands on blocked ground, so a villager works it from the tile next to it
    // (the owner's touching rule, 2026-10-29). The distance is the assertion, not the exact tile.
    let at = world.position(villager).expect("a position");
    let scale = world.scale();
    let dx = (at.0.raw() - node_position.0.raw()).abs() / scale;
    let dy = (at.1.raw() - node_position.1.raw()).abs() / scale;
    assert!(
        dx <= 1 && dy <= 1,
        "the villager should be on or beside the node: at {at:?}, node {node_position:?}"
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
    // Measure from the **first productive tick**, not from "arrival": a villager works a forest tree from the tile
    // beside it, so it is within a tile for a moment before it starts taking (the owner's touching rule, 2026-10-29).
    let mut started = None;
    for tick in 0..2_000 {
        world.step().expect("step");
        if world.carry(villager).raw() > 0 {
            started = Some((tick, world.carry(villager).raw()));
            break;
        }
    }
    let (started_at, first) = started.expect("the villager must start taking from the node");

    for _ in 0..20 {
        world.step().expect("step");
    }
    let gained = world.carry(villager).raw() - first;
    assert_eq!(
        gained,
        per_second.raw(),
        "one second of gathering (first take at tick {started_at}) must be 7800 units, not {gained}"
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

// --- farms --------------------------------------------------------------------------------------
//
// A farm is a building the villager works: the one entity in the engine that is neither a resource
// node nor a producer. `[farm]` has been in economy.toml since the beginning and nothing read it.

/// Run until the site is finished, or give up loudly: a test that silently stops building would look
/// like a gather failure later.
fn finish_building(world: &mut World, site: EntityId) {
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            return;
        }
        world.step().expect("step");
    }
    panic!("the building never finished");
}

/// Build a farm for player 1 next to the town centre and return its id once it is up.
fn a_finished_farm(world: &mut World) -> EntityId {
    let villager = first_villager(world);
    let site = (4, 4);
    world
        .submit_checked(Command::Build {
            villager,
            unit: FARM,
            at: site,
        })
        .expect("a farm costs wood the player has");
    world.step().expect("step");
    let farm = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(FARM))
        .map(|e| e.id)
        .next()
        .expect("the site exists");
    finish_building(world, farm);
    farm
}

const FARM: u32 = 50;

#[test]
fn a_villager_works_a_farm_for_food() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let farm = a_finished_farm(&mut world);
    let villager = first_villager(&world);
    let before_food = world.stock(1, slot("food"));
    let before_farm = amount_of(&world, farm);
    assert!(before_farm.raw() > 0, "a finished farm holds food");

    world
        .submit_checked(Command::Gather {
            villager,
            node: farm,
        })
        .expect("a farm can be worked");

    let mut banked = 0;
    for _ in 0..6_000 {
        world.step().expect("step");
        let now = world.stock(1, slot("food"));
        if now > before_food {
            banked = now - before_food;
            break;
        }
    }
    assert!(banked > 0, "the villager brought food home from the farm");
    assert!(
        amount_of(&world, farm).raw() < before_farm.raw(),
        "the food came out of the farm, not from nowhere"
    );
}

#[test]
fn a_farm_holds_only_what_content_gives_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_researchable(&fx, 20);
    let farm = a_finished_farm(&mut world);
    let villager = first_villager(&world);
    let before_food = world.stock(1, slot("food"));
    world
        .submit_checked(Command::Gather {
            villager,
            node: farm,
        })
        .expect("a farm can be worked");

    // Long enough for many trips: the farm runs out and the total is what content said, not more.
    for _ in 0..60_000 {
        world.step().expect("step");
    }
    assert_eq!(
        world.stock(1, slot("food")) - before_food,
        20,
        "twenty food in the farm is twenty food in the bank"
    );
    assert_eq!(amount_of(&world, farm).raw(), 0, "and the farm is spent");
}

#[test]
fn a_farm_still_going_up_cannot_be_worked() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: FARM,
            at: (4, 4),
        })
        .expect("build");
    world.step().expect("step");
    let farm = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(FARM))
        .map(|e| e.id)
        .next()
        .expect("the site exists");

    match world.submit_checked(Command::Gather {
        villager,
        node: farm,
    }) {
        Err(SimError::BuildingNotReady { id }) => assert_eq!(id, farm),
        other => panic!("expected BuildingNotReady, got {other:?}"),
    }
}

// --- the farm food line ---------------------------------------------------------------------------
//
// Horse Collar, Heavy Plow and Crop Rotation are kind-1 commands on dat resource 36: +75, +125, +175.
// The importer decodes them; content names which resource id that is; the engine adds it to farm food.

/// Feudal Age, the gate Horse Collar hangs off.
fn grant_feudal(world: &mut World, player: u8) {
    for tech in [659, 101] {
        world
            .grant_tech(player, tech)
            .expect("seeding the Feudal Age");
    }
}

/// A world whose player 1 can afford a 75/75 tech without gathering first.
fn seeded_rich(fx: &Fixture) -> World {
    seeded_researchable(fx, fx.economy.farm.food_per_farm)
}

/// The same, with the farm's own budget overridden so a plot can be eaten inside a test.
fn seeded_researchable(fx: &Fixture, food_per_farm: u32) -> World {
    let mut economy = fx.economy.clone();
    economy.farm.food_per_farm = food_per_farm;
    economy.start.stock = [("food".to_owned(), 500), ("wood".to_owned(), 500)]
        .into_iter()
        .collect();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // Research needs the tech and effect tables; the rest of this suite never asks for them. With
    // them comes availability: the pack gates the farm itself behind tech 216 ("Farms (make avail)",
    // free and instant), and this world has not met its prerequisites.
    let techs =
        ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs.toml");
    let effects =
        ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).expect("effects.toml");
    world
        .load_research(&techs, &effects)
        .expect("research tables");
    world.grant_tech(1, 216).expect("farming is available");
    world
}

/// A mill, which is where the pack puts the farm upgrades (tech location 68).
fn a_mill(world: &mut World, at: (u16, u16)) -> EntityId {
    let villager = first_villager(world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: MILL,
            at,
        })
        .expect("a mill costs 100 wood the player has");
    world.step().expect("step");
    let mill = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(MILL))
        .map(|e| e.id)
        .next()
        .expect("the mill site exists");
    finish_building(world, mill);
    mill
}

fn research(world: &mut World, tech: u32) {
    let mill = a_mill(world, (6, 6));
    world
        .submit_checked(Command::Research {
            building: mill,
            tech,
        })
        .expect("the research order");
    // Horse Collar is 20 s; the world is at 20 ticks per second, and the order is applied on the
    // first step, so this budget is a wide margin rather than a measured number.
    for _ in 0..2_000 {
        world.step().expect("step");
    }
}

#[test]
fn horse_collar_gives_an_existing_farm_more_food() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_rich(&fx);
    let farm = a_finished_farm(&mut world);
    let base = amount_of(&world, farm).raw();
    assert_eq!(base, 250 * world.scale(), "content says 250 per farm");

    grant_feudal(&mut world, 1);
    research(&mut world, HORSE_COLLAR);

    assert_eq!(
        amount_of(&world, farm).raw(),
        base + 75 * world.scale(),
        "Horse Collar adds 75 to a farm already standing"
    );
}

#[test]
fn a_farm_built_after_the_upgrade_holds_the_new_total() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_rich(&fx);
    grant_feudal(&mut world, 1);
    research(&mut world, HORSE_COLLAR);
    let farm = a_finished_farm(&mut world);
    assert_eq!(
        amount_of(&world, farm).raw(),
        325 * world.scale(),
        "250 base plus Horse Collar's 75"
    );
}

/// A plot that has given everything already holds nothing, and an upgrade does not resurrect it: the
/// food is gone, and a spent plot is a barren one.
#[test]
fn an_upgrade_does_not_refill_a_spent_farm() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_researchable(&fx, 20);
    let farm = a_finished_farm(&mut world);
    world
        .submit_checked(Command::Gather {
            villager: first_villager(&world),
            node: farm,
        })
        .expect("work the plot");
    // Eat the whole plot: twenty food at 0.4/s is one trip of two loads, and the margin is wide.
    for _ in 0..20_000 {
        world.step().expect("step");
        if amount_of(&world, farm).raw() == 0 {
            break;
        }
    }
    assert_eq!(amount_of(&world, farm).raw(), 0, "the plot is spent");

    grant_feudal(&mut world, 1);
    research(&mut world, HORSE_COLLAR);
    assert_eq!(amount_of(&world, farm).raw(), 0, "a spent plot stays spent");
}

const HORSE_COLLAR: u32 = 14;
const MILL: u32 = 68;

/// *"Villager should continue with the nearest similar source"* (owner, 2026-10-29): a tree that runs out sends the
/// lumberjack to the next tree, not to a standstill. Before this a villager worked exactly one node and then idled,
/// which is what stopped the acceptance match with 139 wood in the bank.
#[test]
fn a_depleted_node_sends_the_villager_to_the_next_one_of_its_kind() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let wood = slot("wood");
    let villager = first_villager(&world);
    let scale = world.scale();
    let from = world.position(villager).expect("a position");

    // Two trees of our own: a scrap right beside the villager and a full one two tiles further out. The scrap empties
    // in one trip, so the test measures the re-task rather than a two-hundred-tick walk.
    let mut place = |offset: i64, amount: u32| -> EntityId {
        let id = EntityId(
            world
                .entities()
                .iter()
                .map(|entity| entity.id.0)
                .max()
                .unwrap_or(0)
                + 1,
        );
        world
            .submit_checked(Command::Spawn {
                id,
                player: 0,
                kind: EntityKind::Resource(wood),
                unit: None,
                position: (
                    Fixed::from_raw(from.0.raw() + offset * scale),
                    Fixed::from_raw(from.1.raw() + offset * scale),
                ),
                speed: Fixed::ZERO,
                hp: 0,
                amount,
            })
            .expect("a tree");
        world.step().expect("one tick");
        id
    };
    // Three whole units, not one: a sub-unit load cannot be banked (the deposit takes whole resources), so a
    // one-unit node leaves the villager holding 0.99 of nothing and the test measures the wrong thing.
    let scrap = place(2, 3);
    let _plenty = place(4, 100);

    world
        .submit_checked(Command::Gather {
            villager,
            node: scrap,
        })
        .expect("a villager can be sent at a tree");

    // The rule is "the nearest source of the same kind", not "the one this test planted": a map tree may well be
    // nearer, and insisting on the seeded one would fail a correct engine (it did, on the first run).
    let mut retasked = None;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if let Some(ruage_sim::Task::Gather { node, .. }) =
            world.entity(villager).map(|entity| entity.task)
            && node != scrap
        {
            retasked = Some(node);
            break;
        }
    }
    let next = retasked.unwrap_or_else(|| {
        panic!(
            "a lumberjack whose tree ran out walks to the next tree; task is {:?}, refusal {:?}",
            world.entity(villager).map(|entity| entity.task),
            world
                .last_refusal()
                .map(|(id, why, _)| (id, why.to_string()))
        )
    });
    assert!(
        matches!(
            world.entity(next).map(|entity| entity.kind),
            Some(EntityKind::Resource(slot)) if slot == wood
        ),
        "and the next node is wood, not something else: {next:?}"
    );
}
