//! Fog, and what a player *remembers*. The simulation already answers three states per tile (unseen, explored,
//! visible); this is about what the picture keeps: a building the player has seen stays on the screen when nobody is
//! watching it any more, and a unit does not. Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render};

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
    combat: ruage_data::Combat,
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
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

/// A house of player 2 and a villager of player 2, both placed where player 1's base can see them.
fn enemy_in_sight(fx: &Fixture) -> (World, EntityId, EntityId) {
    let mut world = seeded(fx);
    let scale = world.scale();
    let house = fx.units.root(world.role_unit("house").expect("house"));
    let villager = world.role_unit("villager").expect("villager");
    let building = EntityId(9_100);
    let unit = EntityId(9_101);
    for (id, kind, unit_id, at) in [
        (building, EntityKind::Building, house, (20u16, 20u16)),
        (unit, EntityKind::Unit, villager, (20, 21)),
    ] {
        world
            .submit_checked(Command::Spawn {
                id,
                player: 2,
                kind,
                unit: Some(unit_id),
                position: (
                    Fixed::from_raw(i64::from(at.0) * scale),
                    Fixed::from_raw(i64::from(at.1) * scale),
                ),
                speed: Fixed::ZERO,
                hp: if kind == EntityKind::Building {
                    550
                } else {
                    25
                },
                amount: 0,
            })
            .expect("a spawned enemy");
    }
    for _ in 0..2 {
        world.step().expect("step");
    }
    (world, building, unit)
}

/// Take player 1's sight away by deleting everything they own: the ground stays explored, nothing is watched.
fn blind(world: &mut World) {
    let mine: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1)
        .map(|entity| entity.id)
        .collect();
    assert!(!mine.is_empty(), "player 1 owns nothing to delete");
    for id in mine {
        let _ = world.submit_checked(Command::Delete {
            player: 1,
            entity: id,
        });
    }
    for _ in 0..2 {
        world.step().expect("step");
    }
}

#[test]
fn a_building_the_player_has_seen_is_remembered_and_a_unit_is_not() {
    let Some(fx) = fixture() else { return };
    let (mut world, building, unit) = enemy_in_sight(&fx);

    // While it is watched it is simply visible.
    let seen = Snapshot::of(&world, &fx.map, 1);
    let watch = seen
        .entities
        .iter()
        .find(|entity| entity.id == building.0)
        .expect("the enemy house is in sight");
    assert!(!watch.remembered, "a watched building is not a memory");
    assert!(
        seen.entities.iter().any(|entity| entity.id == unit.0),
        "the enemy villager in sight is missing"
    );

    blind(&mut world);
    let memory = Snapshot::of(&world, &fx.map, 1);
    let house = memory
        .entities
        .iter()
        .find(|entity| entity.id == building.0)
        .expect("a building the player has seen vanished when nobody was watching it");
    assert!(
        house.remembered,
        "the house is drawn as if it were under the player's eye"
    );
    assert!(
        !memory.entities.iter().any(|entity| entity.id == unit.0),
        "a unit was remembered: armies do not leave a picture behind"
    );
}

#[test]
fn ground_nobody_has_scouted_stays_empty() {
    let Some(fx) = fixture() else { return };
    let (world, _, _) = enemy_in_sight(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    // The enemy's own town centre, across the map and never scouted, is still not in player 1's view.
    let far = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 2
                && entity.kind == EntityKind::Building
                && entity.id != EntityId(9_100)
        })
        .expect("the enemy town centre")
        .id;
    assert!(
        snapshot.entities.iter().all(|entity| entity.id != far.0),
        "an unscouted corner of the map leaked into the view"
    );
}

/// A remembered building is drawn **dimmer** than a watched one, because the ground under it is drawn as memory; a
/// memory painted in full colour reads as a live building and is how fog gets called a bug.
#[test]
fn a_remembered_building_is_drawn_dimmer_than_a_watched_one() {
    let Some(fx) = fixture() else { return };
    let (mut world, building, _) = enemy_in_sight(&fx);
    let camera = Camera::new((640, 480)).centred_on((20.0, 20.0));
    let watched = render(&Snapshot::of(&world, &fx.map, 1), &camera);
    let at = camera.project((20.0, 20.0));
    let lit = watched.pixel(at.0 as u32, at.1 as u32);
    assert_eq!(lit, palette::BUILDING, "the watched house was not drawn");

    blind(&mut world);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    assert!(
        snapshot
            .entities
            .iter()
            .any(|entity| entity.id == building.0),
        "the house is not in the view to be dimmed"
    );
    let remembered = render(&snapshot, &camera);
    let dim = remembered.pixel(at.0 as u32, at.1 as u32);
    assert_ne!(
        dim, lit,
        "a remembered building was drawn at full brightness"
    );
    let luma = |pixel: [u8; 4]| u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
    assert!(
        luma(dim) < luma(lit),
        "a remembered building is brighter than a watched one: {dim:?} vs {lit:?}"
    );
}
