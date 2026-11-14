//! **A small enough pencil** (D209): a route must be able to bend round a *body* at less than a tile's clearance.
//!
//! The original's search works on continuous rectangles - each body a box of `pos +- radius`, grown by the mover's radius -
//! so it can thread a gap between a man and a wall that is narrower than a tile. Ours builds its graph out of **tile
//! corners**, so the only detours it can express are whole-tile ones: beside a body that stands in a corridor, or next to
//! the walker himself, there is no such detour, the search fails, the fallback draws a straight line, and the walker ghosts
//! through him - measured at 4207 raw against an 8000 reach (D208).
//!
//! Written before the implementation (D7).

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

fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
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
        .expect("a villager can be placed");
}

fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    let (width, height) = (64u16, 64u16);
    for y in 2..height.saturating_sub(2) {
        for x in 2..width.saturating_sub(span) {
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

fn gap(world: &World, a: EntityId, b: EntityId) -> i64 {
    let (ax, ay) = world.position(a).expect("a has a position");
    let (bx, by) = world.position(b).expect("b has a position");
    let (dx, dy) = (ax.raw() - bx.raw(), ay.raw() - by.raw());
    dx * dx + dy * dy
}

/// **A walker steps round a man standing in his lane.** The stander never moves, so the walker's route is planned after he
/// stopped and the search must treat him as ground: the way past is a sub-tile curve round his shoulder, not a whole-tile
/// detour, because there is no free tile between him and the lane.
///
/// The assertion is the reach, not the arrival: a walker that ghosts through the man still *arrives*, which is exactly why
/// this could not be caught by an arrival test.
#[test]
fn a_walker_steps_round_a_man_standing_in_his_lane() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let stander = EntityId(walker.0 + 1);
    // The stander sits **on** the lane, two tiles along, and never gets an order.
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, stander, x + 2, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 5, y),
    });
    world.step().expect("the order lands");

    let reach = world.reach_between(walker, stander).raw();
    let mut closest = i64::MAX;
    for _ in 0..1_200 {
        world.step().expect("step");
        closest = closest.min(gap(&world, walker, stander));
    }
    assert!(
        (tile_of(&world, walker).0 - i64::from(x + 5)).abs() <= 1,
        "the walker has to arrive: he is at {:?}, sent to ({}, {}) - a search that cannot find the shoulder must still \
         reach the goal",
        tile_of(&world, walker),
        x + 5,
        y
    );
    assert!(
        closest >= reach * reach,
        "MEASURED the walker came {} raw from the man standing in his lane, inside the {} raw his footprint needs: the \
         plan has no sub-tile detour, so the fallback drew a straight line through him",
        closest.max(0).isqrt(),
        reach
    );
}

/// **The same, one tile out.** A man standing *beside* the lane - close enough that his shoulder is in the way, far enough
/// that a whole-tile detour exists - must still be cleared, and the walker must not take the long way round the whole
/// block to do it. This is the guard that keeps the sub-tile fix from becoming a licence to route absurdly.
#[test]
fn a_man_beside_the_lane_is_cleared_without_a_detour_round_the_block() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let stander = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    // Half a tile off the lane, so his disc overlaps the straight line but the tiles on either side are free.
    let scale = world.scale();
    let unit = world.start_villager_unit().expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id: stander,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x + 3) * scale),
                Fixed::from_raw(i64::from(y) * scale + scale / 2),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 6, y),
    });

    let reach = world.reach_between(walker, stander).raw();
    let mut closest = i64::MAX;
    for _ in 0..1_200 {
        world.step().expect("step");
        closest = closest.min(gap(&world, walker, stander));
    }
    assert!(
        closest >= reach * reach,
        "MEASURED the walker came {} raw from a man standing half a tile off his lane, inside the {} raw reach",
        closest.max(0).isqrt(),
        reach
    );
}
