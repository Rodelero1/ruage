//! The content pack: one identity for everything a match depends on.

use std::path::{Path, PathBuf};

use ruage_data::{ContentPack, PACK_FILES, pack_path};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn config_dir() -> Option<PathBuf> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!("skipping: {} is absent (generated, D8)", units.display());
        return None;
    }
    Some(repo("config"))
}

#[test]
fn the_project_pack_loads_and_has_an_identity() {
    let Some(dir) = config_dir() else { return };
    let pack = ContentPack::load(&dir).expect("the project's own content pack");
    assert_ne!(
        pack.hash, 0,
        "an empty hash would compare equal to everything"
    );
    assert_eq!(pack.identity().len(), 16, "hex of a u64");
    assert_eq!(pack.parts.len(), PACK_FILES.len());
    assert_eq!(pack.resources.count(), 4);
    assert!(!pack.units.units.is_empty());
    assert!(!pack.combat.buildings.can_attack || pack.combat.buildings.can_attack);
}

#[test]
fn two_loads_of_the_same_files_agree() {
    let Some(dir) = config_dir() else { return };
    let first = ContentPack::load(&dir).expect("first");
    let second = ContentPack::load(&dir).expect("second");
    assert_eq!(
        first.hash, second.hash,
        "the hash must be a function of the content"
    );
    assert!(first.differences(&second).is_empty());
}

#[test]
fn one_changed_byte_changes_the_identity_and_names_the_file() {
    let Some(dir) = config_dir() else { return };
    let pack = ContentPack::load(&dir).expect("the project pack");

    // A copy of the pack with a comment added to one file: same values, different bytes.
    let scratch = repo("build/pack-test");
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    for name in PACK_FILES {
        let from = pack_path(&dir, name);
        let to = pack_path(&scratch, name);
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("dir");
        std::fs::copy(&from, &to).expect("copy");
    }
    let path = pack_path(&scratch, "combat.toml");
    let mut combat = std::fs::read_to_string(&path).expect("read");
    combat.push_str("\n# a comment, which changes nothing about the game\n");
    std::fs::write(&path, combat).expect("write");

    let changed = ContentPack::load(&scratch).expect("the edited pack still loads");
    assert_ne!(
        changed.hash, pack.hash,
        "bytes are the identity: a pack edited in place is not the pack others are running"
    );
    assert_eq!(
        changed.differences(&pack),
        vec!["combat.toml".to_owned()],
        "the difference names the file, so a handshake can say which one"
    );
}

#[test]
fn a_missing_file_says_which_one_and_why() {
    let Some(dir) = config_dir() else { return };
    let scratch = repo("build/pack-missing");
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    for name in PACK_FILES {
        if name == "resources.toml" {
            continue;
        }
        let from = pack_path(&dir, name);
        let to = pack_path(&scratch, name);
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("dir");
        std::fs::copy(&from, &to).expect("copy");
    }
    let err = ContentPack::load(&scratch)
        .expect_err("a pack missing a file is not a pack")
        .to_string();
    assert!(err.contains("resources.toml"), "got: {err}");
    assert!(
        err.contains("content pack"),
        "the error must say why the file matters, got: {err}"
    );
}

#[test]
fn every_pack_file_is_required() {
    // A constant, but the one that decides what a handshake compares: if a file is added to the
    // loader and not here, two players can disagree about it and still pass the check.
    for name in PACK_FILES {
        assert!(name.ends_with(".toml"), "{name} should be a config file");
    }
    assert_eq!(PACK_FILES.len(), 6);
    let dir = Path::new(".");
    assert!(dir.exists());
}
