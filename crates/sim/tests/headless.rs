//! The publishability condition (D4 = B): a stranger with their own install must be able to
//! clone, build and RUN this. So the runnable artefact is itself tested, from the outside, as a
//! process — not just the library API.

use std::path::PathBuf;
use std::process::Command;

fn config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/sim.toml")
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

#[test]
fn runs_a_world_and_reports_tick_hash_and_position() {
    let cfg = config();
    let text = run(&[
        "--config",
        cfg.to_str().unwrap(),
        "--seed",
        "7",
        "--ticks",
        "100",
        "--speed",
        "1.35",
        "--hp",
        "100",
    ]);
    assert!(text.contains("tick=100"), "stdout was:\n{text}");
    assert!(text.contains("hash="), "stdout was:\n{text}");
    // 1.35 tiles/s at 20 Hz for 100 ticks is 6.75 tiles = 135000 units at scale 20000, exactly.
    assert!(
        text.contains("x=135000"),
        "expected an exact displacement, stdout was:\n{text}"
    );
}

#[test]
fn the_same_invocation_twice_gives_byte_identical_output() {
    let cfg = config();
    let args = [
        "--config",
        cfg.to_str().unwrap(),
        "--seed",
        "7",
        "--ticks",
        "250",
        "--speed",
        "0.96",
        "--hp",
        "100",
    ];
    assert_eq!(
        run(&args),
        run(&args),
        "two runs of the same command must produce identical output, hash included"
    );
}

#[test]
fn a_missing_speed_is_an_error_not_a_default() {
    // D6: no numeric literals in shipped code. A default speed of 1.35 would be a game value
    // invented by the CLI, and it would silently give a plausible wrong answer.
    let cfg = config();
    let exe = env!("CARGO_BIN_EXE_ruage-headless");
    let out = Command::new(exe)
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--seed",
            "7",
            "--ticks",
            "10",
            "--hp",
            "100",
        ])
        .output()
        .expect("runner must be executable");
    assert!(!out.status.success(), "a missing --speed must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--speed"), "stderr was:\n{stderr}");
}

#[test]
fn a_repeated_flag_is_refused() {
    // A silently ignored duplicate is how a benchmark ends up running 10 ticks while the
    // command line says 10,000.
    let cfg = config();
    let out = Command::new(env!("CARGO_BIN_EXE_ruage-headless"))
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "--seed",
            "7",
            "--ticks",
            "10",
            "--ticks",
            "20",
            "--speed",
            "1.35",
            "--hp",
            "100",
        ])
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
    assert!(stdout.contains("--config"), "stdout was:\n{stdout}");
    assert!(stdout.contains("--speed"), "stdout was:\n{stdout}");
}

#[test]
fn a_different_seed_still_runs() {
    let cfg = config();
    let text = run(&[
        "--config",
        cfg.to_str().unwrap(),
        "--seed",
        "8",
        "--ticks",
        "10",
        "--speed",
        "0.8",
        "--hp",
        "100",
    ]);
    assert!(text.contains("tick=10"), "stdout was:\n{text}");
}
