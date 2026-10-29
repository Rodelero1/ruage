//! The balance runner: N scripts x M seeds, headless, one row per run (item 8).
//!
//! The owner's aim, in his words: *"I intend to keep system, that allows simulations ... when adding new
//! civilizations, it would be helpful to use the simulations for game balancing."* So the output is a table, not a
//! verdict: when each age arrived, what the economy delivered per minute, who was lost, when the first soldier
//! appeared, and how the match ended. Two runs of one row must agree, or the row is opinion.
//!
//! Usage:
//!   ruage-batch --config config/sim.toml --map config/map.toml --economy config/economy.toml \
//!               --units config/gamedata/units.toml --resources config/resources.toml \
//!               [--combat config/combat.toml] [--techs ...] [--effects ...] [--civs ...] \
//!               [--overrides ...] \
//!               --script config/scenario.toml [--script config/second.toml] \
//!               [--seeds 1,2,3] [--csv]
//!
//! Integers only: this file lives in the sim crate, whose `no_float` gate refuses `f64` outside the authoring
//! exemptions. Times are printed as tenths of a second from integer arithmetic.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use ruage_data::{
    load_civs_file, load_combat_file, load_economy_file, load_effects_file, load_map_file,
    load_overrides_file, load_resources_file, load_sim_file, load_techs_file, load_units_file,
};
use ruage_sim::{Metrics, Runner, World, load_scenario_file};

const USAGE: &str = "usage: ruage-batch --config <sim.toml> --map <map.toml> \
                     --economy <economy.toml> --units <units.toml> \
                     --resources <resources.toml> \
                     [--combat <combat.toml>] [--techs <techs.toml>] [--effects <effects.toml>] \
                     [--civs <civs.toml>] [--overrides <overrides.toml>] \
                     --script <scenario.toml> [--script <another.toml>] \
                     [--seeds <n[,n...]>] [--player <n>] [--csv]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Strict flag parsing: `--script` is the only flag that repeats, and it means "another run", not "the last one".
fn parse(args: &[String]) -> Result<(BTreeMap<String, String>, Vec<String>), String> {
    let mut flags = BTreeMap::new();
    let mut scripts = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].clone();
        if !flag.starts_with("--") {
            return Err(format!("{flag:?} is not a flag"));
        }
        if flag == "--csv" {
            flags.insert(flag, String::new());
            i += 1;
            continue;
        }
        let Some(value) = args.get(i + 1) else {
            return Err(format!("{flag} needs a value"));
        };
        if value.starts_with("--") {
            return Err(format!("{flag} needs a value"));
        }
        if flag == "--script" {
            scripts.push(value.clone());
        } else if flags.insert(flag.clone(), value.clone()).is_some() {
            return Err(format!("{flag} was given twice"));
        }
        i += 2;
    }
    if scripts.is_empty() {
        return Err("--script is required".to_owned());
    }
    Ok((flags, scripts))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let (flags, scripts) = parse(&args)?;
    let required = |name: &str| -> Result<&Path, String> {
        flags
            .get(name)
            .map(Path::new)
            .ok_or_else(|| format!("{name} is required"))
    };
    let settings = load_sim_file(required("--config")?).map_err(|err| err.to_string())?;
    let resources = load_resources_file(required("--resources")?).map_err(|err| err.to_string())?;
    let map = load_map_file(required("--map")?, &resources).map_err(|err| err.to_string())?;
    let economy = load_economy_file(required("--economy")?).map_err(|err| err.to_string())?;
    let mut units = load_units_file(required("--units")?).map_err(|err| err.to_string())?;
    // Hand-written overrides change the table every run shares, so they are applied once, before the loop, and every
    // value replaced is printed: an override nobody can see is indistinguishable from a bug (D45).
    if let Some(path) = flags.get("--overrides") {
        let overrides = load_overrides_file(Path::new(path)).map_err(|err| err.to_string())?;
        for line in overrides
            .apply_units(&mut units)
            .map_err(|err| err.to_string())?
        {
            eprintln!("override {line}");
        }
    }
    let combat = flags
        .get("--combat")
        .map(|path| load_combat_file(Path::new(path)))
        .transpose()
        .map_err(|err| err.to_string())?;
    let research = match (flags.get("--techs"), flags.get("--effects")) {
        (Some(techs), Some(effects)) => Some((
            load_techs_file(Path::new(techs)).map_err(|err| err.to_string())?,
            load_effects_file(Path::new(effects)).map_err(|err| err.to_string())?,
        )),
        (None, None) => None,
        _ => return Err("--techs and --effects go together".to_owned()),
    };
    let civs = flags
        .get("--civs")
        .map(|path| load_civs_file(Path::new(path)))
        .transpose()
        .map_err(|err| err.to_string())?;

    let seeds: Vec<u64> = match flags.get("--seeds") {
        Some(text) => text
            .split(',')
            .map(|one| {
                one.trim()
                    .parse::<u64>()
                    .map_err(|_| format!("{one:?} is not a seed"))
            })
            .collect::<Result<_, _>>()?,
        None => vec![1],
    };
    let player: u8 = flags
        .get("--player")
        .map(|text| {
            text.parse()
                .map_err(|_| format!("{text:?} is not a player"))
        })
        .transpose()?
        .unwrap_or(1);
    let csv = flags.contains_key("--csv");

    if !csv {
        println!(
            "{:<28} {:>5} {:>8} {:>8} {:>6} {:>6} {:>6} {:>4} {:>7} {:>10} {:>16}",
            "script",
            "seed",
            "feudal",
            "castle",
            "food/m",
            "wood/m",
            "gold/m",
            "lost",
            "army",
            "outcome",
            "hash"
        );
    }
    for script in &scripts {
        let scenario = load_scenario_file(Path::new(script)).map_err(|err| err.to_string())?;
        for seed in &seeds {
            let mut world = World::new(&settings, *seed).map_err(|err| err.to_string())?;
            world
                .seed_from(&map, &economy, &units)
                .map_err(|err| err.to_string())?;
            world
                .load_resources(&resources)
                .expect("the pack fits the stockpile");
            world.apply_content_policy();
            if let Some(combat) = &combat {
                world.load_combat(combat);
            }
            if let Some((techs, effects)) = &research {
                world
                    .load_research(techs, effects)
                    .map_err(|err| err.to_string())?;
            }
            if let Some(civs) = &civs {
                world.load_civs(civs).map_err(|err| err.to_string())?;
            }
            let mut runner =
                Runner::new(&scenario, world.tick_rate_hz()).map_err(|err| err.to_string())?;
            let mut metrics = Metrics::new(&world, player, &economy.metrics.military_roles);
            let ticks = u64::try_from(
                (scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64,
            )
            .map_err(|_| "the scenario is longer than a u64 of ticks".to_owned())?;
            let mut refused = 0usize;
            for _ in 0..ticks {
                runner.due(&mut world);
                world
                    .step()
                    .map_err(|err| format!("tick {}: {err}", world.tick()))?;
                metrics.observe(&world);
            }
            for (_, outcome) in &runner.outcomes {
                if *outcome != ruage_sim::Outcome::Ran {
                    refused += 1;
                }
            }
            let name = Path::new(script).file_name().map_or_else(
                || script.clone(),
                |name| name.to_string_lossy().into_owned(),
            );
            let slot = |name: &str| world.slot_of(name).unwrap_or(usize::MAX);
            let rate = |name: &str| match slot(name) {
                usize::MAX => 0,
                slot => metrics.per_minute(slot),
            };
            let seconds = |ticks: Option<u64>| match ticks {
                Some(ticks) => tenths(ticks, world.tick_rate_hz()),
                None => "-".to_owned(),
            };
            let outcome = match world.outcome() {
                Some(ruage_sim::MatchOutcome::Victory { winner }) => format!("victory p{winner}"),
                Some(ruage_sim::MatchOutcome::Draw) => "draw".to_owned(),
                None => format!("open ({refused} refused)"),
            };
            if csv {
                println!(
                    "{name},{seed},{},{},{},{},{},{},{},{},#{:016x}",
                    seconds(metrics.feudal_at()),
                    seconds(metrics.castle_at()),
                    rate("food"),
                    rate("wood"),
                    rate("gold"),
                    metrics.villagers_lost(),
                    seconds(metrics.first_military_at()),
                    outcome,
                    world.state_hash()
                );
            } else {
                println!(
                    "{name:<28} {seed:>5} {:>8} {:>8} {:>6} {:>6} {:>6} {:>4} {:>7} {:>10} #{:012x}",
                    seconds(metrics.feudal_at()),
                    seconds(metrics.castle_at()),
                    rate("food"),
                    rate("wood"),
                    rate("gold"),
                    metrics.villagers_lost(),
                    seconds(metrics.first_military_at()),
                    outcome,
                    world.state_hash()
                );
            }
        }
    }
    Ok(())
}

/// Ticks as seconds to one decimal place, without a float: `45.5` for the 911th tick at 20 Hz.
fn tenths(ticks: u64, tick_rate_hz: u32) -> String {
    let rate = u64::from(tick_rate_hz).max(1);
    let tenths = ticks.saturating_mul(10) / rate;
    format!("{}.{}", tenths / 10, tenths % 10)
}
