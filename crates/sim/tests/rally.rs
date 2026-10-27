//! Rally points: where a building's finished units go.
//!
//! Written before the implementation (D7). Two behaviours, both from the game: a rally on a tile sends the
//! unit there, and a rally on the building **itself** keeps it inside - which is the only way a production
//! building ever holds anybody (D38).

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityId, EntityKind, Task, World};
use std::path::PathBuf;

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
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut economy = fx.economy.clone();
    economy.start.stock = [("food".to_owned(), 4_000), ("wood".to_owned(), 4_000)]
        .into_iter()
        .collect();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
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

fn tile_of(world: &World, id: EntityId) -> (u16, u16) {
    let scale = world.scale();
    let position = world.position(id).expect("position");
    (
        u16::try_from(position.0.raw() / scale).unwrap_or(0),
        u16::try_from(position.1.raw() / scale).unwrap_or(0),
    )
}

const VILLAGER: u32 = 83;

#[test]
fn a_trained_unit_walks_to_the_rally_point() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let here = tile_of(&world, centre);
    let rally = (here.0 + 6, here.1 + 6);
    world
        .submit_checked(Command::Rally {
            building: centre,
            point: Some(rally),
        })
        .expect("a rally point on free ground");
    world.step().expect("step");
    let before = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .count();

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train a villager");
    // 25 s to train plus the walk: six tiles at 0.8 tiles a second is about eight more seconds.
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    let newcomers: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .collect();
    assert_eq!(newcomers.len(), before + 1, "the villager arrived");
    let walker = newcomers
        .iter()
        .copied()
        .find(|id| {
            let tile = tile_of(&world, *id);
            tile.0.abs_diff(rally.0) <= 1 && tile.1.abs_diff(rally.1) <= 1
        })
        .expect("the new villager is at the rally point");
    assert_eq!(
        world.entity(walker).expect("villager").task,
        Task::Idle,
        "it walked there and stopped"
    );
}

/// The half of D38 that needed rally points: a unit rallied to its own building stays inside it.
#[test]
fn a_unit_rallied_to_its_own_building_stays_inside() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let here = tile_of(&world, centre);
    world
        .submit_checked(Command::Rally {
            building: centre,
            point: Some(here),
        })
        .expect("the building itself is a legal rally");
    world.step().expect("step");

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train a villager");
    for _ in 0..600 {
        world.step().expect("step");
    }

    let inside: Vec<&ruage_sim::Entity> = world
        .entities()
        .iter()
        .filter(|e| e.task == (Task::Garrisoned { building: centre }))
        .collect();
    assert_eq!(inside.len(), 1, "the finished villager went inside");
    assert_eq!(inside[0].player, 1);
}

/// No rally means the old behaviour: the unit appears beside the building and stands there.
#[test]
fn without_a_rally_a_trained_unit_stays_where_it_appeared() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..600 {
        world.step().expect("step");
    }
    let home = tile_of(&world, centre);
    let newest = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .max()
        .expect("a unit");
    let tile = tile_of(&world, newest);
    assert!(
        tile.0.abs_diff(home.0) <= 2 && tile.1.abs_diff(home.1) <= 2,
        "it stands beside the building: {tile:?} against {home:?}"
    );
    assert!(matches!(
        world.entity(newest).expect("unit").task,
        Task::Idle
    ));
}

/// A rally point is cleared, and the building goes back to putting units beside itself.
#[test]
fn a_rally_point_can_be_cleared() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let here = tile_of(&world, centre);
    world
        .submit_checked(Command::Rally {
            building: centre,
            point: Some((here.0 + 5, here.1)),
        })
        .expect("a rally");
    world.step().expect("step");
    world
        .submit_checked(Command::Rally {
            building: centre,
            point: None,
        })
        .expect("clearing is an order too");
    world.step().expect("step");
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..600 {
        world.step().expect("step");
    }
    let newest = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .max()
        .expect("a unit");
    let tile = tile_of(&world, newest);
    assert!(
        tile.0.abs_diff(here.0) <= 2 && tile.1.abs_diff(here.1) <= 2,
        "with the rally cleared it stays home: {tile:?} against {here:?}"
    );
}

/// A rally point is a place: it cannot be outside the map, and it cannot be another player's business.
#[test]
fn a_rally_point_outside_the_map_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    match world.submit_checked(Command::Rally {
        building: centre,
        point: Some((30_000, 30_000)),
    }) {
        Err(ruage_sim::SimError::OffMap { .. }) => {}
        other => panic!("expected OffMap, got {other:?}"),
    }
}
