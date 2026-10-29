//! What death leaves behind: the last blow decides the meat (D70, D71).
//!
//! The owner's rule, in his words:
//!
//! - *"If soldiers hurt the boar, but villager makes the last hit, then carcass does give the meat."*
//! - *"If Town Centre kills the boar, it doesn't leave the carcass with food. Villager has to be the one
//!   delivering last blow."*
//!
//! So nothing about the *history* of the wound matters: only the killer of the final blow, which the engine could
//! not name until the damage path was fixed (D71 - a melee blow used to fly as an arrow, and an arrow carried no
//! shooter). These tests are the ones that rule was holding up.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
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

/// Everything a test needs to place a body outside every building's reach: fourteen tiles was measured as
/// outside a town centre's range, and eight was not.
fn place(
    world: &mut World,
    player: u8,
    unit: u32,
    tiles: i64,
    dy: i64,
    hp: i32,
    meat: u32,
) -> EntityId {
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(centre.0.raw() + tiles * scale),
                Fixed::from_raw(centre.1.raw() + dy * scale),
            ),
            // A unit of a player's walks (a zero-speed attacker can never close to melee range - that cost a
            // debug cycle); a Gaia animal stands where it is put. `velocity` starts at rest either way (D64).
            speed: if player == 0 {
                Fixed::ZERO
            } else {
                Fixed::from_raw(scale)
            },
            hp,
            // Whole units: `Command::Spawn` scales `amount` itself, so multiplying here as well would make a
            // 340-food boar worth 340 x scale x scale (the same trap the retaliation fixture had).
            amount: meat,
        })
        .expect("the unit can be placed");
    world.step().expect("one tick");
    id
}

fn hp_of(world: &World, id: EntityId) -> i32 {
    world.entity(id).map_or(0, |entity| entity.hp)
}

/// A villager's last blow leaves the meat: the body is a named unit, it holds the pack's 340, and it is still
/// on the map (the sweep must keep a carcass and clear away everything else that dies).
#[test]
fn the_last_blow_of_a_villager_leaves_the_meat() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let carcass = world
        .role_unit("boar_carcass")
        .expect("a boar carcass role");
    let villager = place(&mut world, 1, 83, 14, 2, 25, 0);
    let boar = place(&mut world, 0, boar_unit, 14, 0, 10, 340);
    let scale = world.scale();

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");

    let mut died = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            died = true;
            break;
        }
    }
    assert!(
        died,
        "three damage a blow against ten health has to kill it"
    );

    let body = world
        .entity(boar)
        .expect("death leaves the body where the animal stood");
    assert_eq!(body.unit, Some(carcass), "a boar's body is BOARX_D");
    assert_eq!(body.hp, 0, "a body has no health");
    assert_eq!(
        body.amount.raw(),
        340 * scale,
        "a villager's last blow leaves the whole carcass"
    );
}

/// A soldier's last blow leaves a body with nothing in it - and the body still exists, which is the half that
/// was missing before: an animal killed by anything but the slaughter used to be swept away.
#[test]
fn a_soldiers_last_blow_leaves_a_body_with_nothing() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let carcass = world
        .role_unit("boar_carcass")
        .expect("a boar carcass role");
    // Dat unit 74 is the militia (40 hp, 4 melee damage): the pack's cheapest soldier.
    let soldier = place(&mut world, 1, 74, 14, 2, 40, 0);
    let boar = place(&mut world, 0, boar_unit, 14, 0, 10, 340);

    world
        .submit_checked(Command::Attack {
            attacker: soldier,
            target: boar,
        })
        .expect("the soldier can be sent at the boar");

    let mut died = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            died = true;
            break;
        }
    }
    assert!(died, "four damage a blow against ten health has to kill it");

    let body = world
        .entity(boar)
        .expect("a soldier's kill still leaves a body");
    assert_eq!(body.unit, Some(carcass));
    assert_eq!(
        body.amount.raw(),
        0,
        "only a villager's last blow leaves meat (the owner's rule)"
    );
}

/// The history of the wound does not matter: a soldier may take the boar down to its last sliver of health and a
/// villager's final blow still leaves the full carcass.
#[test]
fn a_soldier_who_softens_the_boar_does_not_take_the_meat() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let carcass = world
        .role_unit("boar_carcass")
        .expect("a boar carcass role");
    let soldier = place(&mut world, 1, 74, 14, 3, 40, 0);
    let villager = place(&mut world, 1, 83, 14, -2, 25, 0);
    // Fourteen health: three soldier blows take it to two, and a villager's three damage finishes it.
    let boar = place(&mut world, 0, boar_unit, 14, 0, 14, 340);
    let scale = world.scale();
    let home = world.position(soldier).expect("a position");

    world
        .submit_checked(Command::Attack {
            attacker: soldier,
            target: boar,
        })
        .expect("the soldier can be sent at the boar");
    let mut softened = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 4 && hp_of(&world, boar) > 0 {
            softened = true;
            break;
        }
    }
    assert!(softened, "the soldier has to take it down to a sliver");

    // The soldier steps away, so the blow that kills can only be the villager's.
    let scale_for_tile = scale;
    let away = (
        u16::try_from(home.0.raw() / scale_for_tile).expect("a tile"),
        u16::try_from(home.1.raw() / scale_for_tile).expect("a tile"),
    );
    world
        .submit_checked(Command::Move {
            unit: soldier,
            to: away,
        })
        .expect("the soldier can step away");
    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent in");

    let mut died = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            died = true;
            break;
        }
    }
    assert!(died, "the boar has to die");

    let body = world.entity(boar).expect("a body");
    assert_eq!(body.unit, Some(carcass));
    assert_eq!(
        body.amount.raw(),
        340 * scale,
        "the wound history does not decide the meat - the last blow does"
    );
}

/// A body is not a fight. Before the body survived the sweep, an attacker's target simply vanished and the attack
/// task stood down; now the body stays, so "the target is gone" is no longer how a fight with an animal ends. An
/// army that keeps swinging at a carcass for the rest of the match is the failure this catches.
#[test]
fn an_attacker_stops_swinging_once_its_quarry_is_a_body() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let villager = place(&mut world, 1, 83, 14, 2, 25, 0);
    let boar = place(&mut world, 0, boar_unit, 14, 0, 10, 340);

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");
    let mut died = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            died = true;
            break;
        }
    }
    assert!(died, "the boar has to die first");
    assert!(
        world.entity(boar).is_some(),
        "and the body has to be there, or this test proves nothing"
    );

    let mut stopped = false;
    for _ in 0..10 {
        world.step().expect("stepping");
        if !matches!(
            world.entity(villager).map(|entity| entity.task),
            Some(Task::Attack { target, .. }) if target == boar
        ) {
            stopped = true;
            break;
        }
    }
    assert!(
        stopped,
        "a villager whose quarry is now a carcass must stand down, not swing at a body for ever"
    );
}

/// A body rots, slowly, at the rate the pack gives its species: `decay_rate` is **food per second** (boar 0.4, sheep
/// 0.25), which our pack and two published accounts agree on. Ten seconds of a boar's carcass is four food.
#[test]
fn a_carcass_rots_at_the_rate_the_pack_gives() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let villager = place(&mut world, 1, 83, 14, 2, 25, 0);
    let boar = place(&mut world, 0, boar_unit, 14, 0, 10, 340);
    let scale = world.scale();

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            break;
        }
    }
    assert!(hp_of(&world, boar) <= 0, "the boar has to die first");

    // Nobody works the body: the villager walks off, so what is measured is rot and nothing else.
    let away = {
        let scale = world.scale();
        let start = world.position(villager).expect("a position");
        (
            u16::try_from(start.0.raw() / scale).expect("a tile"),
            u16::try_from(start.1.raw() / scale).expect("a tile"),
        )
    };
    world
        .submit_checked(Command::Move {
            unit: villager,
            to: away,
        })
        .expect("the villager can walk away");
    world.step().expect("one tick");
    let before = world.entity(boar).expect("a body").amount.raw();

    // Ten seconds at 20 Hz.
    for _ in 0..200 {
        world.step().expect("stepping");
    }
    let after = world.entity(boar).expect("a body").amount.raw();
    assert_eq!(
        before - after,
        4 * scale,
        "a boar's body rots 0.4 food a second: ten seconds is four food"
    );
}

/// Rot is not decoration: a body that has rotted away is gone from the map, rather than standing there for ever at
/// zero meat - which is what "an emptied body stays for ever" meant before this increment.
#[test]
fn a_carcass_that_has_rotted_away_leaves_the_map() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let villager = place(&mut world, 1, 83, 14, 2, 25, 0);
    let sheep = place(&mut world, 0, sheep_unit, 14, 0, 7, 100);

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: sheep,
        })
        .expect("the villager can be sent at the sheep");
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, sheep) <= 0 {
            break;
        }
    }
    assert!(hp_of(&world, sheep) <= 0, "the sheep has to die first");

    let away = {
        let scale = world.scale();
        let start = world.position(villager).expect("a position");
        (
            u16::try_from(start.0.raw() / scale).expect("a tile"),
            u16::try_from(start.1.raw() / scale).expect("a tile"),
        )
    };
    world
        .submit_checked(Command::Move {
            unit: villager,
            to: away,
        })
        .expect("the villager can walk away");

    // 100 food at a quarter of a food a second is 400 s, so 8,000 ticks - with a margin.
    let mut gone = false;
    for _ in 0..9_000 {
        world.step().expect("stepping");
        if world.entity(sheep).is_none() {
            gone = true;
            break;
        }
    }
    assert!(
        gone,
        "a body with nothing left in it must leave the map: {:?}",
        world.entity(sheep).map(|body| body.amount.raw())
    );
    assert!(
        !world.bookkeeping_touches(sheep),
        "and it must leave nothing behind in the side tables"
    );
}

/// A body is not immortal. A soldier's kill leaves a carcass with **no meat** and therefore no rot - and before this
/// increment that body stood on the map for the rest of the match. Its lifetime is derived from the pack rather than
/// invented: a body lies as long as its species' *full* carcass would take to rot (340 food at 0.4 a second for a
/// boar is 850 s), whether or not it holds anything.
#[test]
fn a_soldiers_body_lies_for_a_carcass_span_and_then_goes() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let soldier = place(&mut world, 1, 74, 14, 2, 40, 0);
    let boar = place(&mut world, 0, boar_unit, 14, 0, 10, 340);

    world
        .submit_checked(Command::Attack {
            attacker: soldier,
            target: boar,
        })
        .expect("the soldier can be sent at the boar");
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) <= 0 {
            break;
        }
    }
    assert!(
        world.entity(boar).is_some(),
        "a soldier's kill still leaves a body"
    );

    // A minute later it is still lying there ...
    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    assert!(
        world.entity(boar).is_some(),
        "an empty body does not vanish the moment it is made"
    );

    // ... and after the span a full carcass of its species would take to rot, it is gone.
    let mut gone = false;
    for _ in 0..17_000 {
        world.step().expect("stepping");
        if world.entity(boar).is_none() {
            gone = true;
            break;
        }
    }
    assert!(gone, "an empty body must not stay on the map for ever");
    assert!(!world.bookkeeping_touches(boar));
}
