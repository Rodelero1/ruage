//! Herding: a gathered animal walks to the drop site and the villager follows it (D70).
//!
//! Written before the implementation (D7). The owner's rule, in two parts, in his words:
//!
//! - *"villager is herding sheep towards closest mill or town centre. So sheep do not close distance to him
//!   but move in the direction of mill or town centre"*;
//! - *"Also the villager follows the sheep herding them towards the gathering point."*
//!
//! So herding is **not a separate order**: it is what gathering from a live animal *is*. The sheep's
//! destination is the nearest drop site of the villager's player; the villager trails it because gathering
//! needs adjacency. Two things follow, and both are tested here because both are easy to get wrong:
//!
//! - a gather node that **moves** has to re-path the villager, which no static node ever needed;
//! - a sheep **nobody is gathering stands still**, or every sheep on the map would walk to a town centre on
//!   its own.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

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
    /// The slaughter is an ordinary attack, so the fixture needs the damage constants - without them
    /// `check_attack` refuses with `NoTables`, and that refusal is now recorded rather than vanishing (item 1).
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

/// Player 1's town centre and the first villager, the two anchors every test needs.
fn seat(world: &World) -> (EntityId, EntityId, (Fixed, Fixed)) {
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre");
    // **The villager, by role.** Since the owner asked for an opening scout (2026-11-04) the player's first unit is a
    // horseman; a seat found by "first unit" would have sat a sheep down for a scout to gather.
    let villager_unit = world.start_villager_unit().expect("a villager role");
    let villager = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Unit
                && entity.unit == Some(villager_unit)
        })
        .expect("a villager");
    (centre.id, villager.id, centre.position)
}

/// Put a sheep on the map a few tiles from the town centre, and hand back its id.
fn a_sheep(world: &mut World, unit: u32, near: (Fixed, Fixed), offset: i64) -> EntityId {
    let scale = world.scale();
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
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(near.0.raw() + offset * scale),
                Fixed::from_raw(near.1.raw() + offset * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 7,
            amount: 0,
        })
        .expect("a sheep on the map");
    // Orders apply on the next tick, so the sheep exists after one step.
    world.step().expect("one tick");
    id
}

/// Put an animal of any species on the map with the health the pack gives it, and hand back its id.
///
/// A separate helper from `a_sheep` because that one hard-codes 7 health and a sheep's meat: a boar spawned at 7 hp
/// reads as "already hurt", which made a first version of the hunter test pass on a blow nobody had struck.
fn an_animal(world: &mut World, unit: u32, near: (Fixed, Fixed), offset: i64, hp: i32) -> EntityId {
    let scale = world.scale();
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
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(near.0.raw() + offset * scale),
                Fixed::from_raw(near.1.raw() + offset * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            // Zero means "worth what the pack says": 340 for a boar (D70).
            amount: 0,
        })
        .expect("an animal on the map");
    world.step().expect("one tick");
    id
}

// PARKED, not passing: increment two of D70 is written but not built. The content and the data side are in
// place (`[hunt] herdable_roles`, the `sheep` role, `Hunt` in the economy schema); what is missing is the
// engine half - `gather_source` accepting a live animal, a spawned animal carrying its `food` as its amount,
// and the per-tick pass that walks the animal to the drop site and lets the villager follow. Each ignore is
// one of those, kept so the expectations are not lost between sessions.
#[test]
fn a_sheep_nobody_is_gathering_stands_still() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, _, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 6);
    let before = world.position(sheep).expect("the sheep is on the map");

    for _ in 0..600 {
        world.step().expect("stepping");
    }

    let after = world.position(sheep).expect("the sheep is still there");
    assert_eq!(before, after, "nothing gathers it, so nothing moves it");
}

/// **A gathered sheep stands still** (D116, the owner's call). It used to walk itself to the nearest drop site with the
/// villager trailing behind - "herding", D70. The owner dropped it: *"Sheep are more likely to be found by scout and player
/// and AI are more likely to direct sheep to the drop site themselves."* So a gathered animal is worked **where it stands**,
/// and moving it is an ordinary order its owner gives.
#[test]
fn a_gathered_sheep_stands_still() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 6);
    let before = world.position(sheep).expect("the sheep");

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a sheep");

    for _ in 0..600 {
        world.step().expect("stepping");
    }

    let after = world
        .position(sheep)
        .expect("the sheep has not been eaten yet");
    assert_eq!(
        after, before,
        "nothing gathers a sheep *into* walking: it stands where it is until its owner moves it"
    );
}

/// **A sheep walks where its owner points** (D116): moving livestock is an ordinary order, which is what the player and the
/// AI do instead of the animal steering itself.
#[test]
fn a_sheep_walks_where_its_owner_points() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, _, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 6);
    let scale = world.scale();
    let goal = {
        let at = world.position(sheep).expect("the sheep");
        (
            u16::try_from(at.0.raw() / scale - 3).expect("in range"),
            u16::try_from(at.1.raw() / scale).expect("in range"),
        )
    };

    world
        .submit_checked(Command::Move {
            unit: sheep,
            to: goal,
        })
        .expect("a sheep can be ordered like any other body");

    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    let at = world.position(sheep).expect("the sheep");
    let (tx, ty) = (at.0.raw() / scale, at.1.raw() / scale);
    assert!(
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1,
        "the sheep walked to {tx},{ty} instead of the tile it was sent to: {goal:?}"
    );
}

#[test]
fn the_villager_follows_the_sheep() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 6);

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a sheep");

    // Walk a while, then check the villager arrived at the sheep it was sent to. It used to *trail* a sheep that was
    // walking itself home (D70); with herding dropped (D116) the sheep stands and this is simply "the gatherer reaches the
    // node it was ordered to work".
    let started = world.position(villager).expect("the villager");
    for _ in 0..300 {
        world.step().expect("stepping");
    }
    let now = world.position(villager).expect("the villager");
    assert_ne!(now, started, "the villager must move, not stand and watch");

    let sheep_at = world.position(sheep).expect("the sheep");
    let (dx, dy) = (
        now.0.raw() - sheep_at.0.raw(),
        now.1.raw() - sheep_at.1.raw(),
    );
    let scale = world.scale();
    let gap_tiles_squared = (dx * dx + dy * dy) / (scale * scale);
    assert!(
        gap_tiles_squared <= 4,
        "the villager has to reach the sheep it was sent to, not stop half-way"
    );
}

/// Step one of the slice: **a live animal is a gather source**, and it is worth its meat (D70).
///
/// Written before the implementation. `gather_source` knew two kinds of source - a resource node and a farm
/// building - and refused everything else with `NotAResource`, so a villager could not be sent to a sheep at
/// all. The meat is the animal's own `food` from the pack (100 for a sheep, 340 for a boar), which the spawn
/// has to carry: a body placed with no amount would be a source of nothing.
#[test]
fn a_live_animal_is_a_gather_source() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    // **Offset 2, not 1.** Offset 1 is the town centre's own 2x2 *solid core* - a sheep standing there is unreachable by
    // any movement rule, and the test only ever passed because a body's centre could stand on the core's corner (D115).
    let sheep = a_sheep(&mut world, sheep_unit, centre, 2);

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a live animal");

    // Walk in, then work: the villager's load is what says it is taking meat rather than standing there.
    let mut carried = 0;
    for _ in 0..600 {
        world.step().expect("stepping");
        if let Some(entity) = world.entities().iter().find(|entity| entity.id == villager) {
            carried = entity.carry.raw();
        }
        if carried > 0 {
            break;
        }
    }
    assert!(
        carried > 0,
        "a villager working a sheep must end up carrying meat"
    );
}

#[test]
fn a_placed_animal_carries_its_meat() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, _, centre) = seat(&world);

    // Placed with no amount, as a scenario would place it: the pack's 100 food is the amount.
    let sheep = a_sheep(&mut world, sheep_unit, centre, 1);
    let amount = world
        .entities()
        .iter()
        .find(|entity| entity.id == sheep)
        .map(|entity| entity.amount.raw())
        .expect("the sheep exists");

    assert_eq!(
        amount,
        100 * world.scale(),
        "a sheep is worth the 100 food the pack gives it, in engine units"
    );
}

/// *Meat implies death* (the owner's rule): a villager standing beside a **live** animal takes nothing out of it.
/// The work while an animal walks is following, not taking - and before this increment the gather pass read the
/// live animal's `amount` and handed its meat to the villager.
///
/// A boar is the subject because it does not herd (only `herdable_roles` walk), so it stands still and the villager
/// works it in place: nothing about the walk can confuse what is being measured.
#[test]
fn a_live_animal_yields_no_meat() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("wild_boar").expect("a boar role");
    let (_, villager, centre) = seat(&world);
    let boar = a_sheep(&mut world, boar_unit, centre, 5);
    let scale = world.scale();
    let meat = world
        .entity(boar)
        .map(|entity| entity.amount.raw())
        .unwrap_or(0);

    world
        .submit_checked(Command::Gather {
            villager,
            node: boar,
        })
        .expect("a villager can be sent to a boar");

    // **While it lives** nothing comes out of it. The villager does end up carrying meat - it hunts the boar and then
    // works the body, which is the rule working - so the assertion has to be about the window, not the endpoint.
    let mut carried_while_alive = 0;
    let mut ticks = 0;
    while ticks < 400 {
        world.step().expect("stepping");
        ticks += 1;
        let alive = world.entity(boar).is_some_and(|entity| entity.hp > 0);
        if !alive {
            break;
        }
        carried_while_alive = world
            .entity(villager)
            .map(|entity| entity.carry.raw())
            .unwrap_or(0);
        if carried_while_alive > 0 {
            break;
        }
    }
    assert_eq!(
        carried_while_alive, 0,
        "a live animal must not yield meat: the villager is hunting it, not eating it"
    );
    assert_eq!(
        world.entity(boar).map(|entity| entity.amount.raw()),
        Some(meat),
        "and the animal is untouched while it lives"
    );
    assert!(
        meat / scale >= 300,
        "the boar carries its 340: {meat} at scale {scale}"
    );
}

/// The shepherd **strikes** the animal at the drop site, the animal dies of that damage, and only then is there a
/// body - the owner's correction, which turned the transform-on-arrival into a real kill (D70).
#[test]
fn the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 4);
    let scale = world.scale();

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a sheep");

    // The villager has to be **ordered to strike** it, and the death has to follow that order. A shepherd kills a sheep
    // in one blow, so "the animal is still alive while the villager swings" is no longer observable: the swing and the
    // death are the same tick. What is observable is the order, and the body it leaves.
    let mut ordered_to_strike = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if matches!(
            world.entity(villager).map(|entity| entity.task),
            Some(Task::Attack { target, .. }) if target == sheep
        ) {
            ordered_to_strike = true;
        }
        let Some(animal) = world.entity(sheep) else {
            break;
        };
        if animal.unit != Some(sheep_unit) {
            break;
        }
    }
    assert!(
        ordered_to_strike,
        "the shepherd has to strike the animal, not have it turn into a corpse on arrival"
    );
    // ... and being struck is not a refusal for a sheep: it has no attack to answer with, so no reaction is
    // attempted and nothing lands in the refusal record (which is how a real refused order becomes visible).
    assert!(
        world.last_refusal().is_none(),
        "a sheep that cannot fight back must not record a refused reaction: {:?}",
        world
            .last_refusal()
            .map(|(id, why, _)| (id, why.to_string()))
    );

    let body = world
        .entity(sheep)
        .expect("the sheep's body stands where it was killed");
    assert_eq!(body.unit, Some(595), "a sheep's body is SHEEP_D");
    assert_eq!(body.hp, 0);
    assert!(
        body.amount.raw() >= 99 * scale,
        "a villager's killing blow leaves the carcass: {}",
        body.amount.raw()
    );

    // The gather order survives the fight: the villager goes back to work on the body it just made.
    let mut carried = 0;
    for _ in 0..1_500 {
        world.step().expect("stepping");
        carried = world
            .entity(villager)
            .map(|entity| entity.carry.raw())
            .unwrap_or(0);
        if carried > 0 {
            break;
        }
    }
    assert!(
        carried > 0,
        "the villager has to carry on gathering from the body it just made, not stand beside it"
    );
}

// ---------------------------------------------------------------------------------------------------
// The owner's three corrections (2026-10-29): hunters shoot, shepherds cut once, and the last arrow counts.
// ---------------------------------------------------------------------------------------------------

/// *"Hunter villager also use the bow while shepherd villager uses the knife."* The pack already says so: **VMHUN
/// 122 has `range = 3.0`** and VMSHE 592 has none, and content maps the `hunt` task to 122 and `herd` to 592. So the
/// engine only has to **choose the right task for the node**: an animal is hunted, a herdable is herded.
///
/// Behaviourally, that means a villager sent at a boar stops three tiles away and shoots it, instead of walking into
/// tusks.
#[test]
fn a_villager_hunting_a_boar_uses_the_bow() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("wild_boar").expect("a boar role");
    let (_, villager, centre) = seat(&world);
    // Fourteen tiles out: at six the town centre still shoots, and the blow being measured would be the tower's.
    let boar = an_animal(&mut world, boar_unit, centre, 14, 75);
    let scale = world.scale();

    world
        .submit_checked(Command::Gather {
            villager,
            node: boar,
        })
        .expect("a villager can be sent at a boar");

    let mut struck_from = None;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if world.entity(boar).is_some_and(|entity| entity.hp < 75) {
            let me = world.position(villager).expect("the villager");
            let it = world.position(boar).expect("the boar");
            let dx = (me.0.raw() - it.0.raw()).abs() / scale;
            let dy = (me.1.raw() - it.1.raw()).abs() / scale;
            struck_from = Some(dx.max(dy));
            break;
        }
    }
    let tiles = struck_from.unwrap_or_else(|| {
        panic!(
            "the hunter has to land a blow; task {:?}, refusal {:?}",
            world.entity(villager).map(|entity| entity.task),
            world
                .last_refusal()
                .map(|(id, why, _)| (id, why.to_string()))
        )
    });
    assert!(
        tiles >= 2,
        "a hunter shoots from the bow's reach, not from {tiles} tile(s) away: the hunt task has to pick VMHUN"
    );
}

/// A **hunter's arrow** is a villager's last hit, so the carcass keeps its meat - the owner's second correction. This
/// only works because the arrow carries its shooter (D71), which is why it belongs beside the melee rule.
#[test]
fn a_hunters_arrow_that_kills_leaves_the_meat() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("wild_boar").expect("a boar role");
    let (_, villager, centre) = seat(&world);
    // Fourteen tiles out, so the only blow that lands is the hunter's (a town centre shoots at six).
    let boar = an_animal(&mut world, boar_unit, centre, 14, 75);
    let scale = world.scale();
    world
        .submit_checked(Command::Gather {
            villager,
            node: boar,
        })
        .expect("a villager can be sent at a boar");

    for _ in 0..3_000 {
        world.step().expect("stepping");
        if world.entity(boar).is_some_and(|entity| entity.hp <= 0) {
            break;
        }
    }
    let body = world.entity(boar).expect("the boar's body has to be there");
    assert_eq!(
        body.unit,
        Some(
            world
                .role_unit("wild_boar_corpse")
                .expect("a boar carcass role")
        ),
        "a kill leaves the body"
    );
    assert!(
        body.amount.raw() > 0,
        "the last blow was a hunter's arrow, which is a villager's: the meat stays (owner's rule)"
    );
    assert!(body.amount.raw() <= 340 * scale);
}

/// *"Shepherd kills the sheep by one blow."* A herdable is livestock, not an opponent: the shepherd's strike takes
/// it outright, so the animal is never seen wounded.
#[test]
fn a_shepherd_kills_a_sheep_with_one_blow() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 4);

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a sheep");

    let mut wounded = 0;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        let Some(entity) = world.entity(sheep) else {
            break;
        };
        if entity.unit == Some(sheep_unit) && entity.hp < 7 {
            wounded += 1;
        }
        if entity.unit != Some(sheep_unit) {
            break;
        }
    }
    assert_eq!(
        wounded, 0,
        "a sheep is killed by one blow, so it is never seen wounded at 1..6 health"
    );
}
