# ADR manifest (migration in progress)

One row per old D-id. Old rows stay authoritative until the ADR is signed off; deletion happens only after full coverage.

| D-id | ADR | State |
|---|---|---|
| D8 | `docs/adr/0001-no-game-data-in-git.md` | Signed 2026-11-25 |
| D4 | `docs/adr/0002-publish-engine-tooling.md` | Signed 2026-11-25 |
| D12 | `docs/adr/0003-generator-separate-repo.md` | Signed 2026-11-25 |
| D21 | `docs/adr/0004-permissive-licence.md` | Signed 2026-11-25 |
| D22 | — | Parked to next slice (bring-your-own-copy binds ADR 0001/0002) |
| D17 | `docs/adr/0005-four-layers.md` | Signed 2026-11-25 |
| D18 | `docs/adr/0006-target-shape.md` | Signed 2026-11-25 |
| D9 | `docs/adr/0007-fixed-point-domain.md` | Draft — needs owner sign-off |
| D10 | `docs/adr/0008-scale-grains.md` | Draft — needs owner sign-off |
| D16 | `docs/adr/0009-tick-rate.md` | Draft — needs owner sign-off |
| D5 | `docs/adr/0010-dat-authoritative.md` | Draft — needs owner sign-off |
| D6 | `docs/adr/0011-config-layer.md` | Draft — needs owner sign-off |
| D15 | `docs/adr/0012-scenario-scope.md` | Draft — needs owner sign-off (A* clause parked to movement) |
| D198 | `docs/adr/0013-script-harness-split.md` | Draft — needs owner sign-off |
| D1, D2, D3, D5–D7, D9–D11, D13–D20, D23…D358 | — | Pending, in trunk order |

Rulings made this slice:
- ADRs numbered from 0001 in migration order (Q11 amended 2026-11-25: owner was wrong; old bank deleted at the end, so no permanent collision).
- Hygiene rows carry explicit "not a game-behavior claim" instead of a fake game citation.
- D22 deferred to keep this slice to four.
