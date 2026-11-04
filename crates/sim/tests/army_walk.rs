//! **The army's walk.** The owner: "militia still get stuck in the forrest", "scout cavalry still gets stuck in gold mine" -
//! and, when asked which order puts them there, "the AI's army walking at the enemy". So the case is attack-move: a unit told
//! to walk at a distant target, through whatever is in the way.
//!
//! The first fix stopped a unit *walking into* a goal tile that was blocked. This one asks the other half: whatever a unit is
//! doing, does it ever **stand** on blocked ground? A unit that does is a unit with no route out.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    if !repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

/// A wood cluster: the first blocked run of three tiles in a row on the map, found rather than written down.
fn a_forest(world: &World) -> (u16, u16) {
    for y in 4..58u16 {
        for x in 4..58u16 {
            if (0..3).all(|d| !world.is_free_ground(x + d, y))
                && world.is_free_ground(x - 2, y)
                && world.is_free_ground(x + 5, y)
            {
                return (x, y);
            }
        }
    }
    panic!("no three-tile wood on this map");
}

fn soldier(world: &mut World, at: (i64, i64), unit: u32) -> EntityId {
    let scale = world.scale();
    let id = next_id(world);
    let hp = 60;
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::from_raw(scale * 3 / 2),
            hp,
            amount: 0,
        })
        .expect("a soldier");
    world.step().expect("one tick");
    id
}

fn tile_of(world: &World, id: EntityId) -> (u16, u16) {
    world
        .position(id)
        .map(|at| world.tile_of_position(at))
        .expect("a position")
}

/// **An attack-move across a wood is never in the wood.** Every tick, for both a soldier and the scout, the unit stands on
/// ground a unit may stand on.
#[test]
fn an_attack_move_across_a_wood_stays_on_free_ground() {
    let Some(mut world) = fresh() else { return };
    let wood = a_forest(&world);
    let militia = world.role_unit("militia").expect("a militia role");
    let scout = world.role_unit("scout_cavalry").expect("a scout role");
    let start = (i64::from(wood.0) - 4, i64::from(wood.1));
    let goal = (wood.0 + 8, wood.1);
    let fighters = [
        soldier(&mut world, start, militia),
        soldier(&mut world, (start.0, start.1 + 1), scout),
    ];
    for id in fighters {
        world
            .submit_checked(Command::AttackMove { unit: id, to: goal })
            .expect("an attack-move through the wood");
    }

    let mut offenders = Vec::new();
    for tick in 0..2_000 {
        world.step().expect("stepping");
        for id in fighters {
            let at = tile_of(&world, id);
            if !world.is_free_ground(at.0, at.1) {
                offenders.push((tick, at));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "units stood on blocked ground at {offenders:?} (wood {wood:?})"
    );
}

/// **A crowd is the hard case.** The army walks as a body, and a body in a gate is where a unit gets *pushed* - the assembler
/// moves bodies off each other, and a shove that ignores the ground lands one inside a tree. Twelve soldiers, one gap.
#[test]
fn a_crowd_walking_through_a_gap_stays_on_free_ground() {
    let Some(mut world) = fresh() else { return };
    let wood = a_forest(&world);
    let militia = world.role_unit("militia").expect("a militia role");
    let start = (i64::from(wood.0) - 4, i64::from(wood.1));
    let goal = (wood.0 + 8, wood.1);
    let mut fighters = Vec::new();
    for step in 0..12i64 {
        // A block of soldiers two deep, so they arrive at the wood together and shove.
        let at = (start.0 - step % 3, start.1 + step / 3);
        fighters.push(soldier(&mut world, at, militia));
    }
    for id in &fighters {
        world
            .submit_checked(Command::AttackMove {
                unit: *id,
                to: goal,
            })
            .expect("an attack-move through the wood");
    }

    let mut offenders = Vec::new();
    for tick in 0..3_000 {
        world.step().expect("stepping");
        for id in &fighters {
            let at = tile_of(&world, *id);
            if !world.is_free_ground(at.0, at.1) {
                offenders.push((tick, at));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "units in a crowd stood on blocked ground at {offenders:?}"
    );
}
