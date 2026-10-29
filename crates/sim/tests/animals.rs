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

/// The tile a fixed position sits on, at the world's scale.
fn tile_of(position: (Fixed, Fixed), scale: i64) -> (i64, i64) {
    (position.0.raw() / scale, position.1.raw() / scale)
}

/// Player 1's town centre and the first villager, the two anchors every test needs.
fn seat(world: &World) -> (EntityId, EntityId, (Fixed, Fixed)) {
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre");
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
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

/// What this entity is doing, if it is still on the map.
fn task_of(world: &World, id: EntityId) -> Option<Task> {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| entity.task)
}

fn distance(world: &World, a: EntityId, b: (Fixed, Fixed)) -> i64 {
    let Some(one) = world.position(a) else {
        return i64::MAX;
    };
    let (dx, dy) = (one.0.raw() - b.0.raw(), one.1.raw() - b.1.raw());
    dx * dx + dy * dy
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

#[test]
fn a_gathered_sheep_walks_toward_the_drop_site() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let (_, villager, centre) = seat(&world);
    let sheep = a_sheep(&mut world, sheep_unit, centre, 6);
    let before = distance(&world, sheep, centre);

    world
        .submit_checked(Command::Gather {
            villager,
            node: sheep,
        })
        .expect("a villager can be sent to a sheep");

    for _ in 0..400 {
        world.step().expect("stepping");
    }

    let after = distance(&world, sheep, centre);
    assert!(
        after < before,
        "the sheep must walk toward the town centre, not away from it: {before} -> {after}"
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

    // Walk a while, then check the pair travelled together: the villager has left where it stood, and it is
    // near the sheep wherever the sheep has got to. The gap is what "follows" means - the sheep's own
    // distance to the drop site is the other test's business.
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
        gap_tiles_squared <= 36,
        "the villager stays with the sheep - it is following, not trailing by half the map"
    );
}

#[test]
fn a_sheep_stops_at_the_drop_site() {
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

    // Long enough to arrive with a slow villager; the sheep is the quick one, so this is generous.
    for _ in 0..1_500 {
        world.step().expect("stepping");
    }

    let scale = world.scale();
    let here = tile_of(world.position(sheep).expect("the sheep"), scale);
    let there = tile_of(centre, scale);
    assert!(
        here.0.abs_diff(there.0) <= 2 && here.1.abs_diff(there.1) <= 2,
        "the sheep should end beside the town centre, not wander past it: {here:?} vs {there:?}"
    );

    // And it is not still walking: whatever it is doing, it is not on its way somewhere else.
    let task = task_of(&world, sheep);
    assert!(
        !matches!(task, Some(Task::Move { .. })) || here == there,
        "a sheep that has arrived should not still have a route to run"
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
    let sheep = a_sheep(&mut world, sheep_unit, centre, 1);

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
    let boar_unit = world.role_unit("boar").expect("a boar role");
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

    let mut carried_later = 0;
    for _ in 0..400 {
        world.step().expect("stepping");
        carried_later = world
            .entity(villager)
            .map(|entity| entity.carry.raw())
            .unwrap_or(0);
        if carried_later > 0 {
            break;
        }
    }
    assert_eq!(
        carried_later, 0,
        "a live animal must not yield meat: the villager is herding it, not eating it"
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

    // The villager has to take a swing at it - and while it swings, the animal is still a sheep with health.
    let mut struck_while_alive = false;
    let mut hurt = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        let Some(animal) = world.entity(sheep) else {
            break;
        };
        // The two facts have to hold *on the same tick*: a swing, and the animal still a sheep. (The attack
        // task lingers for one tick past the killing blow, so an assertion inside this loop would fire on the
        // tick the body appears - which is the tick after the one that matters.)
        if animal.unit == Some(sheep_unit)
            && matches!(
                world.entity(villager).map(|entity| entity.task),
                Some(Task::Attack { target, .. }) if target == sheep
            )
        {
            struck_while_alive = true;
        }
        if animal.unit == Some(sheep_unit) && animal.hp < 7 {
            hurt = true;
        }
        if animal.unit != Some(sheep_unit) {
            break;
        }
    }
    assert!(
        struck_while_alive,
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
    assert!(hurt, "and the animal has to die of that damage");

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
