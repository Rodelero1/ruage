//! Garrison: units inside a building, and buildings that shoot.
//!
//! Written before the implementation (D7). The dat gives every building a `garrison` capacity and 55 of
//! them a real one (a town centre 15, a castle 20), and gives town centres, towers and castles an `attack`
//! array and a `range` - so a building is an armed entity that happens not to move.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, Task, World};

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

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id
}

fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64), hp: i32) -> EntityId {
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

/// The same, but able to walk: an attacker that cannot move never closes to melee.
fn place_moving(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
    speed_tiles: f64,
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
            speed: Fixed::from_raw((speed_tiles * scale as f64) as i64),
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

const VILLAGER: u32 = 83;
const SPEARMAN: u32 = 93;

#[test]
fn a_unit_goes_inside_a_building_and_stops_existing_on_the_map() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    let villager = place(&mut world, 1, 9_900, VILLAGER, tile, 25);

    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: centre,
        })
        .expect("a town centre holds fifteen");
    world.step().expect("step");

    assert_eq!(
        world.entity(villager).expect("villager").task,
        Task::Garrisoned { building: centre },
        "the unit is inside"
    );
    // Inside, it is not on the map: it cannot be attacked, and it does not gather or walk.
    match world.submit_checked(Command::Attack {
        attacker: villager,
        target: centre,
    }) {
        Err(SimError::Garrisoned { id }) => assert_eq!(id, villager),
        other => panic!("expected Garrisoned, got {other:?}"),
    }
}

#[test]
fn a_full_building_refuses_another_unit() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let capacity = u32::from(fx.units.unit(109).expect("town centre").garrison);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    for i in 0..capacity {
        let unit = place(&mut world, 1, 9_910 + i, VILLAGER, tile, 25);
        world
            .submit_checked(Command::Garrison {
                unit,
                building: centre,
            })
            .unwrap_or_else(|err| panic!("garrison {i} of {capacity} refused: {err:?}"));
        world.step().expect("step");
    }
    let extra = place(&mut world, 1, 9_950, VILLAGER, tile, 25);
    match world.submit_checked(Command::Garrison {
        unit: extra,
        building: centre,
    }) {
        Err(SimError::GarrisonFull { id, capacity: full }) => {
            assert_eq!((id, full), (centre, capacity));
        }
        other => panic!("expected GarrisonFull, got {other:?}"),
    }
}

#[test]
fn ungarrisoning_puts_the_units_back_on_the_map() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    let villager = place(&mut world, 1, 9_960, VILLAGER, tile, 25);
    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: centre,
        })
        .expect("garrison");
    world.step().expect("step");

    world
        .submit_checked(Command::Ungarrison { building: centre })
        .expect("ungarrison");
    world.step().expect("step");
    assert_eq!(
        world.entity(villager).expect("villager").task,
        Task::Idle,
        "the unit is out and idle"
    );
    let now = world.position(villager).expect("position");
    let gap = (now.0.raw() - home.0.raw()).abs() + (now.1.raw() - home.1.raw()).abs();
    assert!(
        gap <= 3 * scale,
        "the unit came out beside the building, not across the map"
    );
}

/// A town centre has an attack array and a range: it is an armed entity that does not move.
#[test]
fn a_town_centre_shoots_a_spearman_five_tiles_away() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale, home.1.raw() / scale + 5);
    let target = place(&mut world, 2, 9_970, SPEARMAN, tile, 500);
    let before = world.entity(target).expect("target").hp;

    world
        .submit_checked(Command::Attack {
            attacker: centre,
            target,
        })
        .expect("a town centre reaches six tiles");
    for _ in 0..400 {
        world.step().expect("step");
    }

    assert!(
        world.entity(target).expect("target").hp < before,
        "the arrows landed"
    );
    assert_eq!(
        world.position(centre).expect("position"),
        home,
        "a building does not walk to its target"
    );
}

/// A barracks has no attack array, so ordering it to fight is refused rather than silently ignored.
#[test]
fn a_building_without_attacks_is_unarmed() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let centre = town_centre(&world);
    let home = world.position(centre).expect("position");
    let builder = place(
        &mut world,
        1,
        9_980,
        VILLAGER,
        (home.0.raw() / scale + 2, home.1.raw() / scale),
        25,
    );
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: BARRACKS,
            at: (u16::try_from(home.0.raw() / scale).unwrap() + 3, 2),
        })
        .expect("a barracks");
    world.step().expect("step");
    let site = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(BARRACKS))
        .map(|e| e.id)
        .next()
        .expect("the site");
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            break;
        }
        world.step().expect("step");
    }
    let target = place(
        &mut world,
        2,
        9_990,
        SPEARMAN,
        (home.0.raw() / scale + 2, 6),
        100,
    );
    match world.submit_checked(Command::Attack {
        attacker: site,
        target,
    }) {
        Err(SimError::Unarmed { id }) => assert_eq!(id, site),
        other => panic!("expected Unarmed, got {other:?}"),
    }
}

const BARRACKS: u32 = 12;

/// Published rule: a building ejects its garrison once it is down to 20% of its hit points, and refuses a
/// new one until it is repaired above that. Units inside a ram or a siege tower are the exception (only
/// destruction ejects them), and this engine has no garrisonable rams.
#[test]
fn a_damaged_building_ejects_its_garrison() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    let villager = place(&mut world, 1, 9_995, VILLAGER, tile, 25);
    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: centre,
        })
        .expect("garrison");
    world.step().expect("step");

    // A pile of spearmen until the town centre is under a fifth of its health.
    let target = (home.0.raw() / scale, home.1.raw() / scale + 2);
    for i in 0..12i64 {
        let attacker = place_moving(
            &mut world,
            2,
            9_960 + u32::try_from(i).unwrap(),
            SPEARMAN,
            (target.0 + i % 3, target.1 + 1),
            100,
            1.0,
        );
        world
            .submit_checked(Command::Attack {
                attacker,
                target: centre,
            })
            .expect("attack the town centre");
    }
    for _ in 0..40_000 {
        world.step().expect("step");
        let Some(building) = world.entity(centre) else {
            panic!("the town centre fell: it should have been ejected from first");
        };
        let max = fx.units.unit(109).expect("town centre").hp;
        if building.hp * 5 <= max {
            break;
        }
    }

    let building = world.entity(centre).expect("town centre");
    let max = fx.units.unit(109).expect("town centre").hp;
    assert!(
        building.hp * 5 <= max,
        "the town centre is down to {} of {max}, which is not the twentieth percentile yet",
        building.hp
    );
    assert_eq!(
        world.entity(villager).expect("villager").task,
        Task::Idle,
        "the garrison was ejected, not killed"
    );
    match world.submit_checked(Command::Garrison {
        unit: villager,
        building: centre,
    }) {
        Err(SimError::GarrisonTooDamaged { id }) => assert_eq!(id, centre),
        other => panic!("expected GarrisonTooDamaged, got {other:?}"),
    }
}

/// A unit inside a building is not on the map: the building is the target, and the unit goes with it.
#[test]
fn a_garrisoned_unit_cannot_be_attacked() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    let villager = place(&mut world, 1, 9_996, VILLAGER, tile, 25);
    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: centre,
        })
        .expect("garrison");
    world.step().expect("step");

    let attacker = place(&mut world, 2, 9_997, SPEARMAN, (tile.0, tile.1 + 1), 100);
    match world.submit_checked(Command::Attack {
        attacker,
        target: villager,
    }) {
        Err(SimError::NotATarget { id }) => assert_eq!(id, villager),
        other => panic!("expected NotATarget, got {other:?}"),
    }
}

/// Extra arrows: the published rule is `floor(sum of garrisoned units' dps / building dps)`, where dps is
/// the **pierce** attack over the reload - and only units that deal pierce damage from range count, which
/// is why a Throwing Axeman (melee damage at range 3) adds nothing.
#[test]
fn garrisoned_archers_add_arrows_to_a_town_centre() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);

    // The town centre: pierce 5 over a 2 s reload. An archer: pierce 4 over 2 s, so one archer is
    // worth 2/2.5 = 0.8 extra arrows and three are worth 2.4 -> two extra arrows.
    let target = place(
        &mut world,
        2,
        9_970,
        SPEARMAN,
        (home.0.raw() / scale, home.1.raw() / scale + 5),
        5_000,
    );
    for i in 0..3 {
        let archer = place(&mut world, 1, 9_971 + i, ARCHER, tile, 30);
        world
            .submit_checked(Command::Garrison {
                unit: archer,
                building: centre,
            })
            .expect("an archer fits");
        world.step().expect("step");
    }

    world
        .submit_checked(Command::Attack {
            attacker: centre,
            target,
        })
        .expect("a town centre reaches six tiles");
    world.step().expect("step");
    let arrows = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Projectile)
        .count();
    assert_eq!(
        arrows, 3,
        "one arrow of its own plus two from three archers"
    );
}

/// A unit that fights at range with melee damage adds nothing: the formula counts pierce dps.
#[test]
fn a_throwing_axeman_adds_no_arrows() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let scale = world.scale();
    let home = world.position(centre).expect("position");
    let tile = (home.0.raw() / scale + 1, home.1.raw() / scale);
    let target = place(
        &mut world,
        2,
        9_980,
        SPEARMAN,
        (home.0.raw() / scale, home.1.raw() / scale + 5),
        5_000,
    );
    for i in 0..4 {
        let axeman = place(&mut world, 1, 9_981 + i, THROWING_AXEMAN, tile, 30);
        world
            .submit_checked(Command::Garrison {
                unit: axeman,
                building: centre,
            })
            .expect("terrible, but allowed");
        world.step().expect("step");
    }
    world
        .submit_checked(Command::Attack {
            attacker: centre,
            target,
        })
        .expect("an attack order");
    world.step().expect("step");
    let arrows = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Projectile)
        .count();
    assert_eq!(arrows, 1, "the building's own arrow, and no more");
}

const ARCHER: u32 = 4;
const THROWING_AXEMAN: u32 = 281;

/// Only a building that can fight takes units in from outside; a production building holds nothing but
/// what it has just trained (in the game by setting its rally onto itself), and a unit that has left can
/// never come back. The data draws the line by itself: the barracks, archery range, stable, siege workshop
/// and dock each carry a garrison capacity of ten and **no attack array**, while the town centre, tower and
/// castle carry a capacity *and* an attack.
#[test]
fn a_production_building_does_not_take_units_in_from_outside() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let centre = town_centre(&world);
    let home = world.position(centre).expect("position");
    // A builder has to be able to walk to its site, so this one has a speed.
    let builder = place_moving(
        &mut world,
        1,
        9_950,
        VILLAGER,
        (home.0.raw() / scale + 2, home.1.raw() / scale),
        25,
        0.8,
    );
    let at = (u16::try_from(home.0.raw() / scale).unwrap() + 3, 2);
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: BARRACKS,
            at,
        })
        .expect("a barracks");
    world.step().expect("step");
    let barracks = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(BARRACKS))
        .map(|e| e.id)
        .next()
        .expect("the site");
    for _ in 0..12_000 {
        if world.entity(barracks).map(|e| e.construction.is_none()) == Some(true) {
            break;
        }
        world.step().expect("step");
    }
    assert!(
        world
            .entity(barracks)
            .is_some_and(|e| e.construction.is_none()),
        "the barracks finished, so it can be asked to take someone in"
    );
    // Stand the villager beside its own barracks.
    let site = world.position(barracks).expect("position");
    let tile = (site.0.raw() / scale, site.1.raw() / scale);
    let walker = place(&mut world, 1, 9_990, VILLAGER, (tile.0, tile.1 + 1), 25);

    match world.submit_checked(Command::Garrison {
        unit: walker,
        building: barracks,
    }) {
        Err(SimError::CannotGarrisonHere { id }) => assert_eq!(id, barracks),
        other => panic!("expected CannotGarrisonHere, got {other:?}"),
    }
}
