//! **Stand a body's radius off the square, and go elsewhere when that spot is taken** (D182).
//!
//! The owner's rule, verbatim: *"Can you make the villagers to stand at the distance of half of diameter of the circle from the square they are
//! gathering any resource from? If there is no space at half diameter from another villager and the the resource try another nearest resource of the
//! same kind"*.
//!
//! Half the diameter of the circle **is** its radius - 0.2 of a tile for a villager - so the ring touches the square's edge and the centre stands
//! `0.5 + 0.2 = 0.70` from the tile's centre. That is the number D169 adopted; what was missing is that a worker *reaches* it and *only* works
//! from it, and that a second worker wanting the same square goes somewhere else instead of crowding the ring.
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

/// Two bushes, five tiles apart, on open grass.
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
amount = 300
hp = 300
tiles = [[10,6]]
[[map.resource]]
kind = "food"
amount = 300
hp = 300
tiles = [[11,7]]
[[map.resource]]
kind = "food"
amount = 300
hp = 300
tiles = [[9,8]]
[[map.town_center]]
player = 1
x = 2
y = 12
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
        ruage_data::load_map_str(map_text, "standoff_rule.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn bushes(world: &World) -> Vec<EntityId> {
    world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect()
}

/// **Distance from the point to the edge of the tile it is over**, in tiles.
fn gap_to_edge(world: &World, point: (Fixed, Fixed), tile: (Fixed, Fixed)) -> f64 {
    let scale = world.scale() as f64;
    let (x, y) = (point.0.raw() as f64 / scale, point.1.raw() as f64 / scale);
    let (tx, ty) = (tile.0.raw() as f64 / scale, tile.1.raw() as f64 / scale);
    // The square spans `tile +/- 0.5`; the gap is how far outside it the point is, on whichever axis it is outside.
    let dx = (x - tx).abs() - 0.5;
    let dy = (y - ty).abs() - 0.5;
    (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
}

/// **The ring touches the square**: a body's radius - half the ring's diameter - between the villager's centre and the square's edge.
#[test]
fn a_gatherer_stands_a_body_radius_from_the_square() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = bushes(&world)[0];
    let at = world.position(node).expect("the bush");
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(9_400);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(6 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Gather { villager: id, node })
        .expect("a gather order");
    for _ in 0..400 {
        world.step().expect("a tick");
    }

    let mine = world.position(id).expect("the villager");
    let gap = gap_to_edge(&world, mine, at);
    assert!(
        (0.15..=0.25).contains(&gap),
        "the villager stands {gap:.2} tiles from the square's edge; half the ring's diameter is 0.20 (its centre at 0.70 from the tile's centre)"
    );
}

/// **A square holds one gatherer; the next man is sent to the nearest other square of the same kind.** The owner's rule: *"if there is no space
/// at half diameter from another villager and the resource, try another nearest resource of the same kind"*. Three squares, three villagers, three
/// different squares - not three men round one bush.
#[test]
fn a_taken_square_sends_the_next_gatherer_to_another() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let all = bushes(&world);
    let first = all[0];
    let villager = world.role_unit("villager").expect("villager");
    let mut ids = Vec::new();
    for (index, x) in [6i64, 7, 8].into_iter().enumerate() {
        let id = EntityId(9_410 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(x * scale + scale / 2),
                    Fixed::from_raw(6 * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        ids.push(id);
    }
    world.step().expect("the villagers land");
    for id in &ids {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node: first,
            })
            .expect("a gather order");
    }
    for _ in 0..800 {
        world.step().expect("a tick");
    }

    // Each villager works a **different** square: the order named one square for all three, and the other two were answered with the nearest
    // source of the same kind instead of being crowded round the first.
    let mut working: Vec<(EntityId, EntityId)> = Vec::new();
    for id in &ids {
        let task = world
            .entities()
            .iter()
            .find(|entity| entity.id == *id)
            .map(|entity| entity.task)
            .expect("the villager still exists");
        if let Task::Gather { node, .. } = task {
            working.push((*id, node));
        }
    }
    assert_eq!(working.len(), 3, "all three villagers are gathering");
    assert!(
        working.iter().any(|(_, node)| *node == first),
        "the square that was asked for is still being worked by somebody"
    );
    let mut nodes: Vec<EntityId> = working.iter().map(|(_, node)| *node).collect();
    nodes.sort_by_key(|node| node.0);
    nodes.dedup();
    assert_eq!(
        nodes.len(),
        3,
        "all three villagers are working the same square instead of spreading over the three that exist: {:?}",
        working
    );
}
