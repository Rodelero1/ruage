//! **How often the levelled tier actually answers** (D338): a measurement, not a rule.
//!
//! Run it in release, ignored, and read the four numbers:
//! `cargo test --release -p ruage-sim --test tier_probe -- --ignored --nocapture`
//!
//! It exists because D334 found the tier answering **0 of 121** long walks - the gate compared `avoid.is_empty()` and every long
//! walk in a busy match carries a stander - and because a green suite plus an unchanged hash is not evidence that a branch ran.
//! The count comes from the engine's own instrumentation (`Ground::tier_answers`), so this file cannot disagree with the sim.

use std::path::PathBuf;

use ruage_script::{Runner, load_scenario_file};
use ruage_sim::World;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The acceptance match's world, set up exactly as `ruage-headless` sets it up.
#[test]
#[ignore = "a measurement over a full match: run it in release with --ignored"]
fn how_many_routes_the_tier_answers() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs");
    let effects =
        ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).expect("effects");
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs");
    let scenario = load_scenario_file(&repo("config/acceptance.toml")).expect("scenario");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.apply_policy(&scenario.policy);
    world.load_combat(&combat);
    world.load_research(&techs, &effects).expect("research");
    world.load_civs(&civs).expect("civs");
    let rate = world.tick_rate_hz();
    let mut runner = Runner::new(&scenario, rate).expect("runner");

    // **The comparison this probe exists for** (D344): every tier answer is priced against the flat search's answer for the
    // same walk, so "the tier answered" comes with "and its route was this much longer, in this many points".
    world.compare_tier_routes(true);

    for _ in 0..48_000 {
        runner.due(&mut world);
        world.step().expect("step");
    }
    let shadow = world.tier_shadow();
    println!(
        "tier answers {} of {} routes; {} re-paths; hash {:016x}",
        world.tier_answers(),
        world.routes_planned(),
        world.repaths(),
        world.state_hash()
    );
    println!(
        "tier vs flat, same walks: {} walks, tier {} points / {} raw, flat {} points / {} raw, flat missing {}",
        shadow.walks,
        shadow.tier_points,
        shadow.tier_length,
        shadow.flat_points,
        shadow.flat_length,
        shadow.flat_missing
    );
    let funnel = world.tier_funnel();
    println!(
        "funnel: {} long walks, {} with no coarse route, {} one-cell chains",
        funnel.long_walks, funnel.no_coarse, funnel.one_cell
    );
    println!(
        "waypoints: {} raw -> {} smoothed by the game's pass",
        funnel.waypoints_raw, funnel.waypoints_smoothed
    );
    println!(
        "answers by level: {} at level 0, {} coarser",
        funnel.answered_at_level_zero, funnel.answered_coarse
    );
}

/// **What a hopeless level-0 search costs** (D347, OCR's finding). `coarse_route` has no expansion cap and allocates its
/// `closed` bitmap per call; at level 0 that is the whole tile grid, and an unreachable goal exhausts it before the flat
/// search runs. On the acceptance map this fires for 13 of 105 long walks; the number here is what says whether it can
/// matter on a bigger map, where the frontier's insertion sort makes it quadratic in the expansion count.
///
/// `cargo test --release -p ruage-sim --test tier_probe -- --ignored --nocapture hopeless`
#[test]
#[ignore = "a cost measurement: run it in release with --ignored"]
fn a_hopeless_level_zero_search_costs() {
    // A room sealed by walls, with one tree in the start's 2-tile cell so the descent floors at 0.
    let (width, height) = (72usize, 72usize);
    let mut text = format!("schema = 1\n[map]\nwidth = {width}\nheight = {height}\nterrain = [\n");
    for y in 0..height {
        let mut row = String::with_capacity(width);
        for x in 0..width {
            // A wood wall down the whole height at x = 36, and one tree at (8, 41) - the start's own cell.
            let forest = x == 36 || (x == 8 && y == 41);
            row.push(if forest { 'F' } else { 'G' });
        }
        text.push_str(&format!("  \"{row}\",\n"));
    }
    text.push_str("]\n[[map.resource]]\nkind = \"wood\"\namount = 100\nhp = 20\ntiles = [");
    for y in 0..height {
        text.push_str(&format!("[36,{y}],"));
    }
    text.push_str("[8,41],");
    text.push_str("]\n[[map.town_center]]\nplayer = 1\nx = 2\ny = 2\n[[map.town_center]]\nplayer = 2\nx = 68\ny = 68\n");
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let map = ruage_data::load_map_str(&text, "hopeless.toml", &resources).expect("map");
    let mut ground = ruage_sim::Ground::from_map(&map);
    for node in &map.resources {
        ground.block(node.x, node.y);
    }
    let scale = 20_000;
    let from = (
        ruage_fixed::Fixed::from_raw(8 * scale + scale / 2),
        ruage_fixed::Fixed::from_raw(40 * scale + scale / 2),
    );
    // Past the wall to the south, with no way round it: the goal itself is on open ground.
    let to = (
        ruage_fixed::Fixed::from_raw(60 * scale + scale / 2),
        ruage_fixed::Fixed::from_raw(60 * scale + scale / 2),
    );
    let runs = 100;
    let started = std::time::Instant::now();
    let mut answered = 0;
    for _ in 0..runs {
        if ground.route_avoiding(scale, from, to, 4_000, &[]).is_some() {
            answered += 1;
        }
    }
    let each = started.elapsed().as_secs_f64() / f64::from(runs);
    println!(
        "hopeless level-0 walk: {:.3} ms a plan, {answered} of {runs} answered",
        each * 1000.0
    );
}
