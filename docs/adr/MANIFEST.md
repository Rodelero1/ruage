# ADR manifest (migration in progress)

One row per old D-id. A bank row is deleted when its ADR is signed (owner rule 2026-11-26); this manifest keeps the mapping so the deletion is traceable. D13's row was deleted with ADR 0003 (correction recorded there).

| D-id | ADR | State |
|---|---|---|
| D8 | `docs/adr/0001-no-game-data-in-git.md` | Signed 2026-11-25 |
| D4 | `docs/adr/0002-publish-engine-tooling.md` | Signed 2026-11-25 |
| D12 | `docs/adr/0003-generator-separate-repo.md` | Signed 2026-11-25 |
| D21 | `docs/adr/0004-permissive-licence.md` | Signed 2026-11-25 |
| D22 | — | Parked to next slice (bring-your-own-copy binds ADR 0001/0002) |
| D17 | `docs/adr/0005-four-layers.md` | Signed 2026-11-25 |
| D18 | `docs/adr/0006-target-shape.md` | Signed 2026-11-25 |
| D9 | `docs/adr/0007-fixed-point-domain.md` | Signed 2026-11-26 |
| D10 | `docs/adr/0008-scale-grains.md` | Signed 2026-11-26 |
| D16 | `docs/adr/0009-tick-rate.md` | Signed 2026-11-26 |
| D5 | `docs/adr/0010-dat-authoritative.md` | Signed 2026-11-26 |
| D6 | `docs/adr/0011-config-layer.md` | Signed 2026-11-26 |
| D15 | `docs/adr/0012-scenario-scope.md` | Signed 2026-11-26 |
| D15 (third clause: search half) | `docs/adr/0014-flat-tile-search.md` | Signed 2026-11-27 |
| D198 | `docs/adr/0013-script-harness-split.md` | Signed 2026-11-26 |
| D296 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D297 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D298 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D299 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D300 | — (premise and number withdrawn by D301, recorded in 0015) | Superseded 2026-11-27 |
| D301 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D302 | `docs/adr/0015-three-tier-route.md` | Signed 2026-11-27 |
| D304 | `docs/adr/0014-flat-tile-search.md` (budget rule retired there) | Signed 2026-11-26 |
| D305 | `docs/adr/0015-three-tier-route.md` (gate hole closed by geometry) | Signed 2026-11-27 |
| D303 | `docs/adr/0016-radius-tangency-retired.md` | Signed 2026-11-28 |
| D306 | `docs/adr/0016-radius-tangency-retired.md` | Signed 2026-11-28 |
| D307 | `docs/adr/0016-radius-tangency-retired.md` (conclusion survives, `0x20` gloss corrected) | Signed 2026-11-28 |
| D308 | `docs/adr/0016-radius-tangency-retired.md` | Signed 2026-11-28 |
| D309 | `docs/adr/0016-radius-tangency-retired.md` (stays open, recorded as retirement) | Signed 2026-11-28 |
| D310 | `docs/adr/0016-radius-tangency-retired.md` | Signed 2026-11-28 |
| D169 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D170 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D171 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D172 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D173 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D180 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D181 | `docs/adr/0017-gather-standoff-retired.md` (superseded by D293, retired here) | Signed 2026-11-28 |
| D182 | `docs/adr/0017-gather-standoff-retired.md` (cap half retired; re-target half shared with 0018) | Signed 2026-11-28 |
| D186 | `docs/adr/0017-gather-standoff-retired.md` (superseded by D199, retired here) | Signed 2026-11-28 |
| D188 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D191 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D192 | `docs/adr/0017-gather-standoff-retired.md` (gate retired; take-gate superseded by D285) | Signed 2026-11-28 |
| D199 | `docs/adr/0017-gather-standoff-retired.md` | Signed 2026-11-28 |
| D200a | `docs/adr/0018-crowd-collision-retired.md` (superseded by D200, retired here) | Signed 2026-11-28 |
| D200 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D204 | `docs/adr/0018-crowd-collision-retired.md` (nudge half superseded by D239/D241, re-target stands here; partial closed by 0019) | Signed 2026-11-28 |
| D239 | `docs/adr/0019-pathing-adoption-retired.md` | Signed 2026-11-28 |
| D241 | `docs/adr/0019-pathing-adoption-retired.md` (D204 partial closed by this) | Signed 2026-11-28 |
| D205 | `docs/adr/0018-crowd-collision-retired.md` (built, measured not landable as written — carried) | Signed 2026-11-28 |
| D285 | `docs/adr/0020-take-gate-retired.md` | Signed 2026-11-28 |
| D286 | `docs/adr/0020-take-gate-retired.md` | Signed 2026-11-28 |
| D289 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D290 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D291 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D292 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D293 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D294 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D295 | `docs/adr/0018-crowd-collision-retired.md` | Signed 2026-11-28 |
| D175 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D176 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D177 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D178 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D179 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D183 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D184 | `docs/adr/0021-area-leftovers-retired.md` (guard; conformance UNVERIFIED) | Signed 2026-11-27 |
| D185 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D187 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D189 | `docs/adr/0021-area-leftovers-retired.md` (superseded by D200, retired here) | Signed 2026-11-27 |
| D193 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D194 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D195 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D196 | `docs/adr/0021-area-leftovers-retired.md` | Signed 2026-11-27 |
| D197 | `docs/adr/0021-area-leftovers-retired.md` (removed by D279's re-port, recorded here) | Signed 2026-11-27 |
| D1–D3, D7, D11, D14, D19, D20, D23–D168, D174, D190, D201–D203, D206–D238, D240, D242–D284, D287–D288, D311–D358 | — | Pending, in trunk order |

Rulings made this slice:
- ADRs numbered from 0001 in migration order (Q11 amended 2026-11-25: owner was wrong; old bank deleted at the end, so no permanent collision). Superseded 2026-11-26: bank rows are deleted on sign-off instead (owner rule), so no terminal bulk deletion.
- Hygiene rows carry explicit "not a game-behavior claim" instead of a fake game citation.
- D22 deferred to keep this slice to four.
