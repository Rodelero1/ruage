//! The economy config loader (D6): rates the dat does not settle, plus the task→variant table
//! from D11. Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{load_economy_file, load_economy_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const GOOD: &str = r#"
schema = 1
[farm]
rate = 0.4
use_villager_rate = false
[farm.depletion]
food_per_farm = 250
[gather]
rate_from = "dat"
carry_from = "dat"
drop_sites_from = "dat_variant"
[gather.variant]
wood = 123
gold = 579
stone = 124
forage = 120
[build]
repair_hp_per_second = -1.0
[[train.entry]]
building = "town_centre"
units = ["villager"]
[decay]
enabled = true
rate_from = "dat"
[gather.resource_task]
wood = "wood"
food = "forage"
gold = "gold"
stone = "stone"

[role]
town_centre = 109
villager = 83

[tech_role]
man_at_arms = 222

[start]
town_centre = "town_centre"
villager = "villager"
villagers_per_player = 3

[population]
cap_default = 200
house_population = 5
"#;

#[test]
fn the_real_economy_toml_loads_and_resolves_its_rates() {
    let economy = load_economy_file(&repo("config/economy.toml")).expect("economy.toml must load");

    // 0.4 at scale 20000 is 8000 units, exactly — the D5 farm rate.
    assert_eq!(
        economy
            .farm
            .rate
            .resolve(20_000, "farm.rate")
            .expect("farm rate resolves")
            .raw(),
        8_000
    );
    assert!(!economy.farm.use_villager_rate);
    assert_eq!(economy.farm.food_per_farm, 250);
    assert_eq!(economy.population.cap_default, 200);
    assert_eq!(economy.population.house_population, 5);

    // The task→variant table (D11): ten tasks, and wood is the lumberjack variant.
    assert_eq!(
        economy.gather.variant.len(),
        10,
        "ten tasks in economy.toml"
    );
    assert_eq!(economy.gather.variant.get("wood"), Some(&123));
    assert_eq!(economy.gather.variant.get("farm"), Some(&259));
    assert!(
        !economy.gather.variant.values().any(|&id| id == 83),
        "the base villager (83) must never be a gather source"
    );
}

#[test]
fn an_unresolved_rate_names_its_key() {
    let economy = load_economy_str(GOOD, "economy.toml").expect("fixture loads");
    let err = economy
        .build
        .repair_hp_per_second
        .resolve(20_000, "build.repair_hp_per_second")
        .expect_err("the sentinel must not resolve")
        .to_string();
    assert!(err.contains("repair_hp_per_second"), "got: {err}");
    assert!(err.contains("unresolved"), "got: {err}");
}

#[test]
fn an_unknown_key_is_rejected() {
    let bad = GOOD.replace("house_population = 5", "house_population = 5\nbogus = 1");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("unknown keys must be refused")
        .to_string();
    assert!(err.contains("bogus"), "got: {err}");
}

#[test]
fn a_missing_key_is_rejected() {
    let bad = GOOD.replace("cap_default = 200\n", "");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("missing keys must be refused")
        .to_string();
    assert!(err.contains("cap_default"), "got: {err}");
}

#[test]
fn an_empty_variant_table_contradicts_rate_from_dat() {
    // D11: if the rates come from the dat by variant, a table with no variants is a config that
    // says one thing and means another.
    let bad = GOOD.replace("wood = 123\ngold = 579\n", "");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("an empty gather.variant table must be refused")
        .to_string();
    assert!(err.contains("gather.variant"), "got: {err}");
}

#[test]
fn two_tasks_cannot_share_one_variant() {
    // wood = 123 and gold = 123 would make one variant serve two resources; whichever the
    // lookup loses, the gather rate is wrong, and nothing downstream would notice.
    let bad = GOOD.replace("gold = 579", "gold = 123");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a duplicated variant id must be refused")
        .to_string();
    assert!(err.contains("gather.variant"), "got: {err}");
    assert!(
        err.contains("123"),
        "the error should name the id, got: {err}"
    );
}

#[test]
fn a_zero_variant_id_is_refused() {
    // Unit id 0 exists in the dat (OLD-ACADEMY) but is never a villager; a 0 here means the
    // field was left unset.
    let bad = GOOD.replace("gold = 579", "gold = 0");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a zero variant id must be refused")
        .to_string();
    assert!(err.contains("gather.variant"), "got: {err}");
}

#[test]
fn the_real_economy_names_the_starting_units() {
    let economy = load_economy_file(&repo("config/economy.toml")).expect("economy.toml loads");
    // A start names *roles*, and the role table is what turns them into dat ids (D17): nothing in
    // `crates/` needs to know that a town centre is 109.
    assert_eq!(economy.start.town_centre, "town_centre");
    assert_eq!(economy.start.villager, "villager");
    assert_eq!(economy.start.villagers_per_player, 3);
    assert_eq!(economy.role.get("town_centre"), Some(&109), "RTWC");
    assert_eq!(economy.role.get("villager"), Some(&83), "VMBAS");
}

#[test]
fn the_real_economy_maps_each_resource_name_to_a_gather_task() {
    // Keyed by the pack's names (D17): the economy does not know what a *kind* of resource is, it
    // knows that the resource called "food" is worked by the foraging variant.
    let economy = load_economy_file(&repo("config/economy.toml")).expect("economy.toml loads");
    let task = |name: &str| {
        economy
            .gather
            .resource_task
            .get(name)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("{name} names no gather task"))
    };
    assert_eq!(task("wood"), "wood");
    assert_eq!(task("food"), "forage", "berries are foraged");
    assert_eq!(task("gold"), "gold");
    assert_eq!(task("stone"), "stone");
    // And every one of them is a real variant, which is what the loader guarantees.
    for name in ["wood", "food", "gold", "stone"] {
        assert!(
            economy.gather.variant.contains_key(task(name)),
            "{} is not a variant",
            task(name)
        );
    }
}

#[test]
fn a_resource_task_that_is_not_a_variant_is_refused() {
    let bad = GOOD.replace("food = \"forage\"", "food = \"berries\"");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a task name that names no variant must be refused")
        .to_string();
    assert!(
        err.contains("berries"),
        "the error should name the task, got: {err}"
    );
}

#[test]
fn a_missing_start_block_is_refused() {
    let bad = GOOD.replace("[start]", "[renamed]");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a config that does not say what a match begins with must be refused")
        .to_string();
    assert!(err.to_lowercase().contains("start"), "got: {err}");
}

#[test]
fn start_roles_must_be_defined_and_distinct() {
    // A role of 0 is refused (see `a_role_cannot_be_unit_zero`); a *start* naming the same role
    // twice would seed one unit where a match needs two kinds of thing.
    let same = GOOD.replace("villager = \"villager\"", "villager = \"town_centre\"");
    let err = load_economy_str(&same, "economy.toml")
        .expect_err("one role cannot be both the town centre and the villager")
        .to_string();
    assert!(err.contains("town_centre"), "got: {err}");
}

#[test]
fn the_villager_count_must_be_a_sane_positive_number() {
    for bad_count in ["0", "2000"] {
        let bad = GOOD.replace(
            "villagers_per_player = 3",
            &format!("villagers_per_player = {bad_count}"),
        );
        let err = load_economy_str(&bad, "economy.toml")
            .expect_err("an implausible villager count must be refused")
            .to_string();
        assert!(err.contains("villagers_per_player"), "got: {err}");
    }
}

#[test]
fn the_schema_is_checked() {
    let bad = GOOD.replace("schema = 1", "schema = 7");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("an unknown schema must be refused")
        .to_string();
    assert!(err.contains("schema"), "got: {err}");
}

#[test]
fn a_negative_farm_rate_is_refused() {
    // -1.0 means unresolved everywhere else; a *farm rate* of -1.0 is not a sentinel we honour
    // here, because -1 is never a plausible rate and silently treating it as "unknown" would
    // hide a typo.
    let bad = GOOD.replace("rate = 0.4", "rate = -1.0");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a negative farm rate must be refused")
        .to_string();
    assert!(err.contains("farm.rate"), "got: {err}");
}

#[test]
fn the_real_file_says_who_trains_what() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/economy.toml");
    let economy = load_economy_file(&path).expect("the project's own economy.toml");
    assert!(
        economy.train.allows(109, 83),
        "the town centre trains villagers"
    );
    assert!(economy.train.allows(12, 74), "the barracks trains spearmen");
    assert!(
        !economy.train.allows(109, 74),
        "the town centre must not train spearmen"
    );
    assert!(!economy.train.allows(70, 83), "a house trains nothing");
}

#[test]
fn a_building_listed_twice_is_refused() {
    let bad =
        GOOD.to_owned() + "\n[[train.entry]]\nbuilding = \"town_centre\"\nunits = [\"villager\"]\n";
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a duplicate building entry must be refused")
        .to_string();
    assert!(
        err.contains("town_centre"),
        "the refusal names the role: {err}"
    );
}

#[test]
fn a_building_that_trains_nothing_is_refused() {
    let bad = GOOD.replace("units = [\"villager\"]", "units = []");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("an empty unit list must be refused")
        .to_string();
    assert!(err.contains("trains nothing"), "got: {err}");
}

/// A start that names a role the table does not define is a match nobody can seed: the engine has a
/// word for what to place and no unit behind it.
#[test]
fn a_start_naming_an_undefined_role_is_refused() {
    let bad = GOOD.replace(
        "town_centre = \"town_centre\"",
        "town_centre = \"great_hall\"",
    );
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("a start role with no unit must be refused")
        .to_string();
    assert!(
        err.contains("great_hall"),
        "the error should name it: {err}"
    );
}

/// Unit 0 is a real dat unit, so it is never a role: a zero here would seed a match with whatever
/// id 0 happens to be, and no test would notice.
#[test]
fn a_role_cannot_be_unit_zero() {
    let bad = GOOD.replace("villager = 83", "villager = 0");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("role 0 is not a unit id a match can place")
        .to_string();
    assert!(err.contains("villager"), "got: {err}");
}

/// A role table is the vocabulary scenarios and the start use; empty, it names nothing.
#[test]
fn an_empty_role_table_is_refused() {
    let bad = GOOD.replace("[role]\ntown_centre = 109\nvillager = 83\n", "[role]\n");
    let err = load_economy_str(&bad, "economy.toml")
        .expect_err("no roles means no unit can be named")
        .to_string();
    assert!(err.contains("role"), "got: {err}");
}

/// Techs have no names in the dat (they are string-ids into the language files), so content that must
/// name one needs its own vocabulary - the same shape as unit roles (D17).
#[test]
fn tech_roles_name_techs_and_must_point_at_real_ones() {
    let economy = load_economy_file(&repo("config/economy.toml")).expect("economy.toml loads");
    assert_eq!(
        economy.tech_role.get("man_at_arms"),
        Some(&222),
        "the dat's Man-at-Arms, identified by cost and time in the tests"
    );

    let zero = GOOD.replace("man_at_arms = 222", "man_at_arms = 0");
    let err = load_economy_str(&zero, "economy.toml")
        .expect_err("tech 0 is a real tech, never a role")
        .to_string();
    assert!(err.contains("man_at_arms"), "got: {err}");
}

/// `[[train.entry]]` used to quote dat ids while everything else named roles, which is the one place
/// content could still be silently wrong about which unit it meant. A role is the only spelling now.
#[test]
fn a_train_entry_names_units_by_role() {
    let economy = load_economy_str(GOOD, "economy.toml").expect("fixture loads");
    assert!(
        economy.train.allows(109, 83),
        "town_centre -> villager, resolved through [role]"
    );

    let unknown = GOOD.replace("units = [\"villager\"]", "units = [\"militia\"]");
    let err = load_economy_str(&unknown, "economy.toml")
        .expect_err("a role the table does not define cannot be trained")
        .to_string();
    assert!(err.contains("militia"), "the refusal names it: {err}");
    assert!(err.contains("[role]"), "and says where roles live: {err}");

    // An id where a role belongs is refused rather than quietly accepted, because accepting both
    // spellings is how the numbering creeps back in.
    let numeric = GOOD.replace("units = [\"villager\"]", "units = [83]");
    let err = load_economy_str(&numeric, "economy.toml")
        .expect_err("an id is not a role")
        .to_string();
    assert!(
        err.contains("units"),
        "the refusal points at the field: {err}"
    );
}
