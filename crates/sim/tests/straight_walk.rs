//! **M1: the step walks the line.** The owner's report, three times over, the third with a screenshot:
//!
//! > "Walking still follows squares and goes through their centers - units go either vertically, horizontally or
//! > diagonally, if unit has to go from point A to point B and point B is 60 degrees in some direction, the path is not a
//! > straight line but instead 45 degree walk followed by horizontal walk."
//!
//! The cause is in `step_towards`: each axis is clamped to its own remaining gap, so a heading of 59 degrees is walked as
//! 45 degrees until the horizontal gap closes and then as 0 degrees. These tests assert the shape of the walk itself -
//! every sampled position stays on the line the order gave - because that is the thing the owner can see, and because a
//! test of the endpoint alone passes for a staircase.
//!
//! Written before the implementation (D7). `docs/decisions.md` D110.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

/// A block of free ground `width` x `height` tiles somewhere on the seeded map, so the walk is unobstructed by
/// construction. The seeded base is around the middle; the first free block far from it is used, and the walk runs from
/// the block's top-left corner.
fn open_block(world: &World, width: u16, height: u16) -> Option<(u16, u16)> {
    let (map_width, map_height) = (64u16, 64u16);
    for y in 2..map_height.saturating_sub(height + 2) {
        for x in 2..map_width.saturating_sub(width + 2) {
            // Leave the seeded base alone: it is woods, mines and the opening villagers.
            if (24..40).contains(&x) && (24..40).contains(&y) {
                continue;
            }
            let clear =
                (0..height).all(|dy| (0..width).all(|dx| world.is_free_ground(x + dx, y + dy)));
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

fn place_villager(world: &mut World, tile: (u16, u16)) -> EntityId {
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            // Half a tile a second: a 12-tile walk is 480 ticks, and every tick is a sample.
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawn lands");
    id
}

/// How far `point` lies off the line `from..to`: `(|cross|, length)` in raw units, so a bound of `b` is the integer
/// comparison `cross * cross <= b * b * length * length` and no square root - or float - is needed anywhere.
fn off_line(point: (i128, i128), from: (i128, i128), to: (i128, i128)) -> (i128, i128) {
    let (line_x, line_y) = (to.0 - from.0, to.1 - from.1);
    let (point_x, point_y) = (point.0 - from.0, point.1 - from.1);
    let cross = (point_x * line_y - point_y * line_x).abs();
    (cross, line_x * line_x + line_y * line_y)
}

/// The integer square root, for the diagnostic only: no assertion needs it.
fn root(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }
    let mut guess = value;
    let mut next = (guess + 1) / 2;
    while next < guess {
        guess = next;
        next = (guess + value / guess) / 2;
    }
    guess
}

/// A walk at fifty-nine degrees is one straight line. The staircase is 45 degrees for six tiles and then flat for four, so
/// its worst point is a little over **two tiles** off the line - this bound is a third of a tile, which is slack for the
/// per-tick rounding and nothing else.
#[test]
fn a_walk_at_fifty_nine_degrees_is_one_straight_line() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_block(&world, 8, 12) else {
        return;
    };
    let villager = place_villager(&mut world, (x, y));

    let goal = (x + 6, y + 10);
    world.submit(Command::Move {
        unit: villager,
        to: goal,
    });
    world.step().expect("the order lands");

    let scale = i128::from(world.scale());
    let start = (i128::from(x) * scale, i128::from(y) * scale);
    let end = (i128::from(goal.0) * scale, i128::from(goal.1) * scale);
    let bound = scale / 3;

    let mut worst: Option<(i128, i128)> = None;
    let mut sampled = 0usize;
    let mut arrived = false;
    for _ in 0..2_000 {
        world.step().expect("step");
        let Some(position) = world.position(villager) else {
            break;
        };
        let point = (i128::from(position.0.raw()), i128::from(position.1.raw()));
        sampled += 1;
        let (cross, length) = off_line(point, start, end);
        if cross * cross > bound * bound * length {
            let worse = worst
                .is_none_or(|(old, old_length)| cross * cross * old_length > old * old * length);
            if worse {
                worst = Some((cross, length));
            }
        }
        let near = (point.0 - end.0).abs() <= scale && (point.1 - end.1).abs() <= scale;
        if near {
            arrived = true;
            break;
        }
    }
    assert!(
        sampled > 100,
        "the walk has to be long enough to have a shape: {sampled} samples"
    );
    assert!(
        arrived,
        "the walker never arrived, so its line was never held"
    );
    // Thousandths of a tile: `|cross| / length` is in raw units, and the scale is raw units to a tile.
    let off_in_thousandths = worst.map_or(0, |(cross, length)| {
        cross * 1_000 / (root(length) * scale).max(1)
    });
    assert!(
        worst.is_none(),
        "the walk left its line by {off_in_thousandths} thousandths of a tile, and a staircase on this \
         walk reaches about 2060"
    );
}

/// Arrival is still exact: a unit walking the line lands **on** the point it was given, which is what the waypoint-spend
/// test and every `position == target` arrival rule in the engine rely on. A proportional step that stopped a hair short
/// of every point would look right and stall everything.
#[test]
fn a_walk_lands_exactly_on_its_point() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_block(&world, 6, 6) else {
        return;
    };
    let villager = place_villager(&mut world, (x + 1, y + 1));
    let goal = (x + 4, y + 3);
    world.submit(Command::Move {
        unit: villager,
        to: goal,
    });
    world.step().expect("the order lands");

    let scale = world.scale();
    for _ in 0..2_000 {
        world.step().expect("step");
        let position = world.position(villager).expect("alive");
        // **The middle of the goal tile is the point** (D114): a tile order is not a corner, and the landing is still exact.
        if position.0.raw() == i64::from(goal.0) * scale + scale / 2
            && position.1.raw() == i64::from(goal.1) * scale + scale / 2
        {
            return;
        }
    }
    let position = world.position(villager).expect("alive");
    panic!(
        "the walker ended at ({}, {}) instead of exactly ({}, {})",
        position.0.raw(),
        position.1.raw(),
        i64::from(goal.0) * scale,
        i64::from(goal.1) * scale
    );
}

/// A short diagonal walk is still a diagonal walk - the smallest case, where a proportional step could truncate to zero on
/// one axis and never move at all.
#[test]
fn a_short_diagonal_walk_still_moves() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_block(&world, 5, 5) else {
        return;
    };
    let villager = place_villager(&mut world, (x, y));
    world.submit(Command::Move {
        unit: villager,
        to: (x + 1, y + 1),
    });
    world.step().expect("the order lands");

    let before = world.position(villager).expect("alive");
    for _ in 0..60 {
        world.step().expect("step");
    }
    let after = world.position(villager).expect("alive");
    assert_ne!(
        before, after,
        "a one-tile diagonal walk did not move at all"
    );
}

/// **A body that cannot move does not creep.** `step_towards` gives every axis with a gap at least one raw unit, so that a
/// line is never abandoned on one axis - which means the zero step needs its own guard, or a unit the pack gives no speed
/// walks a raw unit a tick for ever.
#[test]
fn a_unit_with_no_speed_stays_where_it_is() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_block(&world, 4, 4) else {
        return;
    };
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
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
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawn lands");
    let resting = world.position(id).expect("alive");

    world.submit(Command::Move {
        unit: id,
        to: (x + 3, y + 2),
    });
    for _ in 0..50 {
        world.step().expect("step");
    }
    let later = world.position(id).expect("alive");
    assert_eq!(
        resting, later,
        "a unit with no speed moved: {resting:?} -> {later:?}"
    );
}
