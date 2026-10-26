//! Pathing around terrain. Written before the implementation (D7).
//!
//! Before this, a villager walked in a straight line and could cross a forest. That is not a
//! cosmetic problem: on a map where the trees sit between the base and the wood, the villager
//! walks through the wall and the "camp closer than the town centre" economy becomes fiction.

use std::path::PathBuf;

use ruage_data::{Economy, ResourceKind, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
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
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources: ruage_data::load_resources_file(&repo("config/resources.toml"))
            .expect("resources.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

/// A wall of forest down the middle, with a gap at the bottom, and one tree behind it.
///
///     GGGGFGGG
///     GGGGFGFG   <- the tree at (6,1)
///     GGGGFGGG
///     GGGGFGGG
///     GGGGFGGG
///     GGGGFGGG
///     GGGGGGGG   <- the only way round
///     GGGGGGGG
const WALL_MAP: &str = r#"
schema = 1
[map]
width = 8
height = 8
terrain = [
  "GGGGFGGG",
  "GGGGFGFG",
  "GGGGFGGG",
  "GGGGFGGG",
  "GGGGFGGG",
  "GGGGFGGG",
  "GGGGGGGG",
  "GGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
tiles = [[6, 1]]
[[map.town_centre]]
player = 1
x = 1
y = 1
[[map.town_centre]]
player = 2
x = 7
y = 7
"#;

/// The same wall, but the tree is walled in on every side: no path exists at all.
const SEALED_MAP: &str = r#"
schema = 1
[map]
width = 8
height = 8
terrain = [
  "GGGGGGGG",
  "GGGFFFFG",
  "GGGFFFGG",
  "GGGFFFFG",
  "GGGGGGGG",
  "GGGGGGGG",
  "GGGGGGGG",
  "GGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
tiles = [[4, 2]]
[[map.town_centre]]
player = 1
x = 1
y = 1
[[map.town_centre]]
player = 2
x = 7
y = 7
"#;

fn world_with(fx: &Fixture, map_text: &str) -> (World, ruage_data::Map) {
    let map = ruage_data::load_map_str(map_text, "fixture.toml").expect("fixture map");
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources);
    (world, map)
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Unit && e.player == 1)
        .expect("a villager")
        .id
}

fn wood_node(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
        .expect("a tree")
        .id
}

#[test]
fn a_villager_walks_around_a_forest_wall_and_never_enters_it() {
    let Some(fx) = fixture() else { return };
    let (mut world, map) = world_with(&fx, WALL_MAP);
    assert_eq!(
        map.terrain_at(6, 1),
        Some(ruage_data::Terrain::Forest),
        "the fixture's tree must stand on forest"
    );
    let villager = first_villager(&world);
    let node = wood_node(&world);
    let node_position = world.position(node).expect("position");
    let scale = world.scale();

    world.submit(Command::Gather { villager, node });

    let mut arrived = false;
    for tick in 0..4_000 {
        world.step().expect("step");
        let position = world.position(villager).expect("position");
        let tile = (
            u16::try_from(position.0.raw() / scale).expect("in range"),
            u16::try_from(position.1.raw() / scale).expect("in range"),
        );
        let on_the_node = tile
            == (
                u16::try_from(node_position.0.raw() / scale).unwrap(),
                u16::try_from(node_position.1.raw() / scale).unwrap(),
            );
        assert!(
            on_the_node || map.terrain_at(tile.0, tile.1) != Some(ruage_data::Terrain::Forest),
            "at tick {tick} the villager stood inside a tree at {tile:?}, which is not the node it was sent to"
        );
        if position == node_position {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "the villager must reach the tree by going around the wall, not through it"
    );
}

#[test]
fn the_walk_home_and_the_walk_back_are_paths_too() {
    // The order plans a route; the trip home and the return to work were the easy ones to forget.
    // Without a fresh route the villager walks from the town centre straight at the tree and
    // straight through the wall it went around on the way out.
    let Some(fx) = fixture() else { return };
    let (mut world, map) = world_with(&fx, WALL_MAP);
    let villager = first_villager(&world);
    let node = wood_node(&world);
    let node_position = world.position(node).expect("position");
    let scale = world.scale();
    let node_tile = (
        u16::try_from(node_position.0.raw() / scale).unwrap(),
        u16::try_from(node_position.1.raw() / scale).unwrap(),
    );

    world.submit(Command::Gather { villager, node });

    let mut delivered = false;
    for tick in 0..8_000 {
        world.step().expect("step");
        let position = world.position(villager).expect("position");
        let tile = (
            u16::try_from(position.0.raw() / scale).unwrap(),
            u16::try_from(position.1.raw() / scale).unwrap(),
        );
        assert!(
            tile == node_tile
                || map.terrain_at(tile.0, tile.1) != Some(ruage_data::Terrain::Forest),
            "at tick {tick} the villager stood inside a tree at {tile:?} on the way home or back"
        );
        if world.stock(1, ResourceKind::Wood) > 0 {
            delivered = true;
            break;
        }
    }
    assert!(delivered, "the villager must complete a delivery");
}

#[test]
fn a_node_with_no_path_is_refused_at_order_time() {
    let Some(fx) = fixture() else { return };
    let (mut world, _map) = world_with(&fx, SEALED_MAP);
    let villager = first_villager(&world);
    let node = wood_node(&world);

    match world.submit_checked(Command::Gather { villager, node }) {
        Err(SimError::NoPath { .. }) => {}
        other => panic!("expected NoPath, got {other:?}"),
    }
}
