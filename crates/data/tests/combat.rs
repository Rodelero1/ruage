//! `combat.toml`: the damage constants and the refusals around them.

use ruage_data::load_combat_str;

const GOOD: &str = r#"
schema = 1
[damage]
minimum = 1
armour_missing_sentinel = 10000.0
bonus_resist_default = 0.0
[elevation]
attacker_above = 1.25
attacker_below = 0.75
[attack_timing]
damage_at_animation_fraction = 0.5
use_reload_from_dat = true
[buildings]
can_attack = true
"#;

#[test]
fn the_project_file_loads() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/combat.toml");
    let combat = ruage_data::load_combat_file(&path).expect("config/combat.toml");
    assert_eq!(combat.damage.minimum, 1);
    let (minimum, missing) = combat.damage_rules();
    assert_eq!(minimum, 1);
    assert_eq!(
        missing, 10_000,
        "the sentinel is 10000 in the file, not an engine guess"
    );
    assert!(combat.attack_timing.use_reload_from_dat);
    assert!(combat.buildings.can_attack);
}

#[test]
fn a_negative_minimum_is_refused() {
    let bad = GOOD.replace("minimum = 1", "minimum = -1");
    let err = load_combat_str(&bad, "combat.toml")
        .expect_err("a hit cannot heal")
        .to_string();
    assert!(err.contains("damage.minimum"), "got: {err}");
}

#[test]
fn a_zero_sentinel_is_refused() {
    let bad = GOOD.replace(
        "armour_missing_sentinel = 10000.0",
        "armour_missing_sentinel = 0.0",
    );
    let err = load_combat_str(&bad, "combat.toml")
        .expect_err("a zero sentinel would make every unlisted class hit for full")
        .to_string();
    assert!(err.contains("armour_missing_sentinel"), "got: {err}");
}

#[test]
fn a_fraction_outside_the_animation_is_refused() {
    let bad = GOOD.replace(
        "damage_at_animation_fraction = 0.5",
        "damage_at_animation_fraction = 1.5",
    );
    let err = load_combat_str(&bad, "combat.toml")
        .expect_err("1.5 of an animation is not a fraction")
        .to_string();
    assert!(err.contains("damage_at_animation_fraction"), "got: {err}");
}

#[test]
fn an_unknown_key_is_refused() {
    let bad = GOOD.to_owned() + "\n[creep]\nrate = 1.0\n";
    let err = load_combat_str(&bad, "combat.toml")
        .expect_err("unknown tables must be refused")
        .to_string();
    assert!(err.contains("creep"), "got: {err}");
}
