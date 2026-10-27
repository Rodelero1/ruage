//! The civilisations: who a player is, and which effect holds what they cannot have.
//!
//! `civs.toml` is generated (D8) from the dat's civ table. Two fields matter to the engine:
//!
//! * `tech_tree_effect` — the effect whose **kind-102** commands are the techs this civ may *not*
//!   research. That reading is pinned by the data rather than assumed: no civ's list contains Loom,
//!   any age, Wheelbarrow or Forging, which every civ has, and Britain's list contains the upgrade to
//!   the Hussar (428), the Eagle Warrior line (384, 434), the Siege Onager (320) and the Paladin
//!   (265) — all genuine British omissions. A list of what a civ *can* have would include the ages.
//! * `team_bonus_effect` — carried for completeness; nothing applies it yet.
//!
//! The `resources` array (211 numbers of per-civ tuning) is carried as raw numbers and not interpreted:
//! nothing in the engine reads it, and naming its entries from the values alone would be guessing.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{ConfigError, SUPPORTED_SCHEMA};

/// One civilisation.
#[derive(Debug, Clone, PartialEq)]
pub struct Civ {
    pub id: u32,
    pub name: String,
    /// The effect holding this civ's tech-tree restrictions. `None` when the dat says -1.
    pub tech_tree_effect: Option<u32>,
    pub team_bonus_effect: Option<u32>,
    /// Per-civ tuning, as the dat stores it. Uninterpreted; see the module docs.
    pub resources: Vec<f64>,
}

impl Civ {
    /// Is this civ barred from researching `tech`? Only the effect's kind-102 ids matter, and the
    /// effect table is not this structure's business, so the caller resolves them.
    pub fn bars(&self, _tech: u32) -> bool {
        false
    }
}

/// Every civilisation in the pack, in id order, plus a name index for content that names one.
#[derive(Debug, Clone, Default)]
pub struct Civs {
    pub civs: Vec<Civ>,
    by_name: BTreeMap<String, u32>,
}

impl Civs {
    pub fn get(&self, id: u32) -> Option<&Civ> {
        self.civs.iter().find(|civ| civ.id == id)
    }

    /// Look a civ up by the name content uses. Exact match: a near miss is a mistake worth hearing
    /// about, not something to guess at.
    pub fn by_name(&self, name: &str) -> Option<&Civ> {
        self.by_name.get(name).and_then(|id| self.get(*id))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    civ: Vec<RawCiv>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCiv {
    id: u32,
    name: String,
    #[serde(default = "minus_one")]
    tech_tree_effect: i32,
    #[serde(default = "minus_one")]
    team_bonus_effect: i32,
    #[serde(default)]
    resources: Vec<f64>,
}

fn minus_one() -> i32 {
    -1
}

fn effect_id(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
}

pub fn load_civs_str(text: &str, path: &str) -> Result<Civs, ConfigError> {
    let raw: RawFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;
    if raw.schema != SUPPORTED_SCHEMA {
        return Err(ConfigError::Invalid {
            key: format!("{path}: schema"),
            detail: format!("{} is not this engine's schema", raw.schema),
        });
    }

    if !raw.kind.is_empty() && raw.kind != "civs" {
        return Err(ConfigError::Invalid {
            key: format!("{path}: kind"),
            detail: format!("{:?} is not a civ table", raw.kind),
        });
    }

    let mut civs = Vec::with_capacity(raw.civ.len());
    let mut by_name: BTreeMap<String, u32> = BTreeMap::new();
    for civ in raw.civ {
        if let Some(existing) = by_name.insert(civ.name.clone(), civ.id) {
            return Err(ConfigError::Invalid {
                key: format!("{path}: civ[{:?}]", civ.name),
                detail: format!(
                    "two civs are named the same (ids {existing} and {}): content that names a civ \
                     would be ambiguous",
                    civ.id
                ),
            });
        }
        civs.push(Civ {
            id: civ.id,
            name: civ.name,
            tech_tree_effect: effect_id(civ.tech_tree_effect),
            team_bonus_effect: effect_id(civ.team_bonus_effect),
            resources: civ.resources,
        });
    }
    civs.sort_by_key(|civ| civ.id);
    Ok(Civs { civs, by_name })
}

pub fn load_civs_file(path: &Path) -> Result<Civs, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_civs_str(&text, &path.display().to_string())
}
