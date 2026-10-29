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
    // Two Dark Age buildings first: an age needs them (D65). This test is about the hidden prerequisite
    // chain, so the buildings are placed rather than paid for - what matters here is that the tech itself
    // needs nobody's help.
    for role in ["mill", "barracks"] {
        let at = a_free_tile(&world);
        let scale = world.scale();
        let id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Building,
                unit: Some(unit),
                position: (
                    ruage_fixed::Fixed::from_raw(i64::from(at.0) * scale),
                    ruage_fixed::Fixed::from_raw(i64::from(at.1) * scale),
                ),
                speed: ruage_fixed::Fixed::ZERO,
                hp: 600,
                amount: 0,
            })
            .expect("a Dark Age building");
        world.step().expect("step");
    }
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

/// The order path has its own copy of the counting rule, and no tech in the dat can reach it: every
/// tech whose count matters is a free availability or shadow tech, granted by the resolver. So it is
/// pinned with a pack of our own - a tech that lists three prerequisites and needs two.
const TWO_OF_THREE: &str = r#"
schema = 1
kind = "techs"

[[tech]]
id = 900
effect_id = 900
research_time = 1
costs = []
required = [901, 902, 903]
min_required = 2
location = 0

[[tech]]
id = 901
effect_id = 900
research_time = 0
costs = []
required = []
min_required = 0
location = 0

[[tech]]
id = 902
effect_id = 900
research_time = 0
costs = []
required = []
min_required = 0
location = 0

[[tech]]
id = 903
effect_id = 900
research_time = 0
costs = []
required = []
min_required = 0
location = 0

# An availability tech: it makes unit 70 (a house) buildable, and it needs two of the three markers.
# This is the resolver's path, which no tech in the dat can reach - every count-sensitive tech there
# ends up satisfied anyway, because its alternatives are always-true markers.
[[tech]]
id = 910
effect_id = 910
research_time = 0
costs = []
required = [901, 902, 903]
min_required = 2
location = -1

# The pack's own `[start] techs` name 104, and a world without it cannot be seeded.
[[tech]]
id = 104
effect_id = 900
research_time = 0
costs = []
required = []
min_required = 0
location = -1
"#;

const TWO_OF_THREE_EFFECTS: &str = r#"
schema = 1
kind = "effects"

[[effect]]
id = 900
name = "Nothing"
commands = []
modifiers = []
resource_modifiers = []

[[effect]]
id = 910
name = "Houses (make avail)"
commands = [[2, 70, 1, -1, 0.0]]
modifiers = []
resource_modifiers = []
"#;

fn two_of_three_world(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    let techs = ruage_data::load_techs_str(TWO_OF_THREE, "techs.toml").expect("custom techs");
    let effects =
        ruage_data::load_effects_str(TWO_OF_THREE_EFFECTS, "effects.toml").expect("custom effects");
    world.load_research(&techs, &effects).expect("research");
    world
}

fn research_order(world: &mut World, tech: u32) -> Result<(), SimError> {
    let centre = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id;
    world.submit_checked(Command::Research {
        building: centre,
        tech,
    })
}

#[test]
fn a_tech_needs_only_as_many_prerequisites_as_content_says() {
    let Some(fx) = fixture() else { return };
    let mut world = two_of_three_world(&fx);
    world.grant_tech(1, 901).expect("one prerequisite");
    world.grant_tech(1, 902).expect("two prerequisites");
    research_order(&mut world, 900).expect("two of three is enough");
}

#[test]
fn one_of_three_is_not_enough() {
    let Some(fx) = fixture() else { return };
    let mut world = two_of_three_world(&fx);
    world.grant_tech(1, 901).expect("one prerequisite");
    match research_order(&mut world, 900) {
        Err(SimError::MissingPrerequisite { tech, .. }) => assert_eq!(tech, 900),
        other => panic!("expected MissingPrerequisite, got {other:?}"),
    }
}

/// The resolver's copy of the counting rule: an availability tech is granted when enough of its
/// prerequisites are there, and the building it enables follows.
#[test]
fn an_availability_tech_counts_its_prerequisites() {
    let Some(fx) = fixture() else { return };
    let mut world = two_of_three_world(&fx);
    for tech in [901, 902] {
        world.grant_tech(1, tech).expect("a marker");
    }
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    world
        .submit_checked(Command::Build {
            villager,
            unit: HOUSE,
            at,
        })
        .expect("two of three markers make houses available");
}

#[test]
fn an_availability_tech_with_one_marker_is_not_granted() {
    let Some(fx) = fixture() else { return };
    let mut world = two_of_three_world(&fx);
    world.grant_tech(1, 901).expect("one marker");
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    match world.submit_checked(Command::Build {
        villager,
        unit: HOUSE,
        at,
    }) {
        Err(SimError::UnitUnavailable { unit }) => assert_eq!(unit, HOUSE),
        other => panic!("expected UnitUnavailable, got {other:?}"),
    }
}

const HOUSE: u32 = 70;
