//! **What a click on a unit means** (D187). The owner:
//!
//! > *"Some villagers cannot be selected by clicking on them. My expectation is that whenever I click on image representing a unit,
//! > it and only it gets selected. If units are overlapping, the closest one to me gets selected. That also applies when unit is
//! > over the building ... or in front of the building. ... I noticed that some villagers cannot be even chosen by clicking on the
//! > square representing them."*
//!
//! The click was decided **by tile**: the anchor tile, floored, plus the footprint. A body is drawn at `position - 0.5` and stands
//! **on** its point, so a villager at a tile corner is drawn half over each of four tiles while the picker insists it is on one -
//! and the half a player naturally clicks is the half that belongs to the neighbour.
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
        hp: 25,
        resource: None,
        pending: 0,
        production: Default::default(),
        stats: None,
        is_husk: false,
        radius: 0.2,
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
        unit: Some(104),
        ..unit(id, at)
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
    Camera::new((960, 720)).centred_on((10.0, 10.0))
}

/// The pixel a body is drawn at, through the renderer's own conversion.
fn drawn(camera: &Camera, position: (f32, f32)) -> (i32, i32) {
    camera.project(ruage_view::drawn_at(position))
}

fn click(snapshot: &Snapshot, camera: &Camera, pixel: (i32, i32)) -> Vec<u32> {
    let mut selection = ui::Selection::new();
    selection.click(snapshot, camera, pixel);
    selection.ids.clone()
}

/// **A villager standing on a tile corner is selectable on both halves of its body.** Its drawn body straddles four tiles; the
/// picker took only the anchor's, so the left half - the half a player clicks - belonged to the neighbour and selected nothing.
#[test]
fn a_body_drawn_over_two_tiles_is_selectable_on_both() {
    let camera = camera();
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0))]);
    let centre = drawn(&camera, (10.0, 10.0));
    // A point well inside the drawn marker but on the neighbouring tile.
    let half = ruage_view::body_half(&camera);
    for dx in [-half + 1, 0, half - 1] {
        let pixel = (centre.0 + dx, centre.1 - half);
        assert_eq!(
            click(&snapshot, &camera, pixel),
            vec![1],
            "a click at {dx} px from the body's centre (over tile {:?}) must select the villager",
            camera.pick(pixel)
        );
    }
}

/// **Between two overlapping bodies, the one nearer the player wins** - the one drawn on top, which is the one whose `x + y` is
/// greater. Picking by id selected the wrong one about half the time, and always the same wrong one.
#[test]
fn the_body_closest_to_the_viewer_wins() {
    let camera = camera();
    // Two villagers a fifth of a tile apart: their markers overlap almost completely.
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0)), unit(2, (10.2, 10.3))]);
    let front = drawn(&camera, (10.2, 10.3));
    let back = drawn(&camera, (10.0, 10.0));
    // A pixel inside both, taken from the *back* man's own centre: the front body covers it.
    let overlap = ((front.0 + back.0) / 2, back.1 - 2);
    let picked = click(&snapshot, &camera, overlap);
    assert_eq!(
        picked,
        vec![2],
        "the nearer villager must be the one selected at the overlap ({overlap:?})"
    );
}

/// **A villager standing on a farm is the thing clicked, and the farm away from the villager is still the farm.** The building's
/// own ground stays selectable; only the part of it the body covers belongs to the body.
#[test]
fn a_body_over_a_building_wins_where_it_is_drawn() {
    let camera = camera();
    let snapshot = snapshot(vec![
        building(10, (9.0, 9.0), (3, 3)),
        unit(1, (10.5, 10.5)),
    ]);
    let body = drawn(&camera, (10.5, 10.5));
    // On the body: the villager.
    assert_eq!(
        click(&snapshot, &camera, (body.0, body.1 - 2)),
        vec![1],
        "a click on the villager's body, over the farm, must select the villager"
    );
    // Away from the body, still on the farm's footprint: the farm.
    let far = camera.project((11.0, 10.0));
    assert_eq!(
        click(&snapshot, &camera, far),
        vec![10],
        "a click on the farm where nobody stands must select the farm"
    );
}

/// **A click on empty ground clears the selection.** Half a tile from a body is not "a near miss selects it anyway".
#[test]
fn a_click_off_every_body_selects_nothing() {
    let camera = camera();
    let snapshot = snapshot(vec![unit(1, (10.0, 10.0))]);
    let centre = drawn(&camera, (10.0, 10.0));
    let half = ruage_view::body_half(&camera);
    let away = (centre.0 + half + 6, centre.1 - 2);
    assert!(
        click(&snapshot, &camera, away).is_empty(),
        "a click {} px outside the body must select nothing",
        half + 6
    );
}
