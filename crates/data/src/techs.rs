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

use crate::cost::{Cost, RawCost};
use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

/// What a modifier does to the value it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModOp {
    Add,
    Multiply,
}

/// A kind 4 or 5 command, **decoded by the importer**: the dat packs an attack or armour change as
/// `class * 256 + amount` (Forging's 1025 is class 4, +1), and that packing is format knowledge, so
/// it is undone in `ruage-tools` and the engine only ever sees the class and the amount (D17).
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub op: ModOp,
    /// The engine's own vocabulary, as the importer names it: `attack`, `armour`, `hit_points`,
    /// `range`, `speed`, `gather_rate`, `carry`, or `other:<id>` / `unpacked:<id>` for a command the
    /// importer could not decode. An unknown name is carried, not discarded: it is a gap in the
    /// model, and silently dropping it would make a tech look fully applied when it is not.
    pub attribute: String,
    /// The attack/armour class, when the attribute is one of those; -1 otherwise.
    pub attribute_class: i16,
    /// Target unit id, -1 for every unit.
    pub unit: i32,
    /// Target unit class, -1 for every class.
    pub unit_class: i16,
    pub value: f64,
}

/// One `[kind, a, b, c, value]` command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectCommand {
    pub kind: i16,
    pub a: i16,
    pub b: i16,
    pub c: i16,
    pub value: MaybeFixed,
}

/// The values an effect can change, as far as the techs that use them show. The ids come from the
/// data: 0 is hit points (Loom adds 15, Masonry multiplies by 1.1), 1 is range (Fletching adds 1),
/// 8 is armour and 9 is attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    HitPoints,
    Range,
    Armour,
    /// Attack is **packed**: the value is `class * 256 + amount`, so Forging's 1025 is class 4
    /// (melee) plus 1. Any other reading makes the numbers meaningless - a "+1025 attack" upgrade.
    Attack {
        class: u8,
        amount: u8,
    },
    Other(i16),
}

impl Attribute {
    /// `None` for an attribute id nothing has pinned: an unknown attribute is a gap in our model,
    /// not a licence to write it down as something familiar.
    pub fn from_id(id: i16) -> Option<Self> {
        Some(match id {
            0 => Attribute::HitPoints,
            1 => Attribute::Range,
            8 => Attribute::Armour,
            9 => Attribute::Attack {
                class: 0,
                amount: 0,
            },
            _ => return None,
        })
    }

    /// Split a packed attack value. `None` when the value does not fit the packing, which is the
    /// check that keeps the reading honest: 1025 is class 4 + 1, and 1025.5 is not attack at all.
    pub fn attack_from_packed(value: i32) -> Option<Attribute> {
        if !(0..=65_535).contains(&value) {
            return None;
        }
        let class = u8::try_from((value >> 8) & 0xff).ok()?;
        let amount = u8::try_from(value & 0xff).ok()?;
        Some(Attribute::Attack { class, amount })
    }
}

/// What an attribute command applies to: one unit, a whole class, or both unset for "everything the
/// tech's own prerequisites allow".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub unit: Option<u32>,
    pub class: Option<i16>,
}

/// An effect command's meaning, as far as the data has spelled it out.
/// `PartialEq` only: values are fixed point, which has no `Eq`.
#[derive(Debug, Clone, PartialEq)]
pub enum Meaning {
    /// Kind 102: `value` is a tech id this effect takes away.
    DisableTech { tech: u32 },
    /// Kind 101: `a` is an id, and the id **cannot** say which namespace it means. Unit ids and
    /// tech ids share a range: 84 of these commands name an id present in both tables and 13 name
    /// one present only in techs, so no id is unit-only (U16). Reading it as a unit would be wrong
    /// exactly where it matters - a "free lumberjack upgrade" would become a free villager.
    AmbiguousId { id: u32 },
    /// Kind 4: add `value` to `attribute` on `target`. Pinned by Loom (+15 hp on class 4, the
    /// civilians), Forging (+1 melee attack on class 6, the infantry) and Fletching (+1 range and
    /// +1 pierce attack on class 0).
    AddAttribute {
        target: Target,
        attribute: i16,
        value: MaybeFixed,
    },
    /// Kind 5: multiply `attribute` by `value` on `target`. Pinned by Masonry (buildings x1.1 hp,
    /// walls x1.1 hp) - a multiplier, not an addition, or Masonry would add 1.1 hit points.
    MultiplyAttribute {
        target: Target,
        attribute: i16,
        value: MaybeFixed,
    },
    /// A kind nothing has pinned yet. Kept rather than dropped: an effect we cannot read is a fact
    /// about our model, not an absence in the data.
    Unpinned { kind: i16 },
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub id: u32,
    pub name: String,
    /// The raw commands, kept as the record of what the dat said.
    pub commands: Vec<EffectCommand>,
    /// The same commands, decoded (kind 4 and 5 only): what an interpreter actually applies.
    pub modifiers: Vec<Modifier>,
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
    /// Priced like a unit's (D17): reference ids from the generated table, or names a pack defines.
    pub costs: Vec<Cost>,
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
    /// Written by the importer for kinds 4 and 5. Defaulted so a pack written before the decode
    /// existed still loads - and then simply has no modifiers to apply.
    #[serde(default)]
    modifiers: Vec<RawModifier>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModifier {
    op: String,
    attribute: String,
    attribute_class: i16,
    unit: i32,
    unit_class: i16,
    value: f64,
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
    costs: Vec<RawCost>,
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
        let mut modifiers = Vec::with_capacity(effect.modifiers.len());
        for raw in effect.modifiers {
            let op = match raw.op.as_str() {
                "add" => ModOp::Add,
                "multiply" => ModOp::Multiply,
                other => {
                    return Err(ConfigError::Invalid {
                        key: format!("effect[{}].modifiers.op", effect.id),
                        detail: format!("{other:?} is neither \"add\" nor \"multiply\""),
                    });
                }
            };
            if raw.attribute.trim().is_empty() {
                return Err(ConfigError::Invalid {
                    key: format!("effect[{}].modifiers.attribute", effect.id),
                    detail: "a modifier needs an attribute".to_owned(),
                });
            }
            if !raw.value.is_finite() {
                return Err(ConfigError::Invalid {
                    key: format!("effect[{}].modifiers.value", effect.id),
                    detail: format!("{} is not a number", raw.value),
                });
            }
            modifiers.push(Modifier {
                op,
                attribute: raw.attribute.trim().to_owned(),
                attribute_class: raw.attribute_class,
                unit: raw.unit,
                unit_class: raw.unit_class,
                value: raw.value,
            });
        }
        if effects
            .insert(
                effect.id,
                Effect {
                    id: effect.id,
                    name: effect.name,
                    commands,
                    modifiers,
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
        let costs = tech
            .costs
            .iter()
            .map(|c| crate::cost::convert(c, || format!("tech[{}].costs", tech.id)))
            .collect::<Result<Vec<_>, ConfigError>>()?;
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
            4 | 5 => {
                let attribute = command.c;
                if Attribute::from_id(attribute).is_none() && attribute != 9 {
                    return Meaning::Unpinned { kind: command.kind };
                }
                let target = Target {
                    unit: u32::try_from(command.a).ok(),
                    class: (command.b >= 0).then_some(command.b),
                };
                if command.kind == 4 {
                    Meaning::AddAttribute {
                        target,
                        attribute,
                        value: command.value,
                    }
                } else {
                    Meaning::MultiplyAttribute {
                        target,
                        attribute,
                        value: command.value,
                    }
                }
            }
            other => Meaning::Unpinned { kind: other },
        }
    }
}
