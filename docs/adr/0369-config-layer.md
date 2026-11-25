# ADR 0369 — Game data lives in config/ as generated TOML (supersedes D6)

- **D-id alias:** D6 (2026-10-23)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Owner's requirement, stated explicitly. Numbers must be reproducible, diffable, overridable for experiments, and typos must fail loudly.

## Decision

Engine reads TOML at runtime, never the `.dat`. Nothing numeric is hardcoded. Three layers: generated `config/gamedata/` (reproducible, diffable), hand-edited `config/overrides.toml` (survives regeneration), hand-written behavior TOMLs. Loading is strict: unknown or missing key is a startup failure. `MANIFEST.toml` is checked at startup for staleness (dat hash differs → refuse with stale-config error).

## Evidence

- **Game (rung 0):** Not a game-behavior claim. Structural rule about where numbers live.
- **Rung 2 (bounded, read-only):** None applicable.
- **Owner:** Owner's explicit requirement (layering, strictness, staleness check all named by the owner).

## Consequences

If D4 lands publishable, `config/gamedata/` is gitignored and generated at first run (already the case — ADR 0359). Override precedence: `overrides.toml` beats `gamedata/`; regeneration never clobbers it. Later paydowns built on this: D45 `overrides.toml` surface, D50 content-hash handshake, D137 name checks.

## Fixtures

- Strict-loader tests: unknown key → startup error; missing key → error; `-1.0` sentinel refused where not allowed.
- Override precedence test (`overrides.toml` beats `gamedata/`).
- Manifest-staleness test (dat hash differs → startup refuses).
