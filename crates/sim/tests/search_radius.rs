//! **A worker looks as far for its next source as the game says it does** (D175).
//!
//! The dat carries `task_info.search_radius` for every unit: a villager **4.0** tiles, a scout 4.0, a cog 6.0, a fishing ship 12.0, a
//! trebuchet 17-18, and 21 at the widest. That is the number the original bounds its "find another nearest source" search by, and the
//! owner asked for it unit by unit. Packs generated before the field existed carry nothing and keep the old sight-plus-look-around
//! reach, which is why this file tests **both**.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::World;

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn world_with(units: &Units) -> Option<World> {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::present(&repo("config/map.toml"), |path| {
        ruage_data::load_map_file(path, &resources)
    });
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn villager_of(world: &World) -> ruage_sim::EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.unit == Some(83))
        .map(|entity| entity.id)
        .expect("a villager in the opening")
}

/// **With the field**: the worker's reach is the dat's number, in tiles.
#[test]
fn a_villagers_search_reach_is_the_dats_radius() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let mut units = ruage_data::load_units_file(&units_path).expect("units");
    let villager = units.units.get_mut(&83).expect("VMBAS");
    villager.search_radius = ruage_data::MaybeFixed::from_f64(4.0);
    let Some(world) = world_with(&units) else {
        return;
    };
    let scale = world.scale();
    let reach = world.gather_reach(villager_of(&world)) / scale;
    assert_eq!(
        reach, 4,
        "a villager's search radius is 4 tiles in the dat, and the engine looked {reach} instead"
    );
}

/// **Without the field** - every pack generated before D175 - the old reach stands: sight plus a look-around.
#[test]
fn a_pack_without_the_field_keeps_the_old_reach() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let Some(world) = world_with(&units) else {
        return;
    };
    let scale = world.scale();
    let reach = world.gather_reach(villager_of(&world)) / scale;
    let sight = world
        .entities()
        .iter()
        .find(|entity| entity.unit == Some(83))
        .map(|_| {
            units
                .unit(83)
                .and_then(|unit| unit.los.value())
                .unwrap_or(0.0)
        })
        .unwrap_or(0.0);
    assert_eq!(
        reach,
        sight.round() as i64 + 5,
        "without a search radius the reach is the sight plus five tiles"
    );
}
