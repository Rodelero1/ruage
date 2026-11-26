# ADR 0003 — Generator lives in its own repository (supersedes D12, records D13 correction)

- **D-id alias:** D12 (2026-10-23, amended same day); corrects D13 (2026-10-23)
- **Status:** Signed 2026-11-25.

## Context

The dat reader is GPL-3.0. The engine is MIT/Apache. Sharing a repository would export the GPL obligation to the engine's history.

## Decision

`tools/genconfig` and any parser fork live in a separate repository. The engine repo stays pure Rust, never imports Python, and consumes `config/` TOML only. A fork keeps the GPL; the licence boundary is a repository boundary.

## Evidence

- **Game (rung 0):** Not a game-behavior claim.
- **Rung 2 (bounded, read-only):** `librematch/genie` surveyed 2026-10-23. `genie-dat` cross-validated step 1. Correction recorded: `genie-core` is AGPL-3.0 (port of DE 158041, f32 arithmetic), so adopting its readers is withdrawn — the network clause is the strongest obligation available. `GenieDatParser` (GPL-3.0, separate repo) remains the less restrictive choice.
- **Owner:** Owner's suggestion after the GPL framing correction.

## Consequences

Amendment stands: the "fork is likely" reasoning was wrong — per-task lists are readable via `dat.unit_data.tasks`, and `unique_unit_effect_ids`/`name2` are DE-5.9-only, absent from 5.7.2, so no fork is currently needed. Split is a publish-time step, not a code change. Revisit only if the engine ever reads the dat at runtime (would force engine GPL).

## Fixtures

- Engine history: `git log --all -- tools/` holds no parser import.
- Engine consumes TOML only: `crates/` has zero dat-reader imports.

## Sign-off note (2026-11-25)

Signed under reading (a): refined TOML output, same boundary. Readings (b) (own Rust reader) and (c) (GPL parser in-engine) parked; (c) refused while ADR 0004 stands.
