//! **A foundation nobody has worked on is drawn as a light-grey footprint** (D183 §1).
//!
//! The owner: *"When you place a building on the ground, it is still walkable over (should be light grey colour like finished
//! farm or part of town center) and you and your allies see it."* The simulation answers *walkable* (D183); this is the other
//! half - the picture says the ground is claimed without saying it is solid.
//!
//! Written before the change (D7, SPEC §11).

use ruage_sim::EntityKind;
use ruage_view::{Camera, EntityView, Mark, Pose, Silhouette, Snapshot, palette, render};

const SIZE: u16 = 16;

fn snapshot(entities: Vec<EntityView>) -> Snapshot {
    Snapshot {
        tick: 0,
        player: 1,
        scale: 20_000,
        width: SIZE,
        height: SIZE,
        terrain: vec![false; usize::from(SIZE) * usize::from(SIZE)],
        stock: Vec::new(),
        population: 0,
        population_cap: 0,
        outcome: None,
        age: None,
        visible: vec![u64::MAX; (usize::from(SIZE) * usize::from(SIZE)).div_ceil(64)],
        explored: vec![u64::MAX; (usize::from(SIZE) * usize::from(SIZE)).div_ceil(64)],
        ring_scale: 1.5,
        entities,
    }
}

/// A 2x2 house at `at`, `construction = Some(fraction)` for a site - **zero** for one nobody has touched.
fn house(id: u32, at: (f32, f32), construction: Option<f32>) -> EntityView {
    EntityView {
        id,
        player: 1,
        kind: EntityKind::Building,
        unit: Some(70),
        position: at,
        hp_fraction: 1.0,
        hp: 550,
        pose: Pose::Idle,
        carry: 0,
        construction,
        garrison: 0,
        amount: 0,
        resource: None,
        pending: 0,
        production: Default::default(),
        stats: None,
        is_husk: false,
        radius: 0.2,
        remembered: false,
        footprint: (2, 2),
        shape: Silhouette::Square,
        mark: Mark::None,
    }
}

fn camera() -> Camera {
    Camera::new((960, 720)).centred_on((8.0, 8.0))
}

fn pixel_at(frame: &ruage_view::Frame, camera: &Camera, tile: (f32, f32)) -> [u8; 4] {
    let (px, py) = camera.project(tile);
    frame.pixel(px as u32, py as u32)
}

/// **The ghost is there, and it is not the building.** A site at `construction = 0` is painted in the foundation grey; the
/// same building once it is finished is painted as a building. Without this the renderer's own rule (`solids_of` asks
/// `tile_walkable`) draws a fresh foundation as *nothing at all*, because nothing is blocked yet - the ghost would be
/// invisible to its own owner.
#[test]
fn a_foundation_nobody_has_worked_on_is_drawn_in_the_foundation_grey() {
    let camera = camera();
    let ghost = house(1, (4.0, 4.0), Some(0.0));
    let finished = house(2, (10.0, 10.0), None);
    let frame = render(&snapshot(vec![ghost, finished]), &camera);

    assert_eq!(
        pixel_at(&frame, &camera, (4.0, 4.0)),
        palette::FOUNDATION,
        "an untouched foundation is not drawn in the foundation grey"
    );
    assert_eq!(
        pixel_at(&frame, &camera, (4.0, 5.0)),
        palette::FOUNDATION,
        "the ghost's second tile is not drawn: the whole footprint is the site"
    );
    assert_eq!(
        pixel_at(&frame, &camera, (10.0, 10.0)),
        palette::BUILDING,
        "a finished house is not drawn as a building"
    );
}

/// **A site that has been started is a building**, not a ghost: the first stroke is the moment the ground turns solid, and
/// the picture has to agree with the engine about which moment that is.
#[test]
fn a_started_site_is_drawn_as_a_building() {
    let camera = camera();
    let frame = render(&snapshot(vec![house(1, (4.0, 4.0), Some(0.01))]), &camera);
    assert_eq!(
        pixel_at(&frame, &camera, (4.0, 4.0)),
        palette::BUILDING,
        "a foundation under construction is still drawn as an untouched ghost"
    );
}
