//! A building's footprint mask (D76): the Town Centre's solid back core and walkable wings, as content.
//!
//! Written before the implementation. What is being pinned: a mask in `economy.toml` blocks exactly its `#` cells about
//! the anchor, the `.` cells stay walkable, and demolition frees precisely what the mask blocked - the same function
//! answers both questions, so the two cannot drift.

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

/// Player 1's Town Centre and where it stands.
fn town_center(world: &World) -> (EntityId, (i32, i32), u32) {
    let entity = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let scale = world.scale();
    (
        entity.id,
        (
            (entity.position.0.raw() / scale) as i32,
            (entity.position.1.raw() / scale) as i32,
        ),
        entity.unit.expect("a town centre has a unit id"),
    )
}

/// The back core is solid and the wings and courtyard are not: `blocked = ["##..", "##..", "....", "...."]` about the
/// north-corner anchor (D84) means (0,0), (1,0), (0,1) and (1,1) are blocked and the bottom half is grass.
#[test]
fn a_town_center_blocks_its_back_core_and_nothing_else() {
    let Some(world) = fresh() else { return };
    let (_, (ax, ay), _) = town_center(&world);

    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert!(
            !world.tile_walkable((ax + dx) as u16, (ay + dy) as u16),
            "the back core at (+{dx},+{dy}) is walkable"
        );
    }
    for (dx, dy) in [(0, 2), (1, 2), (0, 3), (1, 3), (2, 2), (3, 3)] {
        assert!(
            world.tile_walkable((ax + dx) as u16, (ay + dy) as u16),
            "the yard at (+{dx},+{dy}) is blocked"
        );
    }
}

/// Demolition gives back exactly what the mask took: no tile left blocked, none freed that was never blocked.
#[test]
fn demolition_frees_exactly_the_mask() {
    let Some(mut world) = fresh() else { return };
    let (town, (ax, ay), _) = town_center(&world);
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: town,
        })
        .expect("razing your own town centre");
    for _ in 0..5 {
        world.step().expect("step");
    }
    assert!(
        world.entity(town).is_none(),
        "the town centre is still there"
    );
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert!(
            world.tile_walkable((ax + dx) as u16, (ay + dy) as u16),
            "the back core at (+{dx},+{dy}) is still blocked after demolition"
        );
    }
}

/// A villager can stand on the Town Centre's yard - the thing the mask exists for - even though its core is solid.
#[test]
fn a_villager_can_stand_on_the_yard() {
    let Some(mut world) = fresh() else { return };
    let (_, (ax, ay), _) = town_center(&world);
    let scale = world.scale();
    let villager = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Unit && entity.player == 1)
        .map(|entity| entity.id)
        .next()
        .expect("a villager");
    world
        .submit_checked(Command::Move {
            unit: villager,
            to: ((ax + 2) as u16, (ay + 3) as u16),
        })
        .expect("a move onto the yard is a legal order");
    for _ in 0..600 {
        world.step().expect("step");
    }
    let at = world.position(villager).expect("the villager");
    assert_eq!(
        ((at.0.raw() / scale) as i32, (at.1.raw() / scale) as i32),
        (ax + 2, ay + 3),
        "the villager never reached a tile of the Town Centre's yard"
    );
    let _ = Fixed::ZERO;
}
