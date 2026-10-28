//! A building that dies takes its bookkeeping with it.
//!
//! Written before the implementation (D7). The death sweep already removes the entity and its route;
//! what it did not do is release the *side tables* keyed by its id - its queue, its rally, its units'
//! stances and patrols. That is the same leak `paths` had once: a dead entity's bookkeeping lives on
//! forever and the state hash walks it, so a long match accumulates state nobody can see.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Stance, Task, World};

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

fn spawn_building(world: &mut World, player: u8, id: u32, role: &str, tile: (i64, i64), hp: i32) {
    let scale = world.scale();
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player,
            kind: EntityKind::Building,
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
}

fn spawn_unit(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64), hp: i32) {
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
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
}

#[test]
fn a_destroyed_house_lowers_the_population_cap() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    // A house at one hit point, and a spearman standing next to it.
    spawn_building(&mut world, 2, 9_400, "house", (30, 30), 1);
    spawn_unit(&mut world, 1, 9_401, spearman, (31, 30), 45);
    let before = world.population_cap(2);
    assert!(
        before >= 10,
        "the fixture's house is not counted: cap {before}"
    );

    world
        .submit_checked(Command::Attack {
            attacker: EntityId(9_401),
            target: EntityId(9_400),
        })
        .expect("attack");
    for _ in 0..200 {
        world.step().expect("step");
    }

    assert!(
        world.entity(EntityId(9_400)).is_none(),
        "the house survived one hit at one hit point"
    );
    assert_eq!(
        world.population_cap(2),
        before - 5,
        "a destroyed house still supports five units"
    );
}

#[test]
fn a_razed_building_takes_its_queue_with_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let militia = fx.economy.role["militia"];
    spawn_building(&mut world, 1, 9_402, "barracks", (24, 24), 1_200);

    // Two orders, not one: the head of a queue lives on the building and only the rest live in the
    // side table, so a single order would not exercise the thing this test is about.
    for _ in 0..2 {
        world
            .submit_checked(Command::Train {
                building: EntityId(9_402),
                unit: militia,
            })
            .expect("train");
    }
    world.step().expect("step");
    assert_eq!(
        world.pending_orders(EntityId(9_402)),
        2,
        "the barracks is not making what it was told to make"
    );

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: EntityId(9_402),
        })
        .expect("raze");
    world.step().expect("step");

    assert!(
        world.entity(EntityId(9_402)).is_none(),
        "the barracks is still standing"
    );
    assert_eq!(
        world.pending_orders(EntityId(9_402)),
        0,
        "a razed barracks is still holding a queue"
    );
}

#[test]
fn a_dead_unit_releases_its_interrupted_walk() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    // An enemy house four tiles away, so the attack-move engages without striking: what is being
    // tested is the walk the fight interrupted, not the fight.
    spawn_building(&mut world, 2, 9_404, "house", (24, 20), 550);
    // The baseline is taken before the unit exists, so "back to where it started" means what it says.
    let quiet = world.bookkeeping_entries();
    spawn_unit(&mut world, 1, 9_405, spearman, (20, 20), 45);

    world
        .submit_checked(Command::AttackMove {
            unit: EntityId(9_405),
            to: (26, 20),
        })
        .expect("attack-move");
    // A hunt looks around on a schedule, staggered by entity id, so engagement lands on the first
    // tick where this unit's turn comes round - five at most at the reference setting. The *task* is
    // what says it engaged: the route appears a tick before the fight does.
    let mut engaged = false;
    for _ in 0..5 {
        world.step().expect("step");
        if matches!(
            world.entity(EntityId(9_405)).map(|entity| entity.task),
            Some(Task::Attack { .. })
        ) {
            engaged = true;
            break;
        }
    }
    assert!(
        engaged,
        "the attack-move engaged nothing, so there is no interrupted walk to release"
    );

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: EntityId(9_405),
        })
        .expect("raze");
    world.step().expect("step");
    assert_eq!(
        world.bookkeeping_entries(),
        quiet,
        "a dead unit's interrupted walk is still in the world's bookkeeping"
    );
}

#[test]
fn a_dead_unit_leaves_no_bookkeeping_behind() {
    let Some(fx) = fixture() else { return };
    let spearman = fx.economy.role["spearman"];

    // Two worlds that differ only in what the dead unit had been told to do: a stance and a patrol
    // that outlived the unit would make their hashes disagree, because the hash walks those tables.
    let mut plain = seeded(&fx);
    let mut busy = seeded(&fx);
    for world in [&mut plain, &mut busy] {
        spawn_unit(world, 1, 9_403, spearman, (18, 18), 45);
    }

    busy.submit_checked(Command::Stance {
        unit: EntityId(9_403),
        stance: Stance::Defensive,
    })
    .expect("stance");
    busy.submit_checked(Command::Patrol {
        unit: EntityId(9_403),
        to: (24, 18),
    })
    .expect("patrol");
    // Both worlds step the same number of times: a differing tick counter would explain the hash
    // difference and hide the one this test is looking for.
    plain.step().expect("step");
    busy.step().expect("step");

    for world in [&mut plain, &mut busy] {
        world
            .submit_checked(Command::Delete {
                player: 1,
                entity: EntityId(9_403),
            })
            .expect("raze");
        world.step().expect("step");
        assert!(world.entity(EntityId(9_403)).is_none());
    }

    assert_eq!(
        plain.state_hash(),
        busy.state_hash(),
        "a dead unit's stance or patrol is still in the state hash"
    );
}
