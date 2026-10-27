//! Population and housing: what a player may add to their army, and what raises the ceiling.
//!
//! Written before the implementation (D7). `[population]` has been in `economy.toml` since the
//! beginning and nothing read it, so until now a house was 25 wood and a shape on the map.

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
    civs: ruage_data::Civs,
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
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    seeded_with(fx, &fx.economy, true)
}

/// A world with wood and food to spare: every test here builds and trains, and the price of either is
/// not what is under test.
fn seeded_with(fx: &Fixture, economy: &Economy, civs: bool) -> World {
    let mut economy = economy.clone();
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 1_000),
    ]
    .into_iter()
    .collect();
    let economy = &economy;
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    if civs {
        // The bar rules live in the civ's tech-tree effect, so the effect table comes with them.
        world
            .load_research(&fx.techs, &fx.effects)
            .expect("research tables");
        world.load_civs(&fx.civs).expect("the civ table");
    }
    world
}

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

fn a_free_tile(world: &World) -> (u16, u16) {
    let scale = world.scale();
    let centre = world.entity(town_centre(world)).expect("tc");
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

/// Queue a house and run until it is finished.
fn a_finished_house(world: &mut World) -> EntityId {
    let villager = first_villager(world);
    let at = a_free_tile(world);
    world.submit(Command::Build {
        villager,
        unit: HOUSE,
        at,
    });
    world.step().expect("step");
    let site = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(HOUSE))
        .map(|e| e.id)
        .next()
        .expect("the house site exists");
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            return site;
        }
        world.step().expect("step");
    }
    panic!("the house never finished");
}

/// Fill the player up to their cap by training villagers, and return how many that took.
fn train_until_refused(world: &mut World) -> u32 {
    let centre = town_centre(world);
    let baseline = villager_count(world);
    let mut trained = 0;
    for _ in 0..40 {
        match world.submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        }) {
            Ok(()) => {
                trained += 1;
                // Let the villager arrive so the population is real before the next order: one slot
                // per building, so ordering again too early would be a different refusal.
                let want = baseline + usize::try_from(trained).expect("a small number");
                let mut arrived = false;
                for _ in 0..4_000 {
                    world.step().expect("step");
                    if villager_count(world) >= want {
                        arrived = true;
                        break;
                    }
                }
                if !arrived {
                    // The finished unit waits inside its building when every neighbouring tile is
                    // taken, which is what a crowd around a town centre does. That is the engine
                    // being careful, not the cap, and it is not what this helper measures.
                    return trained - 1;
                }
            }
            Err(SimError::PopulationCapped { .. }) => return trained,
            Err(other) => panic!("expected a population refusal, got {other:?}"),
        }
    }
    panic!("the player trained forty villagers: nothing is capping them");
}

fn villager_count(world: &World) -> usize {
    world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .count()
}

const HOUSE: u32 = 70;
const VILLAGER: u32 = 83;

#[test]
fn a_town_centre_supports_five_units() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Three villagers start, the town centre holds five: two more fit, the third does not.
    assert_eq!(train_until_refused(&mut world), 2);
}

#[test]
fn a_finished_house_raises_the_cap() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    a_finished_house(&mut world);
    assert_eq!(
        train_until_refused(&mut world),
        7,
        "five from the town centre plus five from the house, minus the three villagers already there"
    );
}

#[test]
fn a_house_under_construction_supports_nothing_yet() {
    let Some(fx) = fixture() else { return };
    // Five villagers fill the town centre's own five, so the only thing that can let another one in
    // is a finished house.
    let mut economy = fx.economy.clone();
    economy.start.villagers_per_player = 5;
    let mut world = seeded_with(&fx, &economy, true);
    let villager = first_villager(&world);
    let at = a_free_tile(&world);
    world.submit(Command::Build {
        villager,
        unit: HOUSE,
        at,
    });
    world.step().expect("step");
    let site = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(HOUSE))
        .map(|e| e.id)
        .next()
        .expect("the house site exists");
    assert!(
        world.entity(site).expect("site").construction.is_some(),
        "the house is not built yet"
    );

    let centre = town_centre(&world);
    match world.submit_checked(Command::Train {
        building: centre,
        unit: VILLAGER,
    }) {
        Err(SimError::PopulationCapped {
            cap, population, ..
        }) => {
            assert_eq!((population, cap), (5, 5), "five of five, house not counted");
        }
        other => panic!("expected PopulationCapped, got {other:?}"),
    }

    // Finish it, and the same order is allowed.
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            break;
        }
        world.step().expect("step");
    }
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("a finished house supports five more");
}

/// The Huns cannot build houses at all (their tech tree takes the house away), so a cap that counts
/// houses would leave them at zero for the whole match. Content's ceiling is their ceiling.
#[test]
fn a_civilisation_that_cannot_build_houses_gets_the_whole_cap() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.start.civs = vec!["Huns".to_owned()];
    let mut world = seeded_with(&fx, &economy, true);
    let trained = train_until_refused(&mut world);
    assert!(
        trained >= 20,
        "the Huns train past any house-derived cap, and trained {trained}"
    );
}

/// The ceiling is content's, not the sum of the buildings: forty houses do not buy four hundred units.
#[test]
fn the_cap_stops_at_what_content_allows() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.population.cap_default = 8;
    economy.population.house_population = 5;
    let mut world = seeded_with(&fx, &economy, true);
    a_finished_house(&mut world);
    // Three villagers, cap 8: five more fit, whatever the house arithmetic says.
    assert_eq!(train_until_refused(&mut world), 5);
}
