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

#[ignore = "herding: the engine half is not built yet (D70)"]
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

#[ignore = "herding: the engine half is not built yet (D70)"]
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

    // Walk a while, then check the pair moved together: the villager is near the sheep and both have
    // come closer to the town centre than the villager started.
    let villager_started = distance(&world, villager, centre);
    for _ in 0..300 {
        world.step().expect("stepping");
    }

    let gap = distance(&world, villager, centre).min(distance(&world, sheep, centre));
    assert!(
        gap < villager_started / 4,
        "the villager trails the sheep toward the drop site rather than standing still"
    );
    assert!(
        distance(&world, villager, centre) < villager_started,
        "and the villager itself has moved toward the town centre"
    );
}

#[ignore = "herding: the engine half is not built yet (D70)"]
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
