//! What a player knows about an enemy army, and what it does not (D92).
//!
//! The anti-cheat is structural: a rule may read this memory and nothing else, so an army nobody has watched is
//! **unknown**, not merely distant. Written before the implementation (D7), from the decision the owner made on
//! 2026-11-01: knowledge is a memory of what was seen, never the world's ground truth.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

const KNIGHT: u32 = 38;
const VILLAGER: u32 = 83;

fn place(world: &mut World, id: u32, player: u8, unit: u32, x: i64, y: i64, hp: i32) {
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale)),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("a spawn");
}

/// A world with one player-1 villager at (60, 60) and nothing else of player 1's in that corner, so the only thing
/// the villager can see is what a test puts near it.
fn watching(fx: &Fixture) -> World {
    let mut world = seeded(fx);
    // A memory is installed by the match, not by the loader (D75's shape): a world nobody has given an AI pays nothing.
    world.observe_for(1);
    place(&mut world, 900, 1, VILLAGER, 60, 60, 25);
    // Sightings are taken once a game second; twenty ticks at 20 Hz puts the world on an observation tick.
    for _ in 0..20 {
        world.step().expect("a tick");
    }
    world
}

/// Step to the next observation: sightings are taken when the tick is a whole number of seconds.
fn a_second(world: &mut World) {
    for _ in 0..20 {
        world.step().expect("a tick");
    }
}

#[test]
fn an_army_nobody_has_watched_is_unknown() {
    let Some(fx) = fixture() else { return };
    let mut world = watching(&fx);
    place(&mut world, 901, 2, KNIGHT, 200, 200, 100);
    a_second(&mut world);

    // The knight exists and is nowhere near player 1's sight: nothing is known about it.
    assert!(
        world
            .entities()
            .iter()
            .any(|entity| entity.id == EntityId(901)),
        "the knight is on the map"
    );
    assert_eq!(world.sighting(1, KNIGHT), None);
}

#[test]
fn a_watched_enemy_is_counted_at_the_tick_it_was_seen() {
    let Some(fx) = fixture() else { return };
    let mut world = watching(&fx);
    place(&mut world, 901, 2, KNIGHT, 62, 60, 100);
    a_second(&mut world);

    let (count, at) = world.sighting(1, KNIGHT).expect("the knight is in sight");
    assert_eq!(count, 1);
    assert_eq!(
        at,
        world.tick(),
        "the sighting is stamped with the tick that saw it"
    );
}

#[test]
fn a_sighting_outlives_the_sight() {
    let Some(fx) = fixture() else { return };
    let mut world = watching(&fx);
    place(&mut world, 901, 2, KNIGHT, 62, 60, 100);
    a_second(&mut world);
    let (_, seen_at) = world.sighting(1, KNIGHT).expect("seen");

    // The knight is gone. A memory is not a view: the sighting stays, stamped with when it happened, so a rule can
    // decide for itself how stale is too stale.
    world
        .submit_checked(Command::Delete {
            player: 2,
            entity: EntityId(901),
        })
        .expect("delete");
    a_second(&mut world);
    a_second(&mut world);

    let (count, at) = world.sighting(1, KNIGHT).expect("still believed");
    assert_eq!(
        count, 1,
        "the count is what was seen, not what is there now"
    );
    assert_eq!(
        at, seen_at,
        "the stamp does not move when nothing new is seen"
    );
    assert!(world.tick() > seen_at, "and time has passed since");
}

#[test]
fn one_players_sighting_is_not_anothers() {
    let Some(fx) = fixture() else { return };
    let mut world = watching(&fx);
    place(&mut world, 901, 2, KNIGHT, 62, 60, 100);
    a_second(&mut world);

    assert_eq!(world.sighting(1, KNIGHT).map(|s| s.0), Some(1));
    // Player 3 was not there and has nothing within sight of that tile.
    assert_eq!(world.sighting(3, KNIGHT), None);
}

#[test]
fn a_second_enemy_of_the_same_unit_raises_the_count() {
    let Some(fx) = fixture() else { return };
    let mut world = watching(&fx);
    place(&mut world, 901, 2, KNIGHT, 62, 60, 100);
    place(&mut world, 902, 2, KNIGHT, 61, 61, 100);
    a_second(&mut world);
    assert_eq!(world.sighting(1, KNIGHT).map(|s| s.0), Some(2));
}

#[test]
fn a_run_of_the_same_world_produces_the_same_memory() {
    // Determinism: the memory is rebuilt every tick from the same pass, so two worlds fed the same commands must hold
    // the same map, in the same order, and hash alike.
    let Some(fx) = fixture() else { return };
    let mut a = watching(&fx);
    let mut b = watching(&fx);
    for (world, id) in [(&mut a, 901u32), (&mut b, 901)] {
        place(world, id, 2, KNIGHT, 62, 60, 100);
        place(world, id + 1, 2, KNIGHT, 70, 70, 100);
        for _ in 0..40 {
            world.step().expect("a tick");
        }
    }
    assert_eq!(a.sighting(1, KNIGHT), b.sighting(1, KNIGHT));
    assert_eq!(a.state_hash(), b.state_hash());
}
