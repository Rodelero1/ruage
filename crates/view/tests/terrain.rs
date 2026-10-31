//! Terrain and trees: the ground reads as ground (grass with texture, forest floor) and a forest reads as trees
//! rather than one amber square per node. Written before the implementation (D7, SPEC §11).
//!
//! The rule that decides "is this a tree" carries **no AoE2 knowledge**: a resource node standing on **forest**
//! ground is a tree, because that is what the map says; a node on open ground is a pile of something the map placed
//! there. Nothing in the renderer asks which stockpile column a node fills.

use ruage_sim::EntityKind;
use ruage_view::{Camera, EntityView, Pose, Snapshot, palette, render};

const SIZE: u16 = 16;

/// A snapshot with the given forest tiles, built by hand so the picture is the only thing under test.
fn snapshot(forest: &[(u16, u16)], entities: Vec<EntityView>) -> Snapshot {
    let mut terrain = vec![false; usize::from(SIZE) * usize::from(SIZE)];
    for (x, y) in forest {
        terrain[usize::from(*y) * usize::from(SIZE) + usize::from(*x)] = true;
    }
    Snapshot {
        tick: 0,
        player: 1,
        width: SIZE,
        height: SIZE,
        terrain,
        stock: Vec::new(),
        population: 0,
        population_cap: 0,
        outcome: None,
        visible: vec![u64::MAX; (usize::from(SIZE) * usize::from(SIZE)).div_ceil(64)],
        explored: vec![u64::MAX; (usize::from(SIZE) * usize::from(SIZE)).div_ceil(64)],
        entities,
    }
}

fn node(id: u32, at: (f32, f32), amount: u32) -> EntityView {
    EntityView {
        id,
        player: 0,
        kind: EntityKind::Resource(0),
        unit: None,
        position: at,
        hp_fraction: 0.0,
        pose: Pose::Idle,
        carry: 0,
        construction: None,
        garrison: 0,
        amount,
        pending: 0,
        remembered: false,
        footprint: (1, 1),
    }
}

fn camera() -> Camera {
    // Ten tiles wide at 96-pixel tiles, so tiles 2..14 are all on screen.
    Camera::new((960, 720)).centred_on((8.0, 8.0))
}

fn pixel_at(frame: &ruage_view::Frame, camera: &Camera, tile: (f32, f32)) -> [u8; 4] {
    let (px, py) = camera.project(tile);
    frame.pixel(px as u32, py as u32)
}

/// A node in the forest is a **tree**; the same node on open ground is a **pile**. One map fact, two pictures, and no
/// table of which resource is which.
#[test]
fn a_node_in_the_forest_is_a_tree_and_one_on_grass_is_a_pile() {
    let forest = [(10, 8), (11, 8), (10, 9), (11, 9)];
    let snapshot = snapshot(
        &forest,
        vec![node(1, (4.0, 4.0), 100), node(2, (10.0, 8.0), 100)],
    );
    let camera = camera();
    let frame = render(&snapshot, &camera);

    assert_eq!(
        pixel_at(&frame, &camera, (4.0, 4.0)),
        palette::PILE,
        "a node on open grass was not drawn as a pile"
    );
    assert_eq!(
        pixel_at(&frame, &camera, (10.0, 8.0)),
        palette::CANOPY,
        "a node in the forest was not drawn as a canopy"
    );
}

/// A tree that has been worked out is a **stump**: the entity stays on the map (that is the model), so the picture has
/// to show that it is spent rather than pretend a full canopy is still standing there.
#[test]
fn a_spent_tree_is_a_stump() {
    let forest = [(10, 8)];
    let snapshot = snapshot(&forest, vec![node(2, (10.0, 8.0), 0)]);
    let camera = camera();
    let frame = render(&snapshot, &camera);
    assert_eq!(
        pixel_at(&frame, &camera, (10.0, 8.0)),
        palette::STUMP,
        "an empty node in the forest still reads as a tree"
    );
}

/// Grass is not one flat colour: it has a deterministic tone per tile, so the ground reads as ground. Determinism
/// matters as much as looks - two renders of one snapshot must still be byte-identical.
#[test]
fn grass_has_a_tone_per_tile_and_still_renders_identically() {
    let snapshot = snapshot(&[], Vec::new());
    let camera = camera();
    let first = render(&snapshot, &camera);
    let second = render(&snapshot, &camera);
    assert_eq!(first.to_ppm(), second.to_ppm(), "the tone is not stable");

    let tones: std::collections::BTreeSet<[u8; 4]> = (4i32..12)
        .flat_map(|x| (4i32..12).map(move |y| (x, y)))
        .map(|(x, y)| pixel_at(&first, &camera, (x as f32, y as f32)))
        .collect();
    assert!(
        tones.len() >= 2,
        "every grass tile was the same colour: {} tone(s)",
        tones.len()
    );
    assert!(
        tones.iter().all(|tone| palette::GRASS_TONES.contains(tone)),
        "a grass tile was painted outside the grass palette: {tones:?}"
    );
}

/// Forest floor is **darker than grass**, so a wood reads as a wood from across the screen even before the canopies
/// are drawn - the same reason the original's forest is a solid dark mass.
#[test]
fn forest_floor_is_darker_than_grass() {
    let snapshot = snapshot(&[(12, 12)], Vec::new());
    let camera = camera();
    let frame = render(&snapshot, &camera);
    let grass = pixel_at(&frame, &camera, (4.0, 4.0));
    let forest = pixel_at(&frame, &camera, (12.0, 12.0));

    let luma = |pixel: [u8; 4]| u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]);
    assert!(
        luma(forest) < luma(grass),
        "forest floor ({forest:?}) is not darker than grass ({grass:?})"
    );
}
