//! The publishability condition (D4 = B): a stranger with their own install must be able to
//! clone, build and RUN this. So the runnable artefact is itself tested, from the outside, as a
//! process — not just the library API.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn units() -> PathBuf {
    repo("config/gamedata/units.toml")
}

/// The generated unit table is gitignored (D8), so these tests skip when it is absent.
fn have_units() -> bool {
    if units().exists() {
        return true;
    }
    eprintln!(
        "skipping: {} is not present (generated data, D8)",
        units().display()
    );
    false
}

fn paths() -> [String; 4] {
    [
        repo("config/sim.toml").display().to_string(),
        repo("config/map.toml").display().to_string(),
        repo("config/economy.toml").display().to_string(),
        units().display().to_string(),
    ]
}

fn run(args: &[&str]) -> String {
    let exe = env!("CARGO_BIN_EXE_ruage-headless");
    let out = Command::new(exe)
        .args(args)
        .output()
        .expect("the headless runner must be executable");
    assert!(
        out.status.success(),
        "runner failed: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout must be UTF-8")
}

fn seeded_args(seed: &str, ticks: &str) -> Vec<String> {
    let [sim, map, economy, units] = paths();
    vec![
        "--config".into(),
        sim,
        "--map".into(),
        map,
        "--economy".into(),
        economy,
        "--units".into(),
        units,
        "--seed".into(),
        seed.into(),
        "--ticks".into(),
        ticks.into(),
    ]
}

fn as_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

#[test]
fn runs_a_seeded_map_and_reports_tick_entities_and_hash() {
    if !have_units() {
        return;
    }
    let args = seeded_args("7", "100");
    let text = run(&as_refs(&args));
    assert!(text.contains("tick=100"), "stdout was:\n{text}");
    assert!(
        text.contains("entities=8"),
        "2 town centres + 3 villagers each:\n{text}"
    );
    assert!(text.contains("players=2"), "stdout was:\n{text}");
    assert!(text.contains("hash="), "stdout was:\n{text}");
}

#[test]
fn the_same_invocation_twice_gives_byte_identical_output() {
    if !have_units() {
        return;
    }
    let args = seeded_args("7", "250");
    let refs = as_refs(&args);
    assert_eq!(
        run(&refs),
        run(&refs),
        "two runs of the same command must produce identical output, hash included"
    );
}

#[test]
fn a_missing_map_is_an_error_not_a_default() {
    // D6: the CLI must not invent a map. Without --map there is nothing to seed.
    if !have_units() {
        return;
    }
    let [sim, _map, economy, units] = paths();
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .args([
            "--config",
            &sim,
            "--economy",
            &economy,
            "--units",
            &units,
            "--seed",
            "7",
            "--ticks",
            "10",
        ])
        .output()
        .expect("runner must be executable");
    assert!(!out.status.success(), "a missing --map must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--map"), "stderr was:\n{stderr}");
}

#[test]
fn unit_stats_are_no_longer_accepted_on_the_command_line() {
    // They were removed when seeding landed: a --speed flag is a game value invented by the CLI.
    if !have_units() {
        return;
    }
    let args = seeded_args("7", "10");
    let mut with_speed = as_refs(&args);
    with_speed.extend(["--speed", "1.35"]);
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .args(&with_speed)
        .output()
        .expect("runner must be executable");
    assert!(!out.status.success(), "--speed must be refused");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--speed"), "stderr was:\n{stderr}");
}

#[test]
fn a_repeated_flag_is_refused() {
    // A silently ignored duplicate is how a benchmark ends up running 10 ticks while the
    // command line says 10,000.
    if !have_units() {
        return;
    }
    let args = seeded_args("7", "10");
    let mut doubled = as_refs(&args);
    doubled.extend(["--ticks", "20"]);
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .args(&doubled)
        .output()
        .expect("runner must be executable");
    assert!(!out.status.success(), "a repeated flag must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("repeated"), "stderr was:\n{stderr}");
}

#[test]
fn help_prints_usage_and_succeeds() {
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .arg("--help")
        .output()
        .expect("runner must be executable");
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("usage:"), "stdout was:\n{stdout}");
    assert!(
        stdout.contains("--map"),
        "usage must name the map flag:\n{stdout}"
    );
}

#[test]
fn a_units_path_that_does_not_exist_names_the_file() {
    if !have_units() {
        return;
    }
    let [sim, map, economy, _units] = paths();
    let missing = Path::new("/nonexistent/units.toml");
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .args([
            "--config",
            &sim,
            "--map",
            &map,
            "--economy",
            &economy,
            "--units",
            missing.to_str().unwrap(),
            "--seed",
            "7",
            "--ticks",
            "10",
        ])
        .output()
        .expect("runner must be executable");
    assert!(!out.status.success(), "a missing file must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("units.toml"), "stderr was:\n{stderr}");
}
