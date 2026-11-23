//! **Gatherers take a place each - and press only when no place is free.**
//!
//! This file was written for the opposite rule. Until 2026-11-22 the owner's ruling was *"Drop it and let the crowds jam"* (D289)
//! with the seat book cut (D293), on the reading that the binary computes a depenetration and discards it (D290) - so a second
//! man on a node was *required* to press into the first, and this fixture failed any pair that stood a body's width apart.
//!
//! He saw the result and ruled again (D356's ruling 1, 2026-11-22): *"the second man steps to the next free spot near the same
//! tree/mine (the game's own placement rule), and only goes idle if there is none"*. That is `func_002699A0` - a **side** is
//! picked and the candidate is placed `ext_a + ext_b + 0.01` out from it, refusing any obstructed candidate - and it is the
//! **arrival** rule, not a press floor during a walk: D290 stands, because the depenetration a moving body computes is still
//! discarded. The assertion below is inverted with the ruling, and it says so.
//!
//! Written before the change (D7, SPEC §11); the rule it now pins was watched failing at **0 raw** apart
//! (`docs/reviews/2026-11-22-d357-the-arrival-is-a-placement.md`).

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

/// One tree at (10,10) with room on every side; nothing else to eat or chop.
const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 22
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 50000
hp = 0
tiles = [[10,10]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 20
y = 20
"#;

fn world_of(map_text: &str) -> Option<World> {
    if !repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "press.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must(
        "load_units_file",
        ruage_data::load_units_file(&units_file()),
    );
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn units_file() -> PathBuf {
    repo("config/gamedata/units.toml")
}

fn spawn(world: &mut World, at: (i64, i64), id: u32) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(at.0 * scale + scale / 2),
                Fixed::from_raw(at.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    id
}

/// **Two men on one tree stand pressed, both taking from it.**
#[test]
fn two_gatherers_on_one_tree_press_in() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let tree = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the tree");
    // Both on the same side, two tiles apart, so their seats - if there are seats - are next to each other rather than opposite.
    let a = spawn(&mut world, (12, 10), 9_700);
    let b = spawn(&mut world, (12, 12), 9_701);
    for man in [a, b] {
        world
            .submit_checked(Command::Gather {
                villager: man,
                node: tree,
            })
            .expect("a gather order");
    }

    // Long enough for both to arrive, take, bank and come back.
    let scale = world.scale();
    let mut closest = i64::MAX;
    let mut both_taking = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        let taking = |id: EntityId| {
            world.entity(id).is_some_and(
                |e| matches!(e.task, Task::Gather { progress, .. } if progress.raw() > 0),
            )
        };
        if taking(a) && taking(b) {
            both_taking = true;
            let (pa, pb) = (
                world.position(a).expect("alive"),
                world.position(b).expect("alive"),
            );
            let (dx, dy) = (pa.0.raw() - pb.0.raw(), pa.1.raw() - pb.1.raw());
            closest = closest.min((dx * dx + dy * dy).isqrt());
        }
    }
    assert!(
        both_taking,
        "the two never took from the tree on the same tick, so the fixture measures nothing"
    );
    // A villager's radius is 0.2, so two of them touch at 0.4 of a tile (`2 * radius`). **A place each** means at least that:
    // the rule was the opposite until D356's ruling 1, and the assertion is inverted with it rather than softened.
    let diameter = scale * 2 / 5;
    assert!(
        closest >= diameter,
        "the two stood {:.3} tiles apart at their closest while both working - an arriving man is placed a body's width \
         along the node's side (D356, `func_002699A0`)",
        closest as f64 / scale as f64
    );
}
