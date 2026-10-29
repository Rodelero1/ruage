//! No floating point in the simulation's gameplay code (D67).
//!
//! Borrowed from `openaok`, whose `--verify` gate fails the build on `float`, `double` or `ToFloat()`
//! anywhere in its simulation sources. The rule they enforce is ours already (D9): fixed-point only, so a
//! replay on another machine cannot round differently. A rule with no gate is a rule nobody keeps.
//!
//! Written before the fix (D7): it fails today on `pierce_dps`, which divides damage per second as `f64`
//! and hands that to a ratio that decides how many arrows a garrison fires.
//!
//! Two exemptions, both named rather than silent:
//!
//! - **`script.rs`**: a scenario is *authored* in seconds ("mill at 2:10") and the runner converts to ticks
//!   once. That is an authoring surface, not a simulation value; the ticks it produces are integers.
//! - **stat derivation at load** (`apply_scalar`, `multiply_class`, the hp setup): the dat's percentages
//!   are applied once when the world is built, and the result is stored as `Fixed`. It is arithmetic on
//!   content, not on a match, and it cannot differ between machines because it does not depend on a tick.

use std::path::{Path, PathBuf};

/// Functions in `lib.rs` where a float is allowed because it is load-time content arithmetic.
const LOAD_TIME: &[&str] = &[
    "apply_scalar",
    "multiply_class",
    "scaled_health",
    "scale_fixed",
];

fn sim_sources() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    collect(&root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .expect("reading the sim's sources")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The name of the function a line sits in, by the crude but sufficient rule "the last `fn` above it".
fn enclosing_fn<'a>(lines: &'a [&'a str], upto: usize) -> &'a str {
    lines[..upto]
        .iter()
        .rev()
        .find_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed
                .strip_prefix("fn ")
                .or_else(|| trimmed.strip_prefix("pub fn "))
                .or_else(|| trimmed.strip_prefix("pub(crate) fn "))
                .or_else(|| trimmed.strip_prefix("const fn "))?;
            rest.split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
                .next()
        })
        .unwrap_or("")
}

#[test]
fn the_simulation_keeps_no_floating_point_in_its_tick() {
    let mut offenders = Vec::new();
    for path in sim_sources() {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        // Authored rather than simulated: seconds in, ticks out. A scenario is written as "mill at 2:10"
        // and the harness converts once; nothing in the conversion survives into a match. `batch.rs` is the same
        // surface as `headless.rs` - a runner that reads a scenario's duration - and is named here rather than
        // quietly exempted by pattern.
        if name == "script.rs" || name == "headless.rs" || name == "batch.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("reading a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            // A mention in a comment is prose, not a value.
            let code = line.split("//").next().unwrap_or("");
            if !code.contains("f64") && !code.contains("f32") {
                continue;
            }
            // A declaration line names its own function; every other line belongs to the one above it.
            let owner = enclosing_fn(&lines, index);
            let declared = enclosing_fn(&lines, index + 1);
            if LOAD_TIME.contains(&owner) || LOAD_TIME.contains(&declared) {
                continue;
            }
            offenders.push(format!("{name}:{}: {}", index + 1, line.trim()));
        }
    }

    assert!(
        offenders.is_empty(),
        "the simulation computes in fixed-point only (D9); these lines compute in floating point:\n{}",
        offenders.join("\n")
    );
}
