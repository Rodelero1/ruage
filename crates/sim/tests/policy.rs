//! Villager shares: the engine's side of "keep four on wood" (the game's own AI shape, `docs/ai-of-the-game.md`).
//!
//! The AI never assigns a villager to a tree. It states a **percentage per resource** and the game moves idle villagers
//! to meet it. Written before the implementation (D7).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

fn villagers(world: &World) -> Vec<EntityId> {
    let villager = world.start_villager_unit().expect("a villager role");
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Unit
                && world.is_family(entity.unit.unwrap_or(0), villager)
        })
        .map(|entity| entity.id)
        .collect()
}

/// What a villager is working, as the resource's name, from its gather task's node.
fn working_on(world: &World) -> Vec<String> {
    let mut out = Vec::new();
    for id in villagers(world) {
        let Some(entity) = world.entity(id) else {
            continue;
        };
        let node = match entity.task {
            ruage_sim::Task::Gather { node, .. } | ruage_sim::Task::Return { node, .. } => node,
            _ => continue,
        };
        if let Some(node) = world.entity(node) {
            let slot = match node.kind {
                EntityKind::Resource(slot) => Some(slot),
                EntityKind::Building => world.slot_of("food"),
                _ => None,
            };
            if let Some(slot) = slot
                && let Some(name) = world.resources().and_then(|pack| pack.name(slot))
            {
                out.push(name.to_owned());
            }
        }
    }
    out
}

/// A share table the engine applies: everyone on food, then everyone on wood.
#[test]
fn idle_villagers_follow_the_shares() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let food = world.slot_of("food").expect("food");
    let wood = world.slot_of("wood").expect("wood");

    // Everyone on food: three villagers are idle, and the share says food only.
    world.set_villager_shares(&[(0, vec![(food, 100)])]);
    for _ in 0..200 {
        world.step().expect("stepping");
    }
    let on_food = working_on(&world)
        .iter()
        .filter(|name| name.as_str() == "food")
        .count();
    assert_eq!(
        on_food,
        villagers(&world).len(),
        "every villager should be on food: {:?}",
        working_on(&world)
    );

    // Now the table says wood only: the engine moves them.
    world.set_villager_shares(&[(0, vec![(wood, 100)])]);
    let mut moved = 0;
    for _ in 0..600 {
        world.step().expect("stepping");
        moved = working_on(&world)
            .iter()
            .filter(|name| name.as_str() == "wood")
            .count();
        if moved == villagers(&world).len() {
            break;
        }
    }
    assert_eq!(
        moved,
        villagers(&world).len(),
        "the shares changed, so the villagers should have: {:?}",
        working_on(&world)
    );
}

/// A split table puts the right *number* on each resource, not merely "some".
#[test]
fn a_split_share_divides_the_villagers() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let food = world.slot_of("food").expect("food");
    let wood = world.slot_of("wood").expect("wood");
    world.set_villager_shares(&[(0, vec![(food, 50), (wood, 50)])]);
    for _ in 0..900 {
        world.step().expect("stepping");
    }
    let counts = working_on(&world);
    let on_wood = counts.iter().filter(|name| name.as_str() == "wood").count();
    let on_food = counts.iter().filter(|name| name.as_str() == "food").count();
    assert!(
        on_wood >= 1 && on_food >= 1,
        "a 50/50 table divides them: {on_wood} on wood, {on_food} on food"
    );
}

/// The table is by villager count, exactly as the game's AI writes it: the row with the highest `at_villagers` the
/// player has reached wins.
#[test]
fn the_share_row_follows_the_villager_count() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let food = world.slot_of("food").expect("food");
    let wood = world.slot_of("wood").expect("wood");
    // With fewer than eight villagers food only; from eight upwards, wood.
    world.set_villager_shares(&[(0, vec![(food, 100)]), (8, vec![(wood, 100)])]);
    for _ in 0..300 {
        world.step().expect("stepping");
    }
    let three = working_on(&world);
    assert!(
        three.iter().all(|name| name != "wood"),
        "three villagers are below the row's threshold: {three:?}"
    );
    // Five more villagers, spawned rather than trained: this test is about the row, not about the queue.
    let villager_unit = world.role_unit("villager").expect("a villager role");
    let pace = villagers(&world)
        .first()
        .and_then(|id| world.entity(*id))
        .map(|entity| entity.speed)
        .expect("a villager with a speed");
    let tc = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.position)
        .expect("a town centre");
    let start = villagers(&world).len();
    for index in 0..5u32 {
        let id = EntityId(9_000 + index);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager_unit),
                position: (
                    Fixed::from_raw(tc.0.raw() + (index as i64 + 2) * 65_536),
                    tc.1,
                ),
                speed: pace,
                hp: 25,
                amount: 0,
            })
            .expect("a villager can be spawned");
    }
    for _ in 0..200 {
        world.step().expect("stepping");
    }
    assert_eq!(
        villagers(&world).len(),
        start + 5,
        "the spawned villagers exist"
    );
    for _ in 0..600 {
        world.step().expect("stepping");
    }
    let eight = working_on(&world);
    assert!(
        eight.iter().any(|name| name.as_str() == "wood"),
        "at eight villagers the second row applies and they go to wood: {eight:?}"
    );
}
