//! **Why a route was re-planned** (D193): a body, or the ground. Written before the counters existed, because D192 landed a
//! **+70% rise** in repaths (240 -> 409) and could not say which kind had grown - and "more traffic" and "bodies costing more
//! work" call for opposite responses.
//!
//! The split is instrumentation, not state: nothing reads it in the tick and it is not hashed. The claim pinned here is the one
//! an instrument has to keep: the two parts add up to the total, and a body on the line is counted as a body.

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

fn spawn_at(world: &mut World, player: u8, point: (i64, i64)) -> EntityId {
    let id = next_id(world);
    spawn_with(world, id, player, point)
}

fn spawn_with(world: &mut World, id: EntityId, player: u8, point: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.start_villager_unit().expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(point.0), Fixed::from_raw(point.1)),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    id
}

/// The first row of eight clear tiles outside the seeded base, so the scene is not inside a forest.
fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    for y in 2..62u16 {
        for x in 2..62u16 - span {
            if (0..span).all(|d| world.is_free_ground(x + d, y))
                && !((8..24).contains(&x) && (8..24).contains(&y))
            {
                return Some((x, y));
            }
        }
    }
    None
}

/// **A body in the way is counted as a body**, and the two parts are the total.
///
/// The scene is evasion's: the walker starts on a tile corner and walks six tiles east, and the enemy stands at the **midpoint of
/// the two points**, not on a tile centre - a body aligned to a tile leaves the walker a slide (rule 5) and no repath at all, which
/// is what a first version of this test measured (total 0 after 300 ticks).
#[test]
fn a_body_on_the_line_is_counted_as_a_body() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let scale = world.scale();
    // **Where the walker actually walks.** A body at the midpoint of start and goal is not on the route: the route runs tile centre to
    // tile centre, and a first version of this test placed its enemy on the straight line and measured no collision at all. So the
    // route is sampled first, without an enemy, and the enemy is put on it.
    let on_route = {
        let mut probe = common::fresh().expect("a second world");
        let id = spawn_at(&mut probe, 1, (i64::from(x) * scale, i64::from(y) * scale));
        probe.step().expect("the spawn lands");
        probe
            .submit_checked(Command::Move {
                unit: id,
                to: (x + 6, y),
            })
            .expect("a walk");
        let mut sampled = None;
        for _ in 0..600 {
            probe.step().expect("a tick");
            let at = probe.position(id).expect("the walker");
            if at.0.raw() >= i64::from(x + 3) * scale {
                sampled = Some(at);
                break;
            }
        }
        match sampled {
            Some(point) => point,
            None => return,
        }
    };
    let walker = spawn_at(&mut world, 1, (i64::from(x) * scale, i64::from(y) * scale));
    // **The id is chosen by the caller**: `next_id` reads the entity list and a `Spawn` only lands on the next tick, so two spawns
    // in one tick take the same id - the fixture bug evasion.rs documents, which made this test's enemy overwrite its walker.
    let goal_at = (
        i64::from(x + 6) * scale + scale / 2,
        i64::from(y) * scale + scale / 2,
    );
    let _ = goal_at;
    let _sitter = spawn_with(
        &mut world,
        EntityId(walker.0 + 1),
        2,
        (on_route.0.raw(), on_route.1.raw()),
    );
    world.step().expect("the spawns land");

    let before = world.repaths();
    world.submit(Command::Move {
        unit: walker,
        to: (x + 6, y),
    });
    for _ in 0..2_000 {
        world.step().expect("a tick");
        if world.repaths() > before {
            break;
        }
    }
    let total = world.repaths();
    let body = world.repaths_by_body();
    let ground = world.repaths_by_ground();
    assert_eq!(
        total,
        body + ground,
        "MEASURED repaths total {total}, body {body}, ground {ground} - the parts must be the whole"
    );
    assert!(
        total > before,
        "MEASURED no repath at all after 2,000 ticks with an enemy on the line; the scene never met the body"
    );
    assert_eq!(
        body, total,
        "MEASURED {total} repaths of which {body} were blamed on the body; a body on the line is the body's business,          not the ground's"
    );
}
