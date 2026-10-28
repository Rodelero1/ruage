//! Stances: how much of a fight a unit picks on its own.
//!
//! Written before the implementation (D7). A stance is per unit and deliberately *not* part of `Task`:
//! changing it must not disturb an order, and `Entity` is walked every tick. The four stances the
//! prototype list asks for are aggressive (the default, chase anywhere), defensive (chase only near
//! the ground it was given), stand ground (never move to fight) and no attack (never fight).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, Stance, World};

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

/// The speed the unit table gives this unit, resolved at the world's scale: a test never states a
/// speed of its own (D6).
fn walk_speed(world: &World, fx: &Fixture, unit: u32) -> Fixed {
    fx.units
        .unit(unit)
        .expect("in the table")
        .speed
        .resolve(world.scale(), "speed")
        .expect("a walker")
}

fn place(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
    speed: Fixed,
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
            speed,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity exists");
    (x.raw() / scale, y.raw() / scale)
}

fn distance(a: (i64, i64), b: (i64, i64)) -> i64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).isqrt()
}

#[test]
fn stand_ground_holds_its_tile_while_an_aggressive_unit_closes() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let holder = place(&mut world, 1, 9_300, spearman, (20, 20), 45, speed);
    let chaser = place(&mut world, 1, 9_301, spearman, (20, 24), 45, speed);
    let enemy = place(&mut world, 2, 9_302, spearman, (24, 22), 5, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: holder,
            stance: Stance::StandGround,
        })
        .expect("stance");
    world
        .submit_checked(Command::Attack {
            attacker: holder,
            target: enemy,
        })
        .expect("attack");
    world
        .submit_checked(Command::Attack {
            attacker: chaser,
            target: enemy,
        })
        .expect("attack");

    for _ in 0..400 {
        world.step().expect("step");
    }

    assert_eq!(
        tile_of(&world, holder),
        (20, 20),
        "a unit standing its ground walked to the fight"
    );
    assert!(
        world.entity(enemy).is_none(),
        "the aggressive control never reached the fight either"
    );
}

#[test]
fn stand_ground_still_strikes_what_comes_within_reach() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let holder = place(&mut world, 1, 9_303, spearman, (20, 20), 45, Fixed::ZERO);
    let enemy = place(&mut world, 2, 9_304, spearman, (21, 20), 5, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: holder,
            stance: Stance::StandGround,
        })
        .expect("stance");
    world
        .submit_checked(Command::Attack {
            attacker: holder,
            target: enemy,
        })
        .expect("attack");

    for _ in 0..120 {
        world.step().expect("step");
    }

    assert!(
        world.entity(enemy).is_none(),
        "standing ground does not mean refusing to fight what is already in reach"
    );
}

#[test]
fn no_attack_refuses_a_fight() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let pacifist = place(&mut world, 1, 9_305, spearman, (20, 20), 45, speed);
    let enemy = place(&mut world, 2, 9_306, spearman, (21, 20), 40, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: pacifist,
            stance: Stance::NoAttack,
        })
        .expect("stance");
    let err = world
        .submit_checked(Command::Attack {
            attacker: pacifist,
            target: enemy,
        })
        .expect_err("a unit told not to fight refuses to fight");
    assert!(
        matches!(err, SimError::StanceForbidsAttack { id } if id == pacifist),
        "got {err:?}"
    );
}

#[test]
fn no_attack_walks_past_an_enemy() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let pacifist = place(&mut world, 1, 9_307, spearman, (20, 20), 45, speed);
    let enemy = place(&mut world, 2, 9_308, spearman, (23, 20), 40, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: pacifist,
            stance: Stance::NoAttack,
        })
        .expect("stance");
    world
        .submit_checked(Command::AttackMove {
            unit: pacifist,
            to: (26, 20),
        })
        .expect("attack-move");

    for _ in 0..400 {
        world.step().expect("step");
    }

    assert!(
        world.entity(enemy).is_some(),
        "a unit told not to fight started one on the way"
    );
    assert_eq!(tile_of(&world, pacifist), (26, 20));
}

#[test]
fn a_stand_ground_unit_keeps_walking_past_an_enemy() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let walker = place(&mut world, 1, 9_315, spearman, (20, 20), 45, speed);
    let enemy = place(&mut world, 2, 9_316, spearman, (23, 20), 40, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: walker,
            stance: Stance::StandGround,
        })
        .expect("stance");
    world
        .submit_checked(Command::AttackMove {
            unit: walker,
            to: (26, 20),
        })
        .expect("attack-move");

    for _ in 0..400 {
        world.step().expect("step");
    }

    // Standing ground is about not *leaving* a place, not about refusing to go where it was sent: a
    // unit that stopped to stare at an enemy beside its road would never arrive.
    assert_eq!(
        tile_of(&world, walker),
        (26, 20),
        "a stand-ground unit stopped on its way to a place it was ordered to"
    );
    assert!(
        world.entity(enemy).is_some(),
        "a stand-ground unit picked a fight it was never given"
    );
}

#[test]
fn a_defensive_unit_gives_up_a_chase_beyond_its_ground() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let guard = place(&mut world, 1, 9_309, spearman, (20, 20), 45, speed);
    let runner = place(&mut world, 2, 9_310, spearman, (22, 20), 40, speed);
    // A control that has no stance of its own: it chases wherever the target goes.
    let hunter = place(&mut world, 1, 9_311, spearman, (24, 20), 45, speed);

    world
        .submit_checked(Command::Stance {
            unit: guard,
            stance: Stance::Defensive,
        })
        .expect("stance");
    world
        .submit_checked(Command::Attack {
            attacker: guard,
            target: runner,
        })
        .expect("attack");
    world
        .submit_checked(Command::Attack {
            attacker: hunter,
            target: runner,
        })
        .expect("attack");
    // The target runs away, east, for as long as it takes.
    world
        .submit_checked(Command::Move {
            unit: runner,
            to: (40, 20),
        })
        .expect("move");

    for _ in 0..400 {
        world.step().expect("step");
    }

    let guard_at = tile_of(&world, guard);
    assert!(
        distance(guard_at, (20, 20)) <= 2,
        "a defensive unit chased to {guard_at:?}, away from its ground"
    );
    assert!(
        world.entity(runner).is_some(),
        "the runner died, so the chase was never the thing being tested"
    );
}

#[test]
fn a_stance_is_that_unit_s_own() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let speed = walk_speed(&world, &fx, fx.economy.role["spearman"]);
    let spearman = fx.economy.role["spearman"];
    let still = place(&mut world, 1, 9_312, spearman, (20, 20), 45, Fixed::ZERO);
    let free = place(&mut world, 1, 9_313, spearman, (20, 24), 45, speed);
    let enemy = place(&mut world, 2, 9_314, spearman, (24, 22), 5, Fixed::ZERO);

    world
        .submit_checked(Command::Stance {
            unit: still,
            stance: Stance::NoAttack,
        })
        .expect("stance");
    // The other unit was never told anything, and is therefore aggressive by default.
    world
        .submit_checked(Command::AttackMove {
            unit: free,
            to: (24, 22),
        })
        .expect("attack-move");

    for _ in 0..200 {
        world.step().expect("step");
    }

    assert_eq!(
        tile_of(&world, still),
        (20, 20),
        "a stance leaked to a unit that was never given one"
    );
    assert!(
        world.entity(enemy).is_none(),
        "the unit with a default stance did not fight"
    );
}
