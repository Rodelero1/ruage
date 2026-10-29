//! Retaliation: a struck animal turns on whoever struck it (D70).
//!
//! Written before the implementation (D7). The owner's rules for the animal slice, in order:
//!
//! - meat implies death, and the villager's killing blow lands **on arrival at the drop site**;
//! - only a villager's kill leaves food in the body;
//! - a herdable changes hands by line of sight.
//!
//! None of those matter while a boar is scenery. A boar carries 75 health and a 7-damage attack in the pack and
//! uses neither: being hit is not an order, and nothing gives an animal a task of its own. This test says what
//! "retaliation" means - the animal it was struck by becomes its target - and it is the first time an animal
//! issues itself an order.

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

fn task_of(world: &World, id: EntityId) -> Option<Task> {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| entity.task)
}

fn hp_of(world: &World, id: EntityId) -> i32 {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map_or(0, |entity| entity.hp)
}

// The premise these tests rest on is now established from the code and the data, and it corrected the
// previous session's diagnosis: **no blow has ever taken the melee path**, so `retaliate` was never
// called at all - `start_attack`'s "silent refusal" was a misreading of a function that never ran.
// `combat.toml` sets `projectile_speed = 10.0`, and the strike loop turned *every* blow into a
// projectile whenever `flight > 0 && source.0 != aim`; for a melee attacker `aim` is the victim's own
// position, so the two never match and a villager's knife flew like an arrow. Melee damage therefore
// landed in the `landings` loop, which carries no attacker: retaliation never fired, and the last blow
// could not be named.
//
// So this increment is the damage path first: a blow is a projectile only when the attacker's resolved
// range is above zero, and an arrow records its archer so ranged damage is attributable too.
#[test]
fn a_struck_boar_turns_on_whoever_struck_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;

    // A boar two tiles from the town centre: close enough that the villager can strike it without walking far.
    let boar = EntityId(
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
            id: boar,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(boar_unit),
            position: (
                // Fourteen tiles out, deliberately: at eight the town centre still shot it, so the blow
                // being waited for was an arrow - and an arrow lands with its damage, not its archer.
                Fixed::from_raw(centre.0.raw() + 14 * scale),
                Fixed::from_raw(centre.1.raw() + 14 * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 75,
            amount: 0,
        })
        .expect("a boar");
    // Orders apply on the next tick, so the boar exists only after one step.
    world.step().expect("one tick");

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");

    // The villager has to walk in and land a blow first: retaliation is a reaction, not a stance.
    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            struck = true;
            break;
        }
    }
    assert!(
        struck,
        "the villager must land a blow before anything reacts"
    );

    // Then it turns round: its task names the villager, and it hits back.
    let mut turned = false;
    for _ in 0..200 {
        world.step().expect("stepping");
        if matches!(task_of(&world, boar), Some(Task::Attack { target, .. }) if target == villager)
        {
            turned = true;
        }
        if world
            .entities()
            .iter()
            .any(|entity| entity.id == villager && entity.hp < 25)
        {
            break;
        }
    }
    assert!(
        turned,
        "a struck boar must take the villager as its target, not stand there being hit"
    );
}

#[test]
fn an_animal_that_was_never_struck_stays_idle() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let boar = EntityId(
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
            id: boar,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(boar_unit),
            position: (
                Fixed::from_raw(centre.0.raw() + 8 * scale),
                Fixed::from_raw(centre.1.raw() + 8 * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 75,
            amount: 0,
        })
        .expect("a boar");

    for _ in 0..400 {
        world.step().expect("stepping");
    }
    assert!(
        !matches!(task_of(&world, boar), Some(Task::Attack { .. })),
        "a boar nobody has touched has no quarrel with anyone"
    );
}

/// The premise everything downstream rests on: **a villager's knife lands**. If this fails, the slaughter, the
/// carcass and the last-blow rule have nothing under them.
///
/// The sheep is placed twelve tiles from the town centre on purpose: closer, and the town centre shoots it, and
/// the blow that lands is an arrow - which is exactly the confusion the instrumented run exposed.
#[test]
fn a_villagers_knife_lands_on_a_sheep() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;

    let sheep = EntityId(
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
            id: sheep,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(sheep_unit),
            position: (
                Fixed::from_raw(centre.0.raw() + 12 * scale),
                Fixed::from_raw(centre.1.raw() + 12 * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 7,
            amount: 100 * u32::try_from(scale).expect("fits"),
        })
        .expect("a sheep");
    world.step().expect("one tick");

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: sheep,
        })
        .expect("the villager can be sent at the sheep");

    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, sheep) < 7 {
            struck = true;
            break;
        }
    }
    assert!(
        struck,
        "the villager walked there and never struck: its blow either never lands or is not melee"
    );
}

// ---------------------------------------------------------------------------------------------------
// The damage path, the arrow's archer, and the leash.
// ---------------------------------------------------------------------------------------------------

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

/// Place a foot unit from the pack at a tile offset from the map's town centre. Deliberately *far* out
/// when a test means "no building can reach this": at eight tiles a town centre still shoots, and that fed
/// six arrows into a melee test the last time round.
fn spawn_unit(world: &mut World, player: u8, unit: u32, tiles: i64, dy: i64, hp: i32) -> EntityId {
    place(world, player, unit, tiles, dy, hp, 0)
}

/// Place a Gaia animal, carrying its meat. It stands still: every test here has the villager walk to it.
fn spawn_animal(world: &mut World, unit: u32, tiles: i64, hp: i32, meat: u32) -> EntityId {
    place(world, 0, unit, tiles, 0, hp, meat)
}

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
    let id = next_id(world);
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
            // At rest: a spawned body carrying a velocity walks east for ever (D64).
            speed: Fixed::ZERO,
            hp,
            amount: meat * u32::try_from(scale).expect("fits"),
        })
        .expect("the unit can be placed");
    world.step().expect("one tick");
    id
}

fn pos_of(world: &World, id: EntityId) -> (Fixed, Fixed) {
    world.position(id).expect("a position")
}

fn tile_of(world: &World, position: (Fixed, Fixed)) -> (u16, u16) {
    let scale = world.scale();
    (
        u16::try_from(position.0.raw() / scale).expect("a tile"),
        u16::try_from(position.1.raw() / scale).expect("a tile"),
    )
}

fn tiles_between(world: &World, a: (Fixed, Fixed), b: (Fixed, Fixed)) -> i64 {
    let dx = (a.0.raw() - b.0.raw()).abs();
    let dy = (a.1.raw() - b.1.raw()).abs();
    // Chebyshev-ish in whole tiles: the leash is a distance, and a diagonal must not read as 1.4x.
    dx.max(dy) / world.scale()
}

fn villager_of(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

/// The melee path's own control: a walk-up-and-hit fight must not put a single arrow in the world.
///
/// Before this increment every blow became a projectile - `combat.toml`'s `projectile_speed = 10.0` applied
/// to a knife just as much as to a bow - so melee damage landed in the `landings` loop, which carries no
/// attacker. That is why retaliation never fired and why the last blow could not be named.
#[test]
fn a_melee_blow_never_becomes_an_arrow() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let sheep = spawn_animal(&mut world, sheep_unit, 14, 7, 100);
    let villager = villager_of(&world);
    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: sheep,
        })
        .expect("the villager can be sent at the sheep");

    let mut arrows = 0;
    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        arrows += world
            .entities()
            .iter()
            .filter(|entity| entity.kind == EntityKind::Projectile)
            .count();
        if hp_of(&world, sheep) < 7 {
            struck = true;
            break;
        }
    }
    assert!(struck, "the villager has to land the blow at all");
    assert_eq!(
        arrows, 0,
        "a melee blow flew as an arrow: the knife has to land on the tick it is swung"
    );
}

/// A shot must know who loosed it, or nothing downstream can attribute ranged damage - not retaliation,
/// not the last blow that decides the meat.
#[test]
fn an_arrow_carries_its_archer() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    // The archer is dat unit 4 (4 tiles of reach, 2 s reload), placed inside its own range and well
    // outside the town centre's.
    let archer = spawn_unit(&mut world, 1, 4, 12, 0, 30);
    let boar = spawn_animal(&mut world, boar_unit, 15, 75, 340);

    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: boar,
        })
        .expect("the archer can be sent at the boar");

    let mut shot = false;
    for _ in 0..400 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            shot = true;
            break;
        }
    }
    assert!(shot, "the archer never landed an arrow");

    let mut turned = false;
    for _ in 0..200 {
        world.step().expect("stepping");
        if matches!(task_of(&world, boar), Some(Task::Attack { target, .. }) if target == archer) {
            turned = true;
            break;
        }
    }
    assert!(
        turned,
        "an arrow that does not carry its archer leaves a struck animal with nobody to turn on"
    );
}

/// Three states, not two: grazing at home, chasing, and **returning**. The owner's rule - a boar that loses
/// interest goes back to where it was - needs a home position the entity never had.
#[test]
fn a_boar_that_loses_interest_returns_to_where_it_was_struck() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let boar = spawn_animal(&mut world, boar_unit, 14, 75, 340);
    let home = pos_of(&world, boar);
    let villager = villager_of(&world);
    let start = pos_of(&world, villager);

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");
    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            struck = true;
            break;
        }
    }
    assert!(struck, "the villager has to land the blow that provokes it");

    // It gives chase ...
    let mut chased = false;
    for _ in 0..200 {
        world.step().expect("stepping");
        if matches!(task_of(&world, boar), Some(Task::Attack { target, .. }) if target == villager)
        {
            chased = true;
            break;
        }
    }
    assert!(chased, "a struck boar chases before it can lose interest");

    // ... and the hunter leaves without hitting it again, so the interest runs out. It walks back to
    // where it started - a tile that is known reachable, unlike a coordinate picked out of the air (a
    // `(2, 2)` target was `NoPath` on this map, which is a fixture bug, not a leash one).
    let away = tile_of(&world, start);
    world
        .submit_checked(Command::Move {
            unit: villager,
            to: away,
        })
        .expect("the villager can leave");
    let mut turned_back = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if !matches!(task_of(&world, boar), Some(Task::Attack { .. })) {
            turned_back = true;
            break;
        }
    }
    assert!(
        turned_back,
        "a boar with no blow landing on it must lose interest rather than chase for ever"
    );

    let mut home_again = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if tiles_between(&world, pos_of(&world, boar), home) <= 1 {
            home_again = true;
            break;
        }
    }
    assert!(
        home_again,
        "a boar that lost interest must walk back to where it was struck, not stand in the open"
    );
}

/// Each new blow renews the interest - which is what the hunter's second arrow is *for*.
#[test]
fn a_second_blow_turns_a_boar_that_had_given_up() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let boar = spawn_animal(&mut world, boar_unit, 14, 75, 340);
    // A second hunter, idle and out of reach until it is ordered in: the re-provoking blow is an arrow,
    // which is exactly the path the owner's "wait until two arrows are shot" describes.
    let archer = spawn_unit(&mut world, 1, 4, 12, 1, 30);
    let villager = villager_of(&world);
    let start = pos_of(&world, villager);

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");
    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            struck = true;
            break;
        }
    }
    assert!(struck, "the first blow has to land");

    let away = tile_of(&world, start);
    world
        .submit_checked(Command::Move {
            unit: villager,
            to: away,
        })
        .expect("the villager can leave");
    let mut gave_up = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if !matches!(task_of(&world, boar), Some(Task::Attack { .. })) {
            gave_up = true;
            break;
        }
    }
    assert!(
        gave_up,
        "the interest has to expire before it can be renewed"
    );

    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: boar,
        })
        .expect("the archer can be sent at the boar");
    let mut renewed = false;
    for _ in 0..1_500 {
        world.step().expect("stepping");
        if matches!(task_of(&world, boar), Some(Task::Attack { target, .. }) if target == archer) {
            renewed = true;
            break;
        }
    }
    assert!(
        renewed,
        "a second blow has to turn the boar round again, or the lure cannot be renewed"
    );
}

/// A reaction the engine deliberately swallows must still be readable. `retaliate`'s `let _ =` hid a real
/// defect for two sessions; the refusal is recorded now, and a stance that forbids the attack is the one
/// case a test can stage deterministically.
#[test]
fn a_refused_retaliation_is_recorded_rather_than_swallowed() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let boar = spawn_animal(&mut world, boar_unit, 14, 75, 340);
    let villager = villager_of(&world);
    world
        .submit_checked(Command::Stance {
            unit: boar,
            stance: ruage_sim::Stance::NoAttack,
        })
        .expect("a stance can be set");
    world.step().expect("the stance lands");

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");
    let mut struck = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            struck = true;
            break;
        }
    }
    assert!(struck, "the blow has to land for a reaction to be refused");
    let recorded = world.last_refusal();
    assert!(
        matches!(recorded, Some((id, ruage_sim::SimError::StanceForbidsAttack { .. }, _)) if id == boar),
        "a refused retaliation must be readable, not a `let _ =`: got {recorded:?}"
    );
}
