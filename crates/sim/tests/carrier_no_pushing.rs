//! **PARKED - the observable does not witness the rule.** The carrier half of D204 is *"a villager in `Task::Return`
//! is never displaced"*; extending the gate in `push_the_standing` to `Task::Gather | Task::Return` is one line. The
//! fixture below is the acceptance match's own geometry - a crowd of villagers filling and carrying home from one bush
//! to one town centre, so that **every body in it has a gather or return order** - and the observable it was specified
//! with is `World::made_room()`.
//!
//! **Measured, this tree (`46cec66`), 4,000 ticks:** `made_room() == 7`; with the gate extended to
//! `Task::Gather | Task::Return` it is **also `7`** and `push_the_standing` moves nobody in either run. `made_room` is
//! not the displacement counter: it counts the *walker's own* make-room step (`crates/sim/src/world/movement.rs:400` ->
//! `crates/sim/src/world/step.rs:1317`, C1b), a different mechanism from `push_the_standing`, which moves the
//! **standing** body and increments no counter at all. So this assertion is red on `main` and stays red with the rule
//! built: it cannot be watched green, and a rule change nobody has seen fail is what D204's own review forbids.
//!
//! What is true, measured over the acceptance run: on `main` (post-D204) `push_the_standing` moves **161** bodies -
//! **147 `Build`, 12 `Attack`, 2 `Idle`, 0 `Return`, 0 `Gather`** - while `7d00be8` (pre-D204) reproduces the recorded
//! **310**: 232 `Build`, **76 `Return`**, 2 `Idle`. The 76 carriers are gone from the match D204 left behind, and every
//! `Return` body that comes near a shover is already skipped by the `posted || (busy && !jammed)` rule before the nudge
//! step is reached (22,253 candidate considerations over the acceptance run, all skipped). The record is
//! `docs/reviews/2026-11-14-d204-carriers.md`.
//!
//! Kept as the fixture the next attempt starts from, `#[ignore]`d so the suite is not left red.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One bush at (7,6) and one town centre at (7,11), five tiles apart on open ground - the acceptance match's own
/// arrangement: a cluster to forage and a drop site a short walk away.
const MAP: &str = r#"
schema = 1
[map]
width = 20
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 100000
hp = 20
tiles = [[7,6]]
[[map.town_center]]
player = 1
x = 7
y = 11
[[map.town_center]]
player = 2
x = 18
y = 1
"#;

fn world_of(map_text: &str) -> Option<World> {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "carrier_no_pushing.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// The fixture: seven villagers working one bush and carrying the loads home to one town centre. Seven is the ring's
/// own size, so nobody is left standing without an order - the crowd is the ring, which is what makes every body in it
/// a body the D204 rule speaks about.
fn a_crowd_filling_and_carrying_home() -> Option<(World, Vec<EntityId>)> {
    let mut world = world_of(MAP)?;
    let scale = world.scale();
    let bush = world
        .entities()
        .iter()
        .find(|entity| world.resource_tiles(entity.id).first() == Some(&(7, 6)))
        .map(|entity| entity.id)?;
    let villager = world.role_unit("villager")?;
    let spots = [
        (5, 10),
        (6, 10),
        (7, 10),
        (8, 10),
        (9, 10),
        (6, 11),
        (8, 11),
    ];
    let mut crew = Vec::new();
    for (index, (x, y)) in spots.iter().enumerate() {
        let id = EntityId(9_900 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(i64::from(*x) * scale + scale / 2),
                    Fixed::from_raw(i64::from(*y) * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager on clear ground");
        world.step().expect("the villager lands");
        crew.push(id);
    }
    for id in &crew {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node: bush,
            })
            .expect("a gather order");
    }
    Some((world, crew))
}

/// **Nobody carrying a load home is moved** - the carrier half of D204. Parked: the assertion is `made_room() == 0`,
/// which is red on `main` at 7 for a reason that has nothing to do with pushing, and stays red once the gate is
/// extended (see this file's header). Run it with `--ignored` to see the measurement.
#[test]
#[ignore = "parked: made_room counts the walker's own make-room step, not push_the_standing (see docs/reviews/2026-11-14-d204-carriers.md)"]
fn a_crowd_carrying_home_is_never_pushed() {
    let Some((mut world, crew)) = a_crowd_filling_and_carrying_home() else {
        return;
    };
    for _ in 0..4_000 {
        world.step().expect("a tick");
    }
    // **Every body in the fixture holds a gather or return order**, or it is not the crowd D204 is about.
    let mut gathering = 0;
    let mut returning = 0;
    for id in &crew {
        match world.entity(*id).map(|entity| entity.task) {
            Some(Task::Gather { .. }) => gathering += 1,
            Some(Task::Return { .. }) => returning += 1,
            other => {
                panic!("PRECONDITION body {id:?} is not gathering or carrying home: {other:?}")
            }
        }
    }
    assert!(
        returning > 0,
        "PRECONDITION nobody is carrying a load at tick 4,000: {gathering} still gathering"
    );
    assert_eq!(
        world.made_room(),
        0,
        "MEASURED the crowd displaced {} time(s): {gathering} gathering, {returning} carrying home",
        world.made_room()
    );
}
