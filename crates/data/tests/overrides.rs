//! `config/overrides.toml`: hand-written values layered on top of the generated tables.
//!
//! Written before the implementation (D7). The file has been documented since D6 — "hand edits go in
//! `overrides.toml`, every override is logged at startup with old → new" — and nothing ever loaded it, so an
//! edit there was a silent no-op. That is the worst shape a configuration surface can have, and the tests
//! below are as much about **refusing loudly** as about applying anything.

use ruage_data::{ConfigError, Overrides, Units};
use std::path::PathBuf;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn units() -> Option<Units> {
    let path = repo("config/gamedata/units.toml");
    if !path.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            path.display()
        );
        return None;
    }
    Some(ruage_data::load_units_file(&path).expect("units.toml"))
}

const OVERRIDES: &str = r#"
schema = 1

[overrides]
"units.79.min_range" = 0.0
"units.83.hp" = 40
"units.83.attacks" = [[4, 3], [3, 1]]
"#;

#[test]
fn an_override_replaces_a_value_and_is_reported_with_the_old_one() {
    let Some(mut table) = units() else { return };
    let before = table.unit(79).expect("the watch tower").min_range;
    let overrides = Overrides::parse(OVERRIDES, "overrides.toml").expect("the override file");

    let log = overrides.apply_units(&mut table).expect("applied");

    let tower = table.unit(79).expect("the watch tower");
    assert_ne!(
        tower.min_range, before,
        "the tower's dead zone was overridden"
    );
    assert_eq!(
        tower
            .min_range
            .resolve(1_000_000, "min_range")
            .expect("resolved")
            .raw(),
        0,
        "to zero: content may take a dead zone away"
    );
    assert_eq!(
        table.unit(83).expect("the villager").hp,
        40,
        "and a plain number is replaced too"
    );
    assert_eq!(
        table.unit(83).expect("the villager").attacks,
        vec![(4, 3), (3, 1)],
        "an array is replaced wholesale, never merged"
    );
    assert!(
        log.iter().any(|line| line.contains("units.79.min_range")
            && line.contains('1')
            && line.contains('0')),
        "the log says what changed, old to new: {log:?}"
    );
    assert_eq!(log.len(), 3, "one line per override: {log:?}");
}

#[test]
fn an_override_that_names_nothing_is_refused_by_name() {
    let Some(mut table) = units() else { return };
    for (key, wanted) in [
        ("units.99999.hp", "99999"),
        ("units.83.nonsense", "nonsense"),
        // A table the layer does not cover yet: refused rather than ignored, so a user who edits it
        // finds out instead of playing a game that quietly disagrees with their file.
        ("techs.202.research_time", "techs"),
    ] {
        let text = format!("schema = 1\n[overrides]\n\"{key}\" = 1\n");
        let overrides = Overrides::parse(&text, "overrides.toml").expect("parsed");
        let error = overrides
            .apply_units(&mut table)
            .expect_err(&format!("{key} must be refused"));
        let message = format!("{error:?}");
        assert!(
            message.contains(wanted),
            "{key}: the refusal should name {wanted}, said {message}"
        );
    }
}

#[test]
fn an_empty_override_file_changes_nothing() {
    let Some(mut table) = units() else { return };
    let before = table.unit(79).expect("tower").min_range;
    let overrides =
        Overrides::parse("schema = 1\n[overrides]\n", "overrides.toml").expect("parsed");
    assert!(overrides.is_empty());
    assert!(
        overrides
            .apply_units(&mut table)
            .expect("applied")
            .is_empty()
    );
    assert_eq!(table.unit(79).expect("tower").min_range, before);
}

/// A value of the wrong shape is a refusal, not a coercion: `hp = "forty"` must not become zero.
#[test]
fn a_value_of_the_wrong_type_is_refused() {
    let Some(mut table) = units() else { return };
    let text = "schema = 1\n[overrides]\n\"units.83.hp\" = \"forty\"\n";
    let overrides = Overrides::parse(text, "overrides.toml").expect("parsed");
    let error = overrides.apply_units(&mut table).expect_err("refused");
    assert!(
        format!("{error:?}").contains("units.83.hp"),
        "the refusal names the key"
    );
    assert_eq!(
        table.unit(83).expect("villager").hp,
        25,
        "and the table is untouched"
    );
}

/// The repository's own override file must stay loadable: it is a file a human edits, and a broken one would
/// fail at startup for reasons that have nothing to do with the code under test.
#[test]
fn the_repository_override_file_loads() {
    let path = repo("config/overrides.toml");
    if !path.exists() {
        eprintln!("skipping: no overrides.toml in this checkout");
        return;
    }
    let overrides = ruage_data::load_overrides_file(&path).expect("config/overrides.toml");
    let Some(mut table) = units() else { return };
    let log = overrides
        .apply_units(&mut table)
        .expect("the repository's overrides apply cleanly");
    for line in &log {
        println!("{line}");
    }
    let _: Option<ConfigError> = None;
}

/// **Review finding.** `apply_units` walked the entries and wrote as it went, so a file whose second entry was
/// refused left the first one applied - a half-loaded table, which is exactly the sort of state a strict loader
/// exists to prevent. The layer now validates every path and value first and only then writes.
#[test]
fn a_refused_override_applies_nothing_at_all() {
    let Some(mut table) = units() else { return };
    let before = table.unit(79).expect("tower").min_range;
    // Sorted by key, the valid one comes first: `units.79` before `units.83`.
    let text = "schema = 1\n[overrides]\n\"units.79.min_range\" = 0.0\n\"units.83.nonsense\" = 1\n";
    let overrides = Overrides::parse(text, "overrides.toml").expect("parsed");
    overrides
        .apply_units(&mut table)
        .expect_err("the unknown field is refused");
    assert_eq!(
        table.unit(79).expect("tower").min_range,
        before,
        "and the valid entry above it was not applied either: a refused file changes nothing"
    );
}

/// **Review finding.** Nothing stopped `"units.83.hp" = 0`, which makes a unit that cannot exist: it would be
/// dead on arrival, and every max-health computation would floor it. Content may lower a unit's health; it may
/// not take it away.
#[test]
fn an_override_cannot_leave_a_unit_without_health() {
    let Some(mut table) = units() else { return };
    for hp in [0, -5] {
        let text = format!("schema = 1\n[overrides]\n\"units.83.hp\" = {hp}\n");
        let overrides = Overrides::parse(&text, "overrides.toml").expect("parsed");
        let error = overrides
            .apply_units(&mut table)
            .expect_err("health of zero or less is refused");
        assert!(
            format!("{error:?}").contains("units.83.hp"),
            "the refusal names the key"
        );
    }
    assert_eq!(table.unit(83).expect("villager").hp, 25, "untouched");
}
