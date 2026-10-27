//! The generated unit table (`config/gamedata/units.toml`). Written before the implementation
//! (D7).
//!
//! The generated file is gitignored (D8), so the real-file tests skip when it is absent — the
//! same convention as `corpus.rs` and the generator's integration test.

use std::path::PathBuf;

use ruage_data::{CostResource, Terrain, load_map_file, load_units_file, load_units_str};

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
    // Two pairs: 60 food and 75 gold. The dat's third pair for a knight is 1 of resource 4, which
    // is population - a cost the stockpile cannot hold, so the importer does not emit it as one
    // (see `test_population_is_not_a_stockpile_cost` in the tooling's tests).
    assert_eq!(
        knight.costs,
        vec![
            ruage_data::Cost {
                resource: CostResource::ReferenceId(0),
                amount: 60
            },
            ruage_data::Cost {
                resource: CostResource::ReferenceId(3),
                amount: 75
            },
        ]
    );

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

    // `train_time` is the dat's `create_time`: for a unit it is how long it takes to train, for a
    // building it is how long one villager takes to build it. These are published build times -
    // house 25 s, barracks 50 s, farm 15 s - which is what makes the field usable as a build time
    // at all. If the generator ever starts emitting something else, this test says so.
    assert_eq!(
        units.unit(70).expect("HOUS").train_time,
        25,
        "house build time"
    );
    assert_eq!(
        units.unit(12).expect("BRKS").train_time,
        50,
        "barracks build time"
    );
    assert_eq!(
        units.unit(50).expect("FARM").train_time,
        15,
        "farm build time"
    );
    assert_eq!(
        units.unit(68).expect("MILL").train_time,
        35,
        "mill build time"
    );

    // The lumberjack variant carries its drop site (D11), and every drop site resolves - the
    // loader enforces that, so this test doubles as proof it holds for the generated table.
    let lumberjack = units.unit(123).expect("VMLUM exists");
    assert!(lumberjack.drop_sites.contains(&562), "lumber camp");
    let carriers = units
        .units
        .values()
        .filter(|u| !u.drop_sites.is_empty())
        .count();
    assert_eq!(carriers, 46, "46 units carry drop sites");
    for unit in units.units.values() {
        for site in &unit.drop_sites {
            assert!(
                units.unit(*site).is_some(),
                "unit {} names missing drop site {site}",
                unit.id
            );
        }
    }
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
fn a_drop_site_that_is_not_in_the_table_is_refused() {
    // A drop site is a building the villager walks to and deposits in. A dangling id is a
    // villager who can carry wood forever and never deposit it - the kind of bug that shows up
    // ten minutes into a game rather than at startup.
    let bad = GOOD.replace("drop_sites = []", "drop_sites = [562]");
    let err = load_units_str(&bad, "units.toml")
        .expect_err("a dangling drop site must be refused")
        .to_string();
    assert!(
        err.contains("562"),
        "the error should name the id, got: {err}"
    );
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
    let pack = ruage_data::load_resources_file(&repo("config/resources.toml"))
        .expect("resources.toml loads");
    let map = load_map_file(&map_path, &pack).expect("map.toml loads");
    let units = load_units_file(&units_path).expect("units.toml loads");
    for tc in &map.town_centres {
        assert_eq!(map.terrain_at(tc.x, tc.y), Some(Terrain::Grass));
    }
    let villager = units.unit(83).expect("VMBAS");
    assert!(villager.speed.resolve(20_000, "speed").unwrap().raw() > 0);
}

/// A pack of our own names what a unit costs instead of quoting the reference pack's numbering
/// (D17). Without this the engine could only ever price units from AoE2's own resource ids, so an
/// original content pack could not make a building cost anything at all.
#[test]
fn a_cost_may_name_its_resource_instead_of_quoting_a_reference_id() {
    let text = GOOD.replace(
        "costs = [[0, 60], [3, 75]]",
        "costs = [{ resource = \"timber\", amount = 25 }]",
    );
    let units = load_units_str(&text, "units.toml").expect("a named cost is legal content");
    let knight = units.unit(38).expect("the unit");
    assert_eq!(knight.costs.len(), 1);
    assert_eq!(
        knight.costs[0].resource,
        CostResource::Name("timber".to_owned()),
        "the name is the pack's to define"
    );
    assert_eq!(knight.costs[0].amount, 25);
}

/// An age-up replaces a building's unit id with a per-age variant (town centre 109 -> 71, barracks
/// 12 -> 498). Everything that names a building by id has to keep matching after that, so the pack
/// carries the relation and the engine resolves a variant to the unit it came from.
#[test]
fn a_variant_resolves_to_the_unit_it_was_upgraded_from() {
    let path = repo("config/gamedata/units.toml");
    if !path.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            path.display()
        );
        return;
    }
    let units = load_units_file(&path).expect("units.toml");
    assert_eq!(units.root(109), 109, "the town centre is its own root");
    assert_eq!(
        units.root(71),
        109,
        "the feudal town centre is a variant of the dark age one"
    );
    assert_eq!(units.root(12), 12);
    assert_eq!(units.root(498), 12, "the feudal barracks is a variant too");
    assert_eq!(
        units.root(105),
        103,
        "and the feudal blacksmith, which is why research locations have to match by family"
    );
    // A chain (71 -> 141) has to end at the same root rather than one hop up.
    assert_eq!(
        units.root(141),
        109,
        "castle-age town centre, two hops from 109"
    );
    // An id nobody upgrades into, and an id that is not in the table at all.
    assert_eq!(units.root(83), 83);
    assert_eq!(units.root(99_999), 99_999, "an unknown id is its own root");
}
