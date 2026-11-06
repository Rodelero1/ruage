//! **The fixture's own rules** (D130): an absent pack skips, a present pack that will not load fails.
//!
//! These are tests of the test harness, and they exist because it had been lying. Thirty-six files in this
//! directory loaded the pack through `load_*(..).ok()?`, so a broken `economy.toml` became `None`, `fresh()`
//! became `None`, and every test in the file took its `else { return }` and reported **passed**. The first test
//! below was written before the strictness and watched failing exactly that way: the loader returned `None`
//! instead of panicking.
//!
//! Written before the implementation (D7, SPEC §11).

use std::fs;
use std::path::{Path, PathBuf};

mod common;

/// The meta-tests below need a real pack to copy. A tree without one (D8: `config/gamedata/` is generated
/// from the game's install and never committed) skips them, exactly as every other fixture does - the rule
/// this file tests has to apply to this file, or a fresh clone goes red on the tests that describe the rule.
fn a_pack_to_copy() -> bool {
    common::repo("config/gamedata/units.toml").exists()
}

/// A private copy of the shipped pack, so that breaking a file here breaks nothing else.
fn a_copy_of_the_pack(name: &str) -> PathBuf {
    let into = std::env::temp_dir().join(format!("ruage-fixture-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&into);
    copy_tree(&common::repo("config"), &into.join("config"));
    into
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("a directory to copy into");
    for entry in fs::read_dir(from).expect("a readable directory") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("a copied file");
        }
    }
}

/// The text of a caught panic, whichever way it was raised.
fn text_of(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with no text".to_string()
    }
}

#[test]
fn a_pack_that_was_never_generated_skips_the_fixture() {
    let dir = std::env::temp_dir().join(format!("ruage-fixture-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("an empty directory");

    assert!(
        common::load_from(&dir, common::Pack::new().combat().research().civs()).is_none(),
        "a directory with no generated pack must be `None` - a fresh clone has no `config/gamedata/` (D8) - \
         rather than a panic or a world"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_file_in_a_present_pack_fails_the_fixture() {
    if !a_pack_to_copy() {
        return;
    }
    let dir = a_copy_of_the_pack("broken");
    // The premise: the copy is good, so the failure below is caused by the one file we break.
    assert!(
        common::load_from(&dir, common::Pack::new().combat()).is_some(),
        "the copy of the pack must load before it is broken, or this test measures the copy"
    );
    fs::write(dir.join("config/economy.toml"), "nonsense = [\n").expect("a broken economy file");

    let caught = std::panic::catch_unwind(|| common::load_from(&dir, common::Pack::new().combat()));
    let said = match caught {
        Err(payload) => text_of(payload),
        Ok(_) => "nothing: the loader returned instead of panicking".to_string(),
    };
    assert!(
        said.contains("economy.toml"),
        "a present pack that will not load must panic and name the file; it said: {said}"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// The marker is *existence*, not *parseability*: a broken `units.toml` is a broken pack, not a
/// pack that was never generated. Written after the strictness landed, and it passed first time -
/// recorded here rather than dressed up as a RED.
#[test]
fn a_present_but_broken_marker_fails_rather_than_skipping() {
    if !a_pack_to_copy() {
        return;
    }
    let dir = a_copy_of_the_pack("marker");
    assert!(
        common::load_from(&dir, common::Pack::new()).is_some(),
        "the copy of the pack must load before it is broken"
    );
    fs::write(dir.join("config/gamedata/units.toml"), "not = a pack\n")
        .expect("a broken units file");

    let caught = std::panic::catch_unwind(|| common::load_from(&dir, common::Pack::new()));
    let said = match caught {
        Err(payload) => text_of(payload),
        Ok(_) => "nothing: the loader returned instead of panicking".to_string(),
    };
    assert!(
        said.contains("units.toml"),
        "a present-but-broken `units.toml` must fail the fixture, not skip it; it said: {said}"
    );

    let _ = fs::remove_dir_all(&dir);
}
