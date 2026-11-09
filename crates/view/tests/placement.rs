//! **Placing a wall with the mouse** (D156): the first left click starts the run, the second places it, and the ghost the
//! player sees is the line the command will send.
//!
//! The owner's words: *"Walls can be placed vertically, horizontally mass at 45 degrees. First left click starts placement,
//! second click ends it."* And, on ineligible ground: *"you see the building as semi transparent preview and when it is
//! overlapping over ineligible area … it is getting red tone. With walls it works the same way with segments overlapping
//! ineligible area getting red tone."*
//!
//! The state machine has no window in it - a `winit` event loop cannot be driven from a test - so what is proved here is the
//! part that can be wrong: which tiles the run covers, which of them are red, and what the second click sends. Written before
//! the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::placement::{Placement, Segment};
use ruage_view::{Camera, Snapshot, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
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
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy"),
        units: ruage_data::load_units_file(&units).expect("units"),
        resources,
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("stockpile");
    world
}

fn villager(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    // `submit_checked` queues; `step` applies.
    world.step().expect("step");
    id
}

/// An empty stretch of ground: nothing anchored on it and nothing in the walk grid.
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 8..56i64 {
        for x in 4..58i64 {
            let clear = (0..height).all(|dy| {
                (0..width).all(|dx| {
                    let (tx, ty) = ((x + dx) as u16, (y + dy) as u16);
                    world.tile_walkable(tx, ty) && world.is_free_ground(tx, ty)
                })
            });
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

/// The tiles a placement currently shows, in drag order.
fn tiles(placement: &Placement) -> Vec<(u16, u16)> {
    placement.tiles()
}

/// The red ones: `(tile, valid)` where `valid == false` is drawn red.
fn red(segments: &[Segment]) -> Vec<(u16, u16)> {
    segments
        .iter()
        .filter(|s| !s.valid)
        .map(|s| s.tile)
        .collect()
}

// **The two clicks.**

/// **The first click starts the run, the second places it.** Before the first click there is nothing to draw; after it the run
/// is one tile long and follows the pointer; the second click hands over exactly what the ghost showed.
#[test]
fn the_first_click_starts_the_run_and_the_second_places_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let Some(at) = open_patch(&world, 8, 3) else {
        return;
    };
    let worker = villager(&mut world, 9_000, (at.0 - 1, at.1));

    let mut placement = Placement::new(worker, wall);
    assert!(placement.awaiting_start(), "the first click is the start");
    assert!(
        placement.segments(&world).is_empty(),
        "nothing is drawn before the first click"
    );

    let start = (
        u16::try_from(at.0).expect("in range"),
        u16::try_from(at.1).expect("in range"),
    );
    assert!(
        placement.click(&world, start).is_none(),
        "the first click starts the run; it places nothing"
    );
    assert!(!placement.awaiting_start(), "the run has started");
    assert_eq!(tiles(&placement), vec![start], "the run begins as one tile");

    // The pointer moves four tiles along, and the ghost follows.
    let end = (start.0 + 4, start.1);
    placement.hover(end);
    let shown = tiles(&placement);
    assert_eq!(
        shown,
        (0..5).map(|i| (start.0 + i, start.1)).collect::<Vec<_>>(),
        "the ghost is the line under the pointer"
    );

    // The second click: the command carries **exactly** what was drawn.
    let command = placement
        .click(&world, end)
        .expect("the second click places the line");
    match command {
        Command::BuildLine {
            villager: v,
            unit,
            tiles: sent,
        } => {
            assert_eq!(v, worker, "the villager that was selected does the work");
            assert_eq!(unit, wall, "the wall that was chosen");
            assert_eq!(sent, shown, "the command sends the line the ghost showed");
        }
        other => panic!("expected a wall line, got {other:?}"),
    }
}

/// **A tile that cannot be built on is red**, and the red is per segment - the rest of the line stays valid, which is what
/// makes "skip it, keep going" visible before the click.
#[test]
fn a_taken_tile_is_red_and_the_rest_of_the_line_is_not() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let house = world.role_unit("house").expect("house");
    let Some(at) = open_patch(&world, 8, 3) else {
        return;
    };
    let scale = world.scale();
    let blocked = (
        u16::try_from(at.0 + 3).expect("in range"),
        u16::try_from(at.1).expect("in range"),
    );
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_050),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (
                Fixed::from_raw(i64::from(blocked.0) * scale),
                Fixed::from_raw(i64::from(blocked.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("step");
    let worker = villager(&mut world, 9_001, (at.0 - 1, at.1));

    let mut placement = Placement::new(worker, wall);
    let start = (
        u16::try_from(at.0).expect("in range"),
        u16::try_from(at.1).expect("in range"),
    );
    placement.click(&world, start);
    placement.hover((start.0 + 5, start.1));

    let segments = placement.segments(&world);
    let reds = red(&segments);
    assert!(!reds.is_empty(), "the house's tile is red: {segments:?}");
    assert!(
        reds.contains(&blocked),
        "the house's own tile is red: {reds:?}"
    );
    let green = segments.iter().filter(|s| s.valid).count();
    assert!(
        green >= 3,
        "the clear part of the line is still valid: {segments:?}"
    );
}

/// **A run with nothing valid in it places nothing** - and the placement stays up rather than vanishing, so the player can drag
/// somewhere else.
#[test]
fn a_run_with_no_valid_tile_places_nothing() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let worker = villager(&mut world, 9_002, (1, 1));
    let mut placement = Placement::new(worker, wall);
    // A tile the map will not take: the town centre's own ground.
    assert!(
        placement.click(&world, (16, 16)).is_none(),
        "the first click only starts the run"
    );
    assert!(
        placement.segments(&world).iter().all(|s| !s.valid),
        "the town centre's ground is red"
    );
    assert!(
        placement.click(&world, (16, 16)).is_none(),
        "a second click on all-red places nothing"
    );
    assert!(
        !placement.awaiting_start(),
        "and the placement is still up, waiting for a better drag"
    );
}

/// **The ghost is actually drawn, and red where it is red.** The other tests prove which tiles and what command; this one proves
/// the picture says the same thing - a preview that is never drawn is a preview a player does not have.
#[test]
fn the_ghost_is_drawn_in_two_colours() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let Some(at) = open_patch(&world, 8, 3) else {
        return;
    };
    let worker = villager(&mut world, 9_003, (at.0 - 1, at.1));
    let mut placement = Placement::new(worker, wall);
    let start = (
        u16::try_from(at.0).expect("in range"),
        u16::try_from(at.1).expect("in range"),
    );
    placement.click(&world, start);
    placement.hover((start.0 + 2, start.1));

    // One segment is claimed by the town centre's ground - the panel's own tile is taken, so it draws red.
    let segments = vec![
        Segment {
            tile: start,
            valid: true,
        },
        Segment {
            tile: (start.0 + 2, start.1),
            valid: false,
        },
    ];
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = Camera::new((1280, 720)).centred_on((f32::from(start.0), f32::from(start.1)));
    let mut frame = render(&snapshot, &camera);
    ruage_view::draw_placement(&mut frame, &camera, &segments);

    let green_at = camera.project((f32::from(start.0) + 0.5, f32::from(start.1) + 0.5));
    let red_at = camera.project((f32::from(start.0) + 2.5, f32::from(start.1) + 0.5));
    let green = frame.pixel(
        u32::try_from(green_at.0).expect("in view"),
        u32::try_from(green_at.1).expect("in view"),
    );
    let red = frame.pixel(
        u32::try_from(red_at.0).expect("in view"),
        u32::try_from(red_at.1).expect("in view"),
    );
    assert!(
        green[1] > green[0],
        "a valid segment is drawn green-dominant: {green:?}"
    );
    assert!(
        red[0] > red[1],
        "a refused segment is drawn red-dominant: {red:?}"
    );
}
