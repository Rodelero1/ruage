//! A throwaway probe: where does a spawned sheep's position change, and when? (D70, the 0.025-tile creep.)
//!
//! Ignored: it prints, it asserts nothing. Run with
//! `cargo test --test sheep_probe -- --include-ignored --nocapture`.

use std::path::PathBuf;

use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "probe: prints positions, asserts nothing"]
fn where_does_the_sheep_move() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml");
    let economy =
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml");
    let units = ruage_data::load_units_file(&units_path).expect("units.toml");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world
        .load_resources(&resources)
        .expect("the pack fits the stockpile");

    let scale = world.scale();
    let sheep_unit = world.role_unit("sheep").expect("sheep role");
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );

    world
        .submit_checked(Command::Spawn {
            id,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(sheep_unit),
            position: (
                ruage_fixed::Fixed::from_raw(centre.0.raw() + 6 * scale),
                ruage_fixed::Fixed::from_raw(centre.1.raw() + 6 * scale),
            ),
            speed: ruage_fixed::Fixed::from_raw(scale / 2),
            hp: 7,
            amount: u32::try_from(100 * scale).expect("fits"),
        })
        .expect("a sheep");

    let mut last = None;
    for tick in 0..600u64 {
        world.step().expect("stepping");
        let now = world.position(id);
        if now != last {
            let task = world
                .entities()
                .iter()
                .find(|entity| entity.id == id)
                .map(|entity| format!("{:?}", entity.task));
            println!("tick {tick:>3}: {now:?}  task={task:?}");
            last = now;
        }
    }
    println!("spawned at centre {centre:?} + 6 tiles; speed {scale} raw per second");
}
