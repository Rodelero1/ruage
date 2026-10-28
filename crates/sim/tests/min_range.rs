//! Minimum range: the dead zone a siege engine has around itself.
//!
//! Written before the implementation (D7). The dat stores a `min_range` per unit next to `max_range` - a
//! mangonel's is three tiles, a bombard cannon's five - so the rule is data, not a list of siege ids. A unit
//! inside its own dead zone cannot fire: it backs off until it has room, and the engine is the one that has to
//! do the backing off, or a mangonel ordered onto a neighbouring unit would stand there forever.

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
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

fn world_with(fx: &Fixture) -> World {
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
    // No scatter: this is about where a shot may be fired from, not about aiming.
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
}

fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (u16, u16), hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            speed: Fixed::from_raw((0.6 * f64::from(scale as u16)) as i64),
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tiles_between(world: &World, a: EntityId, b: EntityId) -> f64 {
    let scale = f64::from(world.scale() as u16);
    let (a, b) = (world.position(a).expect("a"), world.position(b).expect("b"));
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale;
    (dx * dx + dy * dy).sqrt()
}

const MANGONEL: u32 = 280;
const ARCHER: u32 = 4;
const VILLAGER: u32 = 83;

/// The data's own numbers, asserted first: the whole test rests on a mangonel having a dead zone and an archer
/// having none, so the fixture says so rather than trusting a comment.
#[test]
fn the_data_gives_a_mangonel_a_dead_zone_and_an_archer_none() {
    let Some(fx) = fixture() else { return };
    let mangonel = fx.units.unit(MANGONEL).expect("the mangonel");
    let archer = fx.units.unit(ARCHER).expect("the archer");
    // `MaybeFixed` keeps "unresolved" separate from zero, so the test resolves at a scale of its own.
    let tiles = |value: ruage_data::MaybeFixed| {
        value
            .resolve(1_000_000, "tiles")
            .expect("a resolved value")
            .raw() as f64
            / 1_000_000.0
    };
    assert_eq!(
        tiles(mangonel.min_range) as i64,
        3,
        "a mangonel cannot shoot closer than three tiles"
    );
    assert_eq!(
        tiles(archer.min_range) as i64,
        0,
        "an archer has no dead zone"
    );
    assert!(
        tiles(mangonel.range) > tiles(mangonel.min_range),
        "and its reach is wider than its dead zone, or it could never shoot at all"
    );
}

/// A mangonel told to attack something standing next to it must back off until it has room, then fire.
#[test]
fn a_mangonel_backs_away_from_what_it_cannot_shoot() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx);
    let mangonel = place(&mut world, 1, 9_990, MANGONEL, (10, 10), 50);
    let victim = place(&mut world, 2, 9_991, VILLAGER, (11, 10), 500);
    let start = tiles_between(&world, mangonel, victim);
    assert!(start < 2.0, "it starts inside its own dead zone: {start}");

    world
        .submit_checked(Command::Attack {
            attacker: mangonel,
            target: victim,
        })
        .expect("an attack order");

    let mut fired_at = None;
    for tick in 0..400 {
        world.step().expect("step");
        if world.entity(victim).expect("victim").hp < 500 {
            fired_at = Some(tick);
            break;
        }
    }
    let fired_at = fired_at.expect("the mangonel found room and fired");
    assert!(fired_at > 0, "it did not fire from inside the dead zone");
    let at_shot = tiles_between(&world, mangonel, victim);
    assert!(
        at_shot >= 3.0,
        "it shot from outside its dead zone: {at_shot} tiles at tick {fired_at}"
    );
}

/// The control: an archer standing just as close fires at once, because the data gives it no minimum range.
#[test]
fn an_archer_fires_from_where_a_mangonel_cannot() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx);
    let archer = place(&mut world, 1, 9_992, ARCHER, (10, 10), 30);
    let victim = place(&mut world, 2, 9_993, VILLAGER, (11, 10), 500);
    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("an attack order");
    for _ in 0..30 {
        world.step().expect("step");
    }
    assert!(
        world.entity(victim).expect("victim").hp < 500,
        "the archer has no dead zone, so it hits from a tile away"
    );
    assert!(
        tiles_between(&world, archer, victim) < 2.0,
        "and it did not have to back off to do it"
    );
}
