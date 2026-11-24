//! **D342 (sequence step 7): the 300 row is a cell beside a gold or stone mine, and nothing else.**
//!
//! `FUN_007ed280` prices an empty cell at **10** unless `FUN_007f11f0` answers 1 for the cell or one of its four
//! orthogonal neighbours - and `FUN_007f11f0` answers 1 only for an object whose **class** is **8** (stone mine) or
//! **32** (gold mine). A cell beside a tree, a wall or a cliff is the plain 10.
//!
//! Ours charged 300 to any cell with a blocked orthogonal neighbour - terrain, woods and buildings alike - so the
//! search avoided every wall it passes and hugged every mine.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_sim::{EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_plain()
}

/// The first tile of a resource of this kind, and one free ground tile orthogonally beside it.
fn a_node_and_its_neighbour(world: &World, kind: &str) -> Option<((u16, u16), (u16, u16))> {
    let slot = world.slot_of(kind)?;
    let node = world
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Resource(slot) && entity.amount > ruage_fixed::Fixed::ZERO
        })
        .map(|entity| entity.id)?;
    let (tx, ty) = world.tile_of_position(world.position(node)?);
    for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
        let (Ok(x), Ok(y)) = (
            u16::try_from(i32::from(tx) + dx),
            u16::try_from(i32::from(ty) + dy),
        ) else {
            continue;
        };
        if world.is_free_ground(x, y) {
            return Some(((tx, ty), (x, y)));
        }
    }
    None
}

/// **How much of the map's cost landscape the row moves** - the census behind D342's numbers, printed rather than
/// asserted: `300` under the old rule was "any blocked orthogonal neighbour", under the game's rule it is a mine.
#[test]
#[ignore = "measurement, not a rule"]
fn the_300_census() {
    let Some(world) = fresh() else { return };
    // `config/map.toml` is 64 x 64 (its own header); `is_free_ground` is false off the map, so a tile past the edge is
    // simply not counted.
    let (width, height) = (64u16, 64u16);
    let (mut old_rule, mut new_rule, mut open) = (0u32, 0u32, 0u32);
    for y in 0..height {
        for x in 0..width {
            if !world.is_free_ground(x, y) {
                continue;
            }
            open += 1;
            let blocked_beside = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| {
                    match (
                        u16::try_from(i32::from(x) + dx),
                        u16::try_from(i32::from(y) + dy),
                    ) {
                        (Ok(nx), Ok(ny)) => !world.is_free_ground(nx, ny),
                        _ => false,
                    }
                });
            if blocked_beside {
                old_rule += 1;
            }
            if world.step_cost_at((x, y)) == Some(300) {
                new_rule += 1;
            }
        }
    }
    println!(
        "open tiles {open}; 300 under the old rule (any blocked neighbour) {old_rule}; under D342 (beside a mine) {new_rule}"
    );
}

/// **A cell beside a mine costs 300; a cell beside a tree costs 10.** One scene, both cells, so the fixture cannot pass
/// by giving every neighbour the same row.
#[test]
fn a_mine_is_expensive_to_walk_beside_and_a_tree_is_not() {
    let Some(world) = fresh() else { return };
    let Some((_, beside_gold)) = a_node_and_its_neighbour(&world, "gold") else {
        panic!("no gold mine with free ground beside it on the seeded map");
    };
    let Some((_, beside_wood)) = a_node_and_its_neighbour(&world, "wood") else {
        panic!("no tree with free ground beside it on the seeded map");
    };
    assert_ne!(beside_gold, beside_wood, "the two cells are the same tile");
    assert_eq!(
        world.step_cost_at(beside_gold),
        Some(300),
        "a cell beside a gold mine must cost 300 (the game's mine row)"
    );
    assert_eq!(
        world.step_cost_at(beside_wood),
        Some(10),
        "a cell beside a tree must cost the plain 10 - the game's 300 row is mines, not 'any blocked neighbour'"
    );
}
