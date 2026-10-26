//! The content pack's resource list, and the bridge that keeps it in step with `ResourceKind`.

use std::path::PathBuf;

use ruage_data::{ResourceKind, Resources, load_resources_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn project() -> Resources {
    load_resources(&repo("config/resources.toml"))
}

fn load_resources(path: &std::path::Path) -> Resources {
    ruage_data::load_resources_file(path).expect("config/resources.toml")
}

#[test]
fn the_project_file_is_dense_and_named() {
    let resources = project();
    assert_eq!(
        resources.count(),
        4,
        "the reference pack has four resources"
    );
    assert_eq!(resources.name(0), Some("wood"));
    assert_eq!(resources.name(1), Some("food"));
    assert_eq!(resources.name(2), Some("gold"));
    assert_eq!(resources.name(3), Some("stone"));
    assert_eq!(resources.slot_of("gold"), Some(2));
    assert_eq!(resources.slot_of("silver"), None);
    for (slot, resource) in resources.entries.iter().enumerate() {
        assert_eq!(
            usize::from(resource.slot),
            slot,
            "slots are dense from zero"
        );
    }
}

/// The bridge (D17): while `ResourceKind` exists it *is* the concrete pack, so the content file and
/// the enum must agree on three things at once - the name, the stockpile slot and the reference
/// pack's id. When the enum goes, this test goes with it, and the file becomes the only truth.
#[test]
fn the_content_file_agrees_with_resource_kind() {
    let resources = project();
    for kind in [
        ResourceKind::Wood,
        ResourceKind::Food,
        ResourceKind::Gold,
        ResourceKind::Stone,
    ] {
        let name = format!("{kind:?}").to_lowercase();
        let slot = resources
            .slot_of(&name)
            .unwrap_or_else(|| panic!("{name} is missing from config/resources.toml"));
        assert_eq!(
            slot,
            kind.index(),
            "{name} lives in the wrong stockpile column"
        );
        assert_eq!(
            resources.entries[slot].dat_id,
            Some(u16::from(kind.dat_id())),
            "{name} carries the wrong reference-pack id"
        );
    }
}

#[test]
fn a_gap_in_the_slots_is_refused() {
    let bad = r#"
schema = 1

[[resource]]
slot = 0
name = "wood"

[[resource]]
slot = 2
name = "gold"
"#;
    let err = load_resources_str(bad, "resources.toml")
        .expect_err("a gap would leave a stockpile column nothing can address")
        .to_string();
    assert!(err.contains("dense from 0"), "got: {err}");
}

#[test]
fn a_resource_defined_twice_is_refused() {
    let bad = r#"
schema = 1

[[resource]]
slot = 0
name = "wood"

[[resource]]
slot = 1
name = "wood"
"#;
    let err = load_resources_str(bad, "resources.toml")
        .expect_err("two resources with one name: the lookup would silently take the first")
        .to_string();
    assert!(err.contains("defined twice"), "got: {err}");
}

#[test]
fn a_resource_with_no_name_is_refused() {
    let bad = r#"
schema = 1

[[resource]]
slot = 0
name = "  "
"#;
    let err = load_resources_str(bad, "resources.toml")
        .expect_err("a nameless resource cannot be referenced by any config")
        .to_string();
    assert!(err.contains("needs a name"), "got: {err}");
}

#[test]
fn an_empty_resource_list_is_refused() {
    let err = load_resources_str("schema = 1\n", "resources.toml")
        .expect_err("a match with no resources cannot stockpile anything")
        .to_string();
    assert!(err.contains("no resources"), "got: {err}");
}

#[test]
fn one_reference_id_cannot_serve_two_resources() {
    let bad = r#"
schema = 1

[[resource]]
slot = 0
name = "wood"
dat_id = 1

[[resource]]
slot = 1
name = "food"
dat_id = 1
"#;
    let err = load_resources_str(bad, "resources.toml")
        .expect_err("two resources claiming one reference id would import the same column twice")
        .to_string();
    assert!(err.contains("claimed twice"), "got: {err}");
}
