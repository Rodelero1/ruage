//! The presentation boundary: snapshot, camera, rasteriser, clock.
//!
//! Written before the implementation (D7). List steps 11-14, which are one crate and one boundary: a
//! read-only view of a match, a flat-ground projection with zoom and scroll, tiles as coloured rectangles
//! with units as dots, and a clock that keeps 20 Hz simulation and free-running frames apart.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, TileVision, World};
use ruage_view::{Camera, Clock, Frame, Pose, Snapshot, render};

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

#[test]
fn the_same_match_renders_the_same_frame() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let camera = Camera::new((320, 240)).centred_on((16.0, 16.0));

    let first = render(&Snapshot::of(&world, &fx.map, 1), &camera);
    let second = render(&Snapshot::of(&world, &fx.map, 1), &camera);

    assert_eq!(first, second, "two renders of one world differ");
    assert_eq!(first.to_ppm(), second.to_ppm());
}

#[test]
fn a_golden_view_survives_a_replay() {
    // The view is a pure function of state, so a match that replays identically must render identically -
    // which is the whole claim the presentation layer makes about determinism (D55).
    let Some(fx) = fixture() else { return };
    let mut a = seeded(&fx);
    let mut b = seeded(&fx);
    for world in [&mut a, &mut b] {
        let villager = world
            .entities()
            .iter()
            .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
            .expect("a villager")
            .id;
        world
            .submit_checked(Command::Move {
                unit: villager,
                to: (22, 18),
            })
            .expect("move");
        for _ in 0..200 {
            world.step().expect("step");
        }
    }
    let camera = Camera::new((200, 160)).centred_on((18.0, 18.0));
    assert_eq!(
        render(&Snapshot::of(&a, &fx.map, 1), &camera).to_ppm(),
        render(&Snapshot::of(&b, &fx.map, 1), &camera).to_ppm(),
        "two identical matches rendered differently"
    );
}

#[test]
fn the_camera_projects_and_picks_back() {
    let mut camera = Camera::new((320, 240));
    camera.zoom = 2.0;
    camera.scroll((3.0, 4.0));
    for tile in [(0.0f32, 0.0f32), (10.0, 7.0), (31.0, 22.0)] {
        let pixel = camera.project(tile);
        assert_eq!(camera.pick(pixel), (tile.0 as i32, tile.1 as i32));
    }
    // Zooming out shows more tiles in the same viewport, and picking still inverts it.
    camera.zoom_by(0.5);
    let pixel = camera.project((10.0, 7.0));
    assert_eq!(camera.pick(pixel), (10, 7));
}

#[test]
fn the_frame_draws_ground_units_and_fog() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    // A camera showing the player's own corner: 16,16 is the town centre on the authored map.
    let camera = Camera::new((192, 192)).centred_on((16.0, 16.0));
    let frame = render(&snapshot, &camera);

    let (px, py) = camera.project((16.0, 16.0));
    let centre = frame.pixel(px as u32 + 2, py as u32 + 2);
    assert_ne!(
        centre,
        [8, 8, 12, 255],
        "the town centre's own tile is drawn as unseen fog"
    );

    // The far corner of the map is outside this player's sight: it is drawn, because a screen has to show
    // something, but it is drawn as fog rather than as ground.
    let mut far = camera;
    far.origin = (47.0, 47.0);
    let f = render(&snapshot, &far);
    assert_eq!(
        f.pixel(4, 4),
        [8, 8, 12, 255],
        "ground the player has never seen was drawn as if it were known"
    );

    // A camera pointed past the edge of the map paints nothing: clipping is the renderer's job, and a
    // tile that does not exist is not a tile.
    let mut off_map = camera;
    off_map.origin = (99.0, 99.0);
    let empty = render(&snapshot, &off_map);
    assert_eq!(empty.pixel(8, 8), [0, 0, 0, 0]);
}

#[test]
fn units_are_dots_in_their_player_s_colour() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager");
    assert_eq!(villager.pose, Pose::Idle, "an idle villager is not idle");
    assert!(villager.hp_fraction > 0.9);

    let camera = Camera::new((192, 192)).centred_on((16.0, 16.0));
    let frame = render(&snapshot, &camera);
    let (px, py) = camera.project(villager.position);
    let dot = frame.pixel(px as u32 + 3, py as u32 + 3);
    assert_ne!(
        dot,
        [0, 0, 0, 0],
        "the villager was not drawn at its own position"
    );
}

#[test]
fn the_snapshot_hides_what_the_player_cannot_see() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let mine = Snapshot::of(&world, &fx.map, 1);
    let theirs = Snapshot::of(&world, &fx.map, 2);

    let enemy_town = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Building)
        .expect("the enemy town centre")
        .id;
    assert!(
        mine.entities.iter().all(|entity| entity.id != enemy_town.0),
        "an enemy building across the map is in the player's view"
    );
    assert!(
        theirs
            .entities
            .iter()
            .any(|entity| entity.id == enemy_town.0),
        "player 2 cannot see their own town centre"
    );
    assert_eq!(mine.tile_vision(47, 47), TileVision::Unseen);
}

#[test]
fn sixty_frames_a_second_do_not_drift() {
    let mut clock = Clock::new(20);
    let frame_ms = 1000.0 / 60.0;
    let mut ticks = 0;
    for _ in 0..600 {
        ticks += clock.ticks_for(frame_ms);
    }
    // Ten seconds at 20 Hz is 200 ticks, and 600 frames of a sixtieth is the same ten seconds.
    assert_eq!(
        ticks, 200,
        "the clock drifted: {ticks} ticks in ten seconds"
    );
    assert!((0.0..=1.0).contains(&clock.alpha()));
}

#[test]
fn a_long_frame_catches_up_but_does_not_replay_the_match() {
    let mut clock = Clock::new(20);
    assert_eq!(clock.ticks_for(50.0), 1, "one tick is 50 ms");
    // A second-long stall must not run twenty ticks in one frame.
    assert!(clock.ticks_for(1000.0) <= 5, "catch-up is unbounded");
    // And the backlog is dropped, not carried: the next frame is a normal one.
    assert_eq!(clock.ticks_for(16.0), 0);
}

#[test]
fn a_frame_is_a_ppm_of_the_expected_size() {
    let frame = Frame::new(4, 3);
    let ppm = frame.to_ppm();
    assert!(ppm.starts_with(b"P6\n4 3\n255\n"));
    assert_eq!(ppm.len(), "P6\n4 3\n255\n".len() + 4 * 3 * 3);
}
