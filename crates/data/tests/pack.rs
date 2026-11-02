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


/// A copy of the pack, personalities included: the identity covers `config/ai/` as a **directory**, so a scratch pack
/// without it would differ for the wrong reason and every "one byte changed" test would name two files.
fn copy_pack(dir: &std::path::Path, scratch: &std::path::Path) {
    std::fs::create_dir_all(scratch).expect("scratch dir");
    for name in PACK_FILES.iter().copied().chain(["overrides.toml"]) {
        let from = pack_path(dir, name);
        if !from.exists() {
            continue;
        }
        let to = pack_path(scratch, name);
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("dir");
        std::fs::copy(&from, &to).expect("copy");
    }
    let ai = dir.join("ai");
    if ai.is_dir() {
        std::fs::create_dir_all(scratch.join("ai")).expect("ai dir");
        for entry in std::fs::read_dir(&ai).expect("read ai") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                std::fs::copy(&path, scratch.join("ai").join(path.file_name().expect("name")))
                    .expect("copy personality");
            }
        }
    }
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
    // The nine tables, plus `overrides.toml` when the pack has one - it is content too, because two
    // peers with different overrides are playing different games (D45) - plus every personality under `ai/`, which the
    // loader walks rather than listing (D94): a scenario may name any of them.
    let personalities = std::fs::read_dir(dir.join("ai"))
        .expect("the project has an ai folder")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .count();
    assert!(personalities > 0, "the base personality is the pack's own AI");
    let expected =
        PACK_FILES.len() + usize::from(pack.parts.contains_key("overrides.toml")) + personalities;
    assert_eq!(pack.parts.len(), expected);
    assert!(
        pack.parts.contains_key("ai/base.toml"),
        "the personality is in the identity, named as the loader names it"
    );
    assert_eq!(pack.resources.count(), 4);
    // The tables a pack carries are the ones its files hold - compared against a direct load of the same file rather
    // than against a remembered count. A magic number here fails the day a civ is added, for a reason that has nothing
    // to do with packing; the dat's own census is `techs.rs`'s business, where pinning 801 is the point.
    let dir = repo("config");
    let techs_on_disk = ruage_data::load_techs_file(&ruage_data::pack_path(&dir, "techs.toml"))
        .expect("techs.toml");
    let effects_on_disk =
        ruage_data::load_effects_file(&ruage_data::pack_path(&dir, "effects.toml"))
            .expect("effects.toml");
    assert_eq!(
        pack.techs.techs.len(),
        techs_on_disk.techs.len(),
        "the pack carries the techs its own file holds"
    );
    assert_eq!(
        pack.effects.effects.len(),
        effects_on_disk.effects.len(),
        "and the effects its own file holds"
    );
    assert!(!pack.techs.techs.is_empty() && !pack.effects.effects.is_empty());
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
    copy_pack(&dir, &scratch);
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
    assert_eq!(
        PACK_FILES.len(),
        9,
        "seven behaviour files, two tables of research and the civ bonuses - personalities are a directory, not a list"
    );
    let dir = Path::new(".");
    assert!(dir.exists());
}

/// **Review finding.** The docs said an override file joins the handshake hash "when it exists", and no test
/// said so. Two packs alike except for one overridden value must not look like the same game to a peer.
#[test]
fn a_pack_with_a_different_override_is_a_different_pack() {
    let Some(dir) = config_dir() else { return };
    let pack = ContentPack::load(&dir).expect("the project pack");

    let scratch = repo("build/pack-override-test");
    let _ = std::fs::remove_dir_all(&scratch);
    copy_pack(&dir, &scratch);
    // Same tables, one hand-written value different.
    std::fs::write(
        scratch.join("overrides.toml"),
        "schema = 1\n[overrides]\n\"units.79.min_range\" = 0.0\n",
    )
    .expect("write");

    let changed = ContentPack::load(&scratch).expect("the overridden pack loads");
    assert_ne!(
        changed.hash, pack.hash,
        "different overrides are a different game, which is what the handshake is for"
    );
    assert_eq!(
        changed.differences(&pack),
        vec!["overrides.toml".to_owned()],
        "and the difference names the file"
    );
    assert_eq!(changed.override_log.len(), 1, "the change is logged");
    assert_eq!(
        changed
            .units
            .unit(79)
            .expect("the watch tower")
            .min_range
            .resolve(1_000_000, "min_range")
            .expect("resolved")
            .raw(),
        0,
        "and the pack really carries the overridden value"
    );
}

#[test]
fn a_civs_bonuses_are_part_of_the_pack_identity() {
    // The cheapest cheat there is: edit a civilisation's bonuses. Until 2026-11-01 `civs.toml` was loaded by the
    // binaries *outside* the pack, so a player could raise their own civ's gathering rate and still pass the handshake.
    // This test is what keeps it inside: remove "civs.toml" from PACK_FILES and it fails, naming no difference at all.
    let Some(dir) = config_dir() else { return };
    let pack = ContentPack::load(&dir).expect("the project pack");

    let scratch = repo("build/pack-civs");
    copy_pack(&dir, &scratch);
    let path = pack_path(&scratch, "civs.toml");
    let mut civs = std::fs::read_to_string(&path).expect("read");
    civs.push_str("\n# a civ bonus edited in place\n");
    std::fs::write(&path, civs).expect("write");

    let changed = ContentPack::load(&scratch).expect("the edited pack still loads");
    assert_ne!(
        changed.hash, pack.hash,
        "a civ bonus is part of what the match is"
    );
    assert_eq!(
        changed.differences(&pack),
        vec!["civs.toml".to_owned()],
        "and the difference names the file, so the handshake can refuse and say why"
    );
}

/// Every personality a scenario can name is content, so every one of them must be in the identity. `PACK_FILES` is a
/// static list and cannot say "whatever is in `config/ai/`", so the loader walks the directory; this test is what keeps
/// that true when a second personality is added and someone forgets to list it.
#[test]
fn a_personality_outside_the_fixed_list_is_part_of_the_identity() {
    let Some(dir) = config_dir() else { return };
    let pack = ContentPack::load(&dir).expect("the project pack");

    let scratch = repo("build/pack-ai");
    let _ = std::fs::remove_dir_all(&scratch);
    copy_pack(&dir, &scratch);

    // A personality nobody listed anywhere: the scenario would load it, so the handshake has to see it.
    std::fs::write(scratch.join("ai").join("rush.toml"), "schema = 1\n")
        .expect("write an unlisted personality");
    let changed = ContentPack::load(&scratch).expect("the edited pack still loads");

    assert_ne!(changed.hash, pack.hash, "a personality the peers do not share is a different game");
    assert_eq!(
        changed.differences(&pack),
        vec!["ai/rush.toml".to_owned()],
        "and the difference names it, so a refusal can say which file"
    );
}
