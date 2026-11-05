//! **A route that goes stale must be re-planned, not walked into.** The owner, 2026-11-05: *"I still see scout bumping into
//! trees and getting stuck."*
//!
//! The mover already re-plans when a **body** is in the way (D98a). It does not when **ground** is: `collide` refuses a step onto
//! a blocked tile, tries each axis alone, and then returns `(0, 0)` - the unit stops dead, for ever, because nothing schedules a
//! second look. A route can go stale for ground reasons as easily as for body reasons: a building placed across it, a unit
//! nudged off its line, or a turn clipped by a tree the geometry thought it cleared.
//!
//! `openempire` (`glouw/openempire`, `src/Units.c`) solves this the blunt way and it works: **every unit with a path re-plans
//! every 25 cycles** (`CONFIG_UNIT_PATHING_TIMEOUT_CYCLES`), spread over ten slices a tick, plus boid wall-push. The re-plan is
//! not conditioned on detecting a reason, which is why nothing gets stuck without one.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

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
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

fn next_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// A stretch of clear ground with a free lane between two points, away from the seeded base.
fn lane(world: &World) -> ((i64, i64), (i64, i64)) {
    for y in 40..54u16 {
        for x in 30..48u16 {
            let clear = (0..8).all(|d| world.is_free_ground(x + d, y));
            if clear && world.is_free_ground(x, y) && world.is_free_ground(x + 7, y) {
                return (
                    (i64::from(x), i64::from(y)),
                    (i64::from(x) + 7, i64::from(y)),
                );
            }
        }
    }
    panic!("no clear lane on the map");
}

fn spawn(
    world: &mut World,
    unit: Option<u32>,
    at: (i64, i64),
    kind: EntityKind,
    hp: i32,
) -> EntityId {
    let id = EntityId(next_id(world));
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind,
            unit,
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: if kind == EntityKind::Unit {
                Fixed::from_raw(scale * 12 / 10)
            } else {
                Fixed::ZERO
            },
            hp,
            amount: 0,
        })
        .expect("a spawn on clear ground");
    world.step().expect("one tick");
    id
}

/// **A building dropped across a walk does not freeze the walker.** The route was planned when the ground was free; the house
/// makes it stale; the unit must notice, re-plan and arrive rather than pushing at the wall for the rest of the match.
#[test]
fn a_unit_replans_when_its_route_turns_into_a_wall() {
    let Some(mut world) = fresh() else {
        return;
    };
    let ((sx, sy), (gx, gy)) = lane(&world);
    let villager = world.role_unit("villager");
    let walker = spawn(&mut world, villager, (sx, sy), EntityKind::Unit, 25);
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(gx).expect("in range"),
                u16::try_from(gy).expect("in range"),
            ),
        })
        .expect("a walk along a clear lane");

    // Let it get going, then drop a house across the lane - exactly what a player's base does to a through-route.
    for _ in 0..20 {
        world.step().expect("stepping");
    }
    let moved = world.position(walker).expect("the walker");
    let block_at = (
        moved.0.raw() / world.scale() + 2,
        moved.1.raw() / world.scale(),
    );
    let house = world.role_unit("house");
    spawn(&mut world, house, block_at, EntityKind::Building, 550);

    // Two minutes of match time: long enough to go round a house, far longer than the walk itself.
    let mut arrived = false;
    for _ in 0..2400 {
        world.step().expect("stepping");
        if world.entity(walker).map(|entity| entity.task) == Some(Task::Idle) {
            let at = world.position(walker).expect("the walker");
            let dx = at.0.raw() - gx * world.scale();
            let dy = at.1.raw() - gy * world.scale();
            if dx * dx + dy * dy < 3 * world.scale() * world.scale() {
                arrived = true;
                break;
            }
        }
    }
    assert!(
        arrived,
        "the walker never reached its goal after a house was built across its route: it is stuck at {:?}",
        world.position(walker)
    );
}
