//! Strict config loading.
//!
//! Config is the engine's only input (D6). Three failure modes must never be silent:
//!
//! 1. **Unknown key** — a typo'd `tick_rate` would otherwise be ignored and a default used,
//!    which is how a "configured" engine quietly runs unconfigured.
//! 2. **Missing key** — no defaults are invented for anything in `config/`.
//! 3. **Unresolved value** — `-1.0` means "not yet determined"; the engine must refuse to
//!    start rather than pick a plausible number.

use std::fmt;
use std::path::Path;

use ruage_fixed::Fixed;
use serde::Deserialize;

/// Values that enter simulation state are fixed-point. `-1.0` marks "unknown".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaybeFixed(Option<f64>);

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: String,
        detail: String,
    },
    Parse {
        path: String,
        detail: String,
    },
    /// A value still marked unknown was required.
    Unresolved {
        key: String,
    },
    Invalid {
        key: String,
        detail: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io { path, detail } => write!(f, "{path}: {detail}"),
            ConfigError::Parse { path, detail } => write!(f, "{path}: {detail}"),
            ConfigError::Unresolved { key } => write!(
                f,
                "{key} is -1.0 (unresolved). The engine refuses to invent a value; \
                 settle it in docs/mechanics/ and set it in config/"
            ),
            ConfigError::Invalid { key, detail } => write!(f, "{key}: {detail}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Runtime-facing config, flattened out of the TOML sections.
#[derive(Debug, Clone)]
pub struct Sim {
    pub schema: u32,
    pub tick_rate_hz: u32,
    pub fixed_point_scale: i64,
    pub max_entities: u32,
    pub state_hash_interval_ticks: u32,
    pub command_apply: CommandApply,
    pub determinism: Determinism,
    pub rng: Rng,
    pub paths: Paths,
}

/// Wire format: one struct per TOML table, strict. Kept separate from [`Sim`] because
/// `deny_unknown_fields` and `flatten` cannot be combined in serde.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSimFile {
    schema: u32,
    sim: RawSim,
    determinism: Determinism,
    rng: Rng,
    paths: Paths,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSim {
    tick_rate_hz: u32,
    fixed_point_scale: i64,
    max_entities: u32,
    state_hash_interval_ticks: u32,
    command_apply: CommandApply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandApply {
    NextTick,
    Immediate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Determinism {
    pub floats_in_sim_state: bool,
    pub iteration_order: IterationOrder,
    pub rng_streams: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationOrder {
    EntityId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rng {
    pub algorithm: String,
    pub seed_source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paths {
    pub gamedata: String,
    pub overrides: String,
}

impl MaybeFixed {
    pub fn unresolved() -> Self {
        MaybeFixed(None)
    }

    pub fn from_f64(value: f64) -> Self {
        if value == UNRESOLVED_SENTINEL {
            MaybeFixed(None)
        } else {
            MaybeFixed(Some(value))
        }
    }

    /// Convert to fixed-point at `scale`, or explain why it cannot be.
    ///
    /// The f64 is rendered with its shortest round-trip decimal form and then parsed
    /// exactly, so a value that does not fit the scale is refused rather than rounded.
    pub fn resolve(self, scale: i64, key: &str) -> Result<Fixed, ConfigError> {
        let value = self.0.ok_or_else(|| ConfigError::Unresolved {
            key: key.to_owned(),
        })?;
        let text = format!("{value}");
        Fixed::parse(&text, scale).map_err(|err| ConfigError::Invalid {
            key: key.to_owned(),
            detail: err.to_string(),
        })
    }
}

/// A value still under investigation. Writers put `-1.0`; the loader turns it into a hard
/// error whenever anything asks for it.
const UNRESOLVED_SENTINEL: f64 = -1.0;

/// True when `scale` divides some power of ten, i.e. every decimal value with at most that
/// many places is exactly representable. 20000 does (10^5 / 20000 = 5); 3 does not.
fn scale_is_decimal_exact(scale: i64) -> bool {
    if scale <= 0 {
        return false;
    }
    let scale = i128::from(scale);
    let mut power: i128 = 10;
    for _ in 1..=9 {
        if power % scale == 0 {
            return true;
        }
        power *= 10;
    }
    false
}

pub fn load_sim_str(text: &str, path: &str) -> Result<Sim, ConfigError> {
    let raw: RawSimFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;

    if raw.sim.tick_rate_hz == 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.tick_rate_hz"),
            detail: "must be greater than zero".to_owned(),
        });
    }

    if raw.sim.fixed_point_scale <= 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.fixed_point_scale"),
            detail: "must be positive".to_owned(),
        });
    }

    if !scale_is_decimal_exact(raw.sim.fixed_point_scale) {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.fixed_point_scale"),
            detail: format!(
                "{} cannot represent decimal dat values exactly: the scale must divide a \
                 power of ten (20000 works, 3 does not)",
                raw.sim.fixed_point_scale
            ),
        });
    }

    Ok(Sim {
        schema: raw.schema,
        tick_rate_hz: raw.sim.tick_rate_hz,
        fixed_point_scale: raw.sim.fixed_point_scale,
        max_entities: raw.sim.max_entities,
        state_hash_interval_ticks: raw.sim.state_hash_interval_ticks,
        command_apply: raw.sim.command_apply,
        determinism: raw.determinism,
        rng: raw.rng,
        paths: raw.paths,
    })
}

pub fn load_sim_file(path: &Path) -> Result<Sim, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_sim_str(&text, &path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
schema = 1
[sim]
tick_rate_hz = 20
fixed_point_scale = 20000
max_entities = 4096
state_hash_interval_ticks = 20
command_apply = "next_tick"
[determinism]
floats_in_sim_state = false
iteration_order = "entity_id"
rng_streams = ["sim", "map"]
[rng]
algorithm = "pcg32"
seed_source = "explicit"
[paths]
gamedata = "gamedata"
overrides = "overrides.toml"
"#;

    #[test]
    fn valid_config_loads() {
        let sim = load_sim_str(GOOD, "sim.toml").expect("good config should load");
        assert_eq!(sim.tick_rate_hz, 20);
        assert_eq!(sim.fixed_point_scale, 20_000);
        assert_eq!(sim.max_entities, 4096);
    }

    #[test]
    fn unknown_key_is_rejected_and_named() {
        let bad = GOOD.replace("max_entities = 4096", "max_entities = 4096\nbogus_knob = 7");
        let err = load_sim_str(&bad, "sim.toml").unwrap_err().to_string();
        assert!(
            err.contains("bogus_knob"),
            "error should name the key, got: {err}"
        );
        assert!(
            err.contains("sim.toml"),
            "error should name the file, got: {err}"
        );
    }

    #[test]
    fn missing_key_is_rejected() {
        let bad = GOOD.replace("tick_rate_hz = 20\n", "");
        let err = load_sim_str(&bad, "sim.toml").unwrap_err().to_string();
        assert!(err.contains("tick_rate_hz"), "got: {err}");
    }

    #[test]
    fn scale_must_be_decimal_exact() {
        // A scale of 3 cannot represent 1/10, so no two-decimal dat value is exact.
        let bad = GOOD.replace("fixed_point_scale = 20000", "fixed_point_scale = 3");
        let err = load_sim_str(&bad, "sim.toml").unwrap_err().to_string();
        assert!(err.contains("fixed_point_scale"), "got: {err}");
        assert!(err.contains("decimal"), "got: {err}");
    }

    #[test]
    fn zero_tick_rate_is_rejected() {
        let bad = GOOD.replace("tick_rate_hz = 20", "tick_rate_hz = 0");
        let err = load_sim_str(&bad, "sim.toml").unwrap_err().to_string();
        assert!(err.contains("tick_rate_hz"), "got: {err}");
    }

    #[test]
    fn unresolved_value_cannot_be_resolved_and_names_its_key() {
        let v = MaybeFixed::unresolved();
        let err = v
            .resolve(20_000, "economy.build.hp_per_second_per_villager")
            .unwrap_err()
            .to_string();
        assert!(err.contains("hp_per_second_per_villager"), "got: {err}");
        assert!(err.contains("unresolved"), "got: {err}");
    }

    #[test]
    fn dat_values_resolve_exactly() {
        for (v, want) in [
            (0.4, 8_000i64),
            (0.53, 10_600),
            (1.35, 27_000),
            (35.0, 700_000),
        ] {
            let f = MaybeFixed::from_f64(v).resolve(20_000, "k").unwrap();
            assert_eq!(f.raw(), want, "value {v}");
        }
    }

    #[test]
    fn sub_precision_values_are_refused_not_rounded() {
        // 1.41421356 is a pathing weight; it must not enter the fixed-point domain.
        let err = MaybeFixed::from_f64(1.41421356)
            .resolve(20_000, "movement.pathing.cost_diagonal")
            .unwrap_err()
            .to_string();
        assert!(err.contains("cost_diagonal"), "got: {err}");
    }

    #[test]
    fn the_real_sim_toml_matches_this_schema() {
        // The schema is worthless if it does not describe the file the engine actually loads.
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/sim.toml");
        let sim = load_sim_file(&path).expect("config/sim.toml must load");
        assert_eq!(sim.tick_rate_hz, 20);
        assert_eq!(sim.fixed_point_scale, 20_000);
        assert_eq!(sim.determinism.rng_streams, vec!["sim", "map"]);
    }
}
