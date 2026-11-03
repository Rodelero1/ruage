//! Picking: a click, a rubber band, a double-click and a minimap pixel.
//!
//! `Selection::click`, `band` and `double_click` were written for the window and **nothing exercised them** - three
//! functions of input logic, which is where a renderer is easiest to get subtly wrong (a band that takes the town centre
//! makes every drag a disaster; a double-click that takes the enemy's units is worse). The minimap had no pick at all: it
//! was drawn and never touched.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_sim::EntityKind;
use ruage_view::{Camera, EntityView, Pose, Snapshot, ui};

fn unit(id: u32, at: (f32, f32)) -> EntityView {
    EntityView {
        id,
        player: 1,
        kind: EntityKind::Unit,
        unit: Some(83),
        position: at,
        hp_fraction: 1.0,
        pose: Pose::Idle,
        carry: 0,
        construction: None,
        garrison: 0,
        amount: 0,
        pending: 0,
        remembered: false,
        footprint: (1, 1),
        shape: ruage_view::Silhouette::Square,
        mark: ruage_view::Mark::None,
    }
}

fn building(id: u32, at: (f32, f32), footprint: (u32, u32)) -> EntityView {
    EntityView {
        kind: EntityKind::Building,
        footprint,
        unit: Some(109),
        ..unit(id, at)
    }
}

fn enemy(id: u32, at: (f32, f32)) -> EntityView {
    EntityView {
        player: 2,
        ..unit(id, at)
    }
}

fn snapshot(entities: Vec<EntityView>) -> Snapshot {
    Snapshot {
        tick: 0,
        player: 1,
        width: 32,
        height: 32,
        terrain: vec![false; 32 * 32],
        stock: Vec::new(),
        population: 1,
        population_cap: 5,
        outcome: None,
        visible: vec![u64::MAX; 16],
        explored: vec![u64::MAX; 16],
        entities,
    }
}

fn camera() -> Camera {
    // Big enough that every fixture tile projects inside the viewport: a band whose far corner falls off the screen is a
    // different question (and was how the first version of this test measured the wrong thing).
    Camera::new((960, 720)).centred_on((10.0, 10.0))
}

/// The pixel for a tile, through the same projection the renderer uses, so a fixture cannot disagree with the picture.
fn pixel(camera: &Camera, tile: (f32, f32)) -> (i32, i32) {
    camera.project(tile)
}

/// **A band takes the player's units inside it, and not a building.** The box is a drag, and a drag that picked up the town
/// centre would make every attempt to box three villagers a disaster. The game does not sweep buildings, so neither does the
/// engine's idea of a box.
#[test]
fn a_band_takes_units_and_not_buildings() {
    let camera = camera();
    let snapshot = snapshot(vec![
        unit(1, (10.0, 10.0)),
        unit(2, (14.0, 10.0)),
        building(3, (10.0, 14.0), (2, 2)),
        enemy(4, (12.0, 10.0)),
    ]);
    let mut selection = ui::Selection::new();
    // A box in **screen** space, and a real rectangle: two tiles on one isometric diagonal project to the same x, so
    // (6,6)..(18,18) is a vertical line - the first version of this test boxed nothing and looked like a bug in `band`.
    let (from, to) = (pixel(&camera, (10.0, 10.0)), pixel(&camera, (14.0, 10.0)));
    assert!(from.0 < to.0 && from.1 < to.1, "the box has to be a box");
    selection.band(&snapshot, &camera, from, to);

    let mut ids = selection.ids.clone();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2], "the band took the wrong things");
}

/// **A double-click takes every unit of that kind on the screen**, not the whole map: the game's rule, and the one a player
/// expects when they double-click a villager to gather.
#[test]
fn a_double_click_takes_the_kind_it_landed_on() {
    let camera = camera();
    let snapshot = snapshot(vec![
        unit(1, (10.0, 10.0)),
        unit(2, (11.0, 10.0)),
        unit(3, (30.0, 30.0)),
        enemy(4, (12.0, 10.0)),
    ]);
    let mut selection = ui::Selection::new();
    selection.double_click(&snapshot, &camera, pixel(&camera, (10.0, 10.0)));

    let mut ids = selection.ids.clone();
    ids.sort_unstable();
    assert_eq!(
        ids,
        vec![1, 2],
        "a double-click took the off-screen unit or the enemy's"
    );
}

/// **A click on a building selects the building, anywhere on its ground.** Clicking the body of a town centre - the part a
/// player actually aims at - has to select it, not only its anchor tile.
#[test]
fn a_click_on_the_middle_of_a_building_selects_it() {
    let camera = camera();
    let snapshot = snapshot(vec![building(7, (10.0, 10.0), (4, 4))]);
    let mut selection = ui::Selection::new();
    // A tile inside the footprint and not its anchor.
    selection.click(&snapshot, &camera, pixel(&camera, (11.5, 11.5)));
    assert_eq!(selection.ids, vec![7]);
}

/// **A minimap pixel is a tile.** The minimap is drawn at two pixels a tile in a box in the corner; clicking it has to mean
/// the tile under the pointer, and clicking outside the box has to mean nothing at all.
#[test]
fn a_minimap_pixel_is_a_tile() {
    let (x, y, w, _h) = ui::minimap_rect(960, 720, 64, 64);
    assert_eq!(ui::minimap_tile((x, y), (x, y, w, w), 64, 64), Some((0, 0)));
    assert_eq!(
        ui::minimap_tile((x + 64, y + 32), (x, y, w, w), 64, 64),
        Some((32, 16)),
        "two pixels a tile: pixel 64 is tile 32"
    );
    assert_eq!(
        ui::minimap_tile((x - 4, y - 4), (x, y, w, w), 64, 64),
        None,
        "outside the box is not a tile"
    );
}
