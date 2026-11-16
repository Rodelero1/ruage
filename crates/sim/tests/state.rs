//! **P1a: a world can be written down and read back, and the two are the same world** (D235).
//!
//! The engine's half of P1: the tool-side converter behind the seam has nothing to convert *to* until the world's state can
//! be written and loaded, and the property that matters is not "the file parses" but **"the loaded world ticks the same"**.
//! Anything a write drops that the simulation reads shows up as a divergence inside a hundred ticks; anything derived and not
//! rebuilt on load (the bonus table, the sight cache) shows up immediately.
//!
//! Written before `World::state`/`World::from_state` exist (D7, SPEC §11).

use std::path::PathBuf;

use ruage_sim::{Command, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn pack() -> Option<ruage_data::ContentPack> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(ruage_data::ContentPack::load(&repo("config")).expect("the pack"))
}

/// A match with work in it: two villagers told to gather wood, a house ordered, and three hundred ticks run, so the state
/// carries paths, tasks, timers and a random stream that has advanced.
fn a_match_in_progress(pack: &ruage_data::ContentPack) -> World {
    let mut economy = pack.economy.clone();
    economy.start.civs = vec!["British".to_owned()];
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 2_000),
    ]
    .into_iter()
    .collect();
    let mut world = World::new(&pack.sim, 7).expect("world");
    world
        .seed_from(&pack.map, &economy, &pack.units)
        .expect("seeding");
    world
        .load_resources(&pack.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&pack.techs, &pack.effects)
        .expect("research tables");
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml");
    world.load_civs(&civs).expect("the civ table");

    // A site and an order, so the state carries a path, a task and a construction the write has to keep.
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    let house = world.role_unit("house").expect("a house role");
    let at = (0..40u16)
        .flat_map(|x| (0..40u16).map(move |y| (x, y)))
        .find(|(x, y)| world.site_fits(1, house, (*x, *y)))
        .expect("a free tile for a house");
    world
        .submit_checked(Command::Build {
            villager,
            unit: house,
            at,
        })
        .expect("a house order");
    for _ in 0..300 {
        world.step().expect("step");
    }
    world
}

/// **The round trip is the same world.** Same hash immediately - which catches a derived cache that load did not rebuild - and
/// the same hash after a hundred ticks, which catches a field the write dropped that the tick reads.
#[test]
fn a_world_written_and_read_back_ticks_the_same() {
    let Some(pack) = pack() else { return };
    let mut world = a_match_in_progress(&pack);

    let state = world.state(&pack).expect("the world writes itself down");
    let mut loaded = World::from_state(&pack, &state).expect("and reads itself back");
    assert_eq!(
        loaded.state_hash(),
        world.state_hash(),
        "the loaded world is not the world it was written from"
    );

    for tick in 0..100 {
        world.step().expect("step");
        loaded.step().expect("step");
        assert_eq!(
            loaded.state_hash(),
            world.state_hash(),
            "the two worlds part company at tick {tick} after the round trip"
        );
    }
    assert!(
        world.state_hash() != 0,
        "a world with work in it hashes to something"
    );
}

/// **A state that does not name a field is refused, not defaulted.** The whole point of writing the world as one structure is
/// that a missing field is an error; a loader that fills a gap with `Default` turns a corrupt file into a match that diverges
/// a hundred ticks later with nothing to point at. Truncation is the corruption a file actually suffers (a copy that stopped
/// short), and no part of a cut state may load.
#[test]
fn a_cut_state_is_refused() {
    let Some(pack) = pack() else { return };
    let world = a_match_in_progress(&pack);
    let state = world.state(&pack).expect("the world writes itself down");
    let cut = &state[..state.len() / 2];
    assert!(
        World::from_state(&pack, cut).is_err(),
        "half a state loaded as a world"
    );
}

/// **A state cannot be loaded against another pack.** The tables are the pack's, not the match's: a load against different
/// content would change what every unit does while the file still looks valid, so the identity is checked and named.
#[test]
fn a_state_from_another_pack_is_refused_by_name() {
    let Some(pack) = pack() else { return };
    let world = a_match_in_progress(&pack);
    let state = world.state(&pack).expect("the world writes itself down");

    let mut other = pack.clone();
    other.hash ^= 1;
    let refused = match World::from_state(&other, &state) {
        Err(error) => error,
        Ok(_) => panic!("a different pack was accepted"),
    };
    let text = refused.to_string();
    assert!(
        matches!(refused, ruage_sim::SimError::PackMismatch { .. }),
        "the refusal is not a pack mismatch: {text}"
    );
    assert!(
        text.contains(&pack.identity()) && text.contains(&other.identity()),
        "the refusal must name both packs: {text}"
    );
}
