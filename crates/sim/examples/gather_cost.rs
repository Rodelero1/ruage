//! **What a crowd of gatherers costs**, with no army and no fight: the counters the acceptance run mixes with combat.
//!
//! Run: `cargo run --release -p ruage-sim --example gather_cost`. Eighteen villagers are sent at six bushes on an open map and
//! left to work for ten game-minutes. Nothing fights, nothing is built, so `repaths` and `made_room` here are the *gathering*
//! share of the same counters the acceptance run reports - which is the only way to price a change to ring places without the
//! match's course moving underneath the measurement.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

const MAP: &str = r#"
schema = 1
[map]
width = 26
height = 18
terrain = [
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
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[12,6]]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[13,7]]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[14,8]]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[11,8]]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[12,9]]
[[map.resource]]
kind = "food"
amount = 600
hp = 600
tiles = [[13,10]]
[[map.town_center]]
player = 1
x = 6
y = 12
[[map.town_center]]
player = 2
x = 22
y = 2
"#;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = root.join("config");
    let resources =
        ruage_data::load_resources_file(&config.join("resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&config.join("sim.toml")).expect("sim");
    let map =
        ruage_data::load_map_str(MAP, "gather_cost.toml", &resources).expect("a map from text");
    let economy = ruage_data::load_economy_file(&config.join("economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&config.join("gamedata/units.toml")).expect("units");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("resources");
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("a villager role");

    let nodes: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect();
    for index in 0..18u32 {
        let id = EntityId(7_000 + index);
        let x = 4 + i64::from(index % 6);
        let y = 14 + i64::from(index / 6);
        world.submit(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(x * scale + scale / 2),
                Fixed::from_raw(y * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        });
    }
    for _ in 0..2 {
        world.step().expect("tick");
    }
    for (index, id) in (7_000..7_018u32).map(EntityId).enumerate() {
        world.submit(Command::Gather {
            villager: id,
            node: nodes[index % nodes.len()],
        });
    }

    let hz = world.tick_rate_hz();
    for _ in 0..(600 * hz) {
        world.step().expect("tick");
    }

    let working = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.task, Task::Gather { .. }))
        .count();
    println!(
        "repaths {} | made_room {} | working {} | entities {}",
        world.repaths(),
        world.made_room(),
        working,
        world.entities().len()
    );
}
