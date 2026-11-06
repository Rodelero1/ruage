//! **A pack whose geometry cannot be squared is refused at load** (D128). Found by an external review: every range, reach and
//! collision test in the tick is `dx * dx + dy * dy` on raw fixed-point positions, and `i64` has room for 9.2e18 while the
//! product grows with the **square** of `tiles * scale`.
//!
//! The shipped pack is nowhere near the edge - 64 tiles at 20,000 is 3.2e12, seven orders of magnitude of headroom - so this is
//! not a live defect; it is a **latent** one, and the review was right that a finer scale or a bigger map would reach it. The
//! cheapest honest answer is a refusal at load, where the numbers are known, instead of a wrong distance in a match.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_sim::{SimError, World, max_scale_for};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn shipped() -> Option<(
    ruage_data::Sim,
    ruage_data::Map,
    ruage_data::Economy,
    ruage_data::Units,
    ruage_data::Resources,
)> {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::present(&repo("config/map.toml"), |path| {
        ruage_data::load_map_file(path, &resources)
    });
    let economy = common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units = common::must("load_units_file", ruage_data::load_units_file(&units_path));
    Some((settings, map, economy, units, resources))
}

#[test]
fn the_shipped_pack_seeds() {
    let Some((settings, map, economy, units, _)) = shipped() else {
        return;
    };
    let mut world = World::new(&settings, 7).expect("a world");
    world
        .seed_from(&map, &economy, &units)
        .expect("the shipped pack is well inside the i64 limit");
}

/// **A scale too fine for its map is refused, not played.** The threshold is arithmetic, so the test states it: the largest
/// scale a 64-tile map may use is `max_scale_for(64)`, and one past it must fail.
#[test]
fn a_scale_too_fine_for_the_map_is_refused() {
    let Some((mut settings, mut map, economy, units, _)) = shipped() else {
        return;
    };
    let limit = max_scale_for(u64::from(map.width.max(map.height)));
    assert!(
        limit > 20_000,
        "the shipped scale must be under the limit: limit {limit}"
    );

    // One past the limit: the pack cannot be squared in i64 any more.
    settings.fixed_point_scale = limit + 1;
    map.width = 4_000;
    map.height = 4_000;
    let mut world = World::new(&settings, 7).expect("a world");
    match world.seed_from(&map, &economy, &units) {
        Err(SimError::GeometryOverflow { .. }) => {}
        other => panic!("a pack that cannot be squared must be refused, got {other:?}"),
    }
}

/// The bound itself, stated where a reader can check it: the arithmetic is `2 * ((tiles - 1) * scale)^2 <= i64::MAX`.
#[test]
fn the_bound_is_the_arithmetic_it_claims() {
    for tiles in [2u64, 64, 512, u64::from(u16::MAX)] {
        let scale = max_scale_for(tiles) as i128;
        let span = i128::from(tiles - 1);
        let at_limit = 2 * (span * scale) * (span * scale);
        assert!(
            at_limit <= i128::from(i64::MAX),
            "tiles {tiles}: {at_limit} exceeds i64::MAX at the scale the bound allows"
        );
        let past = 2 * (span * (scale + 1)) * (span * (scale + 1));
        assert!(
            past > i128::from(i64::MAX),
            "tiles {tiles}: one past the bound still fits ({past}), so the bound is too strict"
        );
    }
}
