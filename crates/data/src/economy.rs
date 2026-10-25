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
    /// Which buildings may create which units. Authored (see `config/economy.toml`): the dat
    /// carries unit costs and create times but not the building-to-unit edge we read.
    pub train: Train,
    pub schema: u32,
    pub farm: Farm,
    pub gather: Gather,
    pub build: Build,
    pub decay: Decay,
    pub population: Population,
    /// What a match begins with. Ids are config, never code (D6).
    pub start: Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Start {
    pub town_centre: u32,
    pub villager: u32,
    pub villagers_per_player: u32,
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
    /// Which gather task a node of each map resource kind presents.
    pub resource_task: ResourceTask,
}

/// The join between `config/map.toml` (wood/food/gold/stone) and the variant table above. Food is
/// not a task: our map's food is berries, so it is foraged, while hunting and fishing exist as
/// variants nothing on this map triggers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTask {
    pub wood: String,
    pub food: String,
    pub gold: String,
    pub stone: String,
}

impl ResourceTask {
    /// The task name for a map resource kind. Total, so a new kind cannot be added to the map
    /// without this failing to compile.
    pub fn for_kind(&self, kind: crate::ResourceKind) -> &str {
        match kind {
            crate::ResourceKind::Wood => &self.wood,
            crate::ResourceKind::Food => &self.food,
            crate::ResourceKind::Gold => &self.gold,
            crate::ResourceKind::Stone => &self.stone,
        }
    }

    fn entries(&self) -> [(&'static str, &String); 4] {
        [
            ("wood", &self.wood),
            ("food", &self.food),
            ("gold", &self.gold),
            ("stone", &self.stone),
        ]
    }
}

#[derive(Debug, Clone, Default)]
pub struct Train {
    /// Building unit id -> the unit ids it may create.
    pub by_building: BTreeMap<u32, Vec<u32>>,
}

impl Train {
    /// May this building create this unit?
    pub fn allows(&self, building: u32, unit: u32) -> bool {
        self.by_building
            .get(&building)
            .is_some_and(|units| units.contains(&unit))
    }
}

#[derive(Debug, Clone)]
pub struct Build {
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
    train: RawTrain,
    build: RawBuild,
    decay: RawDecay,
    population: RawPopulation,
    start: RawStart,
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
    resource_task: RawResourceTask,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResourceTask {
    wood: String,
    food: String,
    gold: String,
    stone: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrain {
    entry: Vec<RawTrainEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrainEntry {
    building: u32,
    units: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuild {
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
struct RawStart {
    town_centre: u32,
    villager: u32,
    villagers_per_player: u32,
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

    // One variant per task, and no unset ids: a shared id makes one variant serve two
    // resources, and a zero means the field was never filled in.
    let mut claimed: BTreeMap<u32, &String> = BTreeMap::new();
    for (task, id) in &raw.gather.variant {
        if *id == 0 {
            return Err(invalid(
                path,
                "gather.variant",
                format!("{task} = 0: unit id zero is not a villager; the field looks unset"),
            ));
        }
        if let Some(other) = claimed.insert(*id, task) {
            return Err(invalid(
                path,
                "gather.variant",
                format!(
                    "{task} and {other} both name variant {id}; the mapping must be one-to-one"
                ),
            ));
        }
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

    if raw.start.town_centre == 0 || raw.start.villager == 0 {
        return Err(invalid(
            path,
            "start",
            "unit id zero is not a building or a villager; both fields must be set",
        ));
    }
    if raw.start.town_centre == raw.start.villager {
        return Err(invalid(
            path,
            "start",
            format!(
                "{} is both the town centre and the villager; a match starts with one of each",
                raw.start.town_centre
            ),
        ));
    }
    // A start above 50 is a typo, not a game variant: the dat's own largest starting count is
    // nowhere near it, and spawning a thousand villagers would hide the mistake behind lag.
    if !(1..=50).contains(&raw.start.villagers_per_player) {
        return Err(invalid(
            path,
            "start.villagers_per_player",
            format!("{} is not between 1 and 50", raw.start.villagers_per_player),
        ));
    }

    let resource_task = ResourceTask {
        wood: raw.gather.resource_task.wood,
        food: raw.gather.resource_task.food,
        gold: raw.gather.resource_task.gold,
        stone: raw.gather.resource_task.stone,
    };
    // A task name that names no variant is a typo that would otherwise surface much later, as a
    // villager who walks to a tree and never gathers.
    for (kind, task) in resource_task.entries() {
        if !raw.gather.variant.contains_key(task) {
            return Err(invalid(
                path,
                "gather.resource_task",
                format!("{kind} names task {task:?}, which is not in [gather.variant]"),
            ));
        }
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
            resource_task,
        },
        train: {
            let mut by_building: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for entry in raw.train.entry {
                if entry.building == 0 {
                    return Err(ConfigError::Invalid {
                        key: "train.entry.building".to_owned(),
                        detail: "unit 0 is not a building".to_owned(),
                    });
                }
                if entry.units.is_empty() {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}].units", entry.building),
                        detail: "a building that trains nothing is a mistake, not a rule"
                            .to_owned(),
                    });
                }
                if entry.units.contains(&0) {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}].units", entry.building),
                        detail: "unit 0 is scenery, not something to train".to_owned(),
                    });
                }
                if by_building.insert(entry.building, entry.units).is_some() {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}]", entry.building),
                        detail: "two entries for one building: the later would silently win"
                            .to_owned(),
                    });
                }
            }
            Train { by_building }
        },
        build: Build {
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
        start: Start {
            town_centre: raw.start.town_centre,
            villager: raw.start.villager,
            villagers_per_player: raw.start.villagers_per_player,
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
