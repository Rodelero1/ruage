# `cargo run` started nothing (D164)

*2026-11-08. The owner tried the debug switch and `cargo run --release` answered *"could not determine which binary to run"* - four binaries
across two packages, no `default-run`, so the first command anyone types to start the game is a puzzle.*

## What changed

`crates/view/Cargo.toml` gains `default-run = "window"`, so `cargo run --release -p ruage-view` starts the window and the `frame`
diagnostic binary still needs `--bin frame`. Nothing else moved.

## Performance

No code change: a manifest key. **Measured**: `cargo metadata` resolves cleanly (**0 s** to fail-or-not, no graph work) and
`cargo build --release -p ruage-view --bin window` still links the same binary; the acceptance hash is untouched because the simulation
is not involved.

## Stability

- **The failure mode was a first-run puzzle, not a crash**: a new binary in a package silently re-opens it, which is why the key is set
  explicitly rather than left to a one-binary package.
- **`--bin frame` still works**, so the diagnostic is not lost; only the default got a name.
