# ADR manifest (migration in progress)

One row per old D-id. Old rows stay authoritative until the ADR is signed off; deletion happens only after full coverage.

| D-id | ADR | State |
|---|---|---|
| D8 | `docs/adr/0359-no-game-data-in-git.md` | Draft — needs owner sign-off |
| D4 | `docs/adr/0360-publish-engine-tooling.md` | Draft — needs owner sign-off |
| D12 | `docs/adr/0361-generator-separate-repo.md` | Draft — needs owner sign-off |
| D21 | `docs/adr/0362-permissive-licence.md` | Draft — needs owner sign-off |
| D22 | — | Parked to next slice (bring-your-own-copy binds ADR 0359/0360) |
| D1, D2, D3, D5–D7, D9–D11, D13–D20, D23…D358 | — | Pending, in trunk order |

Rulings made this slice:
- ADRs numbered 0359+ (continue past max D358), not 1–4.
- Hygiene rows carry explicit "not a game-behavior claim" instead of a fake game citation.
- D22 deferred to keep this slice to four.
