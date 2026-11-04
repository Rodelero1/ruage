//! **A camp where a drop site already serves the wood is a wasted camp.**
//!
//! The number is not ours. The reference implementation (OpenAoK, MIT) encodes the original AI's own placement rule as
//! `kCovered = 5` tiles - "a drop site already this near a forest, mine or forage makes another there pointless" - and its
//! camp may be at most `sn-camp-max-distance` = 25 from home. We keep both as content (`[build] drop_site_spacing`) and this
//! file is the guard that the rule reaches the site search rather than only the comments.
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
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn bucket(world: &mut World, at: (i64, i64), amount: u32) -> EntityId {
    let scale = world.scale();
    let wood = world.slot_of("wood").expect("a wood column");
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 0,
            kind: EntityKind::Resource(wood),
            unit: None,
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::ZERO,
            hp: 0,
            amount,
        })
        .expect("a wood pile");
    world.step().expect("one tick");
    id
}

/// The lumberjack's own unit id, which **is** his variant: the pack says which buildings he may hand wood in at (109 the
/// town centre, 562 the camp).
fn lumberjack(world: &World) -> u32 {
    // By role, not "first unit": the player opens with a scout too (2026-11-04), and a horseman carries nothing.
    world.start_villager_unit().expect("a villager role")
}

/// **Wood within five tiles of the town centre is served by it**, and wood on the far side of the map is not: the predicate
/// the site search now asks, measured the way the reference implementation measures it.
#[test]
fn wood_the_town_centre_serves_is_covered_and_far_wood_is_not() {
    let Some(mut world) = fresh() else { return };
    let variant = lumberjack(&world);
    let spacing = world.drop_site_spacing();
    assert_eq!(
        spacing, 5,
        "the pack's spacing is not the reference implementation's 5"
    );

    // The town centre sits at (16, 16) in this map; wood three tiles away is covered, wood eighteen away is not.
    let near = bucket(&mut world, (19, 19), 400);
    let far = bucket(&mut world, (34, 34), 400);
    let tile_of = |world: &World, id: EntityId| {
        world
            .position(id)
            .map(|position| world.tile_of_position(position))
            .expect("a position")
    };
    assert!(
        world.drop_site_within(1, Some(variant), tile_of(&world, near), spacing, true),
        "wood beside the town centre is not served by it"
    );
    assert!(
        !world.drop_site_within(1, Some(variant), tile_of(&world, far), spacing, false),
        "wood across the map is treated as served"
    );
    // The town centre counts **only when the caller says so**: the camp rule asks for the owner's behaviour (a camp by the
    // town centre is wood spent on a walk saved twice), while a mill must be free to stand at the berries beside it.
    assert!(
        !world.drop_site_within(1, Some(variant), tile_of(&world, near), spacing, false),
        "the town centre counts against a mill when it should not"
    );
}
