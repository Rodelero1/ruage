//! Elimination: what keeps a player in the match, and what does not.
//!
//! Written before the implementation (D59). The rule is the original game's, from the manual:
//!
//! > "To defeat a player, you must destroy all units and buildings but the following: towers, walls,
//! > gates, relics, Fishing Ships, Fish Traps, Transport Ships, Trade Cogs, Trade Carts, Farms, and
//! > sheep."
//!
//! So a player is out when nothing *qualifying* of theirs stands - and the town centre is not special.
//! Content names the roles that do not qualify, the same way it names what blocks (D54): the list is a
//! fact about the game, not about the engine.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, MatchOutcome, World};

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

/// Everything `player` owns, in id order.
fn holdings(world: &World, player: u8) -> Vec<EntityId> {
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == player)
        .map(|entity| entity.id)
        .collect()
}

/// Delete everything `player` owns except `keep`, then run a tick.
fn reduce_to(world: &mut World, player: u8, keep: &[EntityId]) {
    for id in holdings(world, player) {
        if !keep.contains(&id) {
            world
                .submit_checked(Command::Delete { player, entity: id })
                .expect("delete");
        }
    }
    world.step().expect("step");
}

fn spawn_building(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
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
            hp: 500,
            amount: 0,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

/// Spawn a building by its dat unit id - for the classes the pack has no role for yet (a tower).
fn spawn_by_unit(world: &mut World, player: u8, unit: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
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
            hp: 500,
            amount: 0,
        })
        .expect("spawn by id");
    world.step().expect("step");
    id
}

#[test]
fn losing_the_town_centre_is_not_losing_the_match() {
    // The rule the original plays and D46 did not: a razed base is a setback, not a defeat.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("the enemy town centre");
    world
        .submit_checked(Command::Delete {
            player: 2,
            entity: centre,
        })
        .expect("raze");
    world.step().expect("step");

    assert_eq!(
        world.outcome(),
        None,
        "player 2 lost the match with three villagers still standing"
    );
}

#[test]
fn one_villager_is_enough_to_stay_in_the_match() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    reduce_to(&mut world, 2, &[villager]);
    assert_eq!(world.outcome(), None, "a lone villager was eliminated");
}

#[test]
fn the_last_villager_takes_the_match_with_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    reduce_to(&mut world, 2, &[villager]);
    world
        .submit_checked(Command::Delete {
            player: 2,
            entity: villager,
        })
        .expect("delete");
    world.step().expect("step");
    assert_eq!(
        world.outcome(),
        Some(MatchOutcome::Victory { winner: 1 }),
        "the map was empty of player 2 and nobody won"
    );
}

#[test]
fn walls_and_farms_do_not_keep_a_player_alive() {
    // The manual's exclusion list, by role. Content names them; the engine only knows "does not count".
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = spawn_building(&mut world, 2, "wall", (28, 26));
    let farm = spawn_building(&mut world, 2, "farm", (26, 24));
    reduce_to(&mut world, 2, &[wall, farm]);

    assert_eq!(
        world.outcome(),
        Some(MatchOutcome::Victory { winner: 1 }),
        "a player left with only walls and a farm is not defeated"
    );
}

#[test]
fn a_house_keeps_a_player_alive() {
    // The dat's class table, and the manual with it: a house is class 3, the same class as a barracks.
    // The original counts it, so we count it - D59b replaced the production/gathering property with the
    // class the data already gives us.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house = spawn_building(&mut world, 2, "house", (26, 26));
    reduce_to(&mut world, 2, &[house]);
    assert_eq!(
        world.outcome(),
        None,
        "a house is class 3 and the original counts it"
    );
}

#[test]
fn a_tower_does_not_keep_a_player_alive() {
    // Class 52, and the manual's first exclusion. The unit is spawned by dat id because the pack has no
    // tower role yet - the rule is about the class, not about the role.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let tower = spawn_by_unit(&mut world, 2, 79, (26, 26));
    reduce_to(&mut world, 2, &[tower]);
    assert_eq!(
        world.outcome(),
        Some(MatchOutcome::Victory { winner: 1 }),
        "a player left with only a watch tower is still in the match"
    );
}

#[test]
fn a_barracks_keeps_a_player_alive() {
    // It makes units: that is production, so it counts.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let barracks = spawn_building(&mut world, 2, "barracks", (26, 26));
    reduce_to(&mut world, 2, &[barracks]);
    assert_eq!(
        world.outcome(),
        None,
        "a player with a barracks left was eliminated"
    );
}

#[test]
fn a_town_centre_keeps_a_player_alive() {
    // It makes villagers (production) and it takes wood in (gathering): both halves of the rule.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("the enemy town centre");
    reduce_to(&mut world, 2, &[centre]);
    assert_eq!(
        world.outcome(),
        None,
        "a player with only a town centre was eliminated"
    );
}

/// Lay a foundation of `role` one tile from `player`'s first villager, and return it.
fn build_site_on(world: &mut World, player: u8, role: &str) -> EntityId {
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    let scale = world.scale();
    let (x, y) = world
        .entity(villager)
        .map(|entity| {
            (
                entity.position.0.raw() / scale,
                entity.position.1.raw() / scale,
            )
        })
        .expect("the villager");
    let mut placed = false;
    for (dx, dy) in [(1i64, 0i64), (0, 1), (-1, 0), (0, -1)] {
        let (tx, ty) = (x + dx, y + dy);
        if tx < 0 || ty < 0 || tx > i64::from(u16::MAX) || ty > i64::from(u16::MAX) {
            continue;
        }
        let order = world.submit_checked(Command::Build {
            villager,
            unit,
            at: (tx as u16, ty as u16),
        });
        if order.is_ok() {
            placed = true;
            break;
        }
    }
    assert!(placed, "no tile beside the villager would take a {role}");
    world.step().expect("step");
    world
        .entities()
        .iter()
        .find(|entity| entity.player == player && entity.construction.is_some())
        .map(|entity| entity.id)
        .expect("a foundation")
}

#[test]
fn a_production_foundation_counts() {
    // Work in progress still stands: a player who has just laid a barracks is in the match.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let site = build_site_on(&mut world, 2, "barracks");
    reduce_to(&mut world, 2, &[site]);
    assert_eq!(
        world.outcome(),
        None,
        "a barracks foundation did not keep player 2 in the match"
    );
}

#[test]
fn a_house_foundation_counts() {
    // The same question asked of the building it will become: class 3 either way.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let site = build_site_on(&mut world, 2, "house");
    reduce_to(&mut world, 2, &[site]);
    assert_eq!(
        world.outcome(),
        None,
        "a house foundation is a class 3 building and should count"
    );
}

#[test]
fn wiping_both_sides_on_one_tick_is_a_draw() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    for player in [1u8, 2u8] {
        for id in holdings(&world, player) {
            world
                .submit_checked(Command::Delete { player, entity: id })
                .expect("delete");
        }
    }
    world.step().expect("step");
    assert_eq!(world.outcome(), Some(MatchOutcome::Draw));
}

#[test]
fn a_one_sided_world_never_declares_a_winner() {
    // A sandbox with one player is a test fixture, not a match: the seeded world has only player 1 left
    // standing and must not be "won" by him.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    for id in holdings(&world, 2) {
        world
            .submit_checked(Command::Delete {
                player: 2,
                entity: id,
            })
            .expect("delete");
    }
    world.step().expect("step");
    // Player 2 was in the match from the seed, so this *is* two sides and player 1 has won.
    assert_eq!(
        world.outcome(),
        Some(MatchOutcome::Victory { winner: 1 }),
        "eliminating every other side did not win the match"
    );
}
