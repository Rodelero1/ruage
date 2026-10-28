//! Attack-move, patrol, and the plain move they are built on.
//!
//! Written before the implementation (D7). Attack-move and patrol are *one* task with two arrival
//! rules: walk towards a point while looking for a fight, and either stop there (attack-move) or
//! turn round (patrol). The fight is the existing `Task::Attack`, so what has to be new is only what
//! happens to the walk that the fight interrupted.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
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
    combat: ruage_data::Combat,
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

/// A unit that can walk: speed is resolved from the unit table, exactly as a trained one's would be,
/// so the test never states a speed of its own (D6).
fn march(
    world: &mut World,
    fx: &Fixture,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
) -> EntityId {
    let scale = world.scale();
    let speed = fx
        .units
        .unit(unit)
        .unwrap_or_else(|| panic!("unit {unit} is not in the table"))
        .speed
        .resolve(scale, "speed")
        .expect("a walkable unit");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

/// A unit that stands where it is put: the spawn hook gives a mover its speed as velocity, so a
/// target that must not wander has to be spawned with no speed at all.
fn place_still(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity exists");
    (x.raw() / scale, y.raw() / scale)
}

#[test]
fn an_attack_move_engages_what_it_meets_and_then_carries_on() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mine = march(&mut world, &fx, 1, 9_100, spearman, (20, 20), 45);
    let theirs = place_still(&mut world, 2, 9_101, spearman, (23, 20), 5);

    world
        .submit_checked(Command::AttackMove {
            unit: mine,
            to: (26, 20),
        })
        .expect("attack-move");

    for _ in 0..400 {
        world.step().expect("step");
    }

    assert!(
        world.entity(theirs).is_none(),
        "the spearman walked past an enemy without fighting it"
    );
    assert_eq!(
        tile_of(&world, mine),
        (26, 20),
        "the order was to go somewhere, and the fight was on the way"
    );
}

#[test]
fn a_plain_move_walks_past_the_same_enemy() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mine = march(&mut world, &fx, 1, 9_102, spearman, (20, 20), 45);
    let theirs = place_still(&mut world, 2, 9_103, spearman, (23, 20), 5);

    world
        .submit_checked(Command::Move {
            unit: mine,
            to: (26, 20),
        })
        .expect("move");

    for _ in 0..400 {
        world.step().expect("step");
    }

    assert!(
        world.entity(theirs).is_some(),
        "a plain move fought something: attack-move would then be no different"
    );
    assert_eq!(tile_of(&world, mine), (26, 20));
}

#[test]
fn an_attack_move_ignores_an_enemy_outside_its_reach() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mine = march(&mut world, &fx, 1, 9_104, spearman, (20, 20), 45);
    // Far enough that no aggro radius can reach it from the route.
    let theirs = place_still(&mut world, 2, 9_105, spearman, (20, 40), 5);

    world
        .submit_checked(Command::AttackMove {
            unit: mine,
            to: (26, 20),
        })
        .expect("attack-move");
    for _ in 0..400 {
        world.step().expect("step");
    }

    assert!(
        world.entity(theirs).is_some(),
        "the unit chased an enemy it should never have seen"
    );
    assert_eq!(tile_of(&world, mine), (26, 20));
}

#[test]
fn a_patrol_turns_round_and_comes_back() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mine = march(&mut world, &fx, 1, 9_106, spearman, (20, 20), 45);

    world
        .submit_checked(Command::Patrol {
            unit: mine,
            to: (24, 20),
        })
        .expect("patrol");

    let mut furthest = 0;
    let mut came_back = false;
    for _ in 0..1200 {
        world.step().expect("step");
        let (x, _) = tile_of(&world, mine);
        furthest = furthest.max(x);
        if furthest >= 24 && x <= 20 {
            came_back = true;
            break;
        }
    }

    assert!(
        furthest >= 24,
        "the patrol never reached its far end: {furthest}"
    );
    assert!(
        came_back,
        "the patrol reached {furthest} and never came back to where it started"
    );
}

#[test]
fn an_order_off_the_map_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mine = march(&mut world, &fx, 1, 9_108, spearman, (20, 20), 45);
    let err = world
        .submit_checked(Command::AttackMove {
            unit: mine,
            to: (9_999, 9_999),
        })
        .expect_err("there is no such tile");
    assert!(matches!(err, SimError::OffMap { .. }), "got {err:?}");
}

#[test]
fn a_building_cannot_be_ordered_to_patrol() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id;
    let err = world
        .submit_checked(Command::Patrol {
            unit: centre,
            to: (24, 20),
        })
        .expect_err("a building does not walk");
    assert!(
        matches!(err, SimError::NotAUnit { id } if id == centre),
        "got {err:?}"
    );
}
