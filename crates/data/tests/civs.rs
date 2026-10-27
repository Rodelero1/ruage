//! The civ table: who the players are, and which effect carries their tech-tree restrictions.

use std::path::PathBuf;

use ruage_data::{load_civs_file, load_civs_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const GOOD: &str = r#"
schema = 1
kind = "civs"

[[civ]]
id = 0
name = "Gaia"
tech_tree_effect = 348
team_bonus_effect = -1
resources = [1.0, 2.0]

[[civ]]
id = 1
name = "British"
tech_tree_effect = 254
team_bonus_effect = 255
resources = []
"#;

#[test]
fn a_civ_carries_its_name_and_the_effect_that_holds_its_tech_tree() {
    let civs = load_civs_str(GOOD, "civs.toml").expect("the fixture loads");
    assert_eq!(civs.civs.len(), 2, "gaia is a civ row too");
    let british = civs.by_name("British").expect("British is in the table");
    assert_eq!(british.id, 1);
    assert_eq!(british.tech_tree_effect, Some(254));
    assert_eq!(british.team_bonus_effect, Some(255));
    assert!(
        civs.by_name("Martian").is_none(),
        "and a civ the pack has never heard of is simply absent"
    );
}

#[test]
fn a_civ_without_a_tree_effect_is_allowed_and_means_no_restrictions() {
    let text = GOOD.replace("tech_tree_effect = 254", "tech_tree_effect = -1");
    let civs = load_civs_str(&text, "civs.toml").expect("the fixture loads");
    let british = civs.by_name("British").expect("British");
    assert_eq!(
        british.tech_tree_effect, None,
        "-1 is the dat's \"nothing\""
    );
}

#[test]
fn a_duplicate_civ_name_is_refused() {
    let text = GOOD.replacen("name = \"British\"", "name = \"Gaia\"", 1);
    let err = load_civs_str(&text, "civs.toml")
        .expect_err("two civs with one name: content that names a civ would be ambiguous")
        .to_string();
    assert!(err.contains("Gaia"), "got: {err}");
}

#[test]
fn the_real_civ_table_has_the_civilisations_and_their_trees() {
    let path = repo("config/gamedata/civs.toml");
    if !path.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            path.display()
        );
        return;
    }
    let civs = load_civs_file(&path).expect("civs.toml");
    assert_eq!(civs.civs.len(), 32, "gaia plus the 31 playable civs");
    let british = civs.by_name("British").expect("civ 1 is British");
    assert_eq!(british.id, 1);
    assert_eq!(
        british.tech_tree_effect,
        Some(254),
        "the effect whose kind-102 commands are the techs the British cannot research"
    );
}
