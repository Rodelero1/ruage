//! **A man standing on a ghost leaves it, and the site gets built** (D193). The owner's rule book has said this since D183: a fresh
//! foundation is walkable, everybody standing on it walks off by the shortest route, and construction starts once the footprint is
//! empty.
//!
//! It stopped working when the gather standoff moved to the tile's border: in the content pack's map a villager ordered off a ghost
//! oscillated on its edge at `x = 4.95` for 2,000 ticks while the builder walked at the same approach tile, and the house sat at
//! **hp 1**. Two of the player's own units contesting one tile, neither passing the other.
//!
//! Written before the fix (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn spawn(world: &mut World, at: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("a villager role");
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(at.0 * scale + scale / 2),
                Fixed::from_raw(at.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let at = world.position(id).expect("a position");
    (
        at.0.raw().div_euclid(world.scale()),
        at.1.raw().div_euclid(world.scale()),
    )
}

/// **One man on the ghost, one builder, and the house still gets built.** This is the content pack's failure in miniature: the man on
/// the site has to leave even though the builder's own approach tile is in his way.
#[test]
fn a_man_on_a_ghost_leaves_and_the_house_gets_built() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let house = world.role_unit("house").expect("a house role");
    let (ax, ay) = (18u16, 18u16);
    for dy in 0..8u16 {
        for dx in 0..8u16 {
            common::look_away(&mut world, 1, (ax + dx, ay + dy));
        }
    }
    // The builder, and a second villager **standing on the ground the house will take**.
    // **The builder approaches from the east and the man on the ghost is at its east edge**: the tile he must step onto is the
    // tile the builder walks to, which is the shape the content pack's failure had.
    let builder = spawn(&mut world, (23, 21));
    let squatter = spawn(&mut world, (20, 20));
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (ax, ay + 2),
        })
        .expect("a house");

    let mut started = false;
    let mut left = false;
    for _ in 0..3_000 {
        world.step().expect("a tick");
        let progress = world
            .entities()
            .iter()
            .filter(|entity| entity.unit == Some(house) && entity.player == 1)
            .filter_map(|entity| entity.construction)
            .map(|construction| construction.progress_ticks)
            .max()
            .unwrap_or(0);
        if progress > 0 {
            started = true;
            break;
        }
        let (tx, ty) = tile_of(&world, squatter);
        let (lo_x, hi_x) = (i64::from(ax), i64::from(ax) + 2);
        let (lo_y, hi_y) = (i64::from(ay) + 2, i64::from(ay) + 4);
        if !(lo_x..hi_x).contains(&tx) || !(lo_y..hi_y).contains(&ty) {
            left = true;
        }
    }
    assert!(
        left,
        "MEASURED the villager still at {:?}: he never left the footprint, so the site could not start",
        tile_of(&world, squatter)
    );
    assert!(
        started,
        "MEASURED the house with no progress after the footprint cleared: the builder never worked"
    );
}
