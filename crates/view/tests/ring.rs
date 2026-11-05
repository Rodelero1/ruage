//! **The ring under a unit is the unit's own size** (owner, 2026-11-05: *"Units have around legs a circle. Smaller units have
//! smaller circle (infantry, villagers...) and bigger units have bigger circle (cavalry, elephants, siege units, ships...)"*,
//! with the answers: selected units only, the pack's radius times a config factor, drawn as an ellipse on the ground plane).
//!
//! Before this, every selected thing got a ring the size of its **footprint**, and a unit's footprint is one tile - so a
//! villager, a knight and a trebuchet all wore the same 0.9-tile diamond.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_sim::EntityKind;
use ruage_view::{Camera, EntityView, Highlights, Pose, Snapshot, palette, render_with};

fn unit(id: u32, at: (f32, f32), radius: f32) -> EntityView {
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
        hp: 25,
        resource: None,
        pending: 0,
        stats: None,
        is_husk: false,
        radius,
        remembered: false,
        footprint: (1, 1),
        shape: ruage_view::Silhouette::Square,
        mark: ruage_view::Mark::None,
    }
}

fn snapshot(entities: Vec<EntityView>) -> Snapshot {
    Snapshot {
        tick: 0,
        player: 1,
        scale: 20_000,
        width: 32,
        height: 32,
        terrain: vec![false; 32 * 32],
        stock: Vec::new(),
        population: 1,
        population_cap: 5,
        outcome: None,
        age: None,
        visible: vec![u64::MAX; 16],
        explored: vec![u64::MAX; 16],
        ring_scale: 1.5,
        entities,
    }
}

fn camera() -> Camera {
    Camera::new((480, 320)).centred_on((10.0, 10.0))
}

/// How wide the selection ring is, in pixels: the **horizontal extent** of ring-coloured pixels in a band of rows around the
/// unit's feet. An outlined diamond has only two pixels on its widest row, so a run length measures nothing - the distance
/// between the leftmost and rightmost ring pixel is what a player reads as the ring's width.
fn ring_width(snapshot: &Snapshot, at: (f32, f32)) -> u32 {
    let camera = camera();
    let selected = snapshot
        .entities
        .iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    let frame = render_with(
        snapshot,
        &camera,
        &Highlights {
            selected: &selected,
            order: None,
        },
    );
    let centre = camera.project(at);
    let mut left = i32::MAX;
    let mut right = i32::MIN;
    for dy in -30..=30 {
        let y = centre.1 + dy;
        if y < 0 || y as u32 >= frame.height {
            continue;
        }
        for dx in -80..=80 {
            let x = centre.0 + dx;
            if x < 0 || x as u32 >= frame.width {
                continue;
            }
            if frame.pixel(x as u32, y as u32) == palette::SELECTION {
                left = left.min(x);
                right = right.max(x);
            }
        }
    }
    if right < left {
        0
    } else {
        (right - left + 1) as u32
    }
}

#[test]
fn a_bigger_unit_wears_a_bigger_ring() {
    // 0.2 is the villager and every infantryman; 0.5 is a mangonel, a trebuchet or a galley.
    let small = ring_width(&snapshot(vec![unit(1, (10.0, 10.0), 0.2)]), (10.0, 10.0));
    let large = ring_width(&snapshot(vec![unit(1, (10.0, 10.0), 0.5)]), (10.0, 10.0));
    assert!(
        small > 0,
        "a selected unit draws no ring at all: the search found no selection pixels"
    );
    println!("ring widths: 0.2 tiles -> {small} px, 0.5 tiles -> {large} px");
    assert!(
        large > small + 8,
        "a 0.5-tile unit ({large} px) must wear a visibly wider ring than a 0.2-tile one ({small} px)"
    );
}

#[test]
fn the_ring_is_the_packs_radius_times_the_config_factor() {
    // The ellipse's widest row is its horizontal diameter: 2 * radius * ring_scale, in tile half-widths.
    let camera = camera();
    let (rx, _) = camera.radii();
    let radius = 0.2f32;
    let scale = 1.5f32;
    let expected = (2.0 * radius * scale * rx) as u32;
    let measured = ring_width(&snapshot(vec![unit(1, (10.0, 10.0), radius)]), (10.0, 10.0));
    assert!(
        measured.abs_diff(expected) <= 6,
        "the ring is {measured} px wide where 2 * {radius} * {scale} tiles is {expected} px"
    );
}

#[test]
fn an_unselected_unit_wears_none() {
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0), 0.2)]);
    let camera = camera();
    let frame = render_with(
        &snapshot,
        &camera,
        &Highlights {
            selected: &[],
            order: None,
        },
    );
    let centre = camera.project((10.0, 10.0));
    let mut lit = 0;
    for dy in -20..=20 {
        for dx in -60..=60 {
            let (x, y) = (centre.0 + dx, centre.1 + dy);
            if x >= 0
                && y >= 0
                && (x as u32) < frame.width
                && (y as u32) < frame.height
                && frame.pixel(x as u32, y as u32) == palette::SELECTION
            {
                lit += 1;
            }
        }
    }
    assert_eq!(lit, 0, "an unselected unit must not wear a ring");
}
