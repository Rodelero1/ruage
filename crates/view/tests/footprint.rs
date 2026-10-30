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
    let town = world.role_unit("town_centre").expect("town centre");
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
    let town = world.role_unit("town_centre").expect("town centre");

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
fn a_building_is_drawn_as_the_diamond_its_footprint_covers() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let (across, down) = town.footprint;
    assert!(
        across >= 2,
        "a building with a one-tile footprint proves nothing"
    );

    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    let (min_x, min_y, max_x, max_y) = ruage_view::footprint_tiles(anchor, town.footprint);
    // The diamond is centred on the ground the footprint covers, not on its anchor tile.
    let middle = ((min_x + max_x) as f32 / 2.0, (min_y + max_y) as f32 / 2.0);
    let camera = Camera::new((640, 480)).centred_on(middle);
    let frame = render(&snapshot, &camera);
    let (cx, cy) = camera.project(middle);
    let px = |x: f32, y: f32| -> [u8; 4] {
        let (x, y) = ((cx as f32 + x) as u32, (cy as f32 + y) as u32);
        frame.pixel(x, y)
    };
    let half_width = camera.tile_pixels * camera.zoom / 2.0 * across as f32;
    let half_height = camera.tile_pixels * camera.zoom / 4.0 * down as f32;

    assert_eq!(
        px(0.0, 0.0),
        palette::BUILDING,
        "the middle of the building is not drawn as a building"
    );
    assert_eq!(
        px(0.0, half_height * 0.6),
        palette::BUILDING,
        "below the centre, still inside a {across}x{down} footprint, is not building"
    );
    assert_eq!(
        px(half_width * 0.6, 0.0),
        palette::BUILDING,
        "to the right of the centre, still inside, is not building"
    );
    assert_ne!(
        px(0.0, half_height * 1.4),
        palette::BUILDING,
        "past the bottom vertex of the diamond is still painted as building"
    );
    assert_ne!(
        px(half_width * 1.4, 0.0),
        palette::BUILDING,
        "past the right vertex of the diamond is still painted as building"
    );

    // The diamond must be on the **ground its footprint covers**, not on its anchor tile: those differ by half a
    // tile for an even footprint, which no shape-based test above can see (a shifted diamond is the same diamond).
    // So measure where the silhouette actually starts along the centre column and the centre row.
    // Furthest building pixel, not the contiguous run: villagers standing inside the town centre are drawn over it
    // (units are painted last), and a silhouette interrupted by its own garrison still ends where the diamond does.
    let reach = half_width.ceil() as i32 + 4;
    let up = (1..reach)
        .filter(|i| frame.pixel(cx as u32, (cy - i) as u32) == palette::BUILDING)
        .max()
        .expect("no building pixel above the centre");
    assert!(
        (up as f32 - half_height).abs() <= 2.0,
        "the top vertex is {up} px above the centre, expected {half_height}: the diamond is not on its footprint"
    );
    let left = (1..reach)
        .filter(|i| frame.pixel((cx - i) as u32, cy as u32) == palette::BUILDING)
        .max()
        .expect("no building pixel left of the centre");
    assert!(
        (left as f32 - half_width).abs() <= 2.0,
        "the left vertex is {left} px left of the centre, expected {half_width}"
    );
    // The corners of the bounding box are *outside* a diamond: painting them would mean the building was drawn as
    // a square, which is the mistake the shape exists to avoid.
    assert_ne!(
        px(half_width * 0.85, half_height * 0.85),
        palette::BUILDING,
        "the diamond's bounding-box corner is painted: the building was drawn as a square"
    );
}

/// Clicking the body of a building selects it, not only the single tile its anchor stands on: the screen shows a
/// diamond several tiles wide, so that is the target the player is aiming at.
#[test]
fn a_click_anywhere_on_a_building_selects_it() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let town = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre")
        .clone();
    let scale = world.scale();
    let anchor = (
        (town.position.0.raw() / scale) as i32,
        (town.position.1.raw() / scale) as i32,
    );
    let footprint = world
        .footprint_of(town.unit.expect("a town centre has a unit id"))
        .expect("a footprint from the pack");
    let (min_x, min_y, max_x, max_y) = ruage_view::footprint_tiles(anchor, footprint);

    let mut on_the_building = 0;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            match ruage_view::pick_at(&world, (x, y)) {
                Some(id) if id == town.id => on_the_building += 1,
                Some(other) => {
                    // A unit standing on a footprint tile outranks the building - the documented tie-break - so the
                    // only other acceptable answer is a unit on that very tile.
                    let entity = world.entity(other).expect("a picked entity");
                    assert_eq!(
                        entity.kind,
                        EntityKind::Unit,
                        "clicking tile ({x},{y}) inside a {}x{} footprint picked something else",
                        footprint.0,
                        footprint.1
                    );
                    assert_eq!(
                        (
                            (entity.position.0.raw() / scale) as i32,
                            (entity.position.1.raw() / scale) as i32
                        ),
                        (x, y),
                        "a unit won the pick from a tile it does not stand on"
                    );
                }
                None => panic!(
                    "clicking tile ({x},{y}) inside a {}x{} footprint hit nothing",
                    footprint.0, footprint.1
                ),
            }
        }
    }
    assert!(
        on_the_building > 0,
        "no tile of the footprint selected the building itself"
    );
    // One tile outside the footprint is not the building.
    for outside in [
        (min_x - 1, anchor.1),
        (max_x + 1, anchor.1),
        (anchor.0, min_y - 1),
        (anchor.0, max_y + 1),
    ] {
        assert_ne!(
            ruage_view::pick_at(&world, outside),
            Some(town.id),
            "clicking {outside:?} outside the footprint still selected the building"
        );
    }
}

/// A diamond field must cover the ground with **no seams**: neighbouring diamonds share an edge exactly, so a
/// screenful of terrain has no unpainted pixels in it. Scanline filling is where this goes wrong - a row that is
/// skipped, or a half-width rounded down at a vertex, leaves a one-pixel crack that reads as fog on screen.
#[test]
fn the_diamond_grid_has_no_seams() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    // Well inside the map, over ground the player can see: 512x384 at 96-pixel tiles covers about ten tiles.
    let camera = Camera::new((512, 384)).centred_on((20.0, 20.0));
    let frame = render(&snapshot, &camera);

    let mut unpainted = 0;
    for y in 64..320 {
        for x in 96..416 {
            if frame.pixel(x, y) == [0, 0, 0, 0] {
                unpainted += 1;
            }
        }
    }
    assert_eq!(
        unpainted, 0,
        "{unpainted} pixels of ground were left unpainted"
    );
}

/// The snapshot says how many units are inside a building. This pins the count before the per-entity scan behind it
/// became a single pass - the old shape was O(n^2) on the frame path, which is invisible at 126 entities and is an
/// army's worth of comparisons twice a second once a HUD draws it.
#[test]
fn the_snapshot_counts_what_is_inside_a_building() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let town = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre")
        .id;
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Unit && entity.player == 1)
        .expect("a villager")
        .id;
    world
        .submit_checked(Command::Garrison {
            unit: villager,
            building: town,
        })
        .expect("a villager may stand inside a town centre");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let building = snapshot
        .entities
        .iter()
        .find(|entity| entity.id == town.0)
        .expect("the town centre is in view");
    assert_eq!(building.garrison, 1, "one villager inside, none counted");
    let inside = snapshot
        .entities
        .iter()
        .find(|entity| entity.id == villager.0)
        .expect("the garrisoned villager");
    assert_eq!(inside.pose, ruage_view::Pose::Garrisoned);
    assert_eq!(inside.garrison, 0, "a unit does not hold units");
}
