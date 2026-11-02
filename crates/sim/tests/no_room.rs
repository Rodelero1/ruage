//! D85: what happens to a trained unit that has nowhere to stand.
//!
//! Written before the implementation (D7). The owner's rule, after the footprint masks exposed it: a finished unit with
//! **no free tile beside its building** is garrisoned **inside that building**; when there is **no garrison room either**,
//! training **pauses** at that building with the reason in state - never dropped, never silently retried. The engine owns
//! the reason (`NoRoom`); the words on screen are the view's (`docs/ui.md`).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, TrainingPause, World};

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
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

/// A world with food and wood to spare: every test here trains, and the price of a villager is not what is under test.
fn seeded(fx: &Fixture, economy: &Economy) -> World {
    let mut economy = economy.clone();
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 1_000),
    ]
    .into_iter()
    .collect();
    // The blockers below are real units of the player's, so they hold population - and the ceiling would pause
    // training before the yard ever did, which is a different rule (D34) and would make every test here pass or fail
    // for the wrong reason. The ceiling is `min(sum of finished buildings, cap_default)`, so both numbers move; the
    // ceiling has its own tests.
    economy.population.cap_default = 1_000;
    economy.population.town_center_population = 500;
    // A spawned building with nobody working it decays (`[decay] enabled`), and a market left alone for five game
    // minutes was gone by the end of the run - which silently un-blocked its own yard and let the recruit walk out. The
    // rule under test is placement, not rot; decay keeps its own tests.
    economy.decay.enabled = false;
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("the civ table");
    world
}

fn town_center(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id
}

fn villagers(world: &World) -> usize {
    world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .count()
}

fn tile_of(world: &World, position: (Fixed, Fixed)) -> (u16, u16) {
    let scale = world.scale();
    (
        u16::try_from(position.0.raw() / scale).unwrap(),
        u16::try_from(position.1.raw() / scale).unwrap(),
    )
}

fn position_of(world: &World, (x, y): (u16, u16)) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    )
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn occupied(world: &World, tile: (u16, u16)) -> bool {
    world
        .entities()
        .iter()
        .any(|e| tile_of(world, e.position) == tile)
}

/// The tiles a finished unit may be placed on: the 5x5 box round its building that is walkable and holds nothing.
fn free_tiles_in_the_yard(world: &World, building: EntityId) -> Vec<(u16, u16)> {
    let origin = tile_of(world, world.entity(building).expect("building").position);
    let mut free = Vec::new();
    for dy in -2i64..=2 {
        for dx in -2i64..=2 {
            let (x, y) = (i64::from(origin.0) + dx, i64::from(origin.1) + dy);
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (u16::try_from(x).unwrap(), u16::try_from(y).unwrap());
            if world.tile_walkable(x, y) && !occupied(world, (x, y)) {
                free.push((x, y));
            }
        }
    }
    free
}

/// Stand a unit on every tile a recruit could otherwise be placed on, and prove the yard really is full - a fixture that
/// silently leaves a tile free would make every test below pass for the wrong reason.
fn block_the_yard(world: &mut World, building: EntityId) {
    let free = free_tiles_in_the_yard(world, building);
    assert!(
        !free.is_empty(),
        "the fixture needs at least one free tile to block"
    );
    let mut id = next_id(world);
    for (x, y) in free {
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(VILLAGER),
                position: position_of(world, (x, y)),
                speed: Fixed::ZERO,
                hp: 25,
                amount: 0,
            })
            .expect("a blocker stands on the tile");
        // Orders land a tick later, so the next id cannot be read back from the world yet.
        id = EntityId(id.0 + 1);
    }
    world.step().expect("step");
    assert!(
        free_tiles_in_the_yard(world, building).is_empty(),
        "the yard is full: {:?} still free",
        free_tiles_in_the_yard(world, building)
    );
}

const VILLAGER: u32 = 83;
const MARKET: u32 = 84;
const TOWN_CENTRE: u32 = 109;

/// D85 rule 1: no free tile, so the recruit goes **inside** - it is not lost, and the queue behind it moves on.
#[test]
fn a_unit_with_nowhere_to_stand_is_garrisoned_not_dropped() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx, &fx.economy);
    let centre = town_center(&world);
    block_the_yard(&mut world, centre);
    let before = villagers(&world);

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..6_000 {
        world.step().expect("step");
        if villagers(&world) > before {
            break;
        }
    }

    assert_eq!(villagers(&world), before + 1, "the recruit was not lost");
    let recruit = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .find(|e| e.task == (Task::Garrisoned { building: centre }))
        .expect("a recruit with nowhere to stand is garrisoned in its building");
    assert_eq!(
        world.training_pause(centre),
        None,
        "the queue is not paused: the recruit was placed inside"
    );
    assert!(
        recruit.hp > 0,
        "a sheltered recruit is a live unit, not a phantom"
    );
}

/// D85 rule 2: nowhere to stand **and** no room inside, so the queue pauses with a reason a panel can draw.
#[test]
fn a_building_with_no_room_and_nowhere_to_stand_pauses_with_a_reason() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    // A market shelters nobody (its `garrison` is 0 in the dat), so this is the "no room either" case without
    // garrisoning fifteen peasants first. Content decides what a building makes; the engine owns the rule.
    economy.train.by_building.insert(MARKET, vec![VILLAGER]);
    let mut world = seeded(&fx, &economy);
    let market = next_id(&world);
    let spot = position_of(&world, (20, 20));
    world
        .submit_checked(Command::Spawn {
            id: market,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(MARKET),
            position: spot,
            speed: Fixed::ZERO,
            hp: 1_800,
            amount: 0,
        })
        .expect("a market");
    world.step().expect("step");
    block_the_yard(&mut world, market);
    let before = villagers(&world);

    world
        .submit_checked(Command::Train {
            building: market,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..6_000 {
        world.step().expect("step");
    }

    assert_eq!(
        villagers(&world),
        before,
        "no recruit appears out of thin air"
    );
    assert!(
        world
            .entity(market)
            .is_some_and(|entity| entity.training.is_some()),
        "the job stays in the building rather than being dropped"
    );
    assert_eq!(
        world.training_pause(market),
        Some(TrainingPause::NoRoom),
        "a queue that cannot place anything says why"
    );
}

/// The reason is **state, not an event**: it clears the frame the tile frees, and the queue advances.
#[test]
fn the_reason_clears_the_tick_a_tile_frees() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.train.by_building.insert(MARKET, vec![VILLAGER]);
    let mut world = seeded(&fx, &economy);
    let market = next_id(&world);
    let spot = position_of(&world, (20, 20));
    world
        .submit_checked(Command::Spawn {
            id: market,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(MARKET),
            position: spot,
            speed: Fixed::ZERO,
            hp: 1_800,
            amount: 0,
        })
        .expect("a market");
    world.step().expect("step");
    block_the_yard(&mut world, market);
    let before = villagers(&world);
    world
        .submit_checked(Command::Train {
            building: market,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..6_000 {
        world.step().expect("step");
    }
    assert_eq!(
        world.training_pause(market),
        Some(TrainingPause::NoRoom),
        "precondition: the queue is blocked"
    );

    // A blocker dies, so a tile beside the building is free again.
    let blocker = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .max_by_key(|e| e.id.0)
        .expect("a blocker")
        .id;
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: blocker,
        })
        .expect("delete the blocker");
    for _ in 0..200 {
        world.step().expect("step");
        if villagers(&world) >= before {
            break;
        }
    }

    assert!(
        villagers(&world) >= before,
        "the freed tile let the recruit out: {} villagers, expected {before}",
        villagers(&world)
    );
    assert_eq!(
        world.training_pause(market),
        None,
        "the message goes away the same frame the block does"
    );
}

/// A building already at or below its eject threshold shelters nobody (D37) - the rule that pushes a garrison **out** is
/// the same reason a recruit is not put **in**. With nowhere to stand and nothing to shelter in, the queue says why.
#[test]
fn a_building_too_damaged_to_shelter_reports_no_room() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx, &fx.economy);
    let centre = next_id(&world);
    let spot = position_of(&world, (20, 20));
    // A town centre at ten hit points: its maximum is 2,400 and the eject fraction is a fifth, so this one has already
    // given its garrison up.
    world
        .submit_checked(Command::Spawn {
            id: centre,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(TOWN_CENTRE),
            position: spot,
            speed: Fixed::ZERO,
            hp: 10,
            amount: 0,
        })
        .expect("a wrecked town centre");
    world.step().expect("step");
    block_the_yard(&mut world, centre);
    let before = villagers(&world);

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("train");
    for _ in 0..6_000 {
        world.step().expect("step");
    }

    assert_eq!(
        villagers(&world),
        before,
        "no recruit appears out of thin air"
    );
    assert_eq!(
        world.training_pause(centre),
        Some(TrainingPause::NoRoom),
        "a wounded building cannot shelter the recruit it made"
    );
}
