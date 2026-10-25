//! `config/gamedata/techs.toml` and `effects.toml`: research and the effect commands it applies.
//!
//! A tech is a price and a pointer to an effect; an effect is a list of commands. The commands are
//! `[kind, a, b, c, value]` quintuples whose arguments are *not* self-describing, so the meanings
//! this module relies on are pinned by tests against the data (see `crates/data/tests/effects.rs`),
//! not by modding lore:
//!
//! - kind 102's fifth argument is an id that must resolve to a **tech** — so 102 disables techs;
//! - kind 101's second argument is an id that must resolve to a **unit** — so 101 enables/disables
//!   units.
//!
//! Everything else stays `Unknown` until a test can say what it means.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

/// One `[kind, a, b, c, value]` command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectCommand {
    pub kind: i16,
    pub a: i16,
    pub b: i16,
    pub c: i16,
    pub value: MaybeFixed,
}

/// An effect command's meaning, as far as the data has spelled it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Meaning {
    /// Kind 102: `value` is a tech id this effect takes away.
    DisableTech { tech: u32 },
    /// Kind 101: `a` is an id, and the id **cannot** say which namespace it means. Unit ids and
    /// tech ids share a range: 84 of these commands name an id present in both tables and 13 name
    /// one present only in techs, so no id is unit-only (U16). Reading it as a unit would be wrong
    /// exactly where it matters - a "free lumberjack upgrade" would become a free villager.
    AmbiguousId { id: u32 },
    /// A kind nothing has pinned yet. Kept rather than dropped: an effect we cannot read is a fact
    /// about our model, not an absence in the data.
    Unpinned { kind: i16 },
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub id: u32,
    pub name: String,
    pub commands: Vec<EffectCommand>,
}

#[derive(Debug, Clone)]
pub struct Effects {
    /// By id, in id order.
    pub effects: BTreeMap<u32, Effect>,
}

impl Effects {
    pub fn get(&self, id: u32) -> Option<&Effect> {
        self.effects.get(&id)
    }

    /// Every command kind that appears, with how many effects use it. This is the census the
    /// interpreter is written from: a kind that never appears does not need implementing, and the
    /// kinds that do appear are counted rather than assumed.
    pub fn census(&self) -> BTreeMap<i16, usize> {
        let mut counts: BTreeMap<i16, usize> = BTreeMap::new();
        for effect in self.effects.values() {
            for command in &effect.commands {
                *counts.entry(command.kind).or_default() += 1;
            }
        }
        counts
    }

    pub fn total_commands(&self) -> usize {
        self.effects.values().map(|e| e.commands.len()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct Tech {
    pub id: u32,
    /// `None` when the dat says -1: a tech that applies nothing. Present in the data, so it is
    /// representable rather than impossible.
    pub effect_id: Option<u32>,
    pub research_time: i32,
    pub costs: Vec<(u8, u16)>,
    pub required: Vec<i32>,
    pub location: i16,
}

#[derive(Debug, Clone)]
pub struct Techs {
    /// By id, in id order.
    pub techs: BTreeMap<u32, Tech>,
}

impl Techs {
    pub fn get(&self, id: u32) -> Option<&Tech> {
        self.techs.get(&id)
    }
}

// --- wire format ------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEffects {
    schema: u32,
    kind: String,
    effect: Vec<RawEffect>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEffect {
    id: u32,
    name: String,
    commands: Vec<[f64; 5]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTechs {
    schema: u32,
    kind: String,
    tech: Vec<RawTech>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTech {
    id: u32,
    effect_id: i32,
    research_time: i32,
    costs: Vec<[f64; 2]>,
    required: Vec<i32>,
    location: i16,
}

fn check_header(schema: u32, kind: &str, want: &str, path: &str) -> Result<(), ConfigError> {
    if schema != SUPPORTED_SCHEMA {
        return Err(ConfigError::Invalid {
            key: "schema".to_owned(),
            detail: format!(
                "{} is schema {schema}, this build reads {SUPPORTED_SCHEMA}",
                path
            ),
        });
    }
    if kind != want {
        return Err(ConfigError::Invalid {
            key: "kind".to_owned(),
            detail: format!("{} says kind {kind:?}, expected {want:?}", path),
        });
    }
    Ok(())
}

pub fn load_effects_str(text: &str, path: &str) -> Result<Effects, ConfigError> {
    let raw: RawEffects = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;
    check_header(raw.schema, &raw.kind, "effects", path)?;

    let mut effects = BTreeMap::new();
    for effect in raw.effect {
        let mut commands = Vec::with_capacity(effect.commands.len());
        for quintuple in effect.commands {
            let [kind, a, b, c, value] = quintuple;
            // The first four fields are integers in the file even though they are written as
            // floats; a non-integral one means the layout is not what we think it is.
            let int = |what: &str, number: f64| -> Result<i16, ConfigError> {
                if number.fract() != 0.0 {
                    return Err(ConfigError::Invalid {
                        key: format!("effect[{}].commands.{what}", effect.id),
                        detail: format!("{number} is not an integer argument"),
                    });
                }
                i16::try_from(number as i64).map_err(|_| ConfigError::Invalid {
                    key: format!("effect[{}].commands.{what}", effect.id),
                    detail: format!("{number} does not fit in the field"),
                })
            };
            commands.push(EffectCommand {
                kind: int("kind", kind)?,
                a: int("a", a)?,
                b: int("b", b)?,
                c: int("c", c)?,
                value: MaybeFixed::from_f64(value),
            });
        }
        if effects
            .insert(
                effect.id,
                Effect {
                    id: effect.id,
                    name: effect.name,
                    commands,
                },
            )
            .is_some()
        {
            return Err(ConfigError::Invalid {
                key: format!("effect[{}]", effect.id),
                detail: "defined twice".to_owned(),
            });
        }
    }
    Ok(Effects { effects })
}

pub fn load_effects_file(path: &std::path::Path) -> Result<Effects, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_effects_str(&text, &path.display().to_string())
}

pub fn load_techs_str(text: &str, path: &str) -> Result<Techs, ConfigError> {
    let raw: RawTechs = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;
    check_header(raw.schema, &raw.kind, "techs", path)?;

    let mut techs = BTreeMap::new();
    for tech in raw.tech {
        if tech.research_time < 0 {
            return Err(ConfigError::Invalid {
                key: format!("tech[{}].research_time", tech.id),
                detail: format!(
                    "{} is negative: -1 usually means \"not researchable\", which is a rule this \
                     loader should not invent",
                    tech.research_time
                ),
            });
        }
        let mut costs = Vec::with_capacity(tech.costs.len());
        for pair in tech.costs {
            let [resource, amount] = pair;
            let resource = u8::try_from(resource as i64).map_err(|_| ConfigError::Invalid {
                key: format!("tech[{}].costs", tech.id),
                detail: format!("resource id {resource} is not a byte"),
            })?;
            let amount = u16::try_from(amount as i64).map_err(|_| ConfigError::Invalid {
                key: format!("tech[{}].costs", tech.id),
                detail: format!("amount {amount} is not a cost"),
            })?;
            costs.push((resource, amount));
        }
        if techs
            .insert(
                tech.id,
                Tech {
                    id: tech.id,
                    effect_id: u32::try_from(tech.effect_id).ok(),
                    research_time: tech.research_time,
                    costs,
                    required: tech.required,
                    location: tech.location,
                },
            )
            .is_some()
        {
            return Err(ConfigError::Invalid {
                key: format!("tech[{}]", tech.id),
                detail: "defined twice".to_owned(),
            });
        }
    }
    Ok(Techs { techs })
}

pub fn load_techs_file(path: &std::path::Path) -> Result<Techs, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_techs_str(&text, &path.display().to_string())
}

impl Effects {
    /// What a command is, as far as the data can say. Tech and unit ids are checked for existence
    /// by the caller, which is what turns "kind 102" from a guess into a meaning.
    pub fn meaning(command: &EffectCommand, techs: &Techs) -> Meaning {
        match command.kind {
            102 => {
                let id = command.value.resolve(1_000, "effect.value").ok();
                let id = id.and_then(|fixed| u32::try_from(fixed.raw() / 1_000).ok());
                match id {
                    Some(id) if techs.get(id).is_some() => Meaning::DisableTech { tech: id },
                    _ => Meaning::Unpinned { kind: 102 },
                }
            }
            101 => {
                let id = u32::try_from(command.a).unwrap_or(0);
                Meaning::AmbiguousId { id }
            }
            other => Meaning::Unpinned { kind: other },
        }
    }
}
