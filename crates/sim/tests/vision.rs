//! Vision: what a player can see, and what they merely remember.
//!
//! Written before the implementation (D7). Three states per tile, and the difference between the last two
//! is the whole point of fog: **unseen** (never looked at), **explored** (looked at once, remembered) and
//! **visible** (looked at now). The radius is the unit's own `los` from the dat - 844 units carry one, and
//! until now nothing asked.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, TileVision, World};

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

/// The speed the unit table gives this unit. A scout that has to walk has to have one, and a test
/// never states a speed of its own (D6).
fn walk_speed(world: &World, fx: &Fixture, unit: u32) -> Fixed {
    fx.units
        .unit(unit)
        .expect("in the table")
        .speed
        .resolve(world.scale(), "speed")
        .expect("a walker")
}

fn place_unit(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    speed: Fixed,
) -> EntityId {
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
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
    EntityId(id)
}

/// A unit's own sight radius from the table, in whole tiles: the tests never state a radius.
fn sight(world: &World, fx: &Fixture, unit: u32) -> i64 {
    fx.units
        .unit(unit)
        .expect("in the table")
        .los
        .resolve(world.scale(), "los")
        .expect("a sight radius")
        .raw()
        .div_euclid(world.scale())
}

#[test]
fn a_player_sees_around_its_own_and_not_across_the_map() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let mine = villager_of(&world, 1);
    let (x, y) = world
        .position(mine)
        .map(|(x, y)| {
            (
                x.raw().div_euclid(world.scale()) as u16,
                y.raw().div_euclid(world.scale()) as u16,
            )
        })
        .expect("a villager");

    assert_eq!(
        world.vision(1, (x, y)),
        TileVision::Visible,
        "a player cannot see the ground under their own villager"
    );
    assert_eq!(
        world.vision(1, (47, 47)),
        TileVision::Unseen,
        "the enemy's town centre is visible from across the map"
    );
}

#[test]
fn the_radius_is_the_unit_s_own_line_of_sight() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["villager"]);
    let villager = fx.economy.role["villager"];
    let r = sight(&world, &fx, villager);
    assert!(r >= 1, "a villager with no sight would see nothing");
    let scout = place_unit(&mut world, 1, 9_500, villager, (30, 30), speed);

    assert_eq!(
        world.vision(1, (30, 30)),
        TileVision::Visible,
        "standing somewhere is not seeing it"
    );
    assert_eq!(
        world.vision(1, ((30 + r) as u16, 30)),
        TileVision::Visible,
        "the edge of this unit's own sight radius is not visible"
    );
    assert_eq!(
        world.vision(1, ((30 + r + 2) as u16, 30)),
        TileVision::Unseen,
        "sight reaches further than the unit's `los`"
    );
    let _ = scout;
}

#[test]
fn a_building_sees_around_itself_when_it_appears() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let tower = fx.economy.role["castle"];
    // A building does not move, so nothing else will ever stamp its disc: if arriving does not count as
    // looking, a castle built in the dark stays blind for the rest of the match.
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_520),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(tower),
            position: (Fixed::from_raw(30 * scale), Fixed::from_raw(30 * scale)),
            speed: Fixed::ZERO,
            hp: 4_800,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");

    assert_eq!(
        world.vision(1, (30, 30)),
        TileVision::Visible,
        "a building that was just finished cannot see the ground it stands on"
    );
}

#[test]
fn explored_ground_stays_known_after_the_unit_leaves() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["villager"]);
    let villager = fx.economy.role["villager"];
    let scout = place_unit(&mut world, 1, 9_501, villager, (30, 30), speed);
    assert_eq!(world.vision(1, (30, 30)), TileVision::Visible);

    // Walk it away: the ground it stood on is remembered, not forgotten.
    world
        .submit_checked(Command::Move {
            unit: scout,
            to: (44, 44),
        })
        .expect("move");
    for _ in 0..600 {
        world.step().expect("step");
    }

    assert_eq!(
        world.vision(1, (30, 30)),
        TileVision::Explored,
        "ground a unit has left should be remembered, not forgotten"
    );
    assert_eq!(
        world.vision(1, (44, 44)),
        TileVision::Visible,
        "the unit cannot see where it is standing"
    );
}

#[test]
fn an_enemy_is_only_visible_while_somebody_watches() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["villager"]);
    let villager = fx.economy.role["villager"];
    let spearman = fx.economy.role["spearman"];
    let enemy = place_unit(&mut world, 2, 9_502, spearman, (31, 31), speed);
    assert!(
        !world.visible_to(1, enemy),
        "an enemy on a tile nobody has looked at is visible"
    );

    let scout = place_unit(&mut world, 1, 9_503, villager, (30, 30), speed);
    assert!(
        world.visible_to(1, enemy),
        "an enemy beside our own unit is not visible"
    );
    assert!(
        world.visible_to(2, enemy),
        "a player cannot see their own unit"
    );

    world
        .submit_checked(Command::Move {
            unit: scout,
            to: (44, 44),
        })
        .expect("move");
    for _ in 0..600 {
        world.step().expect("step");
    }
    assert!(
        !world.visible_to(1, enemy),
        "an enemy stayed visible after the only unit that could see it walked away"
    );
}

#[test]
fn vision_is_per_player() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let mine = villager_of(&world, 1);
    let theirs = villager_of(&world, 2);
    let tile_of = |world: &World, id: EntityId| {
        let (x, y) = world.position(id).expect("home");
        (
            x.raw().div_euclid(world.scale()) as u16,
            y.raw().div_euclid(world.scale()) as u16,
        )
    };
    let (x, y) = tile_of(&world, theirs);

    assert_eq!(
        world.vision(2, (x, y)),
        TileVision::Visible,
        "player 2 cannot see their own villager"
    );
    let _ = mine;
}

#[test]
fn a_world_without_units_sees_nothing() {
    // The control: vision comes from entities, so a world with none has every tile unseen. Without this,
    // "everything is visible" would pass every other test in this file.
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    for (x, y) in [(0u16, 0u16), (30, 30), (63, 63)] {
        assert_eq!(
            world.vision(1, (x, y)),
            TileVision::Unseen,
            "an empty world sees {x},{y}"
        );
    }
}

fn villager_of(world: &World, player: u8) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .expect("a starting villager")
        .id
}
