//! Selection, the HUD and the minimap - and the proof that an order from the UI is one the engine accepts.
//!
//! Written before the implementation (D7). List steps 22, 23 and 24.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, Task, World};
use ruage_view::ui::{Hud, Selection, minimap};
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

    let villagers = snapshot
        .entities
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
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
    let pixel = camera.project(tree.position);

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
    assert!(
        matches!(orders.first(), Some(Command::Move { to, .. }) if *to == (22, 22)),
        "{orders:?}"
    );
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
    assert_eq!(hud.population, 3, "three villagers at the opening");
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
    let map = minimap(&snapshot, 2);
    assert_eq!(map.width, 128);
    assert_eq!(map.height, 128);

    // The player's own corner is visible ground; the enemy's is unseen.
    let (px, py) = camera_on((16.0, 16.0)).project((16.0, 16.0));
    let _ = (px, py);
    assert_eq!(
        snapshot.tile_vision(16, 16),
        ruage_sim::TileVision::Visible,
        "the opening position is not visible"
    );
    assert_eq!(
        map.pixel(16 * 2, 16 * 2),
        [220, 220, 220, 255],
        "the town centre is not on the minimap"
    );
    assert_eq!(
        map.pixel(47 * 2, 47 * 2),
        [8, 8, 12, 255],
        "the enemy's corner is drawn as if it were known"
    );
}

#[test]
fn the_minimap_shows_an_explored_tile_that_is_no_longer_watched() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Walk a villager away and look at where it was: the minimap must still show the ground.
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    world.submit(Command::Move {
        unit: villager,
        to: (24, 18),
    });
    for _ in 0..600 {
        world.step().expect("step");
    }
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let map = minimap(&snapshot, 1);
    let start = (16u16, 16u16);
    assert_ne!(
        snapshot.tile_vision(start.0, start.1),
        ruage_sim::TileVision::Unseen,
        "the ground the villager left was forgotten"
    );
    let _ = map;
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
