//! **A villager stands with his circle touching the tile, not at a ring round its centre** (D192). The owner, looking at the
//! renderer and then at his own saves: *"villagers seem standing too far from the berry bush ... their circles are touching the
//! square"*, and *"center stays at least half the diameter of the circle (however big that circle is)"*.
//!
//! Measured in his saves: men at **0.70** from a resource tile's centre, which is `0.5 + 0.2` - the circle touching the border - and
//! at **0.99** where two edges meet. The engine stood them at **1.156**, the row behind, which is why the front row never worked.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::tiles::distance_to_tile;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Two bushes, five tiles apart, on open grass.
const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 16
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
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 900
hp = 900
tiles = [[12,7]]
[[map.resource]]
kind = "food"
amount = 900
hp = 900
tiles = [[8,10]]
[[map.town_center]]
player = 1
x = 4
y = 13
[[map.town_center]]
player = 2
x = 19
y = 2
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
        ruage_data::load_map_str(map_text, "touching.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn nodes(world: &World) -> Vec<EntityId> {
    world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect()
}

fn tiles_of(world: &World, node: EntityId) -> Vec<(i64, i64)> {
    world
        .resource_tiles(node)
        .into_iter()
        .map(|(x, y)| (i64::from(x), i64::from(y)))
        .collect()
}

fn spawn(world: &mut World, id: EntityId, tile: (i64, i64)) {
    let unit = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
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
}

/// **The circle touches the tile and does not enter it.** A villager's radius is 0.2, so his distance from the resource tile's
/// border must be 0.2 - near enough to work, never inside. The old rule put him 1.156 from the tile's centre, 0.65 clear of it.
#[test]
fn a_woodcutter_stands_with_his_circle_on_the_tree() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let bush = nodes(&world)[0];
    let tile = tiles_of(&world, bush)[0];
    let id = EntityId(8_400);
    spawn(&mut world, id, (10, 5));
    world
        .submit_checked(Command::Gather {
            villager: id,
            node: bush,
        })
        .expect("a gather order");
    for _ in 0..1_200 {
        world.step().expect("a tick");
    }

    let at = world.position(id).expect("a position");
    let clearance = distance_to_tile(at, tile, scale);
    let radius = scale / 5; // a villager's 0.2
    assert!(
        clearance >= radius - scale / 100,
        "MEASURED the villager {:.3} tiles from the bush tile; his circle is {:.2} and may not be inside it",
        clearance as f64 / scale as f64,
        radius as f64 / scale as f64
    );
    assert!(
        clearance <= radius + scale / 20,
        "MEASURED the villager {:.3} tiles from the bush tile, and his circle is {:.2} - he is standing away from it, not \
         touching it. The front row touches (0.5 + 0.2 = 0.70 from the tile's centre); the engine used 1.156",
        clearance as f64 / scale as f64,
        radius as f64 / scale as f64
    );
    assert!(
        matches!(world.entity(id).map(|e| e.task), Some(Task::Gather { .. })),
        "and he has to be working, not merely standing there"
    );
}

/// **Ten men on one bush, every circle touching it and none overlapping another**: what the owner's own save shows, and what the
/// eleven touching places the geometry module measures allow.
#[test]
fn ten_men_touch_one_bush_without_overlapping_it_or_each_other() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let bush = nodes(&world)[0];
    let tile = tiles_of(&world, bush)[0];
    let radius = scale / 5;
    let mut ids = Vec::new();
    for index in 0..10i64 {
        let id = EntityId(8_500 + index as u32);
        spawn(&mut world, id, (9 + index % 4, 3 + index / 4));
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: bush,
            })
            .expect("a gather order");
        ids.push(id);
    }
    for _ in 0..1_800 {
        world.step().expect("a tick");
    }

    let working: Vec<EntityId> = ids
        .iter()
        .copied()
        .filter(|id| {
            matches!(world.entity(*id).map(|e| e.task), Some(Task::Gather { node, .. }) if node == bush)
        })
        .collect();
    assert_eq!(
        working.len(),
        10,
        "MEASURED {} of 10 working the bush; eleven touching places fit one tile",
        working.len()
    );
    let mut spots = Vec::new();
    for id in &working {
        let at = world.position(*id).expect("a position");
        let clearance = distance_to_tile(at, tile, scale);
        assert!(
            clearance >= radius - scale / 50,
            "MEASURED a villager {:.3} tiles from the bush tile: his circle is inside it",
            clearance as f64 / scale as f64
        );
        assert!(
            clearance <= radius + scale / 12,
            "MEASURED a villager {:.3} tiles from the bush tile, standing away from it rather than touching it",
            clearance as f64 / scale as f64
        );
        spots.push(at);
    }
    let mut closest = f64::MAX;
    for a in 0..spots.len() {
        for b in (a + 1)..spots.len() {
            let dx = (spots[a].0.raw() - spots[b].0.raw()) as f64 / scale as f64;
            let dy = (spots[a].1.raw() - spots[b].1.raw()) as f64 / scale as f64;
            closest = closest.min((dx * dx + dy * dy).sqrt());
        }
    }
    assert!(
        closest >= 0.39,
        "MEASURED the closest pair {closest:.3} tiles apart; a villager's circle is 0.40 wide"
    );
}
