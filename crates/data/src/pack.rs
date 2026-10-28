//! The content pack: every file a match depends on, loaded together and identified as one thing.
//!
//! Two players in a LAN match must be running the same content, or they desync within a minute and
//! blame the network. The pack therefore carries a **content hash** — a fingerprint of the bytes of
//! its files, in a fixed order — which goes into the match handshake next to `tick_rate_hz` and
//! `fixed_point_scale` (D16). Peers that disagree refuse the match instead of playing a game that
//! will quietly diverge.
//!
//! The hash is over **file bytes, not parsed values**, deliberately:
//!
//! - it is what "are we running the same data" actually means, and it cannot be defeated by a field
//!   the loader ignores;
//! - it is cheap and needs no stable serialisation of every struct;
//! - its cost is real and accepted: editing a comment changes the hash, so a modded pack must be
//!   re-published rather than tweaked in place. That is the correct behaviour for a shared pack.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{
    Combat, ConfigError, Economy, Effects, Map, Resources, Sim, Techs, Units, load_combat_file,
    load_economy_file, load_effects_file, load_map_file, load_resources_file, load_sim_file,
    load_techs_file, load_units_file,
};

/// Every file a match reads. The order here is the order the hash is computed in, so adding a file
/// changes every pack's hash - which is correct, because a build that needs a file an older build
/// did not is not the same content.
pub const PACK_FILES: [&str; 8] = [
    "sim.toml",
    "economy.toml",
    "map.toml",
    "units.toml",
    "combat.toml",
    "resources.toml",
    // Research is content too: a tech's price and timer, and the commands its effect applies. A
    // peer running different techs is playing a different game, so the handshake must see them.
    "techs.toml",
    "effects.toml",
];

/// Where a pack file lives relative to the config directory. `units.toml` is under `gamedata/`
/// because it is generated (D8) while the rest are hand-written; a pack is one thing to a handshake
/// regardless of how the directory is arranged on disk.
pub fn pack_path(dir: &Path, name: &str) -> PathBuf {
    match name {
        // Generated from the game data (D8), and therefore under `gamedata/`.
        "units.toml" | "techs.toml" | "effects.toml" => dir.join("gamedata").join(name),
        other => dir.join(other),
    }
}

#[derive(Debug, Clone)]
pub struct ContentPack {
    pub sim: Sim,
    pub economy: Economy,
    pub map: Map,
    pub units: Units,
    pub combat: Combat,
    pub resources: Resources,
    /// Research: the tech table and the effect commands it points at.
    pub techs: Techs,
    pub effects: Effects,
    /// FNV-1a over `(name, length, bytes)` for each file in `PACK_FILES` order.
    pub hash: u64,
    /// Which files went into the hash, so a handshake can explain a mismatch instead of saying
    /// "different hash".
    pub parts: BTreeMap<String, u64>,
    /// Hand-written values layered over the generated tables, and one line per value replaced. Empty
    /// when the pack has no `overrides.toml` (D45).
    pub override_log: Vec<String>,
}

impl ContentPack {
    /// The identity a handshake compares. Hex because logs and chat messages are its usual home.
    pub fn identity(&self) -> String {
        format!("{:016x}", self.hash)
    }

    /// Load every file in `dir`, hashing as it goes.
    ///
    /// `units.toml` is generated from the game data and is not in version control (D8), so a pack
    /// cannot be built from a bare clone until the generator has run. The error says which file is
    /// missing rather than "no such file" from deep inside a loader.
    pub fn load(dir: &Path) -> Result<Self, ConfigError> {
        let mut parts = BTreeMap::new();
        let mut bytes: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
        for name in PACK_FILES {
            let path: PathBuf = pack_path(dir, name);
            let data = std::fs::read(&path).map_err(|err| ConfigError::Io {
                path: path.display().to_string(),
                detail: format!(
                    "{name} is part of the content pack; if it is generated, run the generator \
                     first ({err})"
                ),
            })?;
            parts.insert(name.to_owned(), fnv1a(&data));
            bytes.insert(name, data);
        }

        let sim = load_sim_file(&dir.join("sim.toml"))?;
        let economy = load_economy_file(&dir.join("economy.toml"))?;
        // Resources before the map: the map names its resources, and only the pack can turn a name
        // into a stockpile column (D17).
        let resources = load_resources_file(&dir.join("resources.toml"))?;
        let map = load_map_file(&dir.join("map.toml"), &resources)?;
        let mut units = load_units_file(&pack_path(dir, "units.toml"))?;
        let combat = load_combat_file(&dir.join("combat.toml"))?;
        let techs = load_techs_file(&pack_path(dir, "techs.toml"))?;
        let effects = load_effects_file(&pack_path(dir, "effects.toml"))?;

        // Hand-written overrides, applied after everything is loaded. The file is optional - a pack
        // written before the layer existed has none, and must keep its hash - but when it is there it
        // is part of the pack's identity: two peers with different overrides are playing different
        // games, which is exactly what the handshake exists to refuse (D16, D45).
        let mut override_log = Vec::new();
        let overrides_path = dir.join("overrides.toml");
        if overrides_path.exists() {
            let data = std::fs::read(&overrides_path).map_err(|err| ConfigError::Io {
                path: overrides_path.display().to_string(),
                detail: err.to_string(),
            })?;
            parts.insert("overrides.toml".to_owned(), fnv1a(&data));
            let overrides = crate::load_overrides_file(&overrides_path)?;
            override_log = overrides.apply_units(&mut units)?;
        }

        // The pack hash mixes the per-file hashes *with their names*, so two files swapping
        // contents cannot produce the same pack hash as an honest pack.
        let mut hash = 0xcbf2_9ce4_8422_2325;
        for name in PACK_FILES {
            hash = fnv1a_update(hash, name.as_bytes());
            let part = parts[name];
            hash = fnv1a_update(hash, &part.to_le_bytes());
        }
        // `overrides.toml` is in the hash only when it exists, so a pack without it hashes exactly as
        // it did before this layer existed.
        if let Some(part) = parts.get("overrides.toml") {
            hash = fnv1a_update(hash, b"overrides.toml");
            hash = fnv1a_update(hash, &part.to_le_bytes());
        }

        Ok(ContentPack {
            sim,
            economy,
            map,
            units,
            combat,
            resources,
            techs,
            effects,
            hash,
            parts,
            override_log,
        })
    }

    /// Files whose hash differs from another pack's. Empty means the two agree.
    pub fn differences(&self, other: &ContentPack) -> Vec<String> {
        let mut out = Vec::new();
        for name in PACK_FILES.iter().copied().chain(["overrides.toml"]) {
            let mine = self.parts.get(name).copied();
            let theirs = other.parts.get(name).copied();
            if mine != theirs {
                out.push(name.to_owned());
            }
        }
        out
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn fnv1a(bytes: &[u8]) -> u64 {
    fnv1a_update(FNV_OFFSET, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_mixes_names_and_contents() {
        // Two files with swapped contents must not hash the same as the honest arrangement: a pack
        // that renames its files is a different pack.
        let a = fnv1a(b"one");
        let b = fnv1a(b"two");
        let honest = fnv1a_update(fnv1a_update(FNV_OFFSET, b"x"), &a.to_le_bytes());
        let honest = fnv1a_update(honest, b"y");
        let honest = fnv1a_update(honest, &b.to_le_bytes());
        let swapped = fnv1a_update(fnv1a_update(FNV_OFFSET, b"x"), &b.to_le_bytes());
        let swapped = fnv1a_update(swapped, b"y");
        let swapped = fnv1a_update(swapped, &a.to_le_bytes());
        assert_ne!(honest, swapped);
    }
}
