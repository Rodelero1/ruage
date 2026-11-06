//! A: an order carries a **point**, not a tile centre (the owner's report - tiles are for terrain, resource and building
//! placement; movement is free of them). Written before the implementation (D7).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh()
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

fn tile_position(world: &World, x: u16, y: u16) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    )
}

/// **Where a body ends up when it is sent to a tile**: the middle of it (D114). The origin above stays the anchor a spawn
/// and a building's footprint are placed from, so the two are named differently on purpose.
fn tile_centre(world: &World, x: u16, y: u16) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale + scale / 2),
        Fixed::from_raw(i64::from(y) * scale + scale / 2),
    )
}

fn place_unit(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: tile_position(world, x, y),
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

/// A point **several tiles away**: the route has more than one step, so the last leg is the one that has to aim at the
/// point rather than at the tile's centre. This is the test the `end` field exists for - a point inside the unit's own
/// tile would pass without it, because an empty route falls back to the order's own target.
#[test]
fn a_point_several_tiles_away_is_reached_exactly() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let villager = next_id(&world);
    place_unit(&mut world, villager, x, y);
    world.step().expect("the spawn lands");

    let scale = world.scale();
    let target = (
        Fixed::from_raw(i64::from(x + 3) * scale + scale * 2 / 5),
        Fixed::from_raw(i64::from(y) * scale + scale / 4),
    );
    world.submit(Command::WalkTo {
        unit: villager,
        to: target,
    });
    // **The straightness is the assertion, not the destination.** Both a route that ends on the point and one that ends
    // on the last tile's centre reach the point eventually - the second walks centre-then-point, because an empty route
    // falls back to the order's own target. What separates them is the detour: with the exact end the unit never comes
    // near that centre, and without it, it walks through it.
    let centre = tile_position(&world, x + 3, y);
    let mut closest = i64::MAX;
    for _ in 0..800 {
        world.step().expect("step");
        if let Some(position) = world.position(villager) {
            let (dx, dy) = (
                position.0.raw() - centre.0.raw(),
                position.1.raw() - centre.1.raw(),
            );
            closest = closest.min(dx * dx + dy * dy);
        }
    }
    assert_eq!(
        world.position(villager),
        Some(target),
        "the walk ends on the point it was sent to (task {:?})",
        world.entity(villager).map(|e| format!("{:?}", e.task))
    );
    let tenth = scale / 10;
    assert!(
        closest > tenth * tenth,
        "the last leg has to aim at the point, not detour through the tile centre (closest approach {closest} raw, \
         a tenth of a tile is {})",
        tenth * tenth
    );
}

/// Three quarters of a tile away, inside the *same* tile: an empty route, so the mover falls back to its own target. This
/// is the case that passes **without** the `end` field, and it is kept because it is what a player's click does most of
/// the time - a nudge, not a march.
#[test]
fn a_point_inside_its_own_tile_is_still_reached() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 4) else {
        return;
    };
    let villager = next_id(&world);
    place_unit(&mut world, villager, x, y);
    world.step().expect("the spawn lands");

    let scale = world.scale();
    let target = (
        Fixed::from_raw(i64::from(x) * scale + scale * 3 / 4),
        Fixed::from_raw(i64::from(y) * scale + scale / 4),
    );
    world.submit(Command::WalkTo {
        unit: villager,
        to: target,
    });
    for _ in 0..400 {
        world.step().expect("step");
    }
    assert_eq!(
        world.position(villager),
        Some(target),
        "the unit has to stand on the point it was sent to, not on the tile centre (task {:?}, refusal {:?})",
        world.entity(villager).map(|e| format!("{:?}", e.task)),
        world
            .last_refusal()
            .map(|(id, why, tick)| (id.0, why.to_string(), tick))
    );
}

/// The old tile order still means the tile's centre: two ways to say where, one path through the engine.
#[test]
fn a_tile_order_still_means_its_centre() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let villager = next_id(&world);
    place_unit(&mut world, villager, x, y);
    world.step().expect("the spawn lands");

    world.submit(Command::Move {
        unit: villager,
        to: (x + 3, y),
    });
    for _ in 0..400 {
        world.step().expect("step");
    }
    assert_eq!(
        world.position(villager),
        Some(tile_centre(&world, x + 3, y)),
        "a tile order is the centre of that tile"
    );
}
