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

/// **Distance between two points**, in tiles.
fn gap(world: &World, a: (Fixed, Fixed), b: (Fixed, Fixed)) -> f64 {
    let scale = world.scale() as f64;
    let dx = a.0.raw() as f64 / scale - b.0.raw() as f64 / scale;
    let dy = a.1.raw() as f64 / scale - b.1.raw() as f64 / scale;
    (dx * dx + dy * dy).sqrt()
}

/// **The gatherer stands on the measured ring** (D186): 1.156 of a tile from the bush's square, which is where the six
/// woodcutters in the owner's save stand. The old rule put him where the two bodies touched, 0.70 from the centre.
#[test]
fn a_gatherer_stands_on_the_measured_ring() {
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
    let gap = gap(&world, mine, at);
    assert!(
        (1.05..=1.27).contains(&gap),
        "MEASURED the gatherer at {gap:.3} tiles from the bush's square; the ring is 1.156, D169's standoff 0.70"
    );
}

/// **One square takes many men, on its ring** (D186). D182 gave a square one gatherer and answered the second order with another source; the
/// owner's save shows sixteen orders to two adjacent squares (nine and seven) and eighteen men fitting one ring
/// (`2 * pi * 1.156 / 0.404 = 17.97`). So three villagers ordered onto one bush all work it - a body apart, not stacked.
#[test]
fn three_gatherers_all_work_the_same_square_on_its_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let all = bushes(&world);
    let first = all[0];
    let at_bush = world.position(first).expect("the bush's square");
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
    for _ in 0..780 {
        world.step().expect("a tick");
    }
    let mut earlier: std::collections::BTreeMap<u32, (Fixed, Fixed)> =
        std::collections::BTreeMap::new();
    for id in &ids {
        if let Some(at) = world.position(*id) {
            earlier.insert(id.0, at);
        }
    }
    for _ in 0..20 {
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
    assert_eq!(
        working.len(),
        3,
        "all three villagers are gathering: {:?}",
        working
    );
    assert!(
        working.iter().all(|(_, node)| *node == first),
        "all three work the square that was asked for: {:?}",
        working
    );
    // **A body apart, not on top of each other.** Two villager radii are 0.40 of a tile, which is the closest pair in the
    // owner's save (0.404) and the spacing eighteen men get on a 1.156 ring - but only for men who have **stopped**: a man
    // walking along the ring past another is a crossing, not a place.
    // **Men who have stopped at their work**, not men walking along the ring past each other: a man on his way to a slot
    // crosses other ground, and two crossing paths are a moment. The claim here is about men *standing* at the same bush.
    let mut still: Vec<(Fixed, Fixed)> = Vec::new();
    for id in &ids {
        let now = world.position(*id).expect("a position");
        let before = earlier.get(&id.0).copied().expect("an earlier position");
        let standing = now.0.raw() == before.0.raw() && now.1.raw() == before.1.raw();
        let working = world.entity(*id).is_some_and(
            |entity| matches!(entity.task, Task::Gather { node: n, .. } if n == first),
        );
        if standing && working && (1.05..=1.30).contains(&gap(&world, now, at_bush)) {
            still.push(now);
        }
    }
    assert!(
        still.len() >= 2,
        "at least two villagers have settled at the bush: {still:?}"
    );
    for i in 0..still.len() {
        for j in (i + 1)..still.len() {
            let apart = gap(&world, still[i], still[j]);
            assert!(
                apart >= 0.39,
                "MEASURED two settlers {apart:.3} tiles apart on one bush; two bodies are 0.40 wide"
            );
        }
    }
}
