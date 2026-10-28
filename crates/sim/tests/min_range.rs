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
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

fn world_with(fx: &Fixture) -> World {
    world_with_units(fx, &fx.units)
}

fn world_with_units(fx: &Fixture, units: &Units) -> World {
    let mut economy = fx.economy.clone();
    economy.start.stock = [("food".to_owned(), 4_000), ("wood".to_owned(), 4_000)]
        .into_iter()
        .collect();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world.seed_from(&fx.map, &economy, units).expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // No scatter: this is about where a shot may be fired from, not about aiming.
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
}

/// A target that stands still. `Command::Spawn` is the movement hook - a spawned unit's speed becomes its
/// velocity and it keeps drifting (`docs/mechanics/combat.md`) - so a target that must not move is spawned
/// with no speed at all.
fn place_still(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (u16, u16),
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
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
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

const SKIRMISHER: u32 = 7;
/// Andean Sling: the Inca unique tech. Its effect (572) sets minimum range to zero for the skirmisher line,
/// the slinger and the genitour - "Skirmishers and Slingers have no minimum range".
const ANDEAN_SLING: u32 = 516;

/// A world with the real research tables, so a tech can be granted the way content names it.
fn world_with_research(fx: &Fixture) -> World {
    let mut world = world_with(fx);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world
}

/// What happens when a skirmisher is ordered onto a target standing next to it, and where it was standing
/// when the first blow landed. A skirmisher *can* shoot such a target - it has to step back first - so the
/// thing that changes with Andean Sling is **where the first shot comes from**.
fn skirmisher_against_a_neighbour(fx: &Fixture, andean_sling: bool) -> (Option<usize>, f64) {
    let mut world = world_with_research(fx);
    if andean_sling {
        world
            .grant_tech(1, ANDEAN_SLING)
            .expect("Andean Sling is a researchable tech");
    }
    let skirmisher = place(&mut world, 1, 9_994, SKIRMISHER, (10, 10), 30);
    let victim = place_still(&mut world, 2, 9_995, VILLAGER, (11, 10), 500);
    world
        .submit_checked(Command::Attack {
            attacker: skirmisher,
            target: victim,
        })
        .expect("an attack order");
    for tick in 0..80 {
        world.step().expect("step");
        if world.entity(victim).expect("victim").hp < 500 {
            return (Some(tick), tiles_between(&world, skirmisher, victim));
        }
    }
    (None, tiles_between(&world, skirmisher, victim))
}

/// A skirmisher has a minimum range as well - the dat gives unit 7 one tile - and Andean Sling takes it away.
///
/// On a tile grid a one-tile dead zone only bites if it is measured to the target's **edge** rather than its
/// centre (a neighbour is already a whole tile away centre to centre), which is the model the engine uses:
/// unit radii are data, and two bodies cannot share a tile (D44).
#[test]
fn a_skirmisher_backs_off_before_shooting_and_andean_sling_lets_it_shoot_at_once() {
    let Some(fx) = fixture() else { return };
    let skirmisher = fx.units.unit(SKIRMISHER).expect("the skirmisher");
    let tiles = |value: ruage_data::MaybeFixed| {
        value
            .resolve(1_000_000, "tiles")
            .expect("a resolved value")
            .raw() as f64
            / 1_000_000.0
    };
    assert_eq!(
        tiles(skirmisher.min_range) as i64,
        1,
        "the data gives a skirmisher a one-tile dead zone"
    );

    let (blind_tick, blind_from) = skirmisher_against_a_neighbour(&fx, false);
    let blind_tick = blind_tick.expect("a skirmisher can still shoot, once it has room");
    assert!(
        blind_from > 1.0,
        "without Andean Sling it steps back before it shoots: {blind_from} tiles at tick {blind_tick}"
    );

    let (freed_tick, freed_from) = skirmisher_against_a_neighbour(&fx, true);
    let freed_tick = freed_tick.expect("Andean Sling removes the dead zone, so it shoots");
    assert!(
        freed_from <= 1.0 + 0.01,
        "with Andean Sling it shoots from where it stands: {freed_from} tiles"
    );
    assert!(
        freed_tick <= blind_tick,
        "and so it lands its first blow no later: {freed_tick} against {blind_tick}"
    );
}

const WATCH_TOWER: u32 = 79;
const CASTLE: u32 = 82;
/// Murder Holes: the tech that lets towers and castles shoot at what is standing against them.
const MURDER_HOLES: u32 = 322;

fn place_building(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (u16, u16),
    hp: i32,
) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

/// A tower cannot shoot the unit standing against it - the data gives it a one-tile dead zone - and **Murder
/// Holes** takes that away. The technology is a kind-4 `add -1` on attribute 20, not a `set 0`, which is the
/// other half of the same rule: removals by technology arrive in both shapes.
#[test]
fn a_tower_cannot_shoot_what_stands_against_it_until_murder_holes() {
    let Some(fx) = fixture() else { return };
    let shot = |murder_holes: bool| -> i32 {
        let mut world = world_with_research(&fx);
        if murder_holes {
            world
                .grant_tech(1, MURDER_HOLES)
                .expect("Murder Holes is researchable");
        }
        let tower = place_building(&mut world, 1, 9_996, WATCH_TOWER, (10, 10), 1_020);
        let victim = place_still(&mut world, 2, 9_997, VILLAGER, (11, 10), 500);
        world
            .submit_checked(Command::Attack {
                attacker: tower,
                target: victim,
            })
            .expect("a tower may be told to shoot");
        for _ in 0..80 {
            world.step().expect("step");
        }
        let _ = tower;
        world.entity(victim).expect("victim").hp
    };

    assert_eq!(
        shot(false),
        500,
        "a tower cannot shoot the villager standing against it"
    );
    assert!(
        shot(true) < 500,
        "with Murder Holes it can: the technology removes the dead zone"
    );
}

/// The castle carries the same one-tile dead zone, and Murder Holes lists it too - so the technology is a rule
/// about buildings that fight, not about one building.
#[test]
fn a_castle_has_the_same_dead_zone_and_the_same_technology_removes_it() {
    let Some(fx) = fixture() else { return };
    let castle = fx.units.unit(CASTLE).expect("the castle");
    let tiles = |value: ruage_data::MaybeFixed| {
        value
            .resolve(1_000_000, "tiles")
            .expect("a resolved value")
            .raw() as f64
            / 1_000_000.0
    };
    assert_eq!(tiles(castle.min_range) as i64, 1);
    assert_eq!(
        tiles(fx.units.unit(WATCH_TOWER).expect("tower").min_range) as i64,
        1
    );
    // And the town centre does not: Murder Holes does not list it, and the data agrees.
    assert_eq!(
        tiles(fx.units.unit(109).expect("town centre").min_range) as i64,
        0,
        "a town centre has no dead zone, which is why Murder Holes does not mention it"
    );
}

/// The configuration surface the user asked for, proven end to end: an override in the pack can take a
/// building's dead zone away with no technology involved. This is `overrides.toml` reaching the simulation,
/// not just a parser round-trip in `crates/data`.
#[test]
fn content_can_take_a_towers_dead_zone_away_without_a_technology() {
    let Some(fx) = fixture() else { return };
    let mut units = fx.units.clone();
    let overrides = ruage_data::Overrides::parse(
        "schema = 1\n[overrides]\n\"units.79.min_range\" = 0.0\n",
        "overrides.toml",
    )
    .expect("the override file parses");
    let log = overrides.apply_units(&mut units).expect("and applies");
    assert_eq!(log.len(), 1, "one value changed: {log:?}");

    let mut world = world_with_units(&fx, &units);
    let tower = place_building(&mut world, 1, 9_998, WATCH_TOWER, (10, 10), 1_020);
    let victim = place_still(&mut world, 2, 9_999, VILLAGER, (11, 10), 500);
    world
        .submit_checked(Command::Attack {
            attacker: tower,
            target: victim,
        })
        .expect("a tower may be told to shoot");
    for _ in 0..80 {
        world.step().expect("step");
    }
    assert!(
        world.entity(victim).expect("victim").hp < 500,
        "a tower whose dead zone content removed shoots what stands against it, with no Murder Holes"
    );
}
