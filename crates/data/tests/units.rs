//! The generated unit table (`config/gamedata/units.toml`). Written before the implementation
//! (D7).
//!
//! The generated file is gitignored (D8), so the real-file tests skip when it is absent — the
//! same convention as `corpus.rs` and the generator's integration test.

use std::path::PathBuf;

use ruage_data::{Terrain, load_map_file, load_units_file, load_units_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const GOOD: &str = r#"
schema = 1
kind = "units"
civ = 1
civ_name = "British"

[[unit]]
id = 38
name = "KNGHT"
class = 12
hp = 100
los = 4.0
speed = 1.35
carry = 0
work_rate = 0.0
drop_sites = []
garrison = 0
radius = [0.25, 0.25, 0.0]
costs = [[0, 60], [3, 75]]
attacks = [[4, 10]]
armours = [[4, 2], [3, 2]]
range = 0.0
reload = 1.8
accuracy = 100
train_time = 30
base_id = 38
"#;

#[test]
fn the_real_unit_table_loads_and_answers_known_values() {
    let path = repo("config/gamedata/units.toml");
    if !path.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            path.display()
        );
        return;
    }
    let units = load_units_file(&path).expect("units.toml must load");

    assert_eq!(units.civ, 1);
    assert_eq!(units.civ_name, "British");
    assert_eq!(units.units.len(), 1_166, "1,166 populated ids");

    // The knight, as the generator's own integration test pins it.
    let knight = units.unit(38).expect("knight exists");
    assert_eq!(knight.name, "KNGHT");
    assert_eq!(knight.hp, 100);
    assert_eq!(knight.speed.resolve(20_000, "speed").unwrap().raw(), 27_000);
    assert!(knight.armours.contains(&(3, 2)), "2 pierce armour");
    // Three pairs: 60 food, 75 gold, and 1 of resource 4 (population headroom).
    assert_eq!(knight.costs, vec![(0, 60), (3, 75), (4, 1)]);

    // The base villager: 25 hp, 0.8 tiles/s, and its work_rate is the *build* task's 1.0.
    let villager = units.unit(83).expect("VMBAS exists");
    assert_eq!(villager.hp, 25);
    assert_eq!(
        villager.speed.resolve(20_000, "speed").unwrap().raw(),
        16_000
    );
    assert_eq!(
        villager
            .work_rate
            .resolve(20_000, "work_rate")
            .unwrap()
            .raw(),
        20_000,
        "VMBAS work_rate is 1.0 - the trap D11 exists to prevent"
    );

    // The lumberjack variant carries its drop site (D11).
    let lumberjack = units.unit(123).expect("VMLUM exists");
    assert!(lumberjack.drop_sites.contains(&562), "lumber camp");
}

#[test]
fn a_unit_record_carries_everything_the_sim_needs() {
    let units = load_units_str(GOOD, "units.toml").expect("fixture loads");
    let knight = units.unit(38).expect("present");
    assert_eq!(knight.class, 12);
    assert_eq!(knight.attacks, vec![(4, 10)]);
    assert_eq!(knight.accuracy, 100);
    assert_eq!(knight.train_time, 30);
    assert_eq!(
        knight.reload.resolve(20_000, "reload").unwrap().raw(),
        36_000
    );
    assert!(knight.drop_sites.is_empty());
}

#[test]
fn an_unknown_field_is_refused() {
    let bad = GOOD.replace("train_time = 30", "train_time = 30\nbogus = 1");
    let err = load_units_str(&bad, "units.toml")
        .expect_err("unknown fields must be refused")
        .to_string();
    assert!(err.contains("bogus"), "got: {err}");
}

#[test]
fn a_missing_field_is_refused() {
    let bad = GOOD.replace("accuracy = 100\n", "");
    let err = load_units_str(&bad, "units.toml")
        .expect_err("missing fields must be refused")
        .to_string();
    assert!(err.contains("accuracy"), "got: {err}");
}

#[test]
fn a_duplicate_unit_id_is_refused() {
    // Two records for one id would make the table's contents depend on parse order.
    let dup = format!(
        "{GOOD}{}",
        GOOD.split("[[unit]]")
            .nth(1)
            .map(|s| format!("[[unit]]{s}"))
            .unwrap_or_default()
    );
    let err = load_units_str(&dup, "units.toml")
        .expect_err("a duplicate id must be refused")
        .to_string();
    assert!(err.contains("38"), "got: {err}");
}

#[test]
fn the_units_table_and_the_map_agree_on_the_world() {
    // A cheap cross-check that both loaders describe the same 64x64 world: the town centre tiles
    // the map declares are walkable, and the villager's speed resolves at the engine scale.
    let map_path = repo("config/map.toml");
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: generated data absent");
        return;
    }
    let map = load_map_file(&map_path).expect("map.toml loads");
    let units = load_units_file(&units_path).expect("units.toml loads");
    for tc in &map.town_centres {
        assert_eq!(map.terrain_at(tc.x, tc.y), Some(Terrain::Grass));
    }
    let villager = units.unit(83).expect("VMBAS");
    assert!(villager.speed.resolve(20_000, "speed").unwrap().raw() > 0);
}
