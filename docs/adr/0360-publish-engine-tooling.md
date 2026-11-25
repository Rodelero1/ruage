# ADR 0360 — Publish engine and tooling, keep spec internal (supersedes D4)

- **D-id alias:** D4 (2026-10-23, amended 2026-10-25)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Owner's condition: a stranger with their own install must be able to clone, build and run. Long-term plan is an open-source engine, so provenance is cheaper now than retrofit.

## Decision

Option B: publish engine + tooling, keep `docs/mechanics/`, SPEC ground-truth sections, oracle notes and `config/gamedata/` internal. Licence split: `crates/` MIT/Apache, generator GPL-3.0 in its own repo. Engine code must not cite internal docs. Publish order forced: split generator first (done 2026-10-26, `Rodelero1/ruage-tools`, history rewritten so the parser never appears in it), then the engine repo is publishable as-is.

## Evidence

- **Game (rung 0):** Not a game-behavior claim.
- **Rung 2 (bounded, read-only):** No applicable reading.
- **Owner:** Owner chose B after four options costed; owner set the clone-build-run condition.

## Consequences

Anything under `crates/` must be defensible as original work, never transcription. Blocking gap recorded, not solved here: a public clone cannot yet produce `config/gamedata/` — generator must be run from the separate repo (see ADR 0361).

## Fixtures

- `LICENSE-MIT`, `LICENSE-APACHE`, `README.md` at root.
- Generator split commit history: engine history contains no `tools/` Python import (`git log --all -- tools/` empty of parser).
- Publishability checklist: SPEC §6.2 (retained pointer only).
