//! S1: the original tries a **straight line** before it plans a path (D98a; move_obj.cpp 4,079-4,245).
//!
//! "Looking for a STRAIGHT LINE path … no non-target collisions there, so we can do the straight line path", else
//! "we can't do the straight line path" and the route is searched. The observable difference is the route: a clear line
//! is the goal alone, while a search across open ground stores every tile it crosses. Written before the
//! implementation (D7).

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

fn at(world: &World, x: u16, y: u16) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    )
}

fn place_unit(world: &mut World, id: EntityId, player: u8, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: at(world, x, y),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

/// An open row of free ground, away from the seeded base, so the line between two tiles is clear by construction.
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

/// Six tiles of open ground: the shortcut stores the goal **alone**, where a search would have stored six waypoints.
#[test]
fn an_open_line_is_a_single_waypoint() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let villager = next_id(&world);
    place_unit(&mut world, villager, 1, x, y);
    world.step().expect("the spawn lands");

    world.submit(Command::Move {
        unit: villager,
        to: (x + 6, y),
    });
    world.step().expect("the order lands");
    assert_eq!(
        world.path_length(villager),
        1,
        "a clear line is the goal alone; six waypoints means the search ran"
    );
}

/// A building on the line takes the shortcut away, so the search runs - and the villager still arrives.
#[test]
fn an_obstacle_across_the_line_forces_the_search() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let villager = next_id(&world);
    // **Both ids first**: a `Spawn` lands on the next tick, so `next_id` called twice before a step returns the same id
    // twice and the second spawn is refused as a duplicate (the same fixture trap the evasion tests hit).
    let tree = EntityId(villager.0 + 1);
    place_unit(&mut world, villager, 1, x, y);
    // A **tree**, not a house: a spawned building is a foundation, and D61's rule is that a foundation blocks nobody
    // until a villager first works it - so it would not close the line. A node carrying hit points closes its tile from
    // insertion (D84), which is the obstacle this test is about.
    let wood = world.slot_of("wood").expect("a wood column");
    world
        .submit_checked(Command::Spawn {
            id: tree,
            player: 1,
            kind: EntityKind::Resource(wood),
            unit: None,
            position: at(&world, x + 3, y),
            speed: Fixed::ZERO,
            hp: 20,
            amount: 100,
        })
        .expect("a tree can be placed");
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: villager,
        to: (x + 6, y),
    });
    world.step().expect("the order lands");
    assert!(
        world.path_length(villager) > 1,
        "a blocked line has to be searched, not walked: {} waypoint(s)",
        world.path_length(villager)
    );

    let scale = world.scale();
    let mut arrived = false;
    for _ in 0..2_000 {
        world.step().expect("step");
        let position = world.position(villager).expect("position");
        let dx = (position.0.raw() - i64::from(x + 6) * scale).abs();
        let dy = (position.1.raw() - i64::from(y) * scale).abs();
        if dx <= scale && dy <= scale {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "and it goes round the tree");
}

/// The goal's **own** tile is never a reason to search: you may be ordered at a body, and a node's tile is blocked by
/// the thing you are going to work. The original's line test asks about *non-target* collisions for exactly this reason.
#[test]
fn the_goal_tile_does_not_block_its_own_line() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let sitter = EntityId(walker.0 + 1);
    place_unit(&mut world, walker, 1, x, y);
    place_unit(&mut world, sitter, 2, x + 4, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 4, y),
    });
    world.step().expect("the order lands");
    assert_eq!(
        world.path_length(walker),
        1,
        "the body standing on the goal is not a reason to plan a path"
    );
}

/// **B**: a route that has to bend carries no slack. The pather's own path around a tree holds a node per tile; the stored
/// route jumps to the furthest node a clear line reaches, so it is strictly shorter - and the obstacle is reproduced
/// exactly in a local `Walk`, so the comparison is against the same world the engine planned in.
#[test]
fn a_route_round_a_tree_carries_no_slack() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let mut walk = ruage_sim::Walk::from_map(&map);
    walk.block(x + 3, y);
    let pather = ruage_sim::find_path(&walk, (x, y), (x + 6, y)).expect("the row is reachable");
    assert!(
        pather.len() > 2,
        "the fixture needs a route to trim: {pather:?}"
    );

    let villager = next_id(&world);
    let tree = EntityId(villager.0 + 1);
    place_unit(&mut world, villager, 1, x, y);
    let wood = world.slot_of("wood").expect("a wood column");
    world
        .submit_checked(Command::Spawn {
            id: tree,
            player: 1,
            kind: EntityKind::Resource(wood),
            unit: None,
            position: at(&world, x + 3, y),
            speed: Fixed::ZERO,
            hp: 20,
            amount: 100,
        })
        .expect("a tree can be placed");
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: villager,
        to: (x + 6, y),
    });
    world.step().expect("the order lands");
    let stored = world.path_length(villager);
    assert!(
        stored < pather.len(),
        "the stored route must be shorter than the pather's own polyline: {stored} against {}",
        pather.len()
    );
}
