//! The economy config (`config/economy.toml`): the rates the dat does not settle, plus the
//! task → villager-variant table from D11.
//!
//! Same rules as `sim.toml` (D6): unknown keys and missing keys are startup failures, and `-1.0`
//! means *unresolved*, not zero. The one deliberate difference is `farm.rate`: minus one is never
//! a plausible gather rate, so a negative value there is a typo and is refused rather than
//! silently treated as a sentinel.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

#[derive(Debug, Clone)]
pub struct Economy {
    pub schema: u32,
    pub farm: Farm,
    pub gather: Gather,
    pub build: Build,
    pub decay: Decay,
    pub population: Population,
}

#[derive(Debug, Clone)]
pub struct Farm {
    pub rate: MaybeFixed,
    pub use_villager_rate: bool,
    pub food_per_farm: u32,
}

#[derive(Debug, Clone)]
pub struct Gather {
    pub rate_from: String,
    pub carry_from: String,
    pub drop_sites_from: String,
    /// Task name → the villager-variant unit id that performs it (D11).
    pub variant: BTreeMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct Build {
    pub hp_per_second_per_villager: MaybeFixed,
    pub repair_hp_per_second: MaybeFixed,
}

#[derive(Debug, Clone)]
pub struct Decay {
    pub enabled: bool,
    pub rate_from: String,
}

#[derive(Debug, Clone)]
pub struct Population {
    pub cap_default: u32,
    pub house_population: u32,
}

// --- wire format: one struct per table, strict -------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEconomy {
    schema: u32,
    farm: RawFarm,
    gather: RawGather,
    build: RawBuild,
    decay: RawDecay,
    population: RawPopulation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFarm {
    rate: f64,
    use_villager_rate: bool,
    depletion: RawDepletion,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDepletion {
    food_per_farm: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGather {
    rate_from: String,
    carry_from: String,
    drop_sites_from: String,
    variant: BTreeMap<String, u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuild {
    hp_per_second_per_villager: f64,
    repair_hp_per_second: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDecay {
    enabled: bool,
    rate_from: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPopulation {
    cap_default: u32,
    house_population: u32,
}

fn invalid(path: &str, key: &str, detail: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        key: format!("{path}: {key}"),
        detail: detail.into(),
    }
}

pub fn load_economy_str(text: &str, path: &str) -> Result<Economy, ConfigError> {
    let raw: RawEconomy = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(invalid(
            path,
            "schema",
            format!("{} is not this engine's schema", raw.schema),
        ));
    }

    // A negative rate is a typo, not the unresolved sentinel (see the module docs). NaN is
    // rejected by the same test: `is_finite` is false for it, so it never silently passes.
    let farm_rate = raw.farm.rate;
    if !farm_rate.is_finite() || farm_rate <= 0.0 {
        return Err(invalid(
            path,
            "farm.rate",
            format!(
                "{} is not a rate: it must be greater than zero (-1.0 is only a sentinel where \
                 the docs say so)",
                farm_rate
            ),
        ));
    }

    if raw.gather.variant.is_empty() {
        return Err(invalid(
            path,
            "gather.variant",
            "empty: rate_from = \"dat\" with no variants says one thing and means another (D11)",
        ));
    }

    if raw.population.cap_default == 0 || raw.population.house_population == 0 {
        return Err(invalid(
            path,
            "population",
            "cap_default and house_population must be greater than zero",
        ));
    }

    if raw.farm.depletion.food_per_farm == 0 {
        return Err(invalid(
            path,
            "farm.depletion.food_per_farm",
            "must be greater than zero",
        ));
    }

    Ok(Economy {
        schema: raw.schema,
        farm: Farm {
            rate: MaybeFixed::from_f64(raw.farm.rate),
            use_villager_rate: raw.farm.use_villager_rate,
            food_per_farm: raw.farm.depletion.food_per_farm,
        },
        gather: Gather {
            rate_from: raw.gather.rate_from,
            carry_from: raw.gather.carry_from,
            drop_sites_from: raw.gather.drop_sites_from,
            variant: raw.gather.variant,
        },
        build: Build {
            hp_per_second_per_villager: MaybeFixed::from_f64(raw.build.hp_per_second_per_villager),
            repair_hp_per_second: MaybeFixed::from_f64(raw.build.repair_hp_per_second),
        },
        decay: Decay {
            enabled: raw.decay.enabled,
            rate_from: raw.decay.rate_from,
        },
        population: Population {
            cap_default: raw.population.cap_default,
            house_population: raw.population.house_population,
        },
    })
}

pub fn load_economy_file(path: &Path) -> Result<Economy, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_economy_str(&text, &path.display().to_string())
}
