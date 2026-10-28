//! `config/overrides.toml`: hand-written values layered on top of the generated tables (D6, D45).
//!
//! The point of the layer is that an experiment survives regeneration: `gamedata/` is thrown away and
//! rewritten by the importer, and anything a human changed by hand in it would be lost. The file is
//! path-keyed (`"units.<id>.<field>"`), applies **after** loading, and every value it replaces is reported
//! with its old and new form, because an override nobody can see is indistinguishable from a bug.
//!
//! Two deliberate restrictions:
//!
//! - **A path the layer does not cover is refused**, not ignored. The whole failure mode this file exists to
//!   prevent is "I edited the config and nothing happened".
//! - **Arrays are replaced wholesale.** Merging two attack arrays is almost always a mistake, and a partial
//!   merge would be invisible in the running game.
//!
//! Units are covered today. `techs` and `effects` are named in the refusal rather than silently accepted, so
//! widening the layer is a change here and not a surprise elsewhere.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{ConfigError, MaybeFixed, Units};

/// One value from the override file, as TOML gave it to us.
#[derive(Debug, Clone)]
enum Value {
    Integer(i64),
    Float(f64),
    Array(Vec<Vec<i64>>),
    Other(&'static str),
}

#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// Path → value, in the file's own order of declaration sorted by key, so two runs report the same
    /// thing in the same order.
    entries: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    #[serde(default)]
    overrides: BTreeMap<String, toml::Value>,
}

impl Overrides {
    /// Parse an override file. `path` is only for error messages.
    pub fn parse(text: &str, path: &str) -> Result<Self, ConfigError> {
        let raw: RawFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
            path: path.to_owned(),
            detail: err.to_string(),
        })?;
        if raw.schema != crate::SUPPORTED_SCHEMA {
            return Err(ConfigError::Invalid {
                key: format!("{path}: schema"),
                detail: format!(
                    "{} is not the schema this build reads ({})",
                    raw.schema,
                    crate::SUPPORTED_SCHEMA
                ),
            });
        }
        let mut entries = BTreeMap::new();
        for (key, value) in raw.overrides {
            let value = match value {
                toml::Value::Integer(number) => Value::Integer(number),
                toml::Value::Float(number) => Value::Float(number),
                toml::Value::Array(items) => {
                    let mut rows = Vec::with_capacity(items.len());
                    for item in items {
                        let toml::Value::Array(pair) = item else {
                            return Err(ConfigError::Invalid {
                                key: format!("{path}: {key}"),
                                detail: "an array override must be an array of arrays".to_owned(),
                            });
                        };
                        let mut row = Vec::with_capacity(pair.len());
                        for cell in pair {
                            let toml::Value::Integer(number) = cell else {
                                return Err(ConfigError::Invalid {
                                    key: format!("{path}: {key}"),
                                    detail: "an array override holds whole numbers only".to_owned(),
                                });
                            };
                            row.push(number);
                        }
                        rows.push(row);
                    }
                    Value::Array(rows)
                }
                other => Value::Other(other.type_str()),
            };
            entries.insert(key, value);
        }
        Ok(Self { entries })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The paths this file overrides, for a log line or a test.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Apply every `units.<id>.<field>` override, returning one line per change: `key: old -> new`.
    pub fn apply_units(&self, units: &mut Units) -> Result<Vec<String>, ConfigError> {
        let mut log = Vec::new();
        for (key, value) in &self.entries {
            let parts: Vec<&str> = key.split('.').collect();
            let (table, id, field) = match parts.as_slice() {
                [table, id, field] => (*table, *id, *field),
                _ => {
                    return Err(ConfigError::Invalid {
                        key: key.clone(),
                        detail: "an override path is `<table>.<id>.<field>`".to_owned(),
                    });
                }
            };
            if table != "units" {
                return Err(ConfigError::Invalid {
                    key: key.clone(),
                    detail: format!(
                        "`{table}` is not covered by the override layer yet; units are. \
                         Widening it is a change in crates/data/src/overrides.rs, not a key to guess"
                    ),
                });
            }
            let id: u32 = id.parse().map_err(|_| ConfigError::Invalid {
                key: key.clone(),
                detail: format!("`{id}` is not a unit id"),
            })?;
            let unit = units.unit_mut(id).ok_or_else(|| ConfigError::Invalid {
                key: key.clone(),
                detail: format!("there is no unit {id} in the table"),
            })?;
            let old = match field {
                "hp" => replace_whole(key, &mut unit.hp, value)?,
                "carry" => replace_whole(key, &mut unit.carry, value)?,
                "garrison" => replace_whole(key, &mut unit.garrison, value)?,
                "accuracy" => replace_whole(key, &mut unit.accuracy, value)?,
                "train_time" => replace_whole(key, &mut unit.train_time, value)?,
                "los" => replace_scaled(key, &mut unit.los, value)?,
                "speed" => replace_scaled(key, &mut unit.speed, value)?,
                "work_rate" => replace_scaled(key, &mut unit.work_rate, value)?,
                "range" => replace_scaled(key, &mut unit.range, value)?,
                "reload" => replace_scaled(key, &mut unit.reload, value)?,
                "min_range" => replace_scaled(key, &mut unit.min_range, value)?,
                "attacks" => replace_pairs(key, &mut unit.attacks, value)?,
                "armours" => replace_pairs(key, &mut unit.armours, value)?,
                other => {
                    return Err(ConfigError::Invalid {
                        key: key.clone(),
                        detail: format!(
                            "`{other}` is not a unit field the override layer covers. It covers the \
                             numbers a match depends on: hp, los, speed, carry, work_rate, range, \
                             reload, min_range, accuracy, garrisation, train_time, attacks, armours"
                        ),
                    });
                }
            };
            log.push(format!("{key}: {old} -> {value:?}"));
        }
        Ok(log)
    }
}

fn replace_whole<T>(key: &str, slot: &mut T, value: &Value) -> Result<String, ConfigError>
where
    T: Copy + std::fmt::Debug + TryFrom<i64>,
{
    let Value::Integer(number) = *value else {
        return Err(mismatch(key, "a whole number", value));
    };
    let old = format!("{slot:?}");
    let converted = T::try_from(number).map_err(|_| ConfigError::Invalid {
        key: key.to_owned(),
        detail: format!("{number} does not fit the field"),
    })?;
    *slot = converted;
    Ok(old)
}

fn replace_scaled(key: &str, slot: &mut MaybeFixed, value: &Value) -> Result<String, ConfigError> {
    let number = match *value {
        Value::Integer(number) => number as f64,
        Value::Float(number) => number,
        _ => return Err(mismatch(key, "a number", value)),
    };
    let old = format!("{slot:?}");
    *slot = MaybeFixed::from_f64(number);
    Ok(old)
}

fn replace_pairs(
    key: &str,
    slot: &mut Vec<(i16, i16)>,
    value: &Value,
) -> Result<String, ConfigError> {
    let Value::Array(rows) = value else {
        return Err(mismatch(key, "an array of [class, amount] pairs", value));
    };
    let mut pairs = Vec::with_capacity(rows.len());
    for row in rows {
        let [class, amount] = row.as_slice() else {
            return Err(mismatch(key, "an array of [class, amount] pairs", value));
        };
        let class = i16::try_from(*class).map_err(|_| ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!("{class} is not a class id"),
        })?;
        let amount = i16::try_from(*amount).map_err(|_| ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!("{amount} is not an amount"),
        })?;
        pairs.push((class, amount));
    }
    let old = format!("{slot:?}");
    *slot = pairs;
    Ok(old)
}

fn mismatch(key: &str, wanted: &str, value: &Value) -> ConfigError {
    // A refusal that says what it *got* is worth more than one that only says what it wanted: a TOML
    // string where a number belongs is the mistake users actually make.
    let found = match value {
        Value::Integer(_) => "a whole number",
        Value::Float(_) => "a number with a fraction",
        Value::Array(_) => "an array",
        Value::Other(kind) => kind,
    };
    ConfigError::Invalid {
        key: key.to_owned(),
        detail: format!("expected {wanted}, found {found}"),
    }
}

pub fn load_overrides_str(text: &str, path: &str) -> Result<Overrides, ConfigError> {
    Overrides::parse(text, path)
}

pub fn load_overrides_file(path: &Path) -> Result<Overrides, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    Overrides::parse(&text, &path.display().to_string())
}
