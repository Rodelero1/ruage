//! What a frame costs: the window redraws on every simulation tick, so the price of `Snapshot::of` + `render` is the price
//! of the loop. Ignored by default (it is a measurement, not a rule); run with `--release -- --ignored --nocapture`.

use std::path::PathBuf;
use std::time::Instant;

use ruage_view::{Camera, Snapshot, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "measurement, not a rule"]
fn a_frame_costs_less_than_a_tick() {
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.load_combat(&combat);
    let personality = ruage_script::load_personality_file(&repo("config/ai/base.toml"))
        .expect("the base personality loads");
    world.apply_policy(&personality.policy);
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    let camera = Camera::new((960, 720)).centred_on((16.0, 16.0));
    let frames = 100u32;
    let start = Instant::now();
    for _ in 0..frames {
        let snapshot = Snapshot::of(&world, &map, 1);
        let frame = render(&snapshot, &camera);
        std::hint::black_box(&frame);
    }
    let per_frame = start.elapsed().as_micros() as f64 / f64::from(frames);
    println!(
        "frame: {per_frame:.0} us each at 960x720, {} entities",
        world.entities().len()
    );
    assert!(
        per_frame < 50_000.0,
        "a frame should cost far less than a tick"
    );
}

/// Picking is input logic, and input logic is where a renderer is easiest to get subtly wrong: a click that selects the
/// tile beside the one clicked, or the farm instead of the villager on it. No display needed.
#[test]
fn a_click_picks_the_thing_under_it() {
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");

    // Player 1's town centre sits at (16, 16): one tile in is a hit, and its neighbours are not.
    let hit = ruage_view::pick_at(&world, (16, 16));
    assert!(hit.is_some(), "the town centre's tile picks something");
    assert_eq!(
        ruage_view::pick_at(&world, (30, 30)),
        None,
        "open ground picks nothing"
    );
    assert!(
        ruage_view::pick_at(&world, (15, 15)).is_none_or(|id| Some(id) != hit),
        "the tile beside it picks a different thing"
    );
}

/// **The HUD is part of the frame.** It draws a panel, a row per selected thing and a whole minimap every tick - the
/// minimap being a full tile copy - so it is measured with the picture rather than assumed free. Ignored by default like
/// the frame itself: `--release -- --ignored --nocapture`.
#[test]
#[ignore = "measurement, not a rule"]
fn a_hud_costs_less_than_a_tick() {
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.load_combat(&combat);
    let personality = ruage_script::load_personality_file(&repo("config/ai/base.toml"))
        .expect("the base personality loads");
    world.apply_policy(&personality.policy);
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    let camera = Camera::new((960, 720)).centred_on((16.0, 16.0));
    // Five selected units: the panel's maximum, so the row loop runs at its most expensive.
    let selected: Vec<u32> = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == ruage_sim::EntityKind::Unit && entity.player == 1)
        .map(|entity| entity.id.0)
        .take(5)
        .collect();
    let frames = 100u32;

    let plain = Instant::now();
    for _ in 0..frames {
        let snapshot = Snapshot::of(&world, &map, 1);
        let frame = render(&snapshot, &camera);
        std::hint::black_box(&frame);
    }
    let bare = plain.elapsed().as_micros() as f64 / f64::from(frames);

    let with_hud = Instant::now();
    for _ in 0..frames {
        let snapshot = Snapshot::of(&world, &map, 1);
        let mut frame = render(&snapshot, &camera);
        let hud = ruage_view::ui::Hud::of_selected(&snapshot, &resources, &selected);
        ruage_view::ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
        std::hint::black_box(&frame);
    }
    let full = with_hud.elapsed().as_micros() as f64 / f64::from(frames);
    println!(
        "hud: {full:.0} us with it, {bare:.0} us without, {:.0} us of HUD, {} entities, 5 selected",
        full - bare,
        world.entities().len()
    );
    assert!(full < 50_000.0, "a HUD frame should cost less than a tick");
}
