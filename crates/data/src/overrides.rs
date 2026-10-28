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

use crate::{ConfigError, MaybeFixed, Unit, Units};

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
    ///
    /// **Two passes, on purpose.** The first resolves and validates every entry without touching the table; the
    /// second writes. A one-pass version applied the entries before the first bad one and then returned an error,
    /// leaving a half-loaded table behind - the exact failure a strict loader exists to prevent.
    pub fn apply_units(&self, units: &mut Units) -> Result<Vec<String>, ConfigError> {
        let mut plan: Vec<(u32, Field, Change)> = Vec::with_capacity(self.entries.len());
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
            let field = Field::parse(field).ok_or_else(|| ConfigError::Invalid {
                key: key.clone(),
                detail:
                    "not a unit field the override layer covers. It covers the numbers a match \
                         depends on: hp, los, speed, carry, work_rate, range, reload, min_range, \
                         accuracy, garrison, train_time, attacks, armours"
                        .to_owned(),
            })?;
            let unit = units.unit(id).ok_or_else(|| ConfigError::Invalid {
                key: key.clone(),
                detail: format!("there is no unit {id} in the table"),
            })?;
            let change = field.resolve(key, value, unit)?;
            plan.push((id, field, change));
        }

        // Nothing above this line wrote anything, so the table is either wholly old or wholly new.
        let mut log = Vec::with_capacity(plan.len());
        for (id, field, change) in plan {
            let old = change.old();
            let unit = units.unit_mut(id).ok_or_else(|| ConfigError::Invalid {
                key: format!("units.{id}"),
                detail: "the unit vanished between validation and application".to_owned(),
            })?;
            change.write(unit);
            log.push(format!(
                "units.{id}.{}: {old} -> {written}",
                field.name(),
                written = change.written()
            ));
        }
        Ok(log)
    }
}

/// A writable unit field, named as the override path names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Hp,
    Los,
    Speed,
    Carry,
    WorkRate,
    Range,
    Reload,
    MinRange,
    Accuracy,
    Garrison,
    TrainTime,
    Attacks,
    Armours,
}

impl Field {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "hp" => Self::Hp,
            "los" => Self::Los,
            "speed" => Self::Speed,
            "carry" => Self::Carry,
            "work_rate" => Self::WorkRate,
            "range" => Self::Range,
            "reload" => Self::Reload,
            "min_range" => Self::MinRange,
            "accuracy" => Self::Accuracy,
            "garrison" => Self::Garrison,
            "train_time" => Self::TrainTime,
            "attacks" => Self::Attacks,
            "armours" => Self::Armours,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Hp => "hp",
            Self::Los => "los",
            Self::Speed => "speed",
            Self::Carry => "carry",
            Self::WorkRate => "work_rate",
            Self::Range => "range",
            Self::Reload => "reload",
            Self::MinRange => "min_range",
            Self::Accuracy => "accuracy",
            Self::Garrison => "garrison",
            Self::TrainTime => "train_time",
            Self::Attacks => "attacks",
            Self::Armours => "armours",
        }
    }

    /// Turn a TOML value into the change this field accepts, or refuse it by name.
    fn resolve(self, key: &str, value: &Value, unit: &Unit) -> Result<Change, ConfigError> {
        let change = match self {
            Self::Hp => {
                let hp = whole(key, value, "hit points")?;
                if hp <= 0 {
                    // Content may lower a unit's health; it may not take it away. A unit with no health is
                    // dead on arrival, and every maximum-health computation would floor it back up again.
                    return Err(ConfigError::Invalid {
                        key: key.to_owned(),
                        detail: format!("{hp} hit points is not a unit that can exist"),
                    });
                }
                Change::Hp(hp, unit.hp)
            }
            Self::Carry => Change::Carry(whole(key, value, "carry")?, unit.carry),
            Self::Garrison => {
                Change::Garrison(whole(key, value, "garrison spaces")?, unit.garrison)
            }
            Self::TrainTime => Change::TrainTime(whole(key, value, "seconds")?, unit.train_time),
            Self::Accuracy => {
                let accuracy = whole(key, value, "accuracy percent")?;
                if !(0..=100).contains(&accuracy) {
                    return Err(ConfigError::Invalid {
                        key: key.to_owned(),
                        detail: format!("{accuracy} is not a percentage"),
                    });
                }
                Change::Accuracy(accuracy, unit.accuracy)
            }
            Self::Los => Change::Los(number(key, value)?, unit.los),
            Self::Speed => Change::Speed(number(key, value)?, unit.speed),
            Self::WorkRate => Change::WorkRate(number(key, value)?, unit.work_rate),
            Self::Range => Change::Range(number(key, value)?, unit.range),
            Self::Reload => Change::Reload(number(key, value)?, unit.reload),
            Self::MinRange => {
                let min_range = number(key, value)?;
                if min_range < 0.0 {
                    return Err(ConfigError::Invalid {
                        key: key.to_owned(),
                        detail: format!("{min_range} is not a distance"),
                    });
                }
                Change::MinRange(min_range, unit.min_range)
            }
            Self::Attacks => Change::Attacks(pairs(key, value)?, unit.attacks.clone()),
            Self::Armours => Change::Armours(pairs(key, value)?, unit.armours.clone()),
        };
        Ok(change)
    }
}

/// One validated change: the new value and what it replaces, so the log can say both.
#[derive(Debug)]
enum Change {
    Hp(i32, i32),
    Carry(u16, u16),
    Garrison(u16, u16),
    TrainTime(i32, i32),
    Accuracy(u16, u16),
    Los(f64, MaybeFixed),
    Speed(f64, MaybeFixed),
    WorkRate(f64, MaybeFixed),
    Range(f64, MaybeFixed),
    Reload(f64, MaybeFixed),
    MinRange(f64, MaybeFixed),
    Attacks(Vec<(i16, i16)>, Vec<(i16, i16)>),
    Armours(Vec<(i16, i16)>, Vec<(i16, i16)>),
}

impl Change {
    fn old(&self) -> String {
        match self {
            Self::Hp(..)
            | Self::Carry(..)
            | Self::Garrison(..)
            | Self::TrainTime(..)
            | Self::Accuracy(..) => match self {
                Self::Hp(_, old) | Self::TrainTime(_, old) => old.to_string(),
                Self::Carry(_, old) | Self::Garrison(_, old) | Self::Accuracy(_, old) => {
                    old.to_string()
                }
                _ => String::new(),
            },
            Self::Los(_, old)
            | Self::Speed(_, old)
            | Self::WorkRate(_, old)
            | Self::Range(_, old)
            | Self::Reload(_, old)
            | Self::MinRange(_, old) => format!("{old:?}"),
            Self::Attacks(_, old) | Self::Armours(_, old) => format!("{old:?}"),
        }
    }

    fn written(&self) -> String {
        match self {
            Self::Hp(new, _) | Self::TrainTime(new, _) => new.to_string(),
            Self::Carry(new, _) | Self::Garrison(new, _) | Self::Accuracy(new, _) => {
                new.to_string()
            }
            Self::Los(new, _)
            | Self::Speed(new, _)
            | Self::WorkRate(new, _)
            | Self::Range(new, _)
            | Self::Reload(new, _)
            | Self::MinRange(new, _) => new.to_string(),
            Self::Attacks(new, _) | Self::Armours(new, _) => format!("{new:?}"),
        }
    }

    fn write(&self, unit: &mut Unit) {
        match self {
            Self::Hp(new, _) => unit.hp = *new,
            Self::Carry(new, _) => unit.carry = *new,
            Self::Garrison(new, _) => unit.garrison = *new,
            Self::TrainTime(new, _) => unit.train_time = *new,
            Self::Accuracy(new, _) => unit.accuracy = *new,
            Self::Los(new, _) => unit.los = MaybeFixed::from_f64(*new),
            Self::Speed(new, _) => unit.speed = MaybeFixed::from_f64(*new),
            Self::WorkRate(new, _) => unit.work_rate = MaybeFixed::from_f64(*new),
            Self::Range(new, _) => unit.range = MaybeFixed::from_f64(*new),
            Self::Reload(new, _) => unit.reload = MaybeFixed::from_f64(*new),
            Self::MinRange(new, _) => unit.min_range = MaybeFixed::from_f64(*new),
            Self::Attacks(new, _) => unit.attacks = new.clone(),
            Self::Armours(new, _) => unit.armours = new.clone(),
        }
    }
}

fn whole<T>(key: &str, value: &Value, what: &str) -> Result<T, ConfigError>
where
    T: TryFrom<i64>,
{
    let number = match *value {
        Value::Integer(number) => number,
        Value::Float(number) if number.fract() == 0.0 => number as i64,
        _ => {
            return Err(ConfigError::Invalid {
                key: key.to_owned(),
                detail: format!("expected {what}, found {}", describe(value)),
            });
        }
    };
    T::try_from(number).map_err(|_| ConfigError::Invalid {
        key: key.to_owned(),
        detail: format!("{number} does not fit {what}"),
    })
}

fn number(key: &str, value: &Value) -> Result<f64, ConfigError> {
    match *value {
        Value::Integer(number) => Ok(number as f64),
        Value::Float(number) => Ok(number),
        _ => Err(ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!("expected a number, found {}", describe(value)),
        }),
    }
}

fn pairs(key: &str, value: &Value) -> Result<Vec<(i16, i16)>, ConfigError> {
    let Value::Array(rows) = value else {
        return Err(ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!(
                "expected an array of [class, amount] pairs, found {}",
                describe(value)
            ),
        });
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let [class, amount] = row.as_slice() else {
            return Err(ConfigError::Invalid {
                key: key.to_owned(),
                detail: "each row is a [class, amount] pair".to_owned(),
            });
        };
        let class = i16::try_from(*class).map_err(|_| ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!("{class} is not a class id"),
        })?;
        let amount = i16::try_from(*amount).map_err(|_| ConfigError::Invalid {
            key: key.to_owned(),
            detail: format!("{amount} is not an amount"),
        })?;
        out.push((class, amount));
    }
    Ok(out)
}

fn describe(value: &Value) -> String {
    match value {
        Value::Integer(_) => "a whole number".to_owned(),
        Value::Float(_) => "a number with a fraction".to_owned(),
        Value::Array(_) => "an array".to_owned(),
        Value::Other(kind) => (*kind).to_owned(),
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
