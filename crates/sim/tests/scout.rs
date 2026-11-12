//! **A scout at the start, and it is the only unit that goes looking.**
//!
//! The owner's note (2026-11-04): "player should start with scout cavalry, villagers shouldn't scout."
//!
//! Two claims, and the second is the one that matters: a villager that wanders off is a villager not gathering, so the
//! scouting rule names a role and only that role walks away from the town centre.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_script::{Runner, Scenario};
use ruage_sim::{EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_research()
}

fn tile_of(world: &World, id: ruage_sim::EntityId) -> (u16, u16) {
    world
        .position(id)
        .map(|position| world.tile_of_position(position))
        .expect("a position")
}

/// How many tiles this player has ever seen, counted from the fog's own bitset.
fn seen(world: &World, player: u8) -> u32 {
    world
        .explored_mask(player)
        .iter()
        .map(|word| word.count_ones())
        .sum()
}

fn scouts(world: &World, player: u8) -> Vec<ruage_sim::EntityId> {
    let scout = world.role_unit("scout_cavalry").expect("a scout role");
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .filter(|entity| entity.unit.is_some_and(|unit| world.is_family(unit, scout)))
        .map(|entity| entity.id)
        .collect()
}

/// **One scout a side, at the start.** The pack says so (`[start] scout`, `scouts_per_player`), and both players get one.
#[test]
fn a_match_starts_with_one_scout_a_side() {
    let Some(world) = fresh() else { return };
    for player in [1, 2] {
        assert_eq!(
            scouts(&world, player).len(),
            1,
            "player {player} does not open with exactly one scout"
        );
    }
}

/// **The scout leaves, and nobody else does.** A rule sends the scout at the nearest ground the player has never seen; the
/// villagers, given no such rule, stay at the town centre and work. The claim is about distance, not about orders: a
/// villager more than eight tiles from home is a villager that was sent scouting by something.
#[test]
fn a_scout_goes_looking_and_the_villagers_do_not() {
    let Some(mut world) = fresh() else { return };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 120.0

[[rule]]
player = 1
cooldown_seconds = 2.0
do = "explore"
role = "scout_cavalry"

[[rule]]
player = 2
cooldown_seconds = 2.0
do = "explore"
role = "scout_cavalry"
"#,
    )
    .expect("a scenario the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");

    let start = tile_of(&world, scouts(&world, 1)[0]);
    let explored_before = seen(&world, 1);
    for _ in 0..2400 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    let end = tile_of(&world, scouts(&world, 1)[0]);
    let explored_after = seen(&world, 1);
    assert!(
        explored_after > explored_before,
        "the scout revealed nothing: {explored_before} tiles before, {explored_after} after"
    );
    let moved = (i32::from(end.0) - i32::from(start.0)).abs()
        + (i32::from(end.1) - i32::from(start.1)).abs();
    assert!(
        moved >= 4,
        "the scout did not go anywhere: {start:?} to {end:?}"
    );

    // The villagers: nobody of the worker role is far from home.
    let villager = world.role_unit("villager").expect("a villager role");
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| tile_of(&world, entity.id))
        .expect("a town centre");
    for entity in world.entities() {
        if entity.player != 1 || entity.kind != EntityKind::Unit {
            continue;
        }
        if !entity
            .unit
            .is_some_and(|unit| world.is_family(unit, villager))
        {
            continue;
        }
        let tile = tile_of(&world, entity.id);
        let away = (i32::from(tile.0) - i32::from(centre.0)).abs()
            + (i32::from(tile.1) - i32::from(centre.1)).abs();
        assert!(
            away <= 8,
            "a villager was sent {away} tiles from the town centre at {tile:?}: villagers do not scout"
        );
    }
}
