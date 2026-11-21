# OPEN - the tier runs again (P14); next is P16, bodies as boxes

**Where this stands (D338, `blunt-port`):** suite numbers and the acceptance A/B are in D338 and the review note; conformance
21 / 0 / 0. The *order* to 100% conformance is **`docs/conformance-sequence.md`** - read it first; this file is the immediate
step and the outstanding forks.

**What step 3 changed** (D338): `Ground::route_with` no longer refuses the pyramid when a body is in the avoid list. The gate
was backwards - the game's coarse search *ignores moving bodies* - and it made the tier answer **0 of 121** long walks in the
acceptance match. The tier's leg certification still takes the avoid list, so a chain that would walk through a friend is
refused and the flat search answers: the game's *search* is blind to bodies, but `route_avoiding`'s promise is not. The descent
is checked *before* the search (it is the same quantity), which skips 127 of the match's 185 coarse searches. Witness: `crates/sim/tests/tier_ignores_bodies.rs` - a `World::plan_path` walk through the order
path, long, with a stander near it, whose waypoints can only be cell-edge crossings if the hierarchy answered.

**The finding that matters for the next session:** with the gate gone the tier still answers **0 of 61,276** routes, because
D303's descent floor (`coarse.level >= 2`) rejects 127 of 185 long walks on this wooded map. Loosening it answers 13 (level 1)
or 108 (level 0) and rewrites the match each time - a route-wide slice of its own, with its own A/B. The 16-tile floor was
re-tested and **kept**: lowering it to 8 answers 7 routes and rewrites the match, and it only earns a lower value once the
short-span refinement (step 8) is read.

## The next step: 4 - P16, body collision as boxes

**Prior implementation (read it before editing).** Two bodies collide as **discs**: squared distance against the sum of radii
(`lib.rs:3129`), with `reach_between` (`lib.rs:3259`) and `target_contact` (`lib.rs:3332`) and `body_touches` (`lib.rs:3414`)
built on the same disc, and the step's verdict in `crates/sim/src/world/step.rs` (the refusal block around `:1450-1560`, the
no-arrival stand at `:1881`). The building gate already treats a man as a box (`build.rs:199`, D250). Pinned by
`collision`, `head_on`, `building_passage`, `crowd_piles_up`, `group_overlap`, `overlap` and the probes `collision_probe`,
`step_speed_probe`; the probe `collision_probe::what_a_box_would_refuse_that_a_disc_takes` measured **4,741 of 218,920**
body-steps (2.17%) where both boxes overlap and the discs are clear.

**The rule, read** (`FUN_00840830`, four `<=` on the grown AABBs - see D334 and `docs/movement-in-the-binary.md`): a body is an
AABB grown by the mover's radius, and touching counts. The arrival arithmetic must move in the same slice, or a box refusal and
a disc arrival disagree and the crowd freezes (`reach_between`, `target_contact`, `body_touches`).

**Witness owed:** the probe's 4,741 box-only steps fall to 0, with `building_passage`, `collision`, `head_on` green. Crowds are
stricter under boxes; if the suite reddens on jams, the pause carries it or the slice parks with its numbers.

## The perf item this session inherits

- **The acceptance wall: 15.70 s against 9.72-9.88 s key-only, on a different match** (132 entities, 41,473 re-paths in both
  cases; the sqrt helpers account for 0.16 s). Profile the 15.7 s run **per pass** before touching anything, and price the
  per-search allocations (49 KB a call, ~2.2 GB a match at 45k calls).

## Also ready, independent of those

- **P17 - waypoint substitution** (`mConsecutiveSubstituteCount`): a waypoint a re-plan keeps returning is skipped.
- **A waiting walker is an obstacle**: `standing_bodies` skips any body with a route; the binary's test is velocity zero.
- **The 300 row** (an object of type 8/`0x20` on the cell or beside it) and **the refinement's scan direction** (outward from the
  interpolated point): both reads are done, each is a small slice.
- **The mate rule (step 11)** - evidence gathered, see `docs/prior-art.md` "the mate rule": villagers never join a formation,
  so a villager crowd is solid in the original (bug threads are full of villagers blocking each other). The exemption is an
  explicit per-body **ignore list** (`FUN_00840830:43-54`: count `+0x24c`-adjacent, ids at `+0x254`, skipped on a match), not
  "everyone ordered in one tick". Ruling: villagers solid, the pause carries the jam; a formation model stays a separate gap.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list**
  (`+0x24c`/`+0x250`/`+0x254`) is one read: it is what decides whether a military formation ever fills it.
