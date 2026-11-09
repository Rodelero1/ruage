//! **The town centre's yard is painted as yard, and the selection outline sits outside the block** (D161).
//!
//! The owner, with a screenshot of a villager standing on grey: *"I see villager still goes through the town centre. Maybe it is
//! caused by the fact, that the white diamond laying around the building is too small. What if the white diamond was actually a bit
//! larger than the grey diamond … It would look better because no unit passes through the building."*
//!
//! **Both halves of that are the game's own behaviour**: a building's footprint is not all wall - the town centre keeps a walkable
//! yard, which is why units stand "inside" it and always have - and the selection outline belongs *outside* the silhouette, not
//! inside it. Written before the behaviour (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{EntityKind, World};
use ruage_view::{Camera, Highlights, Snapshot, palette, render_with_solids, solids_of, ui};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy"),
        units: ruage_data::load_units_file(&units).expect("units"),
        resources,
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("stockpile");
    world
}

/// The pixel where a tile's diamond is drawn: its own integer point, lowered a little so a role glyph on the block's middle does
/// not cover it.
fn tile_pixel(camera: &Camera, tile: (u16, u16)) -> (u32, u32) {
    let (rx, ry) = camera.radii();
    let (x, y) = camera.project((f32::from(tile.0), f32::from(tile.1)));
    (
        (x as f32 - rx * 0.45).max(0.0) as u32,
        (y as f32 + ry * 0.45).max(0.0) as u32,
    )
}

/// **The yard is not wall.** A town centre's footprint has tiles the engine blocks and tiles it does not, and the picture must say
/// which is which - that difference is exactly what the owner's screenshot was missing.
#[test]
fn the_town_centre_yard_is_painted_lighter_than_its_walls() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let (across, down) = town.footprint;
    assert!(across >= 3, "a town centre is a big building");

    let solids = solids_of(&world, &snapshot);
    let mask = solids.get(&town.id).expect("the town centre has a mask");
    assert_eq!(
        mask.len(),
        (across * down) as usize,
        "one entry per footprint tile"
    );
    assert!(
        mask.iter().any(|solid| *solid) && mask.iter().any(|solid| !solid),
        "the town centre is part solid and part yard: {mask:?}"
    );

    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    let (min_x, min_y, _, _) = ruage_view::footprint_tiles(anchor, town.footprint);
    let camera = Camera::new((900, 700)).centred_on((
        (min_x + across as i32 / 2) as f32,
        (min_y + down as i32 / 2) as f32,
    ));
    let frame = render_with_solids(&snapshot, &camera, &solids);

    let mut saw_wall = false;
    let mut saw_yard = false;
    for dy in 0..down as i32 {
        for dx in 0..across as i32 {
            let tile = ((min_x + dx) as u16, (min_y + dy) as u16);
            let (px, py) = tile_pixel(&camera, tile);
            if px >= 900 || py >= 700 {
                continue;
            }
            let painted = frame.pixel(px, py);
            let solid = mask[(dy * across as i32 + dx) as usize];
            if solid {
                assert_eq!(painted, palette::BUILDING, "the wall at {tile:?} is grey");
                saw_wall = true;
            } else if painted == palette::BUILDING_YARD {
                saw_yard = true;
            }
        }
    }
    assert!(saw_wall, "no wall tile was checked");
    assert!(
        saw_yard,
        "the yard was painted as wall: it is why a villager looked inside"
    );
}

/// **The selection outline is outside the block**, not inside it. The owner: *"what if the white diamond was actually a bit larger
/// than the grey diamond representing a building … it would look better because no unit passes through the building."*
#[test]
fn the_selection_outline_sits_outside_the_walls() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre");
    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    let (min_x, min_y, max_x, max_y) = ruage_view::footprint_tiles(anchor, town.footprint);
    let middle = ((min_x + max_x) as f32 / 2.0, (min_y + max_y) as f32 / 2.0);
    let camera = Camera::new((900, 700)).centred_on(middle);
    let selected = vec![town.id];
    let frame = ruage_view::render_with_solids_and(
        &snapshot,
        &camera,
        &Highlights {
            selected: &selected,
            order: None,
        },
        &solids_of(&world, &snapshot),
    );

    // Straight down the screen from the centre, the grey ends where the footprint's front row ends; the white outline must be
    // **outside** that, so a column of pixels past the last wall pixel is still SELECTION.
    let (cx, cy) = camera.project(middle);
    let (rx, ry) = camera.radii();
    let _ = rx;
    let front = cy + (ry * town.footprint.1 as f32) as i32;
    let mut outline = None;
    for y in (front - 4)..(front + 8) {
        if !(0..700).contains(&y) {
            continue;
        }
        if frame.pixel(cx as u32, y as u32) == palette::SELECTION {
            outline = Some(y);
        }
    }
    let outline = outline.expect("the selection outline is nowhere near the front edge");
    assert!(
        outline >= front,
        "the outline is {} px above the block's front edge: it is drawn inside the building",
        front - outline
    );
    let _ = ui::hud_panel_height();
}
