//! Resources are **whole units**: a villager carries 10 wood, never 9.27, and a node holds 100, never 87.4.
//!
//! Verified against 0 A.D. (`ResourceGatherer.GATHER_AMOUNT = 1`, `carrying` documented as an *integer* amount, and
//! `ResourceSupply.TakeResources` returning what actually changed, with an exhausted supply stopping the gatherer and
//! re-targeting it) and against the dat's own integer amounts. The fraction belongs in an accumulator, not in the carry -
//! which is where our `0.03`-of-a-unit loads came from (`crates/sim/tests/dropoff.rs`'s symptoms, and the twenty-seven
//! villagers standing beside a mill).
//!
//! Written after the accumulator went in and each test was then shown to bite by mutation (reported in the review).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

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

fn spot(
    world: &mut World,
    kind: EntityKind,
    unit: Option<u32>,
    at: (u16, u16),
    amount: u32,
) -> EntityId {
    let id = next_id(world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: if kind == EntityKind::Unit { 1 } else { 0 },
            kind,
            unit,
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: if kind == EntityKind::Unit { 25 } else { 0 },
            amount,
        })
        .expect("placed");
    world.step().expect("one tick");
    id
}

/// A villager chopping a tree always carries a whole number of units, and the tree always holds a whole number.
#[test]
fn a_load_is_always_a_whole_number_of_units() {
    let Some(mut world) = fresh() else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood column");
    let villager = world.role_unit("villager").expect("a villager role");
    let tree = spot(&mut world, EntityKind::Resource(wood), None, (40, 40), 100);
    let worker = spot(&mut world, EntityKind::Unit, Some(villager), (39, 40), 0);
    world
        .submit_checked(Command::Gather {
            villager: worker,
            node: tree,
        })
        .expect("the tree can be worked");

    let scale = world.scale();
    let mut saw_a_unit = false;
    for tick in 0..2_000 {
        world.step().expect("stepping");
        let carry = world.carry(worker).raw();
        assert_eq!(
            carry % scale,
            0,
            "at tick {tick} the villager carried {:.3} of a unit, and resources are whole",
            carry as f64 / scale as f64
        );
        let left = world.entity(tree).map(|e| e.amount.raw()).unwrap_or(0);
        assert_eq!(
            left % scale,
            0,
            "at tick {tick} the tree held {:.3} of a unit",
            left as f64 / scale as f64
        );
        if carry > 0 {
            saw_a_unit = true;
        }
    }
    assert!(saw_a_unit, "the villager should have taken whole units");
}

/// Two villagers race for a node with one unit left: exactly one of them gets it, and the other is granted nothing.
///
/// The loser does not keep a fraction and does not stand there: with nothing left to take it is sent on (`nearest_like`),
/// which is 0 A.D.'s `StopGathering("TargetInvalidated")` and the owner's rule.
#[test]
fn one_villager_gets_the_last_unit_and_the_other_moves_on() {
    let Some(mut world) = fresh() else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood column");
    let villager = world.role_unit("villager").expect("a villager role");
    // One unit in the node, two villagers, each on its own side.
    let stump = spot(&mut world, EntityKind::Resource(wood), None, (40, 40), 1);
    let a = spot(&mut world, EntityKind::Unit, Some(villager), (39, 40), 0);
    let b = spot(&mut world, EntityKind::Unit, Some(villager), (41, 40), 0);
    for worker in [a, b] {
        world
            .submit_checked(Command::Gather {
                villager: worker,
                node: stump,
            })
            .expect("the stump can be worked");
    }

    let mut carries = Vec::new();
    for _ in 0..2_000 {
        world.step().expect("stepping");
        carries = vec![world.carry(a).raw(), world.carry(b).raw()];
        if carries.iter().any(|c| *c > 0) {
            break;
        }
    }
    let scale = world.scale();
    let took: Vec<i64> = carries.iter().map(|c| c / scale).collect();
    assert_eq!(
        took.iter().sum::<i64>(),
        1,
        "exactly one unit left the node, and exactly one villager carries it: {took:?}"
    );
    assert!(
        took.contains(&1) && took.contains(&0),
        "one villager took the last unit and the other got nothing: {took:?}"
    );
}
