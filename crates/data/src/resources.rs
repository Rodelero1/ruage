//! `config/resources.toml`: the content pack's resource list.
//!
//! This is the engine's view of resources: a dense list of slots with names, defined by content.
//! The dat's own numbering (`dat_id`) is the reference pack's business and lives here so that a
//! cost entry `[1, 25]` can be translated exactly once, at the pack boundary (D17). A pack of our
//! own has no reference ids at all, and then every `dat_id` lookup misses - which is correct.

use serde::Deserialize;

use crate::{ConfigError, SUPPORTED_SCHEMA};

/// Where a resource of this kind may stand on a map. It is content because "wood grows in forests"
/// is a fact about wood: our own pack may put anything it likes in a forest, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSite {
    /// Open ground: grass, and anything else a map calls walkable.
    #[default]
    Open,
    /// A forest tile. A node the pack places here must stand on one, and a node it does not must
    /// not - a tree you can walk through, or gold inside the treeline, is a map that cannot be
    /// played rather than a map that is merely odd.
    Forest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDef {
    /// The stockpile column. Dense from 0, in file order.
    pub slot: u16,
    pub name: String,
    /// The reference pack's id, when this pack came from one. `None` for resources of our own.
    pub dat_id: Option<u16>,
    /// The terrain a map node of this resource must stand on.
    pub site: ResourceSite,
}

#[derive(Debug, Clone)]
pub struct Resources {
    /// In slot order.
    pub entries: Vec<ResourceDef>,
}

impl Resources {
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn slot_of(&self, name: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|resource| resource.name == name)
    }

    /// The stockpile slot for the *reference pack's* id, or `None` when this pack does not carry
    /// that id. This is where the dat's numbering is translated, so `crates/` never has to know it
    /// (D17): a cost entry `[1, 25]` becomes slot `slot_for_dat_id(1)` and nothing above the pack
    /// layer sees the 1.
    ///
    /// A pack of our own has no reference ids at all, so every lookup returns `None` - which is
    /// correct, and is why callers must handle it rather than defaulting to slot 0.
    pub fn slot_for_dat_id(&self, dat_id: u16) -> Option<usize> {
        self.entries
            .iter()
            .position(|resource| resource.dat_id == Some(dat_id))
    }

    pub fn name(&self, slot: usize) -> Option<&str> {
        self.entries
            .get(slot)
            .map(|resource| resource.name.as_str())
    }

    /// The terrain a node in this column must stand on, or `None` when the pack has no such column.
    pub fn site(&self, slot: usize) -> Option<ResourceSite> {
        self.entries.get(slot).map(|resource| resource.site)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    /// Defaulted so a file with no list reaches our own refusal ("no resources") instead of
    /// serde's ("missing field"), which says less.
    #[serde(default)]
    resource: Vec<RawResource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResource {
    slot: u16,
    name: String,
    #[serde(default)]
    dat_id: Option<u16>,
    /// Omitted means open ground; `site = "forest"` says this resource lives in forests.
    #[serde(default)]
    site: ResourceSite,
}

pub fn load_resources_str(text: &str, path: &str) -> Result<Resources, ConfigError> {
    let raw: RawFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(ConfigError::Invalid {
            key: "schema".to_owned(),
            detail: format!(
                "{} is schema {}, this build reads {}",
                path, raw.schema, SUPPORTED_SCHEMA
            ),
        });
    }
    if raw.resource.is_empty() {
        return Err(ConfigError::Invalid {
            key: "resource".to_owned(),
            detail: "a match with no resources cannot stockpile anything".to_owned(),
        });
    }

    let mut entries: Vec<ResourceDef> = Vec::with_capacity(raw.resource.len());
    for (position, resource) in raw.resource.into_iter().enumerate() {
        let name = resource.name.trim();
        if name.is_empty() {
            return Err(ConfigError::Invalid {
                key: format!("resource[{position}].name"),
                detail: "a resource needs a name".to_owned(),
            });
        }
        // Slots are the stockpile columns: dense from zero, and in file order. Anything else means
        // a column nothing can address or two things fighting for one.
        let want = u16::try_from(position).map_err(|_| ConfigError::Invalid {
            key: format!("resource[{position}].slot"),
            detail: "too many resources for the slot field".to_owned(),
        })?;
        if resource.slot != want {
            return Err(ConfigError::Invalid {
                key: format!("resource[{position}].slot"),
                detail: format!(
                    "slot {} where {} was expected: slots must be dense from 0, in file order",
                    resource.slot, want
                ),
            });
        }
        if entries.iter().any(|existing| existing.name == name) {
            return Err(ConfigError::Invalid {
                key: format!("resource[{position}].name"),
                detail: format!("{name:?} is defined twice"),
            });
        }
        if let Some(dat_id) = resource.dat_id
            && entries
                .iter()
                .any(|existing| existing.dat_id == Some(dat_id))
        {
            return Err(ConfigError::Invalid {
                key: format!("resource[{position}].dat_id"),
                detail: format!("dat id {dat_id} is claimed twice"),
            });
        }
        entries.push(ResourceDef {
            slot: resource.slot,
            name: name.to_owned(),
            dat_id: resource.dat_id,
            site: resource.site,
        });
    }

    Ok(Resources { entries })
}

pub fn load_resources_file(path: &std::path::Path) -> Result<Resources, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_resources_str(&text, &path.display().to_string())
}
