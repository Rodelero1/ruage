//! **The job names a pack gives its workers** (D121): the owner wants a villager "renamed to match the task he is doing".
//! The words are content, so the loader's contract is that a word is found by the target's role or by its column, and that a
//! rule which can never match is refused rather than loaded.

use std::path::PathBuf;

use ruage_data::{Jobs, load_jobs_file, load_jobs_str};

fn fixture(text: &str) -> Jobs {
    load_jobs_str(text, "jobs.toml").expect("the file loads")
}

#[test]
fn a_role_names_the_job() {
    let jobs = fixture(
        r#"
schema = 1
[[job]]
target = "farm"
name = "Farmer"
"#,
    );
    assert_eq!(jobs.name_for(Some("farm"), None), Some("Farmer"));
}

#[test]
fn a_column_names_the_job_when_the_target_has_no_role() {
    let jobs = fixture(
        r#"
schema = 1
[[job]]
column = "wood"
name = "Lumberjack"
"#,
    );
    assert_eq!(
        jobs.name_for(None, Some("wood")),
        Some("Lumberjack"),
        "a tree has a column and no role, and it still needs a word"
    );
}

#[test]
fn a_role_beats_a_column() {
    let jobs = fixture(
        r#"
schema = 1
[[job]]
column = "food"
name = "Forager"
[[job]]
target = "sheep"
name = "Shepherd"
"#,
    );
    assert_eq!(
        jobs.name_for(Some("sheep"), Some("food")),
        Some("Shepherd"),
        "the more specific statement wins: a sheep is not forage"
    );
    assert_eq!(
        jobs.name_for(None, Some("food")),
        Some("Forager"),
        "and a berry bush, which has no role, is still forage"
    );
}

#[test]
fn nothing_matching_leaves_the_villager_a_villager() {
    let jobs = fixture(
        r#"
schema = 1
[[job]]
target = "farm"
name = "Farmer"
"#,
    );
    assert_eq!(jobs.name_for(None, None), None);
    assert_eq!(jobs.name_for(Some("mill"), None), None);
    assert_eq!(jobs.name_for(None, Some("stone")), None);
}

#[test]
fn an_entry_that_can_never_match_is_refused() {
    let err = load_jobs_str(
        r#"
schema = 1
[[job]]
name = "Farmer"
"#,
        "jobs.toml",
    );
    assert!(
        err.is_err(),
        "an entry with neither a target nor a column is a rule the author meant to be a rule"
    );
}

#[test]
fn a_nameless_job_is_refused() {
    let err = load_jobs_str(
        r#"
schema = 1
[[job]]
target = "farm"
name = "  "
"#,
        "jobs.toml",
    );
    assert!(err.is_err(), "a job with no word is not a job");
}

#[test]
fn the_shipped_file_names_the_jobs_the_owner_asked_for() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config/jobs.toml");
    let Ok(jobs) = load_jobs_file(&path) else {
        return;
    };
    // The six in the owner's picture, plus the columns a villager actually works.
    assert_eq!(jobs.name_for(Some("farm"), Some("food")), Some("Farmer"));
    assert_eq!(jobs.name_for(Some("sheep"), Some("food")), Some("Shepherd"));
    assert_eq!(jobs.name_for(Some("deer"), Some("food")), Some("Hunter"));
    assert_eq!(
        jobs.name_for(Some("wild_boar"), Some("food")),
        Some("Hunter")
    );
    assert_eq!(jobs.name_for(None, Some("food")), Some("Forager"));
    assert_eq!(jobs.name_for(None, Some("wood")), Some("Lumberjack"));
}
