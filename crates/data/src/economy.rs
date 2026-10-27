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
    /// What a match begins with, by role name. Config, never code (D6).
    pub start: Start,
    /// Role name → unit id. One vocabulary for everything that must not quote the dat's numbering:
    /// the start of a match, the scenario format, and the engine's own seeding (D17).
    pub role: BTreeMap<String, u32>,
    /// Tech role → tech id. Techs have no name in the dat, so content that names one needs this.
    pub tech_role: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    /// Role names, not unit ids: the roles below are the vocabulary content uses, and the engine
    /// never has to know that a town centre is unit 109 (D17).
    pub town_centre: String,
    pub villager: String,
    pub villagers_per_player: u32,
    /// Tech roles a match begins with, already researched. Without them a match could research
    /// nothing at all: every tech in the table lists the Dark Age as a prerequisite.
    pub techs: Vec<String>,
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
    /// Resource *name*, as `config/resources.toml` spells it → the gather task it presents. Keyed by
    /// name because the engine has no resource kinds of its own (D17): the pack says a node is
    /// "timber", this table says timber is worked by the "wood" task.
    pub resource_task: BTreeMap<String, String>,
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
    /// Role name → unit id. Required: content that cannot name a unit at all is not content.
    role: BTreeMap<String, u32>,
    #[serde(default)]
    tech_role: BTreeMap<String, u32>,
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
    /// A resource name → the gather task that works it. Free-form keys: which resources exist is the
    /// content pack's business, and this loader can only check the task side of the join.
    resource_task: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrain {
    entry: Vec<RawTrainEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrainEntry {
    /// A role from `[role]`, never a dat id: everything that names a unit in content names it by
    /// role, and accepting both spellings is how the numbering creeps back in (D17).
    building: String,
    units: Vec<String>,
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
    town_centre: String,
    villager: String,
    villagers_per_player: u32,
    #[serde(default)]
    techs: Vec<String>,
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

    if raw.role.is_empty() {
        return Err(invalid(
            path,
            "role",
            "no roles: the start and every scenario step name units by role, so an empty table \
             names nothing",
        ));
    }
    // Zero is a real unit id, which is exactly why it cannot be a role: the field would read as
    // unset and a match would seed whatever id 0 happens to be.
    for (name, id) in &raw.role {
        if *id == 0 {
            return Err(invalid(
                path,
                "role",
                format!("{name} = 0: unit id zero is a real unit, never a role"),
            ));
        }
    }
    // A starting tech is a *tech role*, because the engine would otherwise have to be told that
    // "dark age" is id 104.
    for name in &raw.start.techs {
        if !raw.tech_role.contains_key(name) {
            return Err(invalid(
                path,
                "start.techs",
                format!(
                    "{name:?} is not a [tech_role], so no tech can be named by it: the match would \
                     start with a tech the pack cannot describe"
                ),
            ));
        }
    }
    // Tech 0 is a real tech ("New Technology"), so it cannot mean "no role" either.
    for (name, id) in &raw.tech_role {
        if *id == 0 {
            return Err(invalid(
                path,
                "tech_role",
                format!("{name} = 0: tech id zero is a real tech, never a role"),
            ));
        }
    }
    for (field, name) in [
        ("town_centre", &raw.start.town_centre),
        ("villager", &raw.start.villager),
    ] {
        if !raw.role.contains_key(name) {
            return Err(invalid(
                path,
                "start",
                format!(
                    "{field} names the role {name:?}, which [role] does not define (it defines: {})",
                    raw.role.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
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

    let resource_task = raw.gather.resource_task;
    // An empty join is a match where nothing on the map can be worked at all.
    if resource_task.is_empty() {
        return Err(invalid(
            path,
            "gather.resource_task",
            "no resource names a gather task: nothing on the map could be worked".to_owned(),
        ));
    }
    // A task name that names no variant is a typo that would otherwise surface much later, as a
    // villager who walks to a tree and never gathers.
    for (resource, task) in &resource_task {
        if !raw.gather.variant.contains_key(task) {
            return Err(invalid(
                path,
                "gather.resource_task",
                format!("{resource} names task {task:?}, which is not in [gather.variant]"),
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
                let role_of = |role: &str| {
                    raw.role.get(role).copied().ok_or_else(|| ConfigError::Invalid {
                        key: format!("train.entry[{:?}]", entry.building),
                        detail: format!(
                            "{role:?} is not a role in this pack's [role] table, so no unit can be \
                             named by it"
                        ),
                    })
                };
                if entry.units.is_empty() {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}].units", entry.building),
                        detail: "a building that trains nothing is a mistake, not a rule"
                            .to_owned(),
                    });
                }
                let building = role_of(&entry.building)?;
                let mut units = Vec::with_capacity(entry.units.len());
                for role in &entry.units {
                    units.push(role_of(role)?);
                }
                if by_building.insert(building, units).is_some() {
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
            techs: raw.start.techs,
        },
        role: raw.role,
        tech_role: raw.tech_role,
    })
}

pub fn load_economy_file(path: &Path) -> Result<Economy, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_economy_str(&text, &path.display().to_string())
}
