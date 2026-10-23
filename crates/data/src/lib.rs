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
    /// Finest decimal grain anywhere in the dat's values (measured: 4 — `work_rate = 0.0111`
    /// on units 110 and 1021). Every value must be representable exactly at the scale.
    pub dat_value_decimals: u32,
    /// Finest decimal grain among quantities integrated once per tick (measured: 3 — the
    /// speed `0.737`). Their per-tick delta must land on a whole number of scale units,
    /// otherwise every unit on the map accumulates rounding error.
    pub per_tick_decimals: u32,
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
    dat_value_decimals: u32,
    per_tick_decimals: u32,
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

/// The only config schema this engine understands. Anything else is a silent behaviour
/// change waiting to happen, so it is refused rather than best-effort parsed.
const SUPPORTED_SCHEMA: u32 = 1;

/// Two rules, both derived from config, both measured against the corpus
/// (`crates/data/tests/corpus.rs`):
///
/// 1. **Representability** — `scale % 10^dat_value_decimals == 0`, so every dat value is an
///    exact integer number of scale units. Nothing is ever rounded on the way into state.
/// 2. **Per-tick motion** — `scale % (tick_rate_hz * 10^per_tick_decimals) == 0`, so a
///    speed's displacement in one tick is exact. At 20 Hz with thousandths that is 20000,
///    which is exactly why 20000 is the scale.
///
/// The earlier check tested reverse divisibility (`10^k % scale == 0`), which accepted 1, 2,
/// 4, 5 and 8 — none of which can represent a single dat value — and rejected 300.
fn decimal_grain(decimals: u32) -> Option<i64> {
    10_i64.checked_pow(decimals)
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

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(ConfigError::Invalid {
            key: format!("{path}: schema"),
            detail: format!(
                "{} is not this engine's schema (supported: {SUPPORTED_SCHEMA})",
                raw.schema
            ),
        });
    }

    let Some(value_grain) = decimal_grain(raw.sim.dat_value_decimals) else {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.dat_value_decimals"),
            detail: format!(
                "{} decimal places is not a representable granularity",
                raw.sim.dat_value_decimals
            ),
        });
    };

    let Some(tick_grain) = decimal_grain(raw.sim.per_tick_decimals) else {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.per_tick_decimals"),
            detail: format!(
                "{} decimal places is not a representable granularity",
                raw.sim.per_tick_decimals
            ),
        });
    };

    if raw.sim.per_tick_decimals > raw.sim.dat_value_decimals {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.per_tick_decimals"),
            detail: format!(
                "{} places is finer than the data itself (dat_value_decimals = {}); \
                 a per-tick quantity cannot be more precise than the dat",
                raw.sim.per_tick_decimals, raw.sim.dat_value_decimals
            ),
        });
    }

    let per_tick = i64::from(raw.sim.tick_rate_hz).checked_mul(tick_grain);
    let Some(per_tick) = per_tick else {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.per_tick_decimals"),
            detail: "tick_rate_hz x 10^per_tick_decimals overflows i64".to_owned(),
        });
    };

    if raw.sim.fixed_point_scale <= 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.fixed_point_scale"),
            detail: "must be positive".to_owned(),
        });
    }

    if raw.sim.fixed_point_scale % value_grain != 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.fixed_point_scale"),
            detail: format!(
                "{} is not a multiple of 10^dat_value_decimals = 10^{} = {value_grain}; \
                 dat values would have to be rounded",
                raw.sim.fixed_point_scale, raw.sim.dat_value_decimals
            ),
        });
    }

    if raw.sim.fixed_point_scale % per_tick != 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.fixed_point_scale"),
            detail: format!(
                "{} is not a multiple of tick_rate_hz x 10^per_tick_decimals = {} x 10^{} \
                 = {per_tick}; per-tick displacement would not be exact",
                raw.sim.fixed_point_scale, raw.sim.tick_rate_hz, raw.sim.per_tick_decimals
            ),
        });
    }

    if raw.sim.max_entities == 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.max_entities"),
            detail: "must be greater than zero".to_owned(),
        });
    }

    if raw.sim.state_hash_interval_ticks == 0 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: sim.state_hash_interval_ticks"),
            detail: "must be greater than zero".to_owned(),
        });
    }

    // These are invariants, not preferences: a config that declares them violated is
    // contradictory, and accepting it would mean the rules live only in prose.
    if raw.determinism.floats_in_sim_state {
        return Err(ConfigError::Invalid {
            key: format!("{path}: determinism.floats_in_sim_state"),
            detail: "floats in simulation state are not permitted (D9); this key \
                     exists only to be explicitly false"
                .to_owned(),
        });
    }

    if raw.determinism.rng_streams.len() < 2 {
        return Err(ConfigError::Invalid {
            key: format!("{path}: determinism.rng_streams"),
            detail: format!(
                "{} stream(s): simulation and map generation need separate, seeded streams",
                raw.determinism.rng_streams.len()
            ),
        });
    }

    let mut seen: Vec<&str> = Vec::with_capacity(raw.determinism.rng_streams.len());
    for stream in &raw.determinism.rng_streams {
        if seen.contains(&stream.as_str()) {
            return Err(ConfigError::Invalid {
                key: format!("{path}: determinism.rng_streams"),
                detail: format!("{stream:?} is listed twice; streams must be distinct"),
            });
        }
        seen.push(stream);
    }

    Ok(Sim {
        schema: raw.schema,
        tick_rate_hz: raw.sim.tick_rate_hz,
        fixed_point_scale: raw.sim.fixed_point_scale,
        dat_value_decimals: raw.sim.dat_value_decimals,
        per_tick_decimals: raw.sim.per_tick_decimals,
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
dat_value_decimals = 4
per_tick_decimals = 3
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

    /// The scale must be able to represent every dat value AND land on a whole number of
    /// scale units per tick. Both halves are derived from config, not from a literal here.
    #[test]
    fn scale_must_give_exact_per_tick_displacement() {
        // 20 Hz, thousandths: 20 * 1000 = 20000 exactly, so 20000 is the minimum.
        let ok = load_sim_str(GOOD, "sim.toml").expect("20000 at 20 Hz must load");
        assert_eq!(ok.fixed_point_scale % (20 * 1000), 0);

        let scaled = |v: &str| {
            GOOD.replace(
                "fixed_point_scale = 20000",
                format!("fixed_point_scale = {v}").as_str(),
            )
        };
        for (label, text) in [
            ("scale 20000 (the configured one)", GOOD.to_string()),
            ("scale 40000 (oversampled)", scaled("40000")),
            ("scale 10000 loses 0.737", scaled("10000")),
            ("scale 300", scaled("300")),
            ("scale 2", scaled("2")),
            ("scale 3", scaled("3")),
        ] {
            let should_pass = label.starts_with("scale 20000") || label.starts_with("scale 40000");
            let got = load_sim_str(&text, "sim.toml");
            assert_eq!(
                got.is_ok(),
                should_pass,
                "{label}: expected {}, got {:?}",
                if should_pass { "Ok" } else { "Invalid" },
                got.map(|s| s.fixed_point_scale).map_err(|e| e.to_string())
            );
        }
    }

    #[test]
    fn tick_rate_and_scale_must_agree() {
        // 60 Hz needs 60000; pairing it with 20000 reintroduces per-tick drift.
        let stale = GOOD.replace("tick_rate_hz = 20", "tick_rate_hz = 60");
        let err = load_sim_str(&stale, "sim.toml")
            .expect_err("60 Hz with scale 20000 must be refused")
            .to_string();
        assert!(err.contains("fixed_point_scale"), "got: {err}");

        let correct = stale.replace("fixed_point_scale = 20000", "fixed_point_scale = 60000");
        assert!(
            load_sim_str(&correct, "sim.toml").is_ok(),
            "60 Hz at scale 60000 must load"
        );
    }

    #[test]
    fn decimal_grains_are_required_and_bounded() {
        for key in ["dat_value_decimals", "per_tick_decimals"] {
            let missing = GOOD.replace(format!("{key} = ").as_str(), "# removed ");
            let err = load_sim_str(&missing, "sim.toml")
                .expect_err("both grains must be required")
                .to_string();
            assert!(err.contains(key), "{key} must be reported, got: {err}");
        }

        let huge = GOOD.replace("dat_value_decimals = 4", "dat_value_decimals = 40");
        assert!(
            load_sim_str(&huge, "sim.toml").is_err(),
            "a granularity of 10^40 cannot be represented"
        );

        // A motion grain finer than the data itself is a contradiction.
        let finer = GOOD.replace("per_tick_decimals = 3", "per_tick_decimals = 5");
        let err = load_sim_str(&finer, "sim.toml")
            .expect_err("per_tick_decimals cannot exceed dat_value_decimals")
            .to_string();
        assert!(err.contains("per_tick_decimals"), "got: {err}");
    }

    #[test]
    fn schema_version_must_be_supported() {
        let bad = GOOD.replace("schema = 1", "schema = 99");
        let err = load_sim_str(&bad, "sim.toml")
            .expect_err("unknown schema must be refused")
            .to_string();
        assert!(err.contains("schema"), "got: {err}");
    }

    #[test]
    fn determinism_invariants_are_rules_not_switches() {
        // Declaring floats in simulation state is a contradiction, not a preference.
        let floats = GOOD.replace("floats_in_sim_state = false", "floats_in_sim_state = true");
        let err = load_sim_str(&floats, "sim.toml")
            .expect_err("floats_in_sim_state = true must be refused")
            .to_string();
        assert!(err.contains("floats_in_sim_state"), "got: {err}");

        // One stream cannot be "separate streams"; duplicates are not separate either.
        for streams in [
            r#"rng_streams = []"#,
            r#"rng_streams = ["sim"]"#,
            r#"rng_streams = ["sim", "sim"]"#,
        ] {
            let bad = GOOD.replace(r#"rng_streams = ["sim", "map"]"#, streams);
            assert!(
                load_sim_str(&bad, "sim.toml").is_err(),
                "{streams} must be refused"
            );
        }
    }

    #[test]
    fn zero_valued_sizes_are_refused() {
        for (old, new, key) in [
            ("max_entities = 4096", "max_entities = 0", "max_entities"),
            (
                "state_hash_interval_ticks = 20",
                "state_hash_interval_ticks = 0",
                "state_hash_interval_ticks",
            ),
        ] {
            let bad = GOOD.replace(old, new);
            let err = load_sim_str(&bad, "sim.toml")
                .expect_err("zero must be refused")
                .to_string();
            assert!(err.contains(key), "got: {err}");
        }
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
    #[allow(clippy::approx_constant)] // deliberately the config's pathing weight, not SQRT_2
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
