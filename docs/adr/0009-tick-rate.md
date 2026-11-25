# ADR 0009 — Tick rate is a per-match setting, 20 Hz default (supersedes D16)

- **D-id alias:** D16 (2026-10-25)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Owner asked whether a higher rate buys anything given no savegame compatibility and no clone is needed. Priced, not guessed.

## Decision

Tick rate is per-match (`tick_rate_hz` + `fixed_point_scale` in handshake and replay header; mismatched peers refuse rather than desync). Default 20 Hz. Nothing in the engine may assume the rate; mod content expresses durations in seconds, engine converts. Revisit only when multiplayer exists and input latency is measurable.

## Evidence

- **Game (rung 0):** The dat's own values sit on a 50 ms grid (20 reload values, all multiples of 0.05) — 20 Hz represents them exactly; a higher rate buys resolution the data does not contain. Supported, not proven, by two independent lines: the 50 ms reload grain and ~10 syncs/s cadence in real HD recordings vs 59 in DE (SPEC §6.1, U12 corroboration).
- **Rung 2 (bounded, read-only):** openage deliberately has no fixed tick (wall-clock event queue, 50 ms cap) — recorded contrast (D14 row, item 3): similarity here would be a mistake, not a model. Cited as what we do not follow, and why.
- **Owner:** Owner's question; pricing accepted. Higher rates buy only input latency (50 ms vs 16.7 ms), tunnelling behavior, and off-grid quantisation — none of them fidelity — against linear CPU cost and 3× tighter §6.1 bands.

## Consequences

`sim.toml` is the source of truth. Changing the default is a config edit, not a migration. The wanted modding work (civ differences, art) lives in the effects/tech interpreter and is clock-independent.

## Fixtures

- Dat reload table (all values multiples of 0.05 — corpus-swept).
- `tick_rate_and_scale_must_agree` (60 Hz with 20 kHz scale refused).
- `unproven`: the 20 Hz value itself remains corroborated-but-unproven (U12 open).
