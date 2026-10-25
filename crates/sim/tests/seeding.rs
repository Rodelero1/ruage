//! Seeding a world from the map and the unit table. Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Terrain, Units, load_economy_file, load_map_file};
use ruage_sim::{EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    units: Units,
}

/// Every input a real match needs. `None` when the generated data is absent (D8).
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
        map: load_map_file(&repo("config/map.toml")).expect("map.toml"),
        economy: load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding must succeed");
    world
}

#[test]
fn a_seeded_map_gives_each_player_a_town_centre_and_three_villagers() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);

    assert_eq!(
        world.entities().len(),
        8,
        "2 town centres + 3 villagers each"
    );
    let players: Vec<u8> = world.entities().iter().map(|e| e.player).collect();
    assert_eq!(players, vec![1, 1, 1, 1, 2, 2, 2, 2], "grouped by player");

    for player in [1, 2] {
        let owned: Vec<&ruage_sim::Entity> = world
            .entities()
            .iter()
            .filter(|e| e.player == player)
            .collect();
        let buildings = owned
            .iter()
            .filter(|e| e.kind == EntityKind::Building)
            .count();
        let units = owned.iter().filter(|e| e.kind == EntityKind::Unit).count();
        assert_eq!(buildings, 1, "player {player} has one town centre");
        assert_eq!(units, 3, "player {player} has three villagers");
    }
}

#[test]
fn ids_run_player_by_player_in_role_order() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let ids: Vec<u32> = world.entities().iter().map(|e| e.id.0).collect();
    assert_eq!(
        ids,
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        "ids are assigned, not hashed"
    );
    assert_eq!(world.entities()[0].unit, fx.economy.start.town_centre);
    assert_eq!(world.entities()[1].unit, fx.economy.start.villager);
}

#[test]
fn villagers_carry_the_stats_from_the_unit_table() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let villager = fx.units.unit(fx.economy.start.villager).expect("VMBAS");
    let tc = fx.units.unit(fx.economy.start.town_centre).expect("RTWC");

    for entity in world.entities() {
        let stats = if entity.kind == EntityKind::Building {
            tc
        } else {
            villager
        };
        assert_eq!(entity.hp, stats.hp, "hp comes from the table");
    }

    // 0.8 tiles/s at scale 20000 is 16000 raw; the per-tick delta is that over 20 Hz = 800.
    let unit = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Unit)
        .expect("a villager");
    assert_eq!(
        unit.velocity.0.raw(),
        16_000,
        "velocity is the resolved dat speed"
    );
}

#[test]
fn nobody_starts_in_a_forest() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let scale = fx.settings.fixed_point_scale;
    for entity in world.entities() {
        let x = u16::try_from(entity.position.0.raw() / scale).expect("in range");
        let y = u16::try_from(entity.position.1.raw() / scale).expect("in range");
        assert_ne!(
            fx.map.terrain_at(x, y),
            Some(Terrain::Forest),
            "entity {} at ({x}, {y}) starts inside a forest",
            entity.id.0
        );
    }
}

#[test]
fn villagers_do_not_share_a_tile() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let mut seen = std::collections::BTreeSet::new();
    for entity in world.entities() {
        assert!(
            seen.insert(entity.position),
            "two entities start on the same tile: {:?}",
            entity.position
        );
    }
}

#[test]
fn two_seeds_of_the_same_inputs_hash_the_same() {
    let Some(fx) = fixture() else { return };
    let a = seeded(&fx);
    let b = seeded(&fx);
    assert_eq!(
        a.state_hash(),
        b.state_hash(),
        "seeding must be deterministic"
    );

    // And a different seed changes nothing yet: no draw happens during seeding, which is a
    // property worth pinning - if seeding ever starts rolling dice, this test says so.
    let mut c = World::new(&fx.settings, 99).expect("world");
    c.seed_from(&fx.map, &fx.economy, &fx.units).expect("seed");
    assert_eq!(
        a.state_hash(),
        c.state_hash(),
        "seeding draws no randomness"
    );
}

#[test]
fn a_start_unit_missing_from_the_table_is_an_error() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.start.villager = 4_242;

    let mut world = World::new(&fx.settings, 7).expect("world");
    let err = world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect_err("a unit id that is not in the table must be refused");
    let text = format!("{err}");
    assert!(
        text.contains("4242"),
        "the error should name the id: {text}"
    );
}
