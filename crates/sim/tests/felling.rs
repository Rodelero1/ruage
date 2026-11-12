//! **A tree takes time to fell** (D129). The pack says a villager's chop is `fell_hp_per_second = 1.5` - "3 hit points
//! every 2 seconds", so a 20 hit point tree stands for 13.3 seconds and falls on the seventh chop. That is the owner's
//! rule and the economy file's own prose; what the engine did with the number was another matter, which is what these
//! tests pin down.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh()
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

/// A tree with the dat's 20 hit points on open ground, and a villager one tile away, already gathering it.
fn a_tree_and_a_chopper(world: &mut World) -> (EntityId, EntityId) {
    let wood = world.slot_of("wood").expect("a wood column");
    let scale = world.scale();
    let tree = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id: tree,
            player: 0,
            kind: EntityKind::Resource(wood),
            unit: None,
            position: (Fixed::from_raw(30 * scale), Fixed::from_raw(30 * scale)),
            speed: Fixed::ZERO,
            hp: 20,
            amount: 1_000,
        })
        .expect("a tree");
    world.step().expect("one tick");

    let villager = world.role_unit("villager").expect("a villager role");
    let pace = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1 && entity.unit.is_some_and(|u| world.is_family(u, villager))
        })
        .map(|entity| entity.speed)
        .expect("a villager with a speed");
    let worker = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id: worker,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(31 * scale), Fixed::from_raw(30 * scale)),
            speed: pace,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("one tick");
    world
        .submit_checked(Command::Gather {
            villager: worker,
            node: tree,
        })
        .expect("the villager takes the tree");
    (tree, worker)
}

/// **Ten seconds in, the tree is still standing.** At 1.5 hit points a second it has lost 15 of its 20.
#[test]
fn a_tree_is_still_standing_ten_seconds_in() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (tree, _) = a_tree_and_a_chopper(&mut world);
    let hz = world.tick_rate_hz();
    for _ in 0..(10 * hz) {
        world.step().expect("stepping");
    }
    let hp = world.entity(tree).map_or(0, |entity| entity.hp);
    assert!(
        hp > 0,
        "a 20 hp tree must still stand 10 s into a 1.5 hp/s chop, but its hit points are {hp}"
    );
    assert!(
        hp <= 20 - 10,
        "ten seconds at 1.5 hp/s must have taken at least ten points off, but {hp} are left"
    );
}

/// **Sixteen seconds in, it is down.** 20 hit points at 1.5 a second is 13.3 seconds - the seventh chop - and the woodcutter
/// spends up to two seconds walking out to the tile's border before the first chop (D192: 0.70 from its centre), so the
/// window is the chop plus the approach. The *rate* is `the_chop_is_a_rate_per_second`'s business, not this window's.
#[test]
fn a_tree_is_down_within_sixteen_seconds() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (tree, _) = a_tree_and_a_chopper(&mut world);
    let hz = world.tick_rate_hz();
    for _ in 0..(16 * hz) {
        world.step().expect("stepping");
    }
    let hp = world.entity(tree).map_or(0, |entity| entity.hp);
    assert_eq!(
        hp, 0,
        "a 20 hp tree must be down after 13.3 s of chopping plus a walk to the ring, but {hp} hit points are left after 16 s"
    );
}

/// **The rate is per second, not per tick.** The same wall-clock chop at another tick rate must fell the tree at the same
/// moment: a rate that scaled with the tick rate would fell it 40 times faster at 40 Hz than at 1 Hz.
#[test]
fn the_chop_is_a_rate_per_second() {
    let Some(mut world) = fresh() else {
        return;
    };
    let (tree, _) = a_tree_and_a_chopper(&mut world);
    let hz = world.tick_rate_hz();
    // Half the intended lifetime: still standing, whatever the tick rate. If the rate were per tick, 40 Hz would have
    // felled twenty trees by now.
    for _ in 0..(hz * 8) {
        world.step().expect("stepping");
    }
    let hp = world.entity(tree).map_or(0, |entity| entity.hp);
    assert!(
        hp > 0,
        "8 s into a 13.3 s chop the tree must stand at {hz} Hz, but it is down"
    );
}
