//! `config/combat.toml`: the damage constants. The per-unit numbers live in `units.toml`.

use serde::Deserialize;

use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

#[derive(Debug, Clone)]
pub struct Combat {
    pub damage: Damage,
    pub elevation: Elevation,
    pub attack_timing: AttackTiming,
    pub buildings: Buildings,
}

#[derive(Debug, Clone, Copy)]
pub struct Damage {
    /// Applied after everything else, so a stopped attack still lands for this much.
    pub minimum: i32,
    /// Armour assumed for a class the target does not list. Large enough that an unlisted class
    /// contributes nothing.
    pub armour_missing_sentinel: MaybeFixed,
    /// Bonus resistance, which AoC/HD has no units for (it is a DE mechanic).
    pub bonus_resist_default: MaybeFixed,
}

#[derive(Debug, Clone, Copy)]
pub struct Elevation {
    pub attacker_above: MaybeFixed,
    pub attacker_below: MaybeFixed,
}

#[derive(Debug, Clone, Copy)]
pub struct AttackTiming {
    pub damage_at_animation_fraction: MaybeFixed,
    pub use_reload_from_dat: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Buildings {
    pub can_attack: bool,
}

impl Combat {
    /// The integer form the simulation uses: the sentinel is a data value, the engine only needs
    /// "what armour does a class nobody listed get".
    pub fn damage_rules(&self) -> (i32, i32) {
        // Any scale works for an integer-read of a whole number; the division must use the same
        // one, which is the trap this line fell into first.
        const SCALE: i64 = 1_000_000;
        let missing = self
            .damage
            .armour_missing_sentinel
            .resolve(SCALE, "damage.armour_missing_sentinel")
            .map(|fixed| i32::try_from(fixed.raw() / SCALE).unwrap_or(i32::MAX))
            .unwrap_or(i32::MAX / 2);
        (self.damage.minimum, missing)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    damage: RawDamage,
    elevation: RawElevation,
    attack_timing: RawAttackTiming,
    buildings: RawBuildings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDamage {
    minimum: i32,
    armour_missing_sentinel: f64,
    bonus_resist_default: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawElevation {
    attacker_above: f64,
    attacker_below: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAttackTiming {
    damage_at_animation_fraction: f64,
    use_reload_from_dat: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuildings {
    can_attack: bool,
}

pub fn load_combat_str(text: &str, path: &str) -> Result<Combat, ConfigError> {
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
    if raw.damage.minimum < 0 {
        return Err(ConfigError::Invalid {
            key: "damage.minimum".to_owned(),
            detail: format!("{} damage: a hit cannot heal a target", raw.damage.minimum),
        });
    }
    if !raw.damage.armour_missing_sentinel.is_finite() || raw.damage.armour_missing_sentinel <= 0.0
    {
        return Err(ConfigError::Invalid {
            key: "damage.armour_missing_sentinel".to_owned(),
            detail: "must be positive: it is the armour of a class nobody listed".to_owned(),
        });
    }
    for (key, value) in [
        ("elevation.attacker_above", raw.elevation.attacker_above),
        ("elevation.attacker_below", raw.elevation.attacker_below),
    ] {
        // `is_finite` before the comparison on purpose: NaN is not greater than zero and not less
        // than it either, so a `value <= 0.0` test alone would let a NaN multiplier through.
        if !value.is_finite() || value <= 0.0 {
            return Err(ConfigError::Invalid {
                key: key.to_owned(),
                detail: format!("a multiplier of {value} is not a multiplier"),
            });
        }
    }
    let fraction = raw.attack_timing.damage_at_animation_fraction;
    if !(0.0..=1.0).contains(&fraction) || !fraction.is_finite() {
        return Err(ConfigError::Invalid {
            key: "attack_timing.damage_at_animation_fraction".to_owned(),
            detail: format!("{fraction} is not a fraction of the animation"),
        });
    }

    Ok(Combat {
        damage: Damage {
            minimum: raw.damage.minimum,
            armour_missing_sentinel: MaybeFixed::from_f64(raw.damage.armour_missing_sentinel),
            bonus_resist_default: MaybeFixed::from_f64(raw.damage.bonus_resist_default),
        },
        elevation: Elevation {
            attacker_above: MaybeFixed::from_f64(raw.elevation.attacker_above),
            attacker_below: MaybeFixed::from_f64(raw.elevation.attacker_below),
        },
        attack_timing: AttackTiming {
            damage_at_animation_fraction: MaybeFixed::from_f64(
                raw.attack_timing.damage_at_animation_fraction,
            ),
            use_reload_from_dat: raw.attack_timing.use_reload_from_dat,
        },
        buildings: Buildings {
            can_attack: raw.buildings.can_attack,
        },
    })
}

pub fn load_combat_file(path: &std::path::Path) -> Result<Combat, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_combat_str(&text, &path.display().to_string())
}
