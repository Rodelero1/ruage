//! Deleting a thing you own, and the two ways a match can end.
//!
//! Written before the implementation (D7). Deleting is smaller than it looks: every consequence a
//! deletion has - freed population, a garrison that dies with its shelter, a route nobody walks
//! any more - is already the *death* path, so the command sets health to zero and lets the end of
//! the tick do the rest. The alternative, removing the entity inside `apply`, would run that path
//! from a place where the entity list is supposed to be fixed.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, MatchOutcome, SimError, World};

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
    combat: ruage_data::Combat,
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

/// Put a building of a content role on the map at a tile. Speed comes from the unit table, as it
/// would for a finished building.
fn build(world: &mut World, player: u8, id: u32, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn own_town_centre(world: &World, player: u8) -> EntityId {
    let role = world.role_unit("town_centre").expect("town centre role");
    world
        .entities()
        .iter()
        .find(|e| e.player == player && e.kind == EntityKind::Building && e.unit == Some(role))
        .expect("the town centre")
        .id
}

#[test]
fn a_player_can_raze_its_own_building() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house = build(&mut world, 1, 9_000, "house", (20, 20));
    let before = world.population_cap(1);

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: house,
        })
        .expect("razing your own house is legal");
    world.step().expect("step");

    assert!(
        world.entity(house).is_none(),
        "the house is still standing after being razed"
    );
    assert_eq!(
        world.population_cap(1),
        before - 5,
        "the cap still counts a house that is gone"
    );
}

#[test]
fn a_player_cannot_raze_something_it_does_not_own() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let enemy = own_town_centre(&world, 2);
    let err = world
        .submit_checked(Command::Delete {
            player: 1,
            entity: enemy,
        })
        .expect_err("an enemy building is not yours to delete");
    assert!(
        matches!(err, SimError::NotYours { id, player: 1 } if id == enemy),
        "expected NotYours, got {err:?}"
    );
}

#[test]
fn a_resource_node_cannot_be_deleted() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let node = world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(_)))
        .expect("a node on the authored map")
        .id;
    let err = world
        .submit_checked(Command::Delete {
            player: 1,
            entity: node,
        })
        .expect_err("a tree belongs to nobody");
    assert!(
        matches!(err, SimError::NotYours { id, player: 1 } if id == node),
        "expected NotYours, got {err:?}"
    );
}

#[test]
fn a_deleted_building_releases_its_garrison() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = own_town_centre(&world, 1);
    let villager = place(&mut world, 1, 9_001, fx.economy.role["villager"], (17, 17));
    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: centre,
        })
        .expect("garrison");
    world.step().expect("step");

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: centre,
        })
        .expect("raze");
    world.step().expect("step");

    assert!(world.entity(centre).is_none(), "the centre still stands");
    // Razing your own building is demolishing it, not losing it in a fight: the villagers inside
    // come out alive. An enemy razing the same building kills them (D46a).
    let survivor = world
        .entity(villager)
        .expect("a shelter you dismantle yourself lets its occupants out");
    assert_eq!(survivor.task, ruage_sim::Task::Idle);
    assert_eq!(survivor.hp, 40, "they came out unharmed");
}

#[test]
fn deleting_a_unit_frees_its_population() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let before = world.population(1);
    let villager = place(&mut world, 1, 9_002, fx.economy.role["villager"], (17, 19));
    assert_eq!(world.population(1), before + 1);
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: villager,
        })
        .expect("raze");
    world.step().expect("step");
    assert_eq!(world.population(1), before, "the dead still count");
}

#[test]
fn razing_the_town_centre_is_not_the_end_of_a_player() {
    // D59 replaced D46's seat rule: a razed base is a setback, because the villagers who survived it can
    // build another. The player is out only when nothing that counts is left.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    assert_eq!(world.outcome(), None, "a match at tick zero is undecided");

    let mine = own_town_centre(&world, 1);
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: mine,
        })
        .expect("raze");
    world.step().expect("step");

    assert_eq!(
        world.outcome(),
        None,
        "player 1 lost the match with villagers still standing to rebuild"
    );

    // And with the villagers gone too, the match does end.
    for id in world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1)
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        world
            .submit_checked(Command::Delete {
                player: 1,
                entity: id,
            })
            .expect("raze");
    }
    world.step().expect("step");
    assert_eq!(
        world.outcome(),
        Some(MatchOutcome::Victory { winner: 2 }),
        "player 1 was left with nothing and the match did not end"
    );
}

#[test]
fn losing_a_house_does_not_end_the_match() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house = build(&mut world, 1, 9_003, "house", (20, 20));
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: house,
        })
        .expect("raze");
    world.step().expect("step");
    assert_eq!(world.outcome(), None, "a house is not a seat");
}

#[test]
fn both_sides_wiped_on_one_tick_is_a_draw() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    for player in [1u8, 2u8] {
        for id in world
            .entities()
            .iter()
            .filter(|entity| entity.player == player)
            .map(|entity| entity.id)
            .collect::<Vec<_>>()
        {
            world
                .submit_checked(Command::Delete { player, entity: id })
                .expect("raze");
        }
    }
    world.step().expect("step");
    assert_eq!(world.outcome(), Some(MatchOutcome::Draw));
}

#[test]
fn an_ended_match_refuses_new_orders() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    for id in world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1)
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        world
            .submit_checked(Command::Delete {
                player: 1,
                entity: id,
            })
            .expect("raze");
    }
    world.step().expect("step");

    // Player 1 is gone, so the order under test is player 2's: an ended match refuses everyone.
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 2 && e.kind == EntityKind::Unit)
        .expect("a starting villager")
        .id;
    let err = world
        .submit_checked(Command::Stop { unit: villager })
        .expect_err("the match is over");
    assert!(
        matches!(err, SimError::MatchOver),
        "expected MatchOver, got {err:?}"
    );
}
