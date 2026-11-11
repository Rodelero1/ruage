//! One loader for every fixture in this directory.
//!
//! Two rules, and they are the whole point of the file (D130):
//!
//! * **The generated pack is absent** - a checkout where `config/gamedata/` has not been produced
//!   (`cargo xtask gamedata`, D8) - so a fixture returns `None` and its tests skip. That is the
//!   documented behaviour for a tree without the game's data, and it must stay.
//! * **The pack is present and a file will not load** - a truncated `economy.toml`, a `sim.toml`
//!   with a contradictory pair - so the fixture **panics**, naming the file and the loader's error.
//!
//! The second rule is the one this module exists for. Thirty-six files in this directory used to
//! write `load_economy_file(..).ok()?`, which turned *every* load error into `None`, which turned
//! `fresh()` into `None`, which turned the `else { return }` at the head of each test into a
//! **passing** test. A broken pack made a third of the suite green instead of red.
//!
//! Each test binary compiles this module for itself, so only some of the entry points are used by
//! any one of them; `dead_code` is allowed here rather than sprinkled over the call sites.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use ruage_data::Resources;
use ruage_sim::World;

/// The seed every hand-copied `fresh()` used. A fixture that needs another seed builds its own
/// world from the pack parts (see the files that do).
const SEED: u64 = 7;

/// A path inside the repository, resolved from this crate's manifest.
pub fn repo(rel: &str) -> PathBuf {
    root().join(rel)
}

/// The repository root: `crates/sim` is two levels down. Canonicalised so that a fixture's panic
/// message reads `/opt/data/workspace/Age2HD/config/economy.toml` rather than `.../crates/sim/../../`.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root exists")
}

/// Which tables a fixture needs beyond the five every fixture loads.
#[derive(Clone, Copy, Default)]
pub struct Pack {
    combat: bool,
    research: bool,
    civs: bool,
}

impl Pack {
    /// Economy, units, map, settings and resources: the four a rule about standing or gathering
    /// needs.
    pub const fn new() -> Self {
        Self {
            combat: false,
            research: false,
            civs: false,
        }
    }

    /// `config/combat.toml`, without which `Command::Attack` is refused with `NoTables`.
    pub const fn combat(mut self) -> Self {
        self.combat = true;
        self
    }

    /// `config/gamedata/techs.toml` and `effects.toml`: an age, a technology, a modifier.
    pub const fn research(mut self) -> Self {
        self.research = true;
        self
    }

    /// `config/gamedata/civs.toml`: what a civilisation may and may not build.
    pub const fn civs(mut self) -> Self {
        self.civs = true;
        self
    }
}

/// The pack every fixture but a handful wants: economy, units and combat, seeded, at [`SEED`].
pub fn fresh() -> Option<World> {
    world(Pack::new().combat())
}

/// A world with no combat tables: for rules that never swing, and for the probes that only walk.
pub fn fresh_plain() -> Option<World> {
    world(Pack::new())
}

/// The pack plus the technology and effect tables: ages, research, modifiers.
pub fn fresh_research() -> Option<World> {
    world(Pack::new().combat().research())
}

/// Everything the game ships: combat, research and the civilisation tables.
pub fn fresh_civs() -> Option<World> {
    world(Pack::new().combat().research().civs())
}

/// The pack with the resource table kept, for a test that asserts on a slot's name.
pub fn fresh_with_resources() -> Option<(World, Resources)> {
    load_from(&root(), Pack::new())
}

fn world(pack: Pack) -> Option<World> {
    load_from(&root(), pack).map(|(world, _)| world)
}

/// Load a pack from `root`. `None` **only** when the generated data is absent.
///
/// The marker is `config/gamedata/units.toml`: a tree without the generated pack has no engine to
/// test against, while a tree *with* it that fails to load one of its hand-written companions is
/// broken and must say so, loudly, naming the file.
pub fn load_from(root: &Path, pack: Pack) -> Option<(World, Resources)> {
    let units_path = root.join("config/gamedata/units.toml");
    if !units_path.exists() {
        return None;
    }
    let at = |rel: &str| root.join(rel);
    let resources = present(
        &at("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = present(&at("config/sim.toml"), ruage_data::load_sim_file);
    let map = present(&at("config/map.toml"), |path| {
        ruage_data::load_map_file(path, &resources)
    });
    let economy = present(&at("config/economy.toml"), ruage_data::load_economy_file);
    let units = present(&units_path, ruage_data::load_units_file);
    let combat = pack
        .combat
        .then(|| present(&at("config/combat.toml"), ruage_data::load_combat_file));
    let research = pack.research.then(|| {
        let techs = present(
            &at("config/gamedata/techs.toml"),
            ruage_data::load_techs_file,
        );
        let effects = present(
            &at("config/gamedata/effects.toml"),
            ruage_data::load_effects_file,
        );
        (techs, effects)
    });
    let civs = pack
        .civs
        .then(|| present(&at("config/gamedata/civs.toml"), ruage_data::load_civs_file));
    let mut world = must("World::new", World::new(&settings, SEED));
    must("seed_from", world.seed_from(&map, &economy, &units));
    must("load_resources", world.load_resources(&resources));
    if let Some(combat) = &combat {
        world.load_combat(combat);
    }
    if let Some((techs, effects)) = &research {
        must("load_research", world.load_research(techs, effects));
    }
    if let Some(civs) = &civs {
        must("load_civs", world.load_civs(civs));
    }
    Some((world, resources))
}

/// **Remember a piece of ground and then look away** (D183 addendum).
///
/// A foundation may be placed blind in **fog** and never in the **black**, so a fixture that builds somewhere off its own
/// base has to have looked there first. A temporary villager of `player` does the looking and is deleted, which leaves the
/// tiles around it `Explored` rather than `Visible`.
pub fn look_away(world: &mut World, player: u8, at: (u16, u16)) {
    use ruage_sim::{Command, EntityId, EntityKind};
    let scale = world.scale();
    let unit = world
        .role_unit("villager")
        .expect("the pack has a villager");
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                ruage_fixed::Fixed::from_raw(i64::from(at.0) * scale + scale / 2),
                ruage_fixed::Fixed::from_raw(i64::from(at.1) * scale + scale / 2),
            ),
            speed: ruage_fixed::Fixed::parse("1.0", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a witness");
    world.step().expect("step");
    world
        .submit_checked(Command::Delete { player, entity: id })
        .expect("the witness is deleted");
    world.step().expect("step");
}

/// Load one table, or fail the fixture with the file's path and the loader's own words.
///
/// This is the rule D130 rests on: a test that cannot read its content has not run, and reporting
/// it as `passed` is worse than reporting it as `failed`.
pub fn present<T, E: std::fmt::Display>(
    path: &Path,
    load: impl FnOnce(&Path) -> Result<T, E>,
) -> T {
    match load(path) {
        Ok(value) => value,
        Err(error) => panic!("{} did not load: {error}", path.display()),
    }
}

/// The same rule for a step that takes no path: the world's own construction and loading.
pub fn must<T, E: std::fmt::Display>(what: &str, outcome: Result<T, E>) -> T {
    match outcome {
        Ok(value) => value,
        Err(error) => panic!("{what} failed: {error}"),
    }
}
