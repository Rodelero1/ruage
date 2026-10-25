//! Headless runner: seed a world from config, optionally play a scripted scenario, report the state.
//!
//! This is the smallest thing that makes "can be cloned, built and run" true, and it is how a
//! replay is checked from the outside: two runs of the same command must print the same hash, and
//! a mismatch is a bug rather than bad luck.
//!
//! Usage:
//!   ruage-headless --config config/sim.toml --map config/map.toml \
//!                  --economy config/economy.toml --units config/gamedata/units.toml \
//!                  --combat config/combat.toml --script config/scenario.toml --seed 7
//!
//!   --ticks N        run exactly N ticks (instead of the script's length)
//!   --report S       print the state every S seconds of match time
//!
//! There are no default game values, and nothing about units is accepted on the command line: who
//! starts, where, and with what stats comes from the map, the economy and the unit table (D6). A
//! `--speed` flag would be a number invented by the CLI. A repeated flag is refused for the same
//! reason: silently taking the first is how a run of 10 ticks is reported as 10,000.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use ruage_data::{
    ResourceKind, load_combat_file, load_economy_file, load_map_file, load_sim_file,
    load_units_file,
};
use ruage_sim::{Outcome, Runner, World, describe, load_scenario_file};

const USAGE: &str = "usage: ruage-headless --config <sim.toml> --map <map.toml> \
                     --economy <economy.toml> --units <units.toml> \
                     [--combat <combat.toml>] [--script <scenario.toml>] \
                     [--ticks <n>] [--report <seconds>] --seed <u64>";

const KNOWN: [&str; 10] = [
    "--config",
    "--map",
    "--economy",
    "--units",
    "--combat",
    "--script",
    "--seed",
    "--ticks",
    "--report",
    "--help",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Strict flag parsing: unknown, valueless, missing and repeated flags are all errors.
fn parse(args: &[String]) -> Result<BTreeMap<&str, &str>, String> {
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let Some(&name) = KNOWN.iter().find(|k| **k == flag) else {
            return Err(format!("{flag} is not a known flag"));
        };
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?;
        if out.insert(name, value.as_str()).is_some() {
            return Err(format!("{name} is repeated"));
        }
        i += 2;
    }
    Ok(out)
}

fn required<'a>(args: &BTreeMap<&'a str, &'a str>, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .copied()
        .ok_or_else(|| format!("{name} is required"))
}

fn number<T: std::str::FromStr>(args: &BTreeMap<&str, &str>, name: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    required(args, name)?
        .parse()
        .map_err(|err| format!("{name}: {err}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let parsed = parse(args)?;

    let settings =
        load_sim_file(Path::new(required(&parsed, "--config")?)).map_err(|err| format!("{err}"))?;
    let map =
        load_map_file(Path::new(required(&parsed, "--map")?)).map_err(|err| format!("{err}"))?;
    let economy = load_economy_file(Path::new(required(&parsed, "--economy")?))
        .map_err(|err| format!("{err}"))?;
    let units = load_units_file(Path::new(required(&parsed, "--units")?))
        .map_err(|err| format!("{err}"))?;
    let seed: u64 = number(&parsed, "--seed")?;

    let mut world = World::new(&settings, seed).map_err(|err| format!("{err}"))?;
    world
        .seed_from(&map, &economy, &units)
        .map_err(|err| format!("{err}"))?;

    if let Some(path) = parsed.get("--combat") {
        let combat = load_combat_file(Path::new(path)).map_err(|err| format!("{err}"))?;
        world.load_combat(&combat);
    }

    let mut runner = match parsed.get("--script") {
        Some(path) => {
            let scenario = load_scenario_file(Path::new(path)).map_err(|err| err.to_string())?;
            let ticks_from_script = u64::try_from(
                (scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64,
            )
            .map_err(|_| "the scenario is longer than a u64 of ticks".to_owned())?;
            let runner = Runner::new(&scenario, world.tick_rate_hz())?;
            Some((runner, ticks_from_script))
        }
        None => None,
    };

    let ticks: u64 = match parsed.get("--ticks") {
        Some(_) => number(&parsed, "--ticks")?,
        None => match &runner {
            Some((_, from_script)) => *from_script,
            None => return Err("--ticks is required when there is no --script".to_owned()),
        },
    };

    let report_every: Option<u64> = match parsed.get("--report") {
        Some(_) => {
            let seconds: f64 = number(&parsed, "--report")?;
            // `is_finite` first: NaN is neither greater nor less than zero, and `seconds <= 0.0`
            // alone would let it through.
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err("--report needs a positive number of seconds".to_owned());
            }
            Some(((seconds * f64::from(world.tick_rate_hz())).round() as u64).max(1))
        }
        None => None,
    };

    for _ in 0..ticks {
        if let Some((runner, _)) = runner.as_mut() {
            runner.due(&mut world);
        }
        world
            .step()
            .map_err(|err| format!("tick {}: {err}", world.tick()))?;
        if report_every.is_some_and(|every| world.tick().is_multiple_of(every)) {
            report(&world);
        }
    }

    if let Some((runner, _)) = runner.as_ref() {
        for (seconds, outcome) in &runner.outcomes {
            if *outcome != Outcome::Ran {
                println!("step at {seconds:>7.1}s {}", describe(outcome));
            }
        }
    }

    let players = world
        .entities()
        .iter()
        .map(|entity| entity.player)
        .filter(|player| *player != 0)
        .max()
        .unwrap_or(0);

    println!(
        "tick={} entities={} players={} hash={:016x}",
        world.tick(),
        world.entities().len(),
        players,
        world.state_hash()
    );
    Ok(())
}

fn report(world: &World) {
    let seconds = world.tick() as f64 / f64::from(world.tick_rate_hz());
    let units = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == ruage_sim::EntityKind::Unit)
        .count();
    let buildings = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == ruage_sim::EntityKind::Building)
        .count();
    println!(
        "{seconds:>7.1}s  wood {:>4}  food {:>4}  gold {:>3}  stone {:>3}  units {units:>3}  buildings {buildings:>2}",
        world.stock(1, ResourceKind::Wood),
        world.stock(1, ResourceKind::Food),
        world.stock(1, ResourceKind::Gold),
        world.stock(1, ResourceKind::Stone),
    );
}
