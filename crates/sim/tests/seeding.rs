//! Seeding a world from the map and the unit table. Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Terrain, Units, load_economy_file, load_map_file};
use ruage_fixed::Fixed;
use ruage_sim::{EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project pack's column for a resource, so a test says "wood" the way content does and the
/// pack is what turns that into a stockpile column (D17).
fn slot(name: &str) -> usize {
    slot_in(
        &ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml"),
        name,
    )
}

fn slot_in(pack: &ruage_data::Resources, name: &str) -> usize {
    pack.slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
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
    // The pack first: the map names its resources and only the pack turns a name into a column.
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding must succeed");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
}

#[test]
fn a_seeded_map_gives_each_player_a_town_centre_and_three_villagers() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);

    let owned_count = world.entities().iter().filter(|e| e.player != 0).count();
    assert_eq!(owned_count, 8, "2 town centres + 3 villagers each");
    let players: Vec<u8> = world
        .entities()
        .iter()
        .filter(|e| e.player != 0)
        .map(|e| e.player)
        .collect();
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
        ids[..8],
        [1, 2, 3, 4, 5, 6, 7, 8],
        "players get ids first, then resource nodes"
    );
    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "the entity list is in ascending id order"
    );
    assert_eq!(world.entities()[0].unit, Some(fx.economy.start.town_centre));
    assert_eq!(world.entities()[1].unit, Some(fx.economy.start.villager));
}

#[test]
fn villagers_carry_the_stats_from_the_unit_table() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let villager = fx.units.unit(fx.economy.start.villager).expect("VMBAS");
    let tc = fx.units.unit(fx.economy.start.town_centre).expect("RTWC");

    for entity in world.entities().iter().filter(|e| e.player != 0) {
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
    assert_eq!(unit.speed.raw(), 16_000, "speed is the resolved dat speed");
    assert_eq!(
        unit.velocity,
        (Fixed::ZERO, Fixed::ZERO),
        "a villager at rest has no motion: speed is a capability, velocity is a movement"
    );
}

#[test]
fn nobody_starts_in_a_forest() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let scale = fx.settings.fixed_point_scale;
    for entity in world.entities().iter().filter(|e| e.player != 0) {
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
fn every_resource_node_becomes_an_entity_carrying_its_amount() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);

    let nodes: Vec<_> = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(_)))
        .collect();
    assert_eq!(
        nodes.len(),
        fx.map.resources.len(),
        "one entity per map node"
    );

    // Nodes hold their remaining amount in fixed point, so totals are compared in whole units.
    let scale = fx.settings.fixed_point_scale;
    let total = |slot: usize| -> i64 {
        nodes
            .iter()
            .filter(|e| e.kind == EntityKind::Resource(slot))
            .map(|e| e.amount.raw())
            .sum::<i64>()
            / scale
    };
    // The map says 90 wood tiles at 100, 12 food at 150, 8 gold at 400, 8 stone at 300.
    assert_eq!(total(slot("wood")), 9_000);
    assert_eq!(total(slot("food")), 1_800);
    assert_eq!(total(slot("gold")), 3_200);
    assert_eq!(total(slot("stone")), 2_400);
}

#[test]
fn a_resource_node_has_no_unit_id() {
    // Unit 0 is a real dat unit, so `unit: 0` would read as a genuine id and hand a tree the
    // stats of an academy. Absence has to be representable.
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    for node in world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(_)))
    {
        assert_eq!(node.unit, None, "node {} claims a unit id", node.id.0);
    }
    for entity in world.entities().iter().filter(|e| e.player != 0) {
        assert!(entity.unit.is_some(), "a player entity must have a unit id");
    }
}

#[test]
fn a_node_stands_on_the_tile_the_map_put_it_on() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let scale = fx.settings.fixed_point_scale;

    for node in &fx.map.resources {
        let expected = (
            ruage_fixed::Fixed::from_raw(i64::from(node.x) * scale),
            ruage_fixed::Fixed::from_raw(i64::from(node.y) * scale),
        );
        let entity = world
            .entities()
            .iter()
            .find(|e| e.position == expected)
            .unwrap_or_else(|| panic!("no entity at the map node ({}, {})", node.x, node.y));
        assert_eq!(entity.kind, EntityKind::Resource(node.slot));
        assert_eq!(
            entity.amount.raw(),
            i64::from(node.amount) * scale,
            "the node's amount is the map's, in fixed point"
        );
    }
}

#[test]
fn a_villager_never_starts_inside_a_resource_node() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let mut node_tiles = std::collections::BTreeSet::new();
    for node in &fx.map.resources {
        node_tiles.insert((node.x, node.y));
    }
    let scale = fx.settings.fixed_point_scale;
    for entity in world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Unit)
    {
        let x = u16::try_from(entity.position.0.raw() / scale).expect("in range");
        let y = u16::try_from(entity.position.1.raw() / scale).expect("in range");
        assert!(
            !node_tiles.contains(&(x, y)),
            "villager {} starts inside a resource node at ({x}, {y})",
            entity.id.0
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
    c.load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
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

#[test]
fn an_idle_villager_stands_still() {
    // `velocity` is a mover's current motion: the movement test hook uses it to walk a nameless
    // entity. A real unit at rest has no motion - its *speed* is a capability, not a direction.
    // Seeding used to set velocity to the villager's speed, so every idle villager marched east
    // forever, invisible in gather tests because they are given a task immediately.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let before: Vec<_> = world
        .entities()
        .iter()
        .filter(|e| e.player == 1)
        .map(|e| (e.id, e.position))
        .collect();
    for _ in 0..200 {
        world.step().expect("step");
    }
    for (id, position) in before {
        let now = world.entity(id).expect("still alive").position;
        assert_eq!(now, position, "entity {} drifted while idle", id.0);
    }
}

#[test]
fn eight_sides_are_representable_and_a_ninth_is_refused() {
    // D20: 4v4 is the ceiling, so stockpiles are eight wide. This test exists because the width
    // used to be hardcoded to two in the type, and a silent ninth player would have panicked on
    // the array rather than being refused.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let at = |x: i64, y: i64| (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale));

    for player in 1..=8u8 {
        world
            .submit_checked(ruage_sim::Command::Spawn {
                id: ruage_sim::EntityId(900 + u32::from(player)),
                player,
                kind: EntityKind::Unit,
                unit: Some(83),
                position: at(30 + i64::from(player), 30),
                speed: Fixed::ZERO,
                hp: 25,
                amount: 0,
            })
            .unwrap_or_else(|err| panic!("player {player} must be playable: {err}"));
    }
    assert_eq!(
        world.stock(8, slot("wood")),
        0,
        "the eighth side has a stockpile, empty"
    );

    let err = world
        .submit_checked(ruage_sim::Command::Spawn {
            id: ruage_sim::EntityId(999),
            player: 9,
            kind: EntityKind::Unit,
            unit: Some(83),
            position: at(40, 40),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect_err("a ninth side has no stockpile and no place");
    assert!(
        err.to_string().contains("outside 1..=8"),
        "the refusal must name the bound, got: {err}"
    );
}
