# ADR 0013 — Scenario language and runners are not the engine (supersedes D198)

- **D-id alias:** D198
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

D17/D18 asked in words for an engine that knows kinds, not particulars; 1,656 lines inside `sim` contradicted them.

## Decision

`crates/script` holds the scenario language, scripted AI, rule runner and personality loader; `crates/harness` holds `ruage-headless` and `ruage-batch`. `ruage-sim` depends on `ruage-data` + `ruage-fixed` and nothing else; arrows never point back into the engine. Engine queries the script layer needs (`start_villager_unit`, `start_town_center_unit`, `is_free_ground`) live in `crates/sim/src/roles.rs`; scenario-needed methods made public are documented as such.

## Evidence

- **Game (rung 0):** Architectural. No game trajectory supports or refutes a crate split.
- **Rung 2 (bounded, read-only):** None applicable.
- **Owner:** Layering requirement inherited from D17/D18 (owner-accepted).

## Consequences

Behaviour unchanged by construction, measured: acceptance run prints the identical hash (`91572e72c33cee93`) from the new crate; workspace gate green (183 test binaries, clippy/fmt clean at the time).

## Fixtures

- Dependency rule: `ruage-sim` must not depend on `ruage-script`/`ruage-harness` (workspace structure).
- Acceptance hash equality across the move (recorded in the row).
