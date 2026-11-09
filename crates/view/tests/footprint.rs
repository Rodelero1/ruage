//! Footprints: a building's silhouette is the dat's own `radius`, and the screen must draw that shape rather
//! than one tile per building. Written before the implementation (D7, SPEC §11).
//!
//! The sizes are **read from the pack** in every assertion here - the expected number is derived from
//! `radius[0]`/`radius[1]` at the run's scale, never typed into the test - so a renderer that keeps drawing one
//! tile (or that hard-codes 4 for the town centre) fails while the content is free to change.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

/// The footprint the pack implies for a unit: `radius_x`/`radius_y` are **half-extents** in tiles, so the
/// tiles across are twice the radius at the run's scale. Derived here rather than assumed, because the whole
/// point of the test is that the number comes from content.
fn footprint_from_pack(fx: &Fixture, scale: i64, id: u32) -> (u32, u32) {
    let unit = fx.units.unit(id).expect("a unit in the pack");
    let across = |radius: ruage_data::MaybeFixed| -> u32 {
        let raw = radius
            .resolve(scale, "radius")
            .expect("a resolved radius")
            .raw();
        let tiles = (raw as f64 / scale as f64 * 2.0).round();
        if tiles >= 1.0 { tiles as u32 } else { 1 }
    };
    (across(unit.radius[0]), across(unit.radius[1]))
}

#[test]
fn the_pack_gives_buildings_three_different_sizes() {
    // A guard on the rest of the file: if content ever gave every building one size, the tests below could
    // pass against a hard-coded number and say nothing.
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let scale = world.scale();
    let town = world.role_unit("town_center").expect("town centre");
    let house = world.role_unit("house").expect("house");
    let barracks = world.role_unit("barracks").expect("barracks");
    let sizes = [
        footprint_from_pack(&fx, scale, town),
        footprint_from_pack(&fx, scale, house),
        footprint_from_pack(&fx, scale, barracks),
    ];
    assert_eq!(sizes[0], (4, 4), "the town centre is 4x4 in the dat");
    assert_eq!(sizes[1], (2, 2), "the house is 2x2 in the dat");
    assert_eq!(sizes[2], (3, 3), "the barracks is 3x3 in the dat");
    assert!(
        sizes.windows(2).all(|pair| pair[0] != pair[1]),
        "three different sizes are what makes the footprint tests bite: {sizes:?}"
    );
}

#[test]
fn the_snapshot_carries_each_entities_footprint_from_the_pack() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let town = world.role_unit("town_center").expect("town centre");

    // A house of the player's own, placed on open ground beside the town centre: a seeded map does not have
    // one, and a footprint nobody draws is a footprint nobody has checked.
    let house = world.role_unit("house").expect("house");
    world
        .submit_checked(Command::Spawn {
            id: ruage_sim::EntityId(9_000),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(20 * scale), Fixed::from_raw(20 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    for _ in 0..2 {
        world.step().expect("step");
    }

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let of = |unit: u32| {
        snapshot
            .entities
            .iter()
            .find(|entity| entity.kind == EntityKind::Building && entity.unit == Some(unit))
            .unwrap_or_else(|| panic!("no building of unit {unit} in view"))
    };

    assert_eq!(
        of(town).footprint,
        footprint_from_pack(&fx, scale, town),
        "the town centre's footprint is not the pack's"
    );
    assert_eq!(
        of(house).footprint,
        footprint_from_pack(&fx, scale, house),
        "the house's footprint is not the pack's"
    );
    assert_ne!(
        of(town).footprint,
        of(house).footprint,
        "a town centre and a house came out the same size"
    );
}

#[test]
fn a_building_is_drawn_as_the_tiles_its_footprint_covers() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let (across, down) = town.footprint;
    assert!(across >= 2, "a one-tile footprint proves nothing");

    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    let (min_x, min_y, max_x, max_y) = ruage_view::footprint_tiles(anchor, town.footprint);
    let middle = ((min_x + max_x) as f32 / 2.0, (min_y + max_y) as f32 / 2.0);
    let camera = Camera::new((640, 480)).centred_on(middle);
    let frame = render(&snapshot, &camera);

    // **One diamond per tile** (D159): the silhouette is exactly the union of the footprint's tiles, so every tile of it is
    // painted at its centre, and the ring around it is not - which is the same set the walk grid blocks.
    // **The tile's own point**, which is where the terrain pass draws that tile's diamond (D160) - a building is drawn on the
    // same points as the ground it covers, so the question "is this tile painted as the building" is asked here.
    let tile_pixel = |x: i32, y: i32| -> [u8; 4] {
        let (rx, ry) = camera.radii();
        let at = camera.project((x as f32, y as f32));
        let (px, py) = (
            (at.0 as f32 - rx * 0.45).max(0.0) as u32,
            (at.1 as f32 - ry * 0.45).max(0.0) as u32,
        );
        frame.pixel(px, py)
    };
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            assert_eq!(
                tile_pixel(x, y),
                palette::BUILDING,
                "the footprint's tile ({x},{y}) is not painted as the building"
            );
        }
    }
    // Units stand on buildings (a town centre's villagers) and are painted last, so a ring tile with a unit on it says nothing
    // either way; every other ring tile must be ground.
    let occupied: Vec<(i32, i32)> = snapshot
        .entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Unit)
        .map(|entity| {
            (
                entity.position.0.floor() as i32,
                entity.position.1.floor() as i32,
            )
        })
        .collect();
    for y in (min_y - 1)..=(max_y + 1) {
        for x in (min_x - 1)..=(max_x + 1) {
            let inside = x >= min_x && x <= max_x && y >= min_y && y <= max_y;
            if inside || x < 0 || y < 0 || occupied.contains(&(x, y)) {
                continue;
            }
            assert_ne!(
                tile_pixel(x, y),
                palette::BUILDING,
                "the tile ({x},{y}) is outside the footprint but painted as the building"
            );
        }
    }
    // And the four corners of the bounding box are outside a **diamond** - painting them would mean the building was drawn as a
    // square, which is the mistake the shape exists to avoid.
    let (cx, cy) = camera.project(middle);
    let half_width = camera.tile_pixels * camera.zoom / 2.0 * across as f32;
    let half_height = camera.tile_pixels * camera.zoom / 4.0 * down as f32;
    let corner = (
        (cx as f32 + half_width * 0.95) as u32,
        (cy as f32 + half_height * 0.95) as u32,
    );
    assert_ne!(
        frame.pixel(corner.0, corner.1),
        palette::BUILDING,
        "the bounding-box corner is painted: the building was drawn as a square"
    );
}
