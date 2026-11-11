//! **A gatherer stands at its work, not half a tile back** (D169).
//!
//! The owner, with two screenshots of the original: *"villagers are standing around 1/2 tile away from what they are doing. In the game they are
//! much closer."* He is right, and the numbers are in the pack: a villager's body radius is **0.2** and a forest tree's **0.5** (both half-extents,
//! straight from the dat), so the two discs touch at **0.7** of a tile. The engine already knows that number - it is `reach_raw`, the sum of the
//! radii - but the **walk** stops wherever the path's last waypoint is, which is the centre of the neighbouring tile, a whole tile away. The gap
//! between those two numbers is the half tile the owner sees.
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

/// One wood block, open ground around it, and the player's town centre far enough that its own footprint is out of the way.
const MAP: &str = r#"
schema = 1
[map]
width = 18
height = 12
terrain = [
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFFFGGGGG",
  "GGGGGGGGGGFFFGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
# A forest tree's hit points, as the pack's own map does it: 20, so it takes ~13 s of chopping to fall.
hp = 20
tiles = [[10,5],[11,5],[12,5],[10,6],[11,6],[12,6]]
[[map.town_center]]
player = 1
x = 2
y = 9
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
        ruage_data::load_map_str(map_text, "standoff.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// The distance between two positions, in tiles.
fn gap(a: (Fixed, Fixed), b: (Fixed, Fixed), scale: i64) -> f64 {
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale as f64;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale as f64;
    (dx * dx + dy * dy).sqrt()
}

/// **The villager ends up on the measured ring** (D186): **1.156** of a tile from the tree's square, which is where six
/// woodcutters in the owner's own save stand (1.105, 1.142, 1.160, 1.170, 1.170, 1.189 - mean 1.156, spread 0.084). D169's
/// 0.70 - the point where the two bodies' radii add up - is nearly half a tile inside it, and the engine stands there no
/// more.
#[test]
fn a_woodcutter_stands_on_the_measured_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    // The tree in the middle of the block, and a villager two tiles **west** of it, on open ground.
    let node = world
        .entities()
        .iter()
        .find(|entity| {
            matches!(entity.kind, EntityKind::Resource(_))
                && entity.position.0.raw() / scale == 10
                && entity.position.1.raw() / scale == 6
        })
        .map(|entity| entity.id)
        .expect("a tree at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_600),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(8 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Gather {
            villager: EntityId(9_600),
            node,
        })
        .expect("a gather order");

    // 200 ticks = 10 s: enough to reach the tree and take several chops, short of the ~13 s a 20 hp tree needs to fall.
    for _ in 0..200 {
        world.step().expect("a tick");
    }

    let here = world.position(EntityId(9_600)).expect("the villager");
    // **Measured from the square's centre**, not the entity's anchor: a node may sit on a tile corner, and the owner's own
    // words are *"each source was a square covering the whole tile"*.
    let square = (
        Fixed::from_raw(10 * scale + scale / 2),
        Fixed::from_raw(6 * scale + scale / 2),
    );
    let apart = gap(here, square, scale);
    assert!(
        (1.05..=1.27).contains(&apart),
        "MEASURED the woodcutter at {apart:.3} tiles from the tree's square; the ring is 1.156 and D169's standoff was 0.70"
    );
    // And it is really working: the tree lost hit points, so this is not a villager that stopped early for another reason.
    let hp = world
        .entities()
        .iter()
        .find(|entity| entity.id == node)
        .map(|entity| entity.hp)
        .unwrap_or(0);
    assert!(
        hp > 0 && hp < 20,
        "the tree is being chopped: {hp} of 20 hit points left"
    );
}
