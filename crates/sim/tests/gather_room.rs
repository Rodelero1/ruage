//! **A node with fewer than nine men is still a node with room** (D186).
//!
//! `claimed_by_other` is the picker's "is this source taken?" - D182's one-worker-a-square written as a question, and it
//! answers **yes** if *anybody* is gathering the node. Under the ring that is wrong: nine men work one square, so a node with
//! two men is a node with seven places free, and a woodcutter whose tree runs dry should join it rather than wander.
//!
//! What the bug looks like: the picker skips every node somebody is already working, finds nothing, and the villager stands
//! still with wood in sight - and on a map where every patch has one man on it, that is *every* villager.
//!
//! Written before the change (D7, SPEC §11).

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

/// Two patches of one tree each, far apart, with room on all sides; a town centre well clear of both.
const MAP: &str = r#"
schema = 1
[map]
width = 26
height = 20
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGFGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
]
# A tiny patch: one villager empties it in a few seconds, so the "next source" search happens inside the test.
[[map.resource]]
kind = "wood"
amount = 12
hp = 0
tiles = [[10,5]]
[[map.resource]]
kind = "wood"
amount = 50000
hp = 0
tiles = [[12,5]]
[[map.town_center]]
player = 1
x = 6
y = 10
[[map.town_center]]
player = 2
x = 24
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
        ruage_data::load_map_str(map_text, "room.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn villager_at(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(7_000 + id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(tile.0 * scale + scale / 2),
                Fixed::from_raw(tile.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    id
}

/// **A man whose patch runs dry joins a patch that already has men on it**, because two men out of nine is room, not a
/// queue. The old picker refused every node anybody was working, so he stood still beside a forest with places free.
#[test]
fn a_woodcutter_whose_tree_runs_out_joins_a_patch_that_already_has_men_on_it() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let nodes: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect();
    assert_eq!(nodes.len(), 2, "two patches");
    let small = nodes
        .iter()
        .copied()
        .find(|id| {
            world
                .entity(*id)
                .is_some_and(|entity| entity.amount.raw() < 1_000 * world.scale())
        })
        .expect("the small patch");
    let large = nodes
        .iter()
        .copied()
        .find(|id| *id != small)
        .expect("the large patch");

    // Two men on the large patch first, so it is a patch "somebody is already working".
    let first = villager_at(&mut world, 1, (10, 7));
    let second = villager_at(&mut world, 2, (13, 7));
    for id in [first, second] {
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: large,
            })
            .expect("a gather order");
    }
    // ... and a third on the small one, which will run out.
    let third = villager_at(&mut world, 3, (10, 8));
    world
        .submit_checked(Command::Gather {
            villager: third,
            node: small,
        })
        .expect("a gather order");

    for _ in 0..2_400 {
        world.step().expect("a tick");
    }

    let task = world.entity(third).expect("the third villager").task;
    assert!(
        matches!(task, Task::Gather { node, .. } if node == large),
        "MEASURED the third woodcutter's task as {task:?}: his patch ran dry and the large patch has two men of nine, so \
         it has room"
    );
}

/// **The tenth man on a full square joins a square that already has one.** This is the picker's own question (`nearest_like_free`,
/// called when `start_gather` finds nine men already on the node) and it is where D182's *"one worker a square"* survives as a
/// **bug**: it called a square with a single man on it *taken*, so the tenth man found nothing and stood still.
#[test]
fn the_tenth_man_on_a_full_square_joins_a_square_that_has_one_man_on_it() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let large = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .find(|entity| entity.amount.raw() > 1_000 * world.scale())
        .map(|entity| entity.id)
        .expect("the large patch");
    let small = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .find(|entity| entity.id != large)
        .map(|entity| entity.id)
        .expect("the small patch");

    // Nine men fill the large patch, one man goes to the small one, and an eleventh arrives expecting a place on the large.
    let mut ids = Vec::new();
    for index in 0..10u32 {
        let tile = (9 + i64::from(index % 5), 8);
        ids.push(villager_at(&mut world, index + 10, tile));
    }
    for (index, id) in ids.iter().enumerate() {
        let node = if index == 9 { small } else { large };
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node,
            })
            .expect("a gather order");
    }
    let eleventh = villager_at(&mut world, 30, (13, 8));
    world
        .submit_checked(Command::Gather {
            villager: eleventh,
            node: large,
        })
        .expect("a gather order");

    for _ in 0..1_200 {
        world.step().expect("a tick");
    }

    let task = world.entity(eleventh).expect("the eleventh villager").task;
    assert!(
        matches!(task, Task::Gather { node, .. } | Task::Return { node, .. } if node == small || node == large),
        "MEASURED the eleventh man's task as {task:?}: the large patch is full at nine and the small one holds a single man \
         of nine, so it has eight places free"
    );
}
