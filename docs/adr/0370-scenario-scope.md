# ADR 0370 — Hand-authored map and scripted opponent; replay is a command list (supersedes D15 in part)

- **D-id alias:** D15 (2026-10-23)
- **Status:** Accepted in part. Old row untouched until owner sign-off.

## Context

Owner's three blockers for step 5, priced: an RMS interpreter is a language implementation, `.per` likewise, `.scx` needs a binary reader — none makes a villager gather wood. The slice's job is proving the loop, not the content pipeline.

## Decision

The map is hand-authored 64×64 in `config/` (forests, berries, gold, two town centres) — not `.rms`, not `.scx`. The opponent is a fixed command list on a tick schedule, which doubles as the replay format. `.rms` (122), `.scx` (49) and `.per` (74) stay future work; adding any is a new decision, not a refactor. Headless, one civ (D3), British vs British.

## Evidence

- **Game (rung 0):** Structural scoping, not behavior. The counts (122/49/74) are install inventory (SPEC §3.5).
- **Rung 2 (bounded, read-only):** None applicable to the scoping half.
- **Owner:** Owner decided all three blockers.

## Consequences

Parked to the movement branch: D15's third clause (A* on the static grid with `movement.toml` weights) — migrated with pathfinding, not here.

## Fixtures

- `config/map.toml` (authored map), `config/scenario.toml` (scripted opponent / replay).
- Determinism proof for the format: same script twice is byte-identical (headless subprocess test).
