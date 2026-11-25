# ADR 0001 — No game data in git (supersedes D8)

- **D-id alias:** D8 (2026-10-23)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

The engine is built against the player's own install. Anything derived from that install is redistributable only by its owner, not by us.

## Decision

Nothing derived from the game files is committed. `Age2HD/`, `config/gamedata/`, `build/`, saves, assets stay gitignored. A pre-commit hook refuses them. Docs quote illustrative values only.

## Evidence

- **Game (rung 0):** Not a game-behavior claim. No dat/binary/save evidence applicable.
- **Rung 2 (bounded, read-only):** No applicable reading. `sld-extractor` declares no licence, so default copyright applies — read, never copy (per D21 row).
- **Owner:** Owner's requirement to comply with EU law.

## Consequences

Clones regenerate data locally via the generator repo against their own install. `docs/licensing.md` holds the full policy. `config/MANIFEST.toml` stays untracked.

## Fixtures

- `.gitignore` refuses `Age2HD/`, `config/gamedata/`, `build/`, `*.aoe2spgame`, `*.aoe2record`, `*.mgz`, `*.mgx`.
- `.githooks/pre-commit` refuses staged game-data paths.
- Policy text: `docs/licensing.md`.
