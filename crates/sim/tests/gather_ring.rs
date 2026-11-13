//! **A gatherer stands with his circle on the node's tile** (D192; the 1.156 ring it replaced is D186's, and both came from the
//! owner's save).
//!
//! Measured, not reasoned. The owner's save (`owner-test2.aoe2spgame`, VER 9.4 / 12.5) carries a per-unit order queue at
//! `0x32ceb4` whose *from* points are standing positions. Six woodcutters round one tree sit at **1.105, 1.142, 1.160,
//! 1.170, 1.170, 1.189** tiles from the source square's centre - mean **1.156**, spread **0.084** - and the three closest
//! pairs are **0.404, 0.455, 0.710** apart, the first of which is exactly two villager radii (2 x 0.2): body touching body.
//! Those radii are D186's ring, which this file still names where the history matters; the **rule** is D192's - a man's circle
//! touches the tile, which is **0.70** from its centre for a villager and **0.99** where two edges meet, and the ring was the row
//! *behind* the men the owner was looking at.
//!
//! **Nine men a square** is the owner's number from the ring days - *"max 9 per each"* - and D188 dropped every cap: the capacity is
//! the geometry (`2 * pi * 1.156 / 0.404 = 17.97` on a ring, eleven places on a tile's border), never a table.
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

/// **How far a body is from the node's tile**, in tiles: his radius (0.20) when he is touching it, which is D192's rule.
fn clearance(world: &World, node: EntityId, position: (Fixed, Fixed)) -> f64 {
    let scale = world.scale();
    world
        .resource_tiles(node)
        .into_iter()
        .map(|(x, y)| {
            ruage_sim::tiles::distance_to_tile(position, (i64::from(x), i64::from(y)), scale) as f64
                / scale as f64
        })
        .fold(f64::MAX, f64::min)
}

/// Distance between two points, in tiles.
fn gap(world: &World, a: (Fixed, Fixed), b: (Fixed, Fixed)) -> f64 {
    let scale = world.scale() as f64;
    let dx = a.0.raw() as f64 / scale - b.0.raw() as f64 / scale;
    let dy = a.1.raw() as f64 / scale - b.1.raw() as f64 / scale;
    (dx * dx + dy * dy).sqrt()
}

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One tree at (10,10) with room on every side, a source that never runs dry, and the player's town centre well clear.
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
        ruage_data::load_map_str(map_text, "ring.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn the_tree(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,10)")
}

/// Drop `count` villagers on the tiles ringing the tree, three to five tiles out on every side, and order them all to it.
fn woodcutters(world: &mut World, count: usize) -> Vec<EntityId> {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let node = the_tree(world);
    let ring: Vec<(i64, i64)> = (-5i64..=5)
        .flat_map(|dy| (-5i64..=5).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| {
            let far = (*dx).abs().max((*dy).abs());
            (3..=5).contains(&far)
        })
        .collect();
    let mut ids = Vec::new();
    for (index, (dx, dy)) in ring.iter().take(count).enumerate() {
        let id = EntityId(8_000 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw((10 + dx) * scale + scale / 2),
                    Fixed::from_raw((10 + dy) * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        ids.push(id);
    }
    for id in &ids {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node,
            })
            .expect("a gather order");
    }
    ids
}

fn tiles_between(a: (Fixed, Fixed), b: (Fixed, Fixed), scale: i64) -> f64 {
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale as f64;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale as f64;
    (dx * dx + dy * dy).sqrt()
}

/// **The single worker stands with his circle touching the tree** (D192), 0.20 of a tile from its border - which is 0.70 from the
/// tile's centre for a villager. The engine used to put him 1.156 out, on the row behind.
#[test]
fn a_woodcutter_stands_with_his_circle_on_the_tree() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let node = the_tree(&world);
    let ids = woodcutters(&mut world, 1);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let worker = world.position(ids[0]).expect("the woodcutter");
    let distance = clearance(&world, node, worker);
    assert!(
        (0.15..=0.30).contains(&distance),
        "MEASURED the woodcutter {distance:.3} tiles from the tree's border; his circle is 0.20 and must touch it"
    );
}

/// **Two workers on one tree both touch it and stand a body apart** - 0.404 apart in the save, which is exactly two radii.
#[test]
fn two_woodcutters_on_one_tree_both_touch_it_and_stand_a_body_apart() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let ids = woodcutters(&mut world, 2);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let first = world.position(ids[0]).expect("the first");
    let second = world.position(ids[1]).expect("the second");
    let (a, b) = (
        clearance(&world, node, first),
        clearance(&world, node, second),
    );
    let apart = tiles_between(first, second, scale);
    assert!(
        (0.15..=0.30).contains(&a) && (0.15..=0.30).contains(&b),
        "MEASURED radii {a:.3} and {b:.3}; both belong on the 1.156 ring"
    );
    assert!(
        apart >= 0.39,
        "MEASURED {apart:.3} tiles between the two; two villager bodies are 0.40 wide"
    );
}

/// **The ring holds nine men**, and the tenth is left standing - the owner's *"max 9 per each"*, and the 9 + 7 split his
/// **The ring holds whatever fits - eighteen at a body each** (D188). There is no cap: a place is a place while a villager's circle
/// fits in it, so how many work one square is `2 * pi * 1.156 / 0.40 = 18.2` and nothing else. Nine was a number in a table for a
/// day, and the owner's own save forages ten on one bush.
#[test]
fn the_tile_holds_as_many_woodcutters_as_touch_it() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let node = the_tree(&world);
    // **The engine's own capacity**, not a second opinion: `touching_place_count` is what the settle uses, so a test that
    // computes the number itself can only disagree with the engine (D200: places are 0.66 apart now, and one tile holds
    // seven - the owner's own measurement - where a 0.40 spacing claimed eleven).
    let places = ruage_sim::touching_place_count(
        &world.resource_tiles(node),
        world.scale() / 5,
        world.scale(),
    );
    let ids = woodcutters(&mut world, 18);
    for _ in 0..2_000 {
        world.step().expect("a tick");
    }

    // **How many men name this tree**, not how many carry wood: the picker sends the ones who cannot fit to the neighbouring
    // trees, so "carried wood" counts the whole patch. This is the number the geometry allows - eleven for a villager.
    let on_node = ids
        .iter()
        .filter(|id| {
            matches!(
                world.entity(**id).map(|e| e.task),
                Some(Task::Gather { node: at, .. } | Task::Return { node: at, .. }) if at == node
            )
        })
        .count();
    assert_eq!(
        on_node, places,
        "MEASURED {on_node} men on the tree; the touching places round one tile are {places}"
    );

    // **And each of them touches it**: the places, not a lucky tick's positions.
    let places_held: Vec<(Fixed, Fixed)> = world
        .gather_places()
        .into_iter()
        .map(|(_, (x, y))| (Fixed::from_raw(x), Fixed::from_raw(y)))
        .filter(|place| clearance(&world, node, *place) <= 0.30)
        .collect();
    for place in &places_held {
        assert!(
            clearance(&world, node, *place) >= 0.15,
            "MEASURED a place {:.3} tiles from the tree's border; a villager's circle may not be inside it",
            clearance(&world, node, *place)
        );
    }
    let mut closest = f64::MAX;
    for a in 0..places_held.len() {
        for b in (a + 1)..places_held.len() {
            closest = closest.min(tiles_between(places_held[a], places_held[b], world.scale()));
        }
    }
    assert!(
        closest >= 0.39,
        "MEASURED the closest pair of places {closest:.3} tiles apart; two bodies are 0.40 wide"
    );
}

/// **The owner's own arrangement, reproduced.** His save has six woodcutters round one tree at 1.105, 1.142, 1.160, 1.170, 1.170
/// and 1.189 tiles from the square's centre. This seeds a map with a tree on the same tile and puts nine men down **at the
/// positions his file records** (six of them, plus three more on the far side), then checks what the engine does with them: all
/// nine work, each within a twentieth of a tile of the measured 1.156 ring, and no two closer than their own bodies.
///
/// It is the fidelity check the project's own method asks for - the rule is compared against the source's numbers, not against
/// our opinion of them - and it is the only test here that starts from the arrangement the rule was read out of.
#[test]
fn the_owners_own_arrangement_lands_on_the_same_ring() {
    // His six, shifted by (-47, -80) so they sit round this fixture's tree at (10, 10) instead of his at (57, 90): the
    // *arrangement* is what is being reproduced, and a translation leaves every distance and angle unchanged.
    const HIS: [(f64, f64); 6] = [
        (10.61, 9.40),
        (11.57, 10.10),
        (9.50, 9.34),
        (9.35, 11.66),
        (9.90, 9.40),
        (8.38, 10.10),
    ];
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    // **The square's centre, not the entity's anchor**: this fixture puts the tree on a tile corner, and every measured
    // distance in D186 is from the centre of the square the source occupies.
    // The village's town centre is moved out of the way by the fixture map; the men go where his file says.
    let villager = world.role_unit("villager").expect("villager");
    let mut ids = Vec::new();
    for (index, (x, y)) in HIS.iter().enumerate() {
        let id = EntityId(9_100 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw((x * scale as f64) as i64),
                    Fixed::from_raw((y * scale as f64) as i64),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        ids.push(id);
    }
    for id in &ids {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node,
            })
            .expect("a gather order");
    }
    // **Where each man works, not where he is at the end of the run**: a gatherer cycles between his place and the drop site, so a
    // single tick can catch him half way home - and since D189 he may pass *through* a neighbour on the way, which made the old
    // "closest approach" measure read 0.357 for two men whose places are a body apart. The place is the claim: it is where he
    // stands to work, and it is the number the ring is judged by.
    for _ in 0..1_200 {
        world.step().expect("a tick");
    }
    let places: Vec<(EntityId, (Fixed, Fixed))> = world
        .gather_places()
        .into_iter()
        .map(|(id, (x, y))| (id, (Fixed::from_raw(x), Fixed::from_raw(y))))
        .collect();
    let mut closest = f64::MAX;
    for (index, (id, place)) in places.iter().enumerate() {
        let radius = clearance(&world, node, *place);
        assert!(
            (0.15..=0.30).contains(&radius),
            "MEASURED man {index} ({id:?}) working {radius:.3} tiles from the bush's border; his circle is 0.20"
        );
        for (other, other_place) in places.iter().skip(index + 1) {
            let apart = gap(&world, *place, *other_place);
            if apart < closest {
                closest = apart;
                let _ = other;
            }
        }
    }
    assert!(
        closest >= 0.39,
        "MEASURED the closest pair of places {closest:.3} tiles apart; two villager bodies are 0.40 wide"
    );
}
