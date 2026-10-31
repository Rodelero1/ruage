//! Selection you can read at a glance: a ring under what is selected, a health bar over it, and a marker where the last
//! order went. Written before the implementation (D7, SPEC §11).
//!
//! It lives **here** rather than in the window binary for one reason: a binary cannot be tested without a display, and
//! "which pixels say selected" is exactly the kind of thing that is wrong for a month and never noticed (D55).

use ruage_sim::EntityKind;
use ruage_view::{Camera, EntityView, Highlights, Pose, Snapshot, palette, render_with};

fn unit(id: u32, at: (f32, f32), hp_fraction: f32) -> EntityView {
    EntityView {
        id,
        player: 1,
        kind: EntityKind::Unit,
        unit: Some(83),
        position: at,
        hp_fraction,
        pose: Pose::Idle,
        carry: 0,
        construction: None,
        garrison: 0,
        amount: 0,
        pending: 0,
        remembered: false,
        footprint: (1, 1),
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
    Camera::new((480, 320)).centred_on((10.0, 10.0))
}

fn count(frame: &ruage_view::Frame, colour: [u8; 4]) -> u32 {
    frame
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| **pixel == colour)
        .count() as u32
}

/// A selected unit is ringed, and an unselected one is not: the whole point of the marker is that it is about *this*
/// unit.
#[test]
fn a_selected_unit_is_ringed_and_an_unselected_one_is_not() {
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0), 1.0), unit(2, (13.0, 10.0), 1.0)]);
    let camera = camera();

    let none = render_with(&snapshot, &camera, &Highlights::default());
    assert_eq!(
        count(&none, palette::SELECTION),
        0,
        "a ring was drawn with nothing selected"
    );

    let selected = render_with(
        &snapshot,
        &camera,
        &Highlights {
            selected: &[1],
            order: None,
        },
    );
    assert!(
        count(&selected, palette::SELECTION) > 8,
        "a selected unit has no ring: {} ring pixels",
        count(&selected, palette::SELECTION)
    );

    // The ring belongs to unit 1, not to unit 2: count the marker pixels near each.
    let near = |at: (i32, i32)| {
        let mut found = 0;
        for dy in -40..40 {
            for dx in -40..40 {
                let (x, y) = (at.0 + dx, at.1 + dy);
                if x >= 0
                    && y >= 0
                    && (x as u32) < selected.width
                    && (y as u32) < selected.height
                    && selected.pixel(x as u32, y as u32) == palette::SELECTION
                {
                    found += 1;
                }
            }
        }
        found
    };
    let (one, two) = (camera.project((10.0, 10.0)), camera.project((13.0, 10.0)));
    assert!(near(one) > 8, "no ring around the selected unit");
    assert_eq!(near(two), 0, "the unselected unit was marked as well");
}

/// The health bar is the fraction it is given: a half-health unit is half green, half red.
#[test]
fn a_health_bar_shows_the_fraction_it_is_given() {
    let full = snapshot(vec![unit(1, (10.0, 10.0), 1.0)]);
    let half = snapshot(vec![unit(1, (10.0, 10.0), 0.5)]);
    let camera = camera();
    let highlights = Highlights {
        selected: &[1],
        order: None,
    };

    let full_frame = render_with(&full, &camera, &highlights);
    let half_frame = render_with(&half, &camera, &highlights);
    let (green_full, red_full) = (
        count(&full_frame, palette::HP_GREEN),
        count(&full_frame, palette::HP_RED),
    );
    let (green_half, red_half) = (
        count(&half_frame, palette::HP_GREEN),
        count(&half_frame, palette::HP_RED),
    );
    assert!(green_full > 0, "a full-health unit has no green bar");
    assert_eq!(red_full, 0, "a full-health unit shows red");
    assert!(
        red_half > 0 && green_half > 0,
        "a wounded unit is not partly red and partly green: {green_half} green, {red_half} red"
    );
    assert!(
        green_half < green_full,
        "the bar did not shorten: {green_half} vs {green_full}"
    );
}

/// An order marker appears where the order went, and nowhere when there is no order.
#[test]
fn an_order_marker_appears_where_the_order_went() {
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0), 1.0)]);
    let camera = camera();
    let without = render_with(&snapshot, &camera, &Highlights::default());
    assert_eq!(count(&without, palette::ORDER), 0, "a marker from nothing");

    let with = render_with(
        &snapshot,
        &camera,
        &Highlights {
            selected: &[],
            order: Some((12.0, 11.0)),
        },
    );
    assert!(
        count(&with, palette::ORDER) > 4,
        "the order marker was not drawn"
    );
    let (px, py) = camera.project((12.0, 11.0));
    assert_eq!(
        with.pixel(px as u32, py as u32),
        palette::ORDER,
        "the marker is not at the tile the order was given for"
    );
}
