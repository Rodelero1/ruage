//! **A crowd ordered to one tile all go to that tile.** The owner, 2026-11-19: *"Drop it and let the crowds jam. I want the start
//! as close to the game as sensible. Upgrades may only come later."* D271 gave every follower a ring place of its own, so a crowd
//! ordered to one spot spread itself around it; the original does not - the players' own reports are men piling onto one tree and
//! standing pressed against each other (`docs/gather-seating-evidence.md`).
//!
//! So the claim is the game's: with three men ordered to one tile, **at least two of them end pressed against a friend** - closer
//! than their own tile centres allow - which can only happen when they share the goal. D271's fan-out makes every man a tile of his
//! own, so no two of them can ever be closer than one tile and this fixture could not be written before the cut.
//!
//! Written before the implementation (D7, SPEC §11).

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
  "GGGGGGGGGGFGGGGGGFGGGG",
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
[[map.resource]]
kind = "wood"
amount = 50000
hp = 0
tiles = [[17,10]]
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
        ruage_data::load_map_str(map_text, "crowd.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
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

/// **Three men, one tile: they pile up.** One arrives; the others stand pressed against him, closer than two tile centres ever are.
#[test]
fn a_crowd_ordered_to_one_tile_piles_up() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let goal = (6u16, 14u16);
    // Three tiles west of the goal, in a column: every man has the same distance to walk and a clear lane.
    let mut men = Vec::new();
    for (index, dy) in [-1i64, 0, 1].iter().enumerate() {
        men.push(spawn(
            &mut world,
            (i64::from(goal.0) - 4, i64::from(goal.1) + dy),
            9_200 + index as u32,
        ));
    }
    for man in &men {
        world
            .submit_checked(Command::Move {
                target: None,
                unit: *man,
                to: goal,
            })
            .expect("a walk to the same tile");
    }
    for _ in 0..400 {
        world.step().expect("stepping");
    }

    let scale = world.scale();
    let mut closest = i64::MAX;
    for (index, man) in men.iter().enumerate() {
        for other in men.iter().skip(index + 1) {
            let (a, b) = (
                world.position(*man).expect("alive"),
                world.position(*other).expect("alive"),
            );
            let (dx, dy) = (
                (a.0.raw() - b.0.raw()).pow(2),
                (a.1.raw() - b.1.raw()).pow(2),
            );
            closest = closest.min(dx + dy);
        }
    }
    // Two body radii is 0.4 tiles; half a tile is the allowance for a step and for the press the game's friends do.
    let pressed = (scale / 2) * (scale / 2);
    assert!(
        closest < pressed,
        "the men must end pressed against each other, as the original's own players describe: the closest pair is \
         {:.3} tiles apart",
        (closest as f64).sqrt() / scale as f64
    );
}
