//! Collision: units do not stand on top of each other when there is room.
//!
//! Written before the implementation (D7). The prototype list asks for "unit collision / pushing", and the
//! owner chose soft avoidance: units slide out of each other's way, and may still end up sharing a tile when
//! there is nowhere else to go or when they were ordered there together - which is what the game does.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

fn walk_speed(world: &World, fx: &Fixture, unit: u32) -> Fixed {
    fx.units
        .unit(unit)
        .expect("in the table")
        .speed
        .resolve(world.scale(), "speed")
        .expect("a walker")
}

fn place(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    speed: Fixed,
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
            speed,
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity exists");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

#[test]
fn a_walker_does_not_finish_on_top_of_a_friend() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let speed = walk_speed(&world, &fx, spearman);
    let stander = place(&mut world, 1, 9_700, spearman, (24, 20), Fixed::ZERO);
    let walker = place(&mut world, 1, 9_701, spearman, (20, 20), speed);

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (24, 20),
        })
        .expect("move");
    for _ in 0..400 {
        world.step().expect("step");
    }

    assert_ne!(
        tile_of(&world, walker),
        tile_of(&world, stander),
        "the walker finished standing inside its friend"
    );
}

#[test]
fn a_crowd_sent_to_one_place_does_not_stack() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let speed = walk_speed(&world, &fx, spearman);
    let a = place(&mut world, 1, 9_702, spearman, (20, 20), speed);
    let b = place(&mut world, 1, 9_703, spearman, (20, 22), speed);
    let c = place(&mut world, 1, 9_704, spearman, (22, 20), speed);

    for unit in [a, b, c] {
        world
            .submit_checked(Command::Move { unit, to: (26, 22) })
            .expect("move");
    }
    for _ in 0..600 {
        world.step().expect("step");
    }

    let tiles = [a, b, c].map(|id| tile_of(&world, id));
    assert!(
        tiles[0] != tiles[1] && tiles[1] != tiles[2] && tiles[0] != tiles[2],
        "three units share a tile in open ground: {tiles:?}"
    );
}

#[test]
fn units_that_already_share_a_tile_are_not_pushed_apart() {
    // The control for D52, and the reason this is avoidance rather than pushing: a unit is not shoved out of
    // the way when it already overlaps. Pushing would be a different mechanic - every push can cascade, and
    // every cascade is state to hash - and the owner chose soft avoidance, not pushing.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let a = place(&mut world, 1, 9_705, spearman, (26, 22), Fixed::ZERO);
    let b = place(&mut world, 1, 9_706, spearman, (26, 22), Fixed::ZERO);

    for _ in 0..20 {
        world.step().expect("step");
    }

    assert_eq!(tile_of(&world, a), (26, 22));
    assert_eq!(
        tile_of(&world, b),
        (26, 22),
        "a standing unit was pushed out of a tile it was already sharing"
    );
}
