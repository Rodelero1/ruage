//! **The one-tile corridor**: the only geometry where "a man who has stopped" decides whether a pair gets past.
//!
//! R1's arm (`TakeAtRest`) let a walker step *through* a body that had stopped and was not yet **past**, which is how the
//! engine resolved a jam it could not otherwise resolve. D242 deleted the arm, measured a two-thousand-tick stand and put it
//! back; the stand it measured was in **open ground**, where the pencil can now bend a route round a standing body - so the
//! fixture that reported it no longer forces an encounter. This one does: walls above and below a single lane, so there is no
//! detour to take, and the only ways past are the game's two - the pause, then the collider becoming **past** - or walking
//! through him.
//!
//! Written before the change it measures (D7): with the arm in place this passes by stepping through the other man; with the
//! arm deleted it must still pass, which is what R1a is for.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn spawn_unit(world: &mut World, id: EntityId, unit: u32, x: u16, y: u16) {
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("spawn");
}

fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    spawn_unit(world, id, unit, x, y);
}

fn wall(world: &mut World, id: EntityId, x: u16, y: u16) {
    let scale = world.scale();
    let unit = world.role_unit("palisade_wall").expect("a wall role");
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 250,
            amount: 0,
        })
        .expect("spawn a wall");
    world.step().expect("the wall lands");
}

fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    for y in 2..62u16 {
        for x in 2..(64 - span) {
            if (0..span).all(|d| world.is_free_ground(x + d, y)) {
                if (8..24).contains(&x) && (8..24).contains(&y) {
                    continue;
                }
                return Some((x, y));
            }
        }
    }
    None
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity has a position");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

/// **Two men in a one-tile lane get past each other** - the case the deleted arm was hiding. Walls above and below, so no
/// route can bend round, and both must reach the other's end within the run.
#[test]
fn a_pair_passes_in_a_one_tile_corridor() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    // Wall the row above and the row below, so the lane is the only ground between the two men.
    let mut wall_id = next_id(&world);
    for d in 0..8u16 {
        wall(&mut world, wall_id, x + d, y - 1);
        wall_id = EntityId(wall_id.0 + 1);
        wall(&mut world, wall_id, x + d, y + 1);
        wall_id = EntityId(wall_id.0 + 1);
    }
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (x + 6, y),
    });
    world.step().expect("a tick between the orders");
    world.submit(Command::Move {
        unit: b,
        to: (x, y),
    });

    let reach = world.reach_between(a, b).raw();
    let mut closest = i64::MAX;
    let mut met = false;
    for _ in 0..2_000 {
        world.step().expect("step");
        let (pa, pb) = (
            world.position(a).expect("alive"),
            world.position(b).expect("alive"),
        );
        let (dx, dy) = (pa.0.raw() - pb.0.raw(), pa.1.raw() - pb.1.raw());
        let apart = (dx * dx + dy * dy).isqrt();
        closest = closest.min(apart);
        if apart <= reach {
            met = true;
        }
    }
    assert!(met, "MEASURED the pair never met in the lane (closest {closest} of a {reach} reach): the fixture is not testing a collision");
    let near = |id: EntityId, goal: (u16, u16)| {
        let (tx, ty) = tile_of(&world, id);
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1
    };
    assert!(
        near(a, (x + 6, y)) && near(b, (x, y)),
        "both have to get past: a at {:?}, b at {:?} after 2,000 ticks, closest {closest} of {reach}",
        tile_of(&world, a),
        tile_of(&world, b)
    );
}
