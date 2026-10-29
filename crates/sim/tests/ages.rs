//! Ages need buildings of the age you are in.
//!
//! Written before the implementation (D7, D65). The rule, from the official learn-to-play page and Liquipedia,
//! which agree on all three parts:
//!
//! - "advancement requires you to construct at least two buildings from the current Age";
//! - the Dark Age buildings that count are the Lumber Camp, Mining Camp, Mill, Dock and **Barracks** - houses,
//!   farms, walls and outposts never do;
//! - **the Castle Age's exception**: the Imperial Age can be reached with two Castle Age buildings *or* a
//!   single Castle.
//!
//! Only finished buildings count: a foundation is not an age.

use std::path::PathBuf;

use ruage_data::{Economy, Effects, Map, Sim as Settings, Techs, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: Techs,
    effects: Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs"),
    })
}

fn world(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("civs");
    world
}

fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// The player's town centre: a finished building that can research an age.
fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("a town centre")
}

fn spawn_building(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world
        .role_unit(role)
        .unwrap_or_else(|| panic!("content has no role {role}"));
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 600,
            amount: 0,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

fn research(world: &mut World, building: EntityId, tech: u32) -> Result<(), SimError> {
    world.submit_checked(Command::Research { building, tech })
}

/// Feudal Age and Castle Age, by the tech ids content names.
const FEUDAL: u32 = 101;
const CASTLE_AGE: u32 = 102;
const IMPERIAL: u32 = 103;

#[test]
fn the_feudal_age_needs_two_dark_age_buildings() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    let tc = town_centre(&world);

    // A town centre is not one of the two: the game counts the Dark Age *economy and military* buildings.
    let refused = research(&mut world, tc, FEUDAL);
    assert!(
        matches!(
            refused,
            Err(SimError::AgeNeedsBuildings { need: 2, have: 0 })
        ),
        "the Feudal Age was researchable with no Dark Age buildings: {refused:?}"
    );

    spawn_building(&mut world, 1, "mill", (18, 20));
    let one = research(&mut world, tc, FEUDAL);
    assert!(
        matches!(one, Err(SimError::AgeNeedsBuildings { need: 2, have: 1 })),
        "one Dark Age building was enough for the Feudal Age: {one:?}"
    );

    spawn_building(&mut world, 1, "barracks", (21, 20));
    let two = research(&mut world, tc, FEUDAL);
    assert!(
        !matches!(two, Err(SimError::AgeNeedsBuildings { .. })),
        "two Dark Age buildings were not enough for the Feudal Age: {two:?}"
    );
}

#[test]
fn a_foundation_does_not_count_towards_an_age() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    let tc = town_centre(&world);
    // A mill foundation: placed, paid for, not yet a building.
    let mill = world.role_unit("mill").expect("mill");
    let scale = world.scale();
    let worker = EntityId(fresh_id(&world));
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id: worker,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(24 * scale), Fixed::from_raw(24 * scale)),
            speed: Fixed::parse("1.0", scale).expect("speed"),
            hp: 25,
            amount: 0,
        })
        .expect("villager");
    world.step().expect("step");
    world
        .submit_checked(Command::Build {
            villager: worker,
            unit: mill,
            at: (25, 24),
        })
        .expect("a mill foundation");
    world.step().expect("step");

    let refused = research(&mut world, tc, FEUDAL);
    assert!(
        matches!(refused, Err(SimError::AgeNeedsBuildings { have: 0, .. })),
        "a foundation counted towards an age: {refused:?}"
    );
}

#[test]
fn a_castle_alone_reaches_the_imperial_age() {
    // The Castle Age's exception: two Castle Age buildings, or one Castle.
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    world.grant_tech(1, FEUDAL).expect("Feudal");
    world.grant_tech(1, CASTLE_AGE).expect("Castle");
    let tc = town_centre(&world);

    let nothing = research(&mut world, tc, IMPERIAL);
    assert!(
        matches!(nothing, Err(SimError::AgeNeedsBuildings { .. })),
        "the Imperial Age was researchable with no Castle Age buildings: {nothing:?}"
    );

    spawn_building(&mut world, 1, "castle", (20, 24));
    let with_castle = research(&mut world, tc, IMPERIAL);
    assert!(
        !matches!(with_castle, Err(SimError::AgeNeedsBuildings { .. })),
        "a Castle did not satisfy the Imperial Age's requirement: {with_castle:?}"
    );
}
