# Review: license ADRs 0359–0362 (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `docs/adr/0359-no-game-data-in-git.md` (D8), `0360-publish-engine-tooling.md` (D4), `0361-generator-separate-repo.md` (D12 + D13 correction), `0362-permissive-licence.md` (D21).
- `GLOSSARY.md` (8 canonical terms), `docs/adr/MANIFEST.md` (4 drafted, D22 parked, rest pending).
- Ruling: numbered 0359+ past max D358, not 1–4. Hygiene rows state "not a game-behavior claim" rather than faking rung-0 evidence. D22 deferred.

## Performance

Zero engine diff by construction: `git status --short` shows only `GLOSSARY.md` + `docs/adr/`. No tick, snapshot or suite number can move. Suite not re-run for this slice; trunk gate (suite green + zero engine diff) is owed at trunk sign-off, not per ADR.

## Stability

Attacked: (1) old rows still present — bank, `docs/live/` untouched, nothing deleted pre-sign-off; (2) aliases — D8/D4/D12/D21 each appear once in ADR headers and manifest; (3) max-D claim — `grep D[0-9]` over the bank tops at D358, so 0359+ collides with nothing; (4) fixtures named exist — `.gitignore`, `.githooks/pre-commit`, `LICENSE-MIT`, `LICENSE-APACHE`, `docs/licensing.md` all present. One error of mine caught while writing: `docs/live/data-tooling.md` D12 text matches the bank row, so no silent amendment was introduced.

## Accepted / refused

- Accepted: four ADRs as drafts needing owner sign-off. Nothing here authorizes deletion.
- Refused: nothing this slice. D22 parking and the 0359+ numbering are rulings already ledgered, not review findings.
