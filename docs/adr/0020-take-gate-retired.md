# ADR 0020: one box ruler decides every distance — take-gate retired as built

**Status:** Signed 2026-11-28. Deleted 2 bank rows on sign-off: D285, D286.

## What this batch is

D285 ports the game's distance: `func_002690E0` subtracts both bodies' sizes per axis and clamps at zero — border to border, overlap exactly 0 — and motion compares it with the order's `rangeDistance`, so a range-0 job arrives when the boxes touch. Four of our approximations went with it. D286 draws the consequence the owner ordered (*"follow the game/decomp"*): an order names its target object, and one box measure answers every distance question — plan, step, and arrival.

This ADR retires the pair as built, and records the six things the port needed beyond the rows, all measured.

## The story, in the order it happened

1. **D285 — the arrival port.** `World::box_gap_sq` is the game's function; `World::node_radius` gives a node its box (resource/building one tile wide, animal its own body). Deleted: `touching` + `touch_tolerance`, `own_place` + eighth-tile, `ring_open`-as-arrival, `beside_adjacent`. Acceptance passed in 52.52 s (55.25 s before); nothing added to the tick.
2. **D286 — four edits, landed together.** `Command::Move` / `Task::Move` gain an optional target entity, filled when the click lands; `ground::Clear` becomes a box (half-extents per axis); the plan's clearances include blocked tiles' boxes, not only bodies; the step asks the grown-box question and a step into the target is cut at its face instead of refused. The step half alone measured 21.81 s but turned three green fixtures red — the plan still worked tiles and handed the step routes it refused — so all four landed as one; acceptance then passed in 33.74 s with the three green.
3. **Six measured extras the rows did not name.** Only the target's *solid* part is a box (farm plot and untouched foundation stay walkable — `solid_boxes`); arrival is within-one-step, never equality (diagonal box-stop sits farther by centres than the radii sum — measured 400,000-tick stall at progress 0); a dat-radius-less body has no box (carcass walk-over); the certified leg is the walked one (trebuchet corner); inside may step out but never deeper (our recovery rule — the original's placement never produces the case); in-reach is box-to-box in every arm (melee, shepherd's blow).

## What survives (live rules, with witnesses)

- **`box_gap_sq`** (`lib.rs:3675`): per-axis subtract-both, clamp 0, squared sum — the game's `func_002690E0` (decomp `unit_action_distance.cpp` verbatim; original returns sqrt, ours keeps the square — the `_sq` label is ours and stays).
- **`node_radius`** (`lib.rs:3462`): unit → own body radius, else half tile.
- **`solid_boxes`** (`lib.rs:3475`): blocked tiles of a footprint only — farm, fresh foundation, town-centre yard stay walkable.
- **`target_contact`** (`lib.rs:3529`): step cut at the first face — *"Setting position to exact waypoint"* (HD `0x7f4687`), the owner's 0.70/0.935 saves.
- **`at_node <= step`** (`step.rs:1638`): one step is the quantized form of the game's continuous contact, pinned by `crates/sim/tests/take_gate.rs` (within-one-step takes, two-steps-out walks, dead predecessors stay deleted — 3/3 green).
- **Order names its target; grown boxes in search and query** (`FUN_007f9d80`, `FUN_00840830` +`0x268`, collide on `<=`); ours collides on overlap (`<`) so a body resting exactly at its radius can still walk a wall — named deliberate difference, contact point unchanged.

## What is gone (mechanism deleted, rows deleted with it on sign-off)

- `touching`/`touch_tolerance`, `own_place`, `ring_open`-as-arrival, `beside_adjacent` (D285, already out).
- Dead gather-arm scaffolding beside the gate plus its orphan cascade (`ring_open`, `ring_point`, `GATHER_RING_MILLI`, `GATHER_PLACES`, lib `SIN` — tiles keeps its own copy): deleted under the pin; absence pinned by the `dead_arrival_predicates_stay_deleted` guard.

## Evidence (D203 order)

1. **Original game first.** Box gap CONFIRMED in decomp (both sizes, clamp 0, overlap→0; HD-binary equivalent not located). Range compare CONFIRMED (`FUN_007f4330`, epsilon 1e-7 @`0x7f6487`, log @`0x7f64a3`; range-zeroing @`0x7f4908`). Target arm CONFIRMED byte-exact (`0x8a8010`, `0x127d`, count==1 gate; twin site motion.cpp:1355 recorded). Ignore list self+target CONFIRMED (`FUN_008007f0`). Grown-box CONFIRMED both sites. Exact waypoint CONFIRMED (`0x7f4687`). Carcass-zero CONFIRMED from dat (`SHEEP_D` etc. `[0,0,0]`).
2. **Adjacent sources second.** openaok (`95cb8a05`): edge-to-edge boxes, `in_range` touch rule, target-carrying paths, grown stander boxes — precedent, metric differs (Euclidean vs per-axis). genie (`6eb73570`): `footprint_gap` the exact subtract-both-clamp-0 shape, `PathData.range/target`, `smoothing_ignore` = unit+target+annexes. Klinger deck: shrink-A/expand-obstructions idiom, "no pushing". Exact waypoint, solid-only, cut-at-face, may-step-out unverifiable in siblings — rest on HD reads, named as such. No contradictions anywhere.
3. **DE third.** Box-gap shape CONFIRMED at `0x141060d90` (per-axis sub, fabs, `maxss 0.0`, sqrt). Range gate shape exists (`0x1410612c1`), bound unresolved — SPLIT. Target-done and exact-waypoint strings have NO DE counterpart (full-dump absent). Grown-box by citation (0018/0019). Extras unverifiable, none contradicted. Full note: `docs/reviews/2026-11-28-0020-crosscheck.md`.
4. **Disagreements parked, not smoothed.** `<` vs `<=` deliberate (wedge guard). Inside-object recovery ours alone. Within-one-step quantization ours (disclosed). DE range-bound identity, `0x14111c7d`-area touching polarity, and HD melee-arm box usage untraced.

## What this commits us to

An order that names what it is about, and one box measure in the plan, the step, and the arrival test. No second distance.

## Sign-off checklist (code half done, verdict owed)

- [x] `take_gate` 3/3 green (2 pin + 1 absence guard)
- [x] Task 2 deletion executed under the pin; `take_gate` + `gather_touching` (3/3) + `acceptance_economy` (2 passed, 1 ignored, 311.65 s on this box — slower than the 52.52 s D285-time baseline; different box/debug build, finding not veto) green
- [x] Full suite triple 942 passed / 3 failed / 23 ignored (reds: 2 known pre-existing `modifiers` Town-Watch + `scratch_crowd3` in foreign untracked `scratch_debug.rs`, untouched) + clippy `-D warnings` clean (lib + pin targets; workspace-wide blocked by the foreign file) + fmt clean, recorded in review note on sign-off
- [x] Owner verdict: sign (2026-11-28)
- [x] On sign: bank rows D285 + D286 deleted, MANIFEST per-D-id rows → 0020, ids stripped from `decision-areas.toml`, index + live files regenned, signoff note with Performance + Stability written, committed by name, pushed
