//! **Wall placement** (D155): a drag from one tile to another becomes a line of wall segments, and the line bends **once**.
//!
//! The owner's rules, in his words: walls are placed *vertically, horizontally* and *at 45 degrees*; *first left click starts
//! placement, second click ends it*; *one corner … could be done*; the line goes *diagonal then straight* — the engine picks
//! the bend (answer A); ineligible segments are **skipped**, not fatal (answer B, "skip it, keep going"); and the line's tiles
//! are handed to the sim as a **list** (answer C).
//!
//! Written before the behaviour exists (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
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
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    Some(Fixture {
        settings: common::present(&repo("config/sim.toml"), ruage_data::load_sim_file),
        map: common::must(
            "load_map_file",
            ruage_data::load_map_file(&repo("config/map.toml"), &resources),
        ),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        units: common::must("load_units_file", ruage_data::load_units_file(&units)),
        resources,
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
}

/// A villager standing on `tile`, able to walk.
fn villager(world: &mut World, player: u8, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    // **`submit_checked` queues; `step` applies** - a command is not in the world until a tick drains it. Without this the
    // villager does not exist yet and the build order below is refused with `MissingEntity`.
    world.step().expect("step");
    id
}

/// An empty patch of ground, away from the seeded base at (16, 16): the project map has water and woods, and a villager
/// spawned on a blocked tile is refused by `insert` while `Spawn` still reports success - which cost one debugging round here.
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 8..56i64 {
        for x in 4..58i64 {
            // Both questions: nothing stands anchored here (`is_free_ground`, which is what `insert` refuses a unit for), and
            // the ground is walkable (`tile_walkable`).
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

/// The tiles a drag from `from` to `to` would lay, as the view would ask for them.
fn line(from: (u16, u16), to: (u16, u16)) -> Vec<(u16, u16)> {
    World::wall_line(from, to)
}

// **The geometry, without a world: the line is the contract the view draws.**

/// A drag along one axis is a straight run, both ends included.
#[test]
fn a_horizontal_drag_is_a_row_and_a_vertical_one_a_column() {
    assert_eq!(
        line((5, 5), (9, 5)),
        vec![(5, 5), (6, 5), (7, 5), (8, 5), (9, 5)],
        "a horizontal drag"
    );
    assert_eq!(
        line((5, 5), (5, 9)),
        vec![(5, 5), (5, 6), (5, 7), (5, 8), (5, 9)],
        "a vertical drag"
    );
    // And the same when dragged back the other way: the line is the same set, in the other order.
    assert_eq!(
        line((9, 5), (5, 5)),
        vec![(9, 5), (8, 5), (7, 5), (6, 5), (5, 5)],
        "a drag the other way"
    );
}

/// **45° touches at corners**: a diagonal drag steps one right and one down, so consecutive segments share a corner - which
/// blocks passage, because a diagonal step in the search requires both of its corners free (`tests/passage.rs`).
#[test]
fn a_diagonal_drag_touches_at_corners() {
    assert_eq!(
        line((5, 5), (9, 9)),
        vec![(5, 5), (6, 6), (7, 7), (8, 8), (9, 9)],
        "45 degrees, corner to corner"
    );
}

/// **A drag that is neither goes 45° first, then straight** - one corner, chosen by the engine (the owner: *"there can be a
/// corner when one part of the wall is horizontal and the other under 45 degree"*).
#[test]
fn a_drag_that_is_neither_goes_diagonal_then_straight() {
    assert_eq!(
        line((5, 5), (9, 7)),
        vec![(5, 5), (6, 6), (7, 7), (8, 7), (9, 7)],
        "the 45 degree leg, then the straight one"
    );
    assert_eq!(
        line((5, 5), (7, 9)),
        vec![(5, 5), (6, 6), (7, 7), (7, 8), (7, 9)],
        "the same the other way up"
    );
}

/// **One click is one segment**: *"one corner in the single wall placement could be done"* starts here - a drag of no length.
#[test]
fn a_single_click_is_one_segment() {
    assert_eq!(line((5, 5), (5, 5)), vec![(5, 5)], "a click, not a drag");
}

/// **Every tile of the line is a segment**, in order, all foundations.
#[test]
fn a_drag_lays_every_segment_of_the_line() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let Some(at) = open_patch(&world, 6, 3) else {
        return;
    };
    let worker = villager(&mut world, 1, 9_000, (at.0 - 2, at.1));
    let tiles: Vec<(u16, u16)> = (0..4)
        .map(|i| {
            (
                u16::try_from(at.0 + i).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            )
        })
        .collect();

    // **Remembered ground** (D183): the line is laid by hand away from the base, and a foundation may be placed blind in
    // fog but never in the black - a segment whose tile nobody has looked at would be refused and the drag would come
    // up one wall short.
    for tile in &tiles {
        common::look_away(&mut world, 1, *tile);
    }
    world
        .submit_checked(Command::BuildLine {
            villager: worker,
            unit: wall,
            tiles: tiles.clone(),
        })
        .expect("a four-tile wall drag");
    // A villager walks to each site and raises it in order: the line is a queue, not four simultaneous foundations.
    for _ in 0..2_400 {
        world.step().expect("step");
    }
    let built: Vec<(u16, u16)> = world
        .entities()
        .iter()
        .filter(|entity| entity.unit == Some(wall))
        .map(|entity| world.tile_of_position(entity.position))
        .collect();
    assert_eq!(built.len(), 4, "four segments, not one: {built:?}");
    for tile in &tiles {
        assert!(
            built.contains(tile),
            "the segment at {tile:?} is missing: {built:?}"
        );
    }
}

/// **An ineligible tile is skipped and the rest stand** (the owner: answer B, "skip it, keep going" - and the view shows that
/// tile red).
#[test]
fn an_ineligible_tile_is_skipped_and_the_rest_stand() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let wall = world.role_unit("palisade_wall").expect("palisade wall");
    let house = world.role_unit("house").expect("house");
    let Some(at) = open_patch(&world, 8, 3) else {
        return;
    };
    let scale = world.scale();
    // A **finished** house stands in the middle of the drag, placed rather than ordered: the drag has to meet an obstacle that
    // is already there, not one that is still a twinkle in another villager's queue.
    let middle = (
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
                Fixed::from_raw(i64::from(middle.0) * scale),
                Fixed::from_raw(i64::from(middle.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    let worker = villager(&mut world, 1, 9_002, (at.0 - 2, at.1));

    let tiles: Vec<(u16, u16)> = (0..6)
        .map(|i| {
            (
                u16::try_from(at.0 + i).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            )
        })
        .collect();
    // **Remembered ground** (D183): the line is laid by hand away from the base, and a foundation may be placed blind in
    // fog but never in the black - a segment whose tile nobody has looked at would be refused and the drag would come
    // up one wall short.
    for tile in &tiles {
        common::look_away(&mut world, 1, *tile);
    }
    world
        .submit_checked(Command::BuildLine {
            villager: worker,
            unit: wall,
            tiles: tiles.clone(),
        })
        .expect("the drag is accepted even with one tile taken");
    for _ in 0..2_400 {
        world.step().expect("step");
    }
    let built: Vec<(u16, u16)> = world
        .entities()
        .iter()
        .filter(|entity| entity.unit == Some(wall))
        .map(|entity| world.tile_of_position(entity.position))
        .collect();
    // A house is 2x2, so it eats two tiles of the line: the four either side still stand, **including the one past it** - which
    // is the whole point of "skip it, keep going".
    assert_eq!(
        built.len(),
        4,
        "the two blocked tiles are skipped, the other four stand: {built:?}, last refusal {:?}",
        world
            .last_refusal()
            .map(|(id, why, tick)| (id, why.to_string(), tick))
    );
    assert!(
        !built.contains(&middle),
        "nothing was placed on the house's tile"
    );
    assert!(
        built.contains(&(middle.0 + 2, middle.1)),
        "the line continues past the house: {built:?}"
    );
}
