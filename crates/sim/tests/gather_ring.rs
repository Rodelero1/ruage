//! **A gatherer stands on a ring 1.156 tiles out, not 0.70** (D186).
//!
//! Measured, not reasoned. The owner's save (`owner-test2.aoe2spgame`, VER 9.4 / 12.5) carries a per-unit order queue at
//! `0x32ceb4` whose *from* points are standing positions. Six woodcutters round one tree sit at **1.105, 1.142, 1.160,
//! 1.170, 1.170, 1.189** tiles from the source square's centre - mean **1.156**, spread **0.084** - and the three closest
//! pairs are **0.404, 0.455, 0.710** apart, the first of which is exactly two villager radii (2 x 0.2): body touching body.
//! **Nine men a square** is the owner's number - *"max 9 per each. 4 on corners and 4 on sides"* - and the file agrees: the
//! sixteen orders over his two adjacent trees came out as **nine and seven**. Eighteen was this reviewer's misreading of the
//! ring's circumference (`2 * pi * 1.156 / 0.404 = 17.97`) as the game's capacity, which the 9 + 7 split refutes.
//!
//! The engine's standoff was `radius[0] + node radius` - 0.2 + 0.5 = **0.70** (D169) - one man per side of the square
//! (D182). That is nearly half a tile inside where the game puts its front row, which is why the owner's photograph and
//! our render do not look alike.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

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

/// **The single worker stands on the measured ring**, 1.156 of a tile from the source square's centre - not the 0.70
/// the two bodies' radii add up to.
#[test]
fn a_woodcutter_stands_on_the_measured_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 1);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let worker = world.position(ids[0]).expect("the woodcutter");
    let distance = tiles_between(worker, tree, scale);
    assert!(
        (1.05..=1.27).contains(&distance),
        "MEASURED the woodcutter at {distance:.3} tiles from the tree; the ring is 1.156 and D169's standoff was 0.70"
    );
}

/// **Two workers on one tree stand a body apart on the same ring** - 0.404 in the save, which is exactly two radii.
#[test]
fn two_woodcutters_on_one_tree_stand_a_body_apart_on_the_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 2);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let first = world.position(ids[0]).expect("the first");
    let second = world.position(ids[1]).expect("the second");
    let (a, b) = (
        tiles_between(first, tree, scale),
        tiles_between(second, tree, scale),
    );
    let apart = tiles_between(first, second, scale);
    assert!(
        (1.05..=1.27).contains(&a) && (1.05..=1.27).contains(&b),
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
fn the_ring_holds_eighteen_woodcutters_at_a_body_each() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 18);
    let mut ever_carried = vec![false; ids.len()];
    for _ in 0..3_000 {
        world.step().expect("a tick");
        for (index, id) in ids.iter().enumerate() {
            if world
                .entity(*id)
                .is_some_and(|entity| entity.carry.raw() > 0)
            {
                ever_carried[index] = true;
            }
        }
    }
    let working = ever_carried.iter().filter(|carried| **carried).count();
    assert_eq!(
        working,
        ruage_sim::ring_capacity(),
        "MEASURED {working} of 18 woodcutters took wood from one tree; the ring's capacity is `2 * pi * 1.156 / 0.40`"
    );

    // **And where they stand is the ring, a body apart**: the places, not a lucky tick's positions, because a man may be walking
    // home with a load or passing through a neighbour on his way back (D189).
    let places: Vec<(Fixed, Fixed)> = world
        .gather_places()
        .into_iter()
        .map(|(_, (x, y))| (Fixed::from_raw(x), Fixed::from_raw(y)))
        .filter(|place| tiles_between(*place, tree, scale) <= 1.27)
        .collect();
    let mut closest = f64::MAX;
    for a in 0..places.len() {
        for b in (a + 1)..places.len() {
            let apart = tiles_between(places[a], places[b], scale);
            closest = closest.min(apart);
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
    let at = world.position(node).expect("the bush");
    let mut closest = f64::MAX;
    for (index, (id, place)) in places.iter().enumerate() {
        let radius = gap(&world, *place, at);
        assert!(
            (1.05..=1.27).contains(&radius),
            "MEASURED man {index} ({id:?}) working at {radius:.3} tiles from the bush; the ring is 1.156"
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
