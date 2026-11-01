//! D88: blows land in order, so two units cannot trade killing blows.
//!
//! The owner's decision (2026-11-01): "units cannot trade killing blows, follow original". Our shape applied every
//! blow of a tick against the state at the start of it, so two units that could each kill the other both died. The
//! original applies blows in order and a unit whose turn comes after it died does not swing.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // Accuracy is its own test; these two stand still and swing at arm's length.
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
}

fn place(world: &mut World, player: u8, id: u32, tile: (i64, i64), hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(VILLAGER),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    id
}

const VILLAGER: u32 = 83;

/// One blow each, in id order: the first kills, so the second never swings.
#[test]
fn a_unit_killed_by_an_earlier_blow_does_not_swing() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let first = place(&mut world, 1, 900, (30, 30), 1);
    let second = place(&mut world, 2, 901, (31, 30), 1);
    world
        .step()
        .expect("the two exist before an order names them");

    world
        .submit_checked(Command::Attack {
            attacker: first,
            target: second,
        })
        .expect("first attacks second");
    world
        .submit_checked(Command::Attack {
            attacker: second,
            target: first,
        })
        .expect("second attacks first");

    // Ten game-seconds is many reloads: without the rule both die inside the first one.
    for _ in 0..200 {
        world.step().expect("step");
    }

    // A dead unit is swept out of the world, so presence is the assertion: the earlier attacker is still
    // there, and the unit that was killed before its turn is gone.
    assert!(
        world.entity(second).is_none(),
        "the lower-id attacker's blow landed, so the second unit is dead"
    );
    assert!(
        world.entity(first).is_some(),
        "the unit killed before its turn never swung, so the first is still standing"
    );
}
