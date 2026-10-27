//! The tech tree: what a player may build and train, and when.
//!
//! Written before the implementation (D7). The rules under test are content's, not the engine's - a
//! unit is available once some tech the player has researched says so (kind-2 `b = 1`), and the tech
//! itself carries the age as a prerequisite. So "archery ranges need the Feudal Age" is spelled once,
//! in the data, as tech 147 requiring 101.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
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
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

/// A world that has the tech tables, a rich player 1, and starts in the Dark Age (the pack's own
/// starting techs, whatever content says they are).
fn seeded(fx: &Fixture) -> World {
    seeded_with(fx, &fx.economy)
}

fn seeded_with(fx: &Fixture, economy: &Economy) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world
}

fn feudal(world: &mut World, player: u8) {
    for tech in [659, 101] {
        world.grant_tech(player, tech).expect("the Feudal Age");
    }
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit && e.construction.is_none())
        .expect("a villager")
        .id
}

/// A free tile two steps from the player's first building: fixed choice, so the test does not depend
/// on the map's resource layout.
fn a_free_tile(world: &World) -> (u16, u16) {
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Building && e.player == 1)
        .expect("the town centre");
    let origin = (
        u16::try_from(centre.position.0.raw() / scale).unwrap(),
        u16::try_from(centre.position.1.raw() / scale).unwrap(),
    );
    for radius in 2..8u16 {
        for (x, y) in [
            (origin.0 + radius, origin.1),
            (origin.0, origin.1 + radius),
            (origin.0 + radius, origin.1 + radius),
        ] {
            if world.is_free_ground(x, y) {
                return (x, y);
            }
        }
    }
    panic!("no free ground near the town centre");
}

const ARCHERY_RANGE: u32 = 87;
const BARRACKS: u32 = 12;
const SPEARMAN: u32 = 93;
const FEUDAL_AGE: u32 = 101;

#[test]
fn the_dark_age_has_no_archery_range() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    let err = world
        .submit_checked(Command::Build {
            villager,
            unit: ARCHERY_RANGE,
            at,
        })
        .expect_err("an archery range is a Feudal building")
        .to_string();
    assert!(
        err.contains("not available"),
        "the refusal names availability: {err}"
    );
}

#[test]
fn the_feudal_age_allows_an_archery_range() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    feudal(&mut world, 1);
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: ARCHERY_RANGE,
            at,
        })
        .expect("the Feudal Age brings the archery range with it");
}

#[test]
fn a_dark_age_barracks_is_allowed_because_the_pack_says_so() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: BARRACKS,
            at,
        })
        .expect("barracks are a Dark Age building: tech 220 needs only the starting techs");
}

/// The same gate decides what a building may *train*: the barracks is there from the Dark Age, the
/// spearman is not, because the tech that makes it available (87, "Pike") wants the Feudal Age.
#[test]
fn a_barracks_cannot_train_a_spearman_in_the_dark_age() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    allow_barracks_spearmen(&mut economy);
    let mut world = seeded_with(&fx, &economy);
    let barracks = a_finished_building(&mut world, BARRACKS);

    match world.submit_checked(Command::Train {
        building: barracks,
        unit: SPEARMAN,
    }) {
        Err(SimError::UnitUnavailable { unit }) => assert_eq!(unit, SPEARMAN),
        other => panic!("expected UnitUnavailable, got {other:?}"),
    }
}

#[test]
fn a_barracks_trains_a_spearman_once_the_feudal_age_is_reached() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    allow_barracks_spearmen(&mut economy);
    let mut world = seeded_with(&fx, &economy);
    let barracks = a_finished_building(&mut world, BARRACKS);
    feudal(&mut world, 1);
    world
        .submit_checked(Command::Train {
            building: barracks,
            unit: SPEARMAN,
        })
        .expect("Feudal spearmen are a barracks' business");
}

/// The shadow techs are what made the tree unreachable: Feudal (101) requires 104 *and* 659, and 659
/// requires 120 and 638 - techs with no effect at all whose only job is to be a prerequisite. They are
/// now granted as soon as their own prerequisites are met, so the age is reachable by playing the game
/// rather than by a test granting half its chain.
#[test]
fn the_feudal_age_is_reachable_without_granting_its_prerequisites_by_hand() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.start.stock = [("food".to_owned(), 600), ("wood".to_owned(), 300)]
        .into_iter()
        .collect();
    let mut world = seeded_with(&fx, &economy);
    let centre = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id;
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: FEUDAL_AGE,
        })
        .expect("500 food is the whole price of the Feudal Age");
    for _ in 0..4_000 {
        world.step().expect("step");
    }
    // The proof that it landed: a Feudal building is now allowed.
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: ARCHERY_RANGE,
            at,
        })
        .expect("the Feudal Age arrived by itself");
}

/// Content decides what a barracks trains, so the suite adds the line it wants to test rather than
/// editing the project's own pack.
fn allow_barracks_spearmen(economy: &mut Economy) {
    economy
        .train
        .by_building
        .entry(BARRACKS)
        .or_default()
        .push(SPEARMAN);
}

/// Build `unit` at a free tile and run the clock until it is finished.
fn a_finished_building(world: &mut World, unit: u32) -> EntityId {
    let villager = first_villager(world);
    let at = a_free_tile(world);
    world
        .submit_checked(Command::Build { villager, unit, at })
        .expect("the build order");
    world.step().expect("step");
    let site = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(unit))
        .map(|e| e.id)
        .next()
        .expect("the site exists");
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            return site;
        }
        world.step().expect("step");
    }
    panic!("the building never finished");
}
