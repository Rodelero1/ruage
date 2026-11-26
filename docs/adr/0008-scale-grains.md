# ADR 0008 — Scale derived from two measured grains (supersedes D10, amends D9)

- **D-id alias:** D10 (2026-10-23)
- **Status:** Signed 2026-11-26.

## Context

D9's "scale must divide a power of ten" was arithmetically inverted: it accepted 1, 2, 4, 5, 8 (none representable for a single dat value) and rejected 300. An adversarial review proved a config could load with `schema = 99`, `floats_in_sim_state = true`, `max_entities = 0`, one RNG stream, or 60 Hz on a 20 kHz scale — under which 34 of 57 speeds lose per-tick exactness.

## Decision

Two derived rules: `scale % 10^dat_value_decimals == 0` (no dat value rounded) and `scale % (tick_rate_hz × 10^per_tick_decimals) == 0` (no per-tick drift). Measured grains: `dat_value_decimals = 4` (`work_rate = 0.0111`, units 110 and 1021), `per_tick_decimals = 3` (speed `0.737`). `config/sim.toml` carries both grains; changing `tick_rate_hz` moves the scale with it (60 Hz → 60000).

## Evidence

- **Game (rung 0):** The grains are measured from the dat: 4-decimal work rates and 3-decimal speeds exist in `config/gamedata/`. `crates/data/tests/corpus.rs` sweeps the corpus and fails if any value ever needs more places — the measurement is re-checked on every regeneration.
- **Rung 2 (bounded, read-only):** None applicable.
- **Owner:** Adversarial-review finding accepted 2026-10-23.

## Consequences

An invariant living only in prose is not an invariant — both grains are config plus a corpus sweep. Contradictory configs are refused at load.

## Fixtures

- `crates/data/tests/corpus.rs` (whole-corpus sweep).
- `scale_must_give_exact_per_tick_displacement`, `tick_rate_and_scale_must_agree` (RED-recorded in SPEC §11).

## Sign-off note (2026-11-26)

Signed as written. D10 bank row deleted on sign-off per the 2026-11-26 rule.
