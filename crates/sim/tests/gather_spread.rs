//! **Two workers on one node stand on different sides of it** (D171).
//!
//! D169 put a gatherer where its body touches the node - 0.7 of a tile - but it picked the side the worker **came from**, so two villagers sent at the
//! same tree settled on the *same point* and then pushed each other around for the rest of the match. The acceptance run showed it: 150 -> 137
//! entities and repaths 88 -> 144. The original fans them round the node, and this is that rule.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One tree, open ground on **all four sides** of it - the point of the map: a worker must be able to take any side.
const MAP: &str = r#"
schema = 1
[map]
width = 18
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[10,6]]
[[map.town_center]]
player = 1
x = 2
y = 12
[[map.town_center]]
player = 2
x = 16
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
        ruage_data::load_map_str(map_text, "spread.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn apart(a: (Fixed, Fixed), b: (Fixed, Fixed), scale: i64) -> f64 {
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale as f64;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale as f64;
    (dx * dx + dy * dy).sqrt()
}

/// **Both villager come from the west** - the side that exposes the bug. Without a spread both settle on the same point.
#[test]
fn two_woodcutters_on_one_tree_take_different_sides() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    // Two villagers on the same side of the tree, three and four tiles west of it.
    for (id, at) in [(9_610u32, (6i64, 6i64)), (9_611, (5, 8))] {
        world
            .submit_checked(Command::Spawn {
                id: EntityId(id),
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
    }
    for id in [9_610u32, 9_611] {
        world
            .submit_checked(Command::Gather {
                villager: EntityId(id),
                node,
            })
            .expect("a gather order");
    }
    for _ in 0..600 {
        world.step().expect("a tick");
    }

    let a = world.position(EntityId(9_610)).expect("the first villager");
    let b = world
        .position(EntityId(9_611))
        .expect("the second villager");
    let tree = world.position(node).expect("the tree");
    // Both are at the tree: each within the touching distance, so neither is still walking when this is measured.
    for (label, at) in [("the first", a), ("the second", b)] {
        let gap = apart(at, tree, scale);
        assert!(
            gap <= 0.75,
            "{label} woodcutter is {gap:.2} tiles from the tree; both should be standing at its edge"
        );
    }
    // ... and they are on **different sides**, because one spot per side is the whole point of the spread.
    let split = apart(a, b, scale);
    assert!(
        split >= 0.6,
        "both woodcutters settled on the same spot: they are {split:.2} tiles apart, and one of them should have taken \\
         another side of the tree"
    );
}
