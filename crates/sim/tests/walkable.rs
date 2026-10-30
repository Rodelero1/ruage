//! What a worker may walk over. Two things the owner stated (2026-10-29) and one he asked to be verified.

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

/// **A farm can be walked over.** Farms are the one building that is a floor, not an obstacle: villagers cross their own
/// farms all the time, and a farm that blocked would wall a player out of their own food. Only `[build] blocking_roles`
/// (the wall) blocks today, and this test is what keeps a farm out of that list.
#[test]
fn a_farm_is_walked_over_not_around() {
    let Some(mut world) = fresh() else {
        return;
    };
    let farm = world.role_unit("farm").expect("a farm role");
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();

    // A farm at (40,40) and a villager that has to cross it: (38,40) to (42,40) is a straight line through the tile.
    let plot = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: plot,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(farm),
            position: (Fixed::from_raw(40 * scale), Fixed::from_raw(40 * scale)),
            speed: Fixed::ZERO,
            hp: 45,
            amount: 250,
        })
        .expect("a farm");
    world.step().expect("one tick");
    assert!(
        world.entity(plot).is_some_and(|e| e.construction.is_none()),
        "the farm should be finished"
    );

    let walker = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: walker,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(38 * scale), Fixed::from_raw(40 * scale)),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("one tick");
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (42, 40),
        })
        .expect("a move across the farm");

    let mut crossed = false;
    for _ in 0..400 {
        world.step().expect("stepping");
        if world.entity(walker).is_some_and(|e| {
            let x = e.position.0.raw() as f64 / scale as f64;
            x > 40.0
        }) {
            crossed = true;
            break;
        }
    }
    assert!(crossed, "a villager walks across a farm, not around it");
}
