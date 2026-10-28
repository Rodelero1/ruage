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

/// Every resource this player has, by slot.
fn stockpile(world: &World, player: u8) -> Vec<u32> {
    (0..4).map(|slot| world.stock(player, slot)).collect()
}

#[test]
fn deleting_a_foundation_refunds_it_in_full() {
    // Verified against the game's behaviour (forums, multiple independent statements): a foundation
    // that has not been worked gives 100% back, and a partially built one gives back the part you have
    // not built yet. Losing what you have already invested is the rule.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villein = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    let before = stockpile(&world, 1);
    let house = world.role_unit("house").expect("house");
    // Four tiles away, so the builder is still walking and the site is a bare foundation.
    let tile = world
        .entity(villein)
        .map(|entity| {
            let scale = world.scale();
            (
                entity.position.0.raw() / scale + 4,
                entity.position.1.raw() / scale,
            )
        })
        .expect("the villager");
    world
        .submit_checked(Command::Build {
            villager: villein,
            unit: house,
            at: (tile.0 as u16, tile.1 as u16),
        })
        .expect("a foundation");
    world.step().expect("step");

    let site = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.construction.is_some())
        .map(|entity| entity.id)
        .expect("the foundation");
    assert_eq!(
        world
            .entity(site)
            .and_then(|entity| entity.construction)
            .map(|construction| construction.progress_ticks),
        Some(0),
        "the test's premise is wrong: the foundation has been worked"
    );
    let paid = stockpile(&world, 1);
    assert_ne!(paid, before, "placing the foundation cost nothing");

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: site,
        })
        .expect("delete");
    world.step().expect("step");
    assert_eq!(
        stockpile(&world, 1),
        before,
        "an untouched foundation did not refund in full"
    );
}

/// Build a house next to a villager, work it until `fraction` of its time has passed, delete it, and
/// report how much wood came back. Two calls on two fresh worlds give the shape of the refund curve
/// without the test needing to know what a house costs - content owns that number.
fn refund_after_work(fx: &Fixture, fraction: f64) -> u32 {
    let mut world = seeded(fx);
    let villein = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    let house = world.role_unit("house").expect("house");
    let (x, y) = world
        .entity(villein)
        .map(|entity| {
            let scale = world.scale();
            (
                entity.position.0.raw() / scale,
                entity.position.1.raw() / scale,
            )
        })
        .expect("the villager");
    let mut placed = false;
    for (dx, dy) in [(1i64, 0i64), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1)] {
        let order = world.submit_checked(Command::Build {
            villager: villein,
            unit: house,
            at: ((x + dx) as u16, (y + dy) as u16),
        });
        if order.is_ok() {
            placed = true;
            break;
        }
    }
    assert!(placed, "no neighbour of the villager would take a house");
    world.step().expect("step");
    let site = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.construction.is_some())
        .map(|entity| entity.id)
        .expect("the foundation");
    for _ in 0..5000 {
        let Some((done, total)) = world
            .entity(site)
            .and_then(|entity| entity.construction)
            .map(|construction| (construction.progress_ticks, construction.total_ticks))
        else {
            break;
        };
        if f64::from(done) >= f64::from(total) * fraction {
            break;
        }
        world.step().expect("step");
    }
    let before = stockpile(&world, 1);
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: site,
        })
        .expect("delete");
    world.step().expect("step");
    // Whatever slot content puts a house's price in: sum the lot rather than assume an order.
    let after: u32 = stockpile(&world, 1).iter().sum();
    let was: u32 = before.iter().sum();
    after.saturating_sub(was)
}

#[test]
fn the_further_a_building_is_built_the_less_deleting_it_returns() {
    // The game's rule, verified from several independent accounts: you lose what you have already put
    // in. A foundation gives everything back, a half-built house about half, a finished one nothing.
    let Some(fx) = fixture() else { return };
    let untouched = refund_after_work(&fx, 0.0);
    let half = refund_after_work(&fx, 0.5);
    assert!(untouched > 0, "an untouched foundation refunded nothing");
    assert!(half > 0, "a half-built house refunded nothing");
    assert!(
        half < untouched,
        "a half-built house refunded as much as an untouched one: {half} vs {untouched}"
    );
    assert!(
        half.abs_diff(untouched / 2) <= 2,
        "a half-built house refunded {half}, which is not about half of {untouched}"
    );
}

#[test]
fn deleting_a_finished_building_refunds_nothing() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house = build(&mut world, 1, 9_100, "house", (21, 21));
    let before = stockpile(&world, 1);
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: house,
        })
        .expect("delete");
    world.step().expect("step");
    assert_eq!(
        stockpile(&world, 1),
        before,
        "deleting a finished building paid its owner back"
    );
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
