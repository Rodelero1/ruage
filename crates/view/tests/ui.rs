//! Selection, the HUD and the minimap - and the proof that an order from the UI is one the engine accepts.
//!
//! Written before the implementation (D7). List steps 22, 23 and 24.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, Task, World};
use ruage_view::ui::{self, Hud, Selection};
use ruage_view::{Camera, Snapshot, render};

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

fn camera_on(tile: (f32, f32)) -> Camera {
    Camera::new((320, 240)).centred_on(tile)
}

#[test]
fn a_click_selects_the_unit_under_the_cursor() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));

    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    let pixel = camera.project(villager.position);

    let mut selection = Selection::new();
    selection.click(&snapshot, &camera, (pixel.0 + 2, pixel.1 + 2));
    assert_eq!(selection.ids, vec![villager.id]);

    // And a click on a *building* selects the building, not "the first unit on the map" - the check that
    // makes the tile matter.
    let centre = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("the town centre");
    let pixel = camera.project(centre.position);
    selection.click(&snapshot, &camera, (pixel.0 + 2, pixel.1 + 2));
    assert_eq!(
        selection.ids,
        vec![centre.id],
        "a click on a building missed it"
    );
}

#[test]
fn a_click_on_empty_ground_clears_the_selection() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));
    let mut selection = Selection::new();
    selection.ids = vec![1, 2, 3];
    // The corner of the visible area is grass with nothing on it.
    selection.click(&snapshot, &camera, (2, 2));
    assert!(selection.ids.is_empty(), "empty ground kept a selection");
}

#[test]
fn a_rubber_band_takes_units_and_leaves_buildings() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));
    let mut selection = Selection::new();
    selection.band(&snapshot, &camera, (0, 0), (320, 240));

    assert!(!selection.ids.is_empty(), "the band selected nothing");
    for id in &selection.ids {
        let entity = snapshot
            .entities
            .iter()
            .find(|e| e.id == *id)
            .expect("selected");
        assert_eq!(entity.kind, EntityKind::Unit, "a box swept up a building");
        assert_eq!(entity.player, snapshot.player);
    }
}

#[test]
fn a_double_click_takes_the_whole_kind() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));
    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    let pixel = camera.project(villager.position);
    let mut selection = Selection::new();
    selection.double_click(&snapshot, &camera, (pixel.0 + 2, pixel.1 + 2));

    // **Same kind**, which since the opening scout is not "every unit the player owns": the player has a horseman too, and
    // double-clicking a villager must not take him.
    let villagers = snapshot
        .entities
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .filter(|entity| entity.unit == villager.unit)
        .count();
    assert_eq!(
        selection.ids.len(),
        villagers,
        "a double click took one villager"
    );
}

#[test]
fn a_right_click_on_a_tree_issues_a_gather_order_the_engine_accepts() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));

    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    let tree = snapshot
        .entities
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .expect("a resource node");
    // The tree is a body at the centre of its tile, so the pixel that shows it is `drawn_at`'s conversion (D168/D172).
    let pixel = camera.project(ruage_view::drawn_at(tree.position));

    let mut selection = Selection::new();
    selection.ids = vec![villager.id];
    let orders = selection.order(&snapshot, &camera, (pixel.0 + 1, pixel.1 + 1));
    assert_eq!(orders.len(), 1, "one selected villager, {orders:?}");
    assert!(
        matches!(orders[0], Command::Gather { .. }),
        "{:?}",
        orders[0]
    );

    for order in orders {
        world
            .submit_checked(order)
            .expect("the UI issued an order the engine refused");
    }
    world.step().expect("step");
    let task = world
        .entity(ruage_sim::EntityId(villager.id))
        .map(|e| e.task);
    assert!(
        matches!(task, Some(Task::Gather { .. }) | Some(Task::Move { .. })),
        "the villager is not doing what it was told: {task:?}"
    );
}

#[test]
fn a_right_click_on_the_ground_is_a_move() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let camera = camera_on((16.0, 16.0));
    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    let empty_tile = (22i32, 22i32);
    let pixel = camera.project((empty_tile.0 as f32, empty_tile.1 as f32));
    let mut selection = Selection::new();
    selection.ids = vec![villager.id];

    let orders = selection.order(&snapshot, &camera, (pixel.0 + 1, pixel.1 + 1));
    // **A point inside the clicked tile**, not the tile itself (the owner, 2026-11-04: units should walk wherever is not
    // blocked, not to the middle of the square). The invariant this test owns is that the ground under the cursor is what
    // the order names; that it is a *point* rather than a tile is `picking.rs`'s
    // `a_right_click_orders_the_point_under_the_cursor`.
    let scale = snapshot.scale;
    match orders.first() {
        Some(Command::WalkTo { to, .. }) => {
            let tile = (to.0.raw().div_euclid(scale), to.1.raw().div_euclid(scale));
            assert_eq!(
                tile,
                (i64::from(empty_tile.0), i64::from(empty_tile.1)),
                "the order landed on tile {tile:?}, not the ground that was clicked"
            );
        }
        other => panic!("a right click on open ground must order a walk: {other:?}"),
    }
    for order in orders {
        world.submit_checked(order).expect("a move order");
    }
}

#[test]
fn the_hud_names_the_resources_the_pack_has() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let mut selection = Selection::new();
    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    selection.ids = vec![villager.id];

    let hud = Hud::of(&snapshot, &fx.resources, &selection);
    let names: Vec<&str> = hud.stock.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"food"), "{names:?}");
    assert!(names.contains(&"wood"), "{names:?}");
    // Four: three villagers and the opening scout, which is a population slot the player has spent (2026-11-04).
    assert_eq!(
        hud.population, 4,
        "three villagers and a scout at the opening"
    );
    assert_eq!(hud.population_cap, 5, "a town centre supports five");
    assert_eq!(hud.selected.len(), 1);
    assert_eq!(hud.selected[0].id, villager.id);
    assert!(hud.outcome.is_none());
}

#[test]
fn the_minimap_draws_fog_terrain_and_the_player_s_units() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);

    // **What the minimap says is what the fog says**, asked at the tile the diamond draws: visible ground where the player
    // stands, unseen ground at the enemy's corner. The square minimap this replaced drew the same colours in a grid; the
    // owner asked for the diamond (2026-11-04) and the colours are the part that must not change with the shape.
    assert_eq!(
        snapshot.tile_vision(16, 16),
        ruage_sim::TileVision::Visible,
        "the opening position is not visible"
    );
    assert_eq!(
        snapshot.tile_vision(47, 47),
        ruage_sim::TileVision::Unseen,
        "the enemy's opening position is visible without a scout reaching it"
    );

    let mut frame = ruage_view::Frame::new(960, 720);
    let hud = ui::Hud::of(&snapshot, &fx.resources, &ui::Selection::new());
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    let rect = ui::minimap_rect(960, 720, snapshot.width, snapshot.height);
    let point = ui::minimap_point((16, 16), rect, snapshot.width, snapshot.height);
    let here = frame.pixel(point.0 as u32, point.1 as u32);
    let enemy = ui::minimap_point((47, 47), rect, snapshot.width, snapshot.height);
    let there = frame.pixel(enemy.0 as u32, enemy.1 as u32);
    assert_ne!(
        here, there,
        "the player's ground and the enemy's look the same"
    );
    assert!(
        there[0] < 20 && there[1] < 20,
        "unseen ground is not dark on the minimap: {there:?}"
    );
}

#[test]
fn the_hud_shows_a_queue() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("the town centre");
    let villager = fx.economy.role["villager"];
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager,
        })
        .expect("train");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let mut selection = Selection::new();
    selection.ids = vec![centre.0];
    let hud = Hud::of(&snapshot, &fx.resources, &selection);
    assert_eq!(hud.selected[0].pending, 1, "the HUD cannot see the queue");
    let _ = render(&snapshot, &camera_on((16.0, 16.0)));
}

/// A click on the **body** of a building selects it. The window's own picking (`ruage_view::pick_at`) learned this
/// when footprints landed; `Selection` kept the anchor-tile rule, and two rules for one click is how a player ends up
/// clicking a town centre and selecting nothing.
#[test]
fn a_click_on_the_body_of_a_building_selects_it() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let town = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("the town centre");
    let (across, down) = town.footprint;
    let anchor = (
        town.position.0.floor() as i32,
        town.position.1.floor() as i32,
    );
    // The rectangle comes from the one function that defines a footprint, not from arithmetic repeated here: repeating it
    // is exactly how the two rules for one click drifted apart (D84).
    let (min_x, min_y, max_x, max_y) = ruage_view::footprint_tiles(anchor, town.footprint);
    let camera = camera_on(town.position);

    // Every tile of the footprint that no unit stands on: selecting the town centre must not depend on which tile of
    // its body a player happens to hit.
    let mut checked = 0;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let tile = (x, y);
            let occupied = snapshot.entities.iter().any(|entity| {
                entity.kind == EntityKind::Unit
                    && (
                        entity.position.0.floor() as i32,
                        entity.position.1.floor() as i32,
                    ) == tile
            });
            if occupied {
                continue;
            }
            let pixel = camera.project((tile.0 as f32, tile.1 as f32));
            let mut selection = Selection::new();
            selection.click(&snapshot, &camera, pixel);
            assert_eq!(
                selection.ids,
                vec![town.id],
                "clicking tile {tile:?} on a {across}x{down} building selected nothing"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 2,
        "no unobstructed tile of the building was clicked"
    );
}
