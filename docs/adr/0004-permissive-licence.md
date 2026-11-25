# ADR 0004 — Permissive engine licence, attribution for art (supersedes D21)

- **D-id alias:** D21 (2026-10-25)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Every comparable engine is GPL-3.0 or AGPL-3.0. Permissive code flows into GPL projects; the reverse does not. The choice costs nothing and keeps distribution and modding-tool options open.

## Decision

Engine and content schema: MIT OR Apache-2.0. Future original art/audio: CC-BY-SA 4.0 with per-asset attribution. Game data: never shipped (ADR 0001). Nothing borrowed from GPL code; unlicensed code (`sld-extractor`) is read-only, never copied.

## Evidence

- **Game (rung 0):** Not a game-behavior claim.
- **Rung 2 (bounded, read-only):** Field survey in `docs/licensing.md` (openage, OpenRA, OpenRCT2, openblack, genie-rs: GPL-3.0; `librematch/genie`: AGPL-3.0). Read for licence facts, nothing copied.
- **Owner:** Survey presented, outlier status accepted.

## Consequences

Modders may copy the schema without a licence question. D22 (bring-your-own-copy) is parked to the next slice, not decided here.

## Fixtures

- `LICENSE-MIT`, `LICENSE-APACHE` at root.
- Policy: `docs/licensing.md`.
