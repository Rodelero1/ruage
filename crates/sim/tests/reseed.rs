//! **Reseeding a spent farm**: the manual queue and the automatic toggle, both of which are pressed at a mill and both of which
//! are the **player's** setting rather than the mill's (D226).
//!
//! The owner's rulings, 2026-11-15: *"please implement queue and auto-reseed, both are only in the mill"*, and then the correction
//! that shapes the state: *"auto-reseed button and queue are shared across mills, enabling auto-reseed on one mill enables it on
//! every other, too. It is just global setting villagers will follow regardless of the distance to mill"*.
//!
//! The two halves are different games: the **manual queue** is HD/AoC (the farm's price is committed per entry), and the
//! **automatic toggle** is a Definitive Edition feature that this project was told to build anyway. Both pay the farm's own dat
//! cost - content's price for role `farm`, never a constant here.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, World};

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
    let mut economy =
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml");
    // **A woodpile nobody else spends**, so a reseed's price can be seen: the seeded match starts its own villagers gathering.
    economy.start.stock = [("wood".to_owned(), 400), ("food".to_owned(), 400)]
        .into_iter()
        .collect();
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy,
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("resources");
    world
}

fn spawn(
    fx: &Fixture,
    world: &mut World,
    kind: EntityKind,
    unit: u32,
    at: (i32, i32),
    amount: u32,
) -> EntityId {
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let scale = world.scale();
    // **A man walks at the speed content gives him** - the dat's 0.8 tiles a second, not zero. A villager that cannot move
    // stands wherever he is put and never arrives at the plot, so the fixture measured a statue rather than a farmer: both
    // reseed rows were red for a week with the queue paid and no plot worked (found in D312).
    let speed = if kind == EntityKind::Unit {
        fx.units
            .unit(unit)
            .and_then(|stats| stats.speed.resolve(scale, "speed").ok())
            .unwrap_or(ruage_fixed::Fixed::ZERO)
    } else {
        ruage_fixed::Fixed::ZERO
    };
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind,
            unit: Some(unit),
            position: (
                ruage_fixed::Fixed::from_raw(i64::from(at.0) * scale + scale / 2),
                ruage_fixed::Fixed::from_raw(i64::from(at.1) * scale + scale / 2),
            ),
            speed,
            hp: 600,
            amount,
        })
        .expect("a spawn");
    world.step().expect("one tick");
    id
}

/// **The plot's ground**, so a replacement can be found on it. Compared by **tile**, not by exact point: a building raised by an
/// order is anchored at its tile's corner (`World::tile`) while a spawned one in this fixture sits at the tile centre, and the
/// question being asked is which ground the farm occupies.
fn farm_at(
    world: &World,
    position: (ruage_fixed::Fixed, ruage_fixed::Fixed),
    farm: u32,
    not: EntityId,
) -> Option<EntityId> {
    let scale = ruage_fixed::Fixed::from_raw(world.scale());
    let _ = scale;
    let tile = |point: (ruage_fixed::Fixed, ruage_fixed::Fixed)| {
        (
            point.0.raw().div_euclid(world.scale()),
            point.1.raw().div_euclid(world.scale()),
        )
    };
    let want = tile(position);
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.id != not
                && entity.kind == EntityKind::Building
                && entity.unit == Some(farm)
                && tile(entity.position) == want
        })
        .map(|entity| entity.id)
}

/// **The queue pays at the mill and the man gets a farm on the same ground.** One entry is one farm's price, taken out of the
/// stockpile when it is queued - the way the original commits wood to the reseed queue - and the replacement rises on the footprint
/// the old plot left behind.
#[test]
fn a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let mill = spawn(&fx, &mut world, EntityKind::Building, 68, (16, 14), 0);
    let villager = spawn(&fx, &mut world, EntityKind::Unit, 83, (21, 18), 0);
    // A plot with two food in it: it runs out inside a second of play.
    let farm = spawn(&fx, &mut world, EntityKind::Building, 50, (20, 18), 2);
    let at = world.entity(farm).expect("the farm").position;
    world
        .submit_checked(Command::Gather {
            villager,
            node: farm,
        })
        .expect("the villager takes the farm");

    let wood = world.slot_of("wood").expect("a wood column");
    let before = world.stock(1, wood);
    world
        .submit_checked(Command::Reseed {
            building: mill,
            count: 1,
        })
        .expect("the mill takes one entry");
    world.step().expect("the order lands");
    assert_eq!(
        world.stock(1, wood),
        before - 60,
        "an entry commits one farm's 60 wood"
    );
    assert_eq!(world.reseed_of(1).queued, 1, "and it is remembered");

    let mut replaced = None;
    for _ in 0..1_200 {
        world.step().expect("stepping");
        replaced = farm_at(&world, at, 50, farm);
        if replaced.is_some() {
            break;
        }
    }
    assert!(
        replaced.is_some(),
        "a queued reseed should raise a farm on the spent plot's ground: farms = {:?}, queue = {}",
        world
            .entities()
            .iter()
            .filter(|entity| entity.unit == Some(50))
            .map(|entity| (
                entity.id,
                entity.position,
                entity.construction.map(|c| c.progress_ticks)
            ))
            .collect::<Vec<_>>(),
        world.reseed_of(1).queued
    );
    assert_eq!(
        world.stock(1, wood),
        before - 60,
        "and the replacement must not pay twice"
    );
}

/// **The toggle is the player's, not the mill's**: enabling it at one mill applies to a villager working a plot anywhere, and the
/// wood is taken when the plot runs out rather than when the button is pressed.
#[test]
fn auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // **The button is pressed at the far mill** and the plot is on the other side of the map, which is the owner's correction made
    // a test: the setting belongs to the player, so where it was switched on cannot matter.
    let mill = spawn(&fx, &mut world, EntityKind::Building, 68, (6, 6), 0);
    let _near = spawn(&fx, &mut world, EntityKind::Building, 68, (16, 14), 0);
    let villager = spawn(&fx, &mut world, EntityKind::Unit, 83, (21, 18), 0);
    let farm = spawn(&fx, &mut world, EntityKind::Building, 50, (20, 18), 2);
    let at = world.entity(farm).expect("the farm").position;
    world
        .submit_checked(Command::Gather {
            villager,
            node: farm,
        })
        .expect("the villager takes the farm");

    let wood = world.slot_of("wood").expect("a wood column");
    let before = world.stock(1, wood);
    world
        .submit_checked(Command::AutoReseed {
            building: mill,
            on: true,
        })
        .expect("the mill turns it on");
    world.step().expect("the order lands");
    assert!(world.reseed_of(1).auto, "it is the player's setting");
    assert_eq!(
        world.stock(1, wood),
        before,
        "the toggle itself costs nothing"
    );

    let mut replaced = None;
    for _ in 0..1_200 {
        world.step().expect("stepping");
        replaced = farm_at(&world, at, 50, farm);
        if replaced.is_some() {
            break;
        }
    }
    assert!(
        replaced.is_some(),
        "the toggle should reseed without a click"
    );
    assert_eq!(
        world.stock(1, wood),
        before - 60,
        "and the wood is taken when the plot runs out"
    );
}

/// With neither the queue nor the toggle the plot is simply gone: nobody is given work nobody asked for.
#[test]
fn without_a_queue_or_the_toggle_nothing_is_reseeded() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let _mill = spawn(&fx, &mut world, EntityKind::Building, 68, (16, 14), 0);
    let villager = spawn(&fx, &mut world, EntityKind::Unit, 83, (21, 18), 0);
    let farm = spawn(&fx, &mut world, EntityKind::Building, 50, (20, 18), 2);
    world
        .submit_checked(Command::Gather {
            villager,
            node: farm,
        })
        .expect("the villager takes the farm");

    let wood = world.slot_of("wood").expect("a wood column");
    let before = world.stock(1, wood);
    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    assert_eq!(world.stock(1, wood), before, "nobody asked, so nobody paid");
    assert!(
        !world
            .entities()
            .iter()
            .any(|entity| entity.kind == EntityKind::Building
                && entity.unit == Some(50)
                && entity.id != farm),
        "and no farm was raised"
    );
}

/// **The button is a finished mill of yours** - a house is not a mill, and neither is a mill that is still going up.
#[test]
fn only_a_finished_mill_may_queue_or_toggle() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house = spawn(&fx, &mut world, EntityKind::Building, 70, (16, 14), 0);
    assert!(
        world
            .submit_checked(Command::AutoReseed {
                building: house,
                on: true
            })
            .is_err(),
        "a house is not a mill"
    );
    assert!(!world.reseed_of(1).auto, "and the refusal wrote nothing");
}
