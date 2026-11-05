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
        hp: 25,
        resource: None,
        pending: 0,
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

/// **A minimap pixel is a tile.** The minimap is a diamond in the corner of the panel; clicking it has to mean the tile under
/// the pointer, and clicking the empty corners of its box - or outside it - has to mean nothing at all.
#[test]
fn a_minimap_pixel_is_a_tile() {
    let (x, y, w, h) = ui::minimap_rect(960, 720, 64, 64);
    let rect = (x, y, w, h);
    // **The top point of the diamond is the map's first tile**, wherever the flat 2:1 diamond puts it in its box.
    let top = ui::minimap_point((0, 0), rect, 64, 64);
    assert_eq!(ui::minimap_tile(top, rect, 64, 64), Some((0, 0)));
    let middle = ui::minimap_point((32, 32), rect, 64, 64);
    let tile =
        ui::minimap_tile(middle, rect, 64, 64).expect("the middle of the map is on the minimap");
    assert!(
        tile.0.abs_diff(32) <= 1 && tile.1.abs_diff(32) <= 1,
        "the middle of the box is {tile:?}, not the middle of the map"
    );
    // A corner of the box is outside the diamond: the panel shows there, and a click on it is not a click on the map.
    assert_eq!(
        ui::minimap_tile((x + 1, y + 1), rect, 64, 64),
        None,
        "the corner of the box read as a tile"
    );
    assert_eq!(
        ui::minimap_tile((x - 4, y - 4), rect, 64, 64),
        None,
        "outside the box is not a tile"
    );
}

/// A resource node: Gaia's, with no role (a map node carries no unit id) and something in it.
fn node(id: u32, at: (f32, f32), slot: usize, amount: u32) -> EntityView {
    EntityView {
        player: 0,
        kind: EntityKind::Resource(slot),
        unit: None,
        amount,
        ..unit(id, at)
    }
}

/// **A tree can be clicked.** The owner's ask: *"trees ... clickable showing hit points and volume of wood they have"*.
/// Selection only ever took the player's own units and buildings, so a node could not be inspected at all - the click
/// landed on nothing and cleared the selection.
#[test]
fn a_click_on_a_tree_selects_it() {
    let camera = camera();
    let snapshot = snapshot(vec![node(11, (10.0, 10.0), 1, 100)]);
    let mut selection = ui::Selection::new();
    selection.click(&snapshot, &camera, pixel(&camera, (10.0, 10.0)));
    assert_eq!(
        selection.ids,
        vec![11],
        "the click did not select the node the player pointed at"
    );
}

/// **A villager standing on a node is what the click meant.** Nodes and bodies share a tile often - a lumberjack works from
/// the tile beside a tree, and a ground node can be walked on - so the order of preference has to be units first, then
/// buildings, then nodes.
#[test]
fn a_unit_on_a_nodes_tile_is_what_the_click_means() {
    let camera = camera();
    let snapshot = snapshot(vec![node(11, (10.0, 10.0), 1, 100), unit(12, (10.0, 10.0))]);
    let mut selection = ui::Selection::new();
    selection.click(&snapshot, &camera, pixel(&camera, (10.0, 10.0)));
    assert_eq!(
        selection.ids,
        vec![12],
        "clicking a villager standing on a node selected the node instead"
    );
}

/// **Somebody else's things are inspectable too** (the owner, 2026-11-04: "enemy hit points on click"), and a node beside
/// them still is. This replaces the earlier rule - which refused an enemy outright - rather than being deleted: the refusal
/// was right until the owner asked for the enemy's numbers.
///
/// What keeps it honest is not the filter here but the snapshot: an enemy is only in it when the player can see them
/// (`Snapshot::of`'s fog rule), so a click cannot reach past the fog.
#[test]
fn a_click_on_an_enemy_selects_it_and_a_node_beside_it_still_selects() {
    let camera = camera();
    let snapshot = snapshot(vec![
        enemy(13, (10.0, 10.0)),
        node(14, (14.0, 10.0), 1, 100),
    ]);
    let mut selection = ui::Selection::new();
    selection.click(&snapshot, &camera, pixel(&camera, (10.0, 10.0)));
    assert_eq!(
        selection.ids,
        vec![13],
        "an enemy's unit was not selectable"
    );
    selection.click(&snapshot, &camera, pixel(&camera, (14.0, 10.0)));
    assert_eq!(selection.ids, vec![14], "a neutral node was not selectable");
}

/// **An enemy building too** - that is the number a player wants before a siege. What it must *not* do is offer its command
/// card; `World::card` refuses a building that is not the player's, and a sim test pins that rather than trusting it.
#[test]
fn a_click_on_an_enemy_building_selects_it() {
    let camera = camera();
    let snapshot = snapshot(vec![EntityView {
        player: 2,
        kind: EntityKind::Building,
        footprint: (2, 2),
        ..unit(21, (10.0, 10.0))
    }]);
    let mut selection = ui::Selection::new();
    selection.click(&snapshot, &camera, pixel(&camera, (11.0, 11.0)));
    assert_eq!(
        selection.ids,
        vec![21],
        "an enemy's building was not selectable"
    );
}

/// **Yours wins a tie.** If one of your units stands where the click lands, the click meant your unit - even when an enemy is
/// inside the same tile, which is a crowd, not a targeting decision.
#[test]
fn an_own_unit_beats_an_enemy_on_the_same_tile() {
    let camera = camera();
    let snapshot = snapshot(vec![enemy(13, (10.0, 10.0)), unit(12, (10.0, 10.0))]);
    let mut selection = ui::Selection::new();
    selection.click(&snapshot, &camera, pixel(&camera, (10.0, 10.0)));
    assert_eq!(
        selection.ids,
        vec![12],
        "the click took the enemy over the player's own unit"
    );
}

/// **A right click is a point, not a tile** (the owner, 2026-11-04: *"I don't want units to walk to the centre of the
/// square. That can walk wherever not blocked."*). The click projects to a continuous world position; rounding it to a tile
/// and passing the *tile* threw that away, so every order - however precisely aimed - ended at the middle of a square.
/// **A husk is a plot** (the owner, 2026-11-04: *"only serves the player who can left click villager, right click on the
/// husk and villager starts planting new farm"*). A villager ordered onto a spent farm of his own side plants a new one there,
/// and the price is the build order's as always - 60 wood, straight out of the pack.
#[test]
fn a_right_click_on_a_husk_plants_a_new_farm() {
    let camera = camera();
    let mut snapshot = snapshot(vec![unit(7, (9.0, 9.0))]);
    let mut husk = building(30, (12.0, 12.0), (3, 3));
    husk.unit = Some(50);
    husk.is_husk = true;
    snapshot.entities.push(husk);
    let mut selection = ui::Selection::new();
    selection.ids.push(7);

    let aimed = pixel(&camera, (12.5, 12.5));
    let tile = camera.pick(aimed);
    let orders = selection.order(&snapshot, &camera, aimed);
    match orders.first() {
        Some(ruage_sim::Command::Build { villager, unit, at }) => {
            assert_eq!(villager.0, 7, "the selected villager plants it");
            assert_eq!(*unit, 50, "the new farm is the farm the husk was");
            assert_eq!(
                *at,
                (tile.0 as u16, tile.1 as u16),
                "on the spent plot, not beside it"
            );
        }
        other => panic!("a click on a husk must plant a farm, not {other:?}"),
    }
}

/// The same click on somebody else's husk is not a build order: his plot is his to replant.
#[test]
fn an_enemy_husk_is_not_ours_to_replant() {
    let camera = camera();
    let mut snapshot = snapshot(vec![unit(7, (9.0, 9.0))]);
    let mut husk = building(30, (12.0, 12.0), (3, 3));
    husk.player = 2;
    husk.unit = Some(50);
    husk.is_husk = true;
    snapshot.entities.push(husk);
    let mut selection = ui::Selection::new();
    selection.ids.push(7);

    let orders = selection.order(&snapshot, &camera, pixel(&camera, (12.5, 12.5)));
    assert!(
        !matches!(orders.first(), Some(ruage_sim::Command::Build { .. })),
        "an enemy's spent farm is not a building site: {orders:?}"
    );
}

#[test]
fn a_right_click_orders_the_point_under_the_cursor() {
    let camera = camera();
    let snapshot = snapshot(vec![unit(7, (9.0, 9.0))]);
    let mut selection = ui::Selection::new();
    selection.ids.push(7);

    // A place inside a tile but nowhere near its middle.
    let aimed = (12.35, 11.8);
    let orders = selection.order(&snapshot, &camera, pixel(&camera, aimed));
    let Some(ruage_sim::Command::WalkTo { to, .. }) = orders.first() else {
        panic!("a click on open ground must order a point, not a tile: {orders:?}");
    };
    let (want_x, want_y) = (aimed.0 * 20_000.0, aimed.1 * 20_000.0);
    assert!(
        (to.0.raw() - want_x as i64).abs() < 600 && (to.1.raw() - want_y as i64).abs() < 600,
        "the order went to ({}, {}) instead of the clicked point ({want_x}, {want_y})",
        to.0.raw(),
        to.1.raw()
    );
}
