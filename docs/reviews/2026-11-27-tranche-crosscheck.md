# Crosscheck: D303, D306–D310 against primary sources (2026-11-27)

Rule: bank rows verbatim from `docs/archive/decisions-2026-11-24.md` §§ cited; evidence order per D203
(game → neighbouring OSS/published → our own files). No binary corpus on this box
(`/opt/data/tools/rea/pseudo-aokhd` absent, no `Age2HD/` dir), so FUN_* reads are checked
against the recorded pseudocode in `docs/archive/research/`, not re-read from the binary.
No code, bank, ADR, or live files touched.

## D303 — "The radius reaches the line test and the step, not the searches" — CONFIRMED (historical, code since deleted)

- Bank claim (l.8237): `plan_path_for` passes `reach = body_radius(entity.unit)` into `route_with`;
  line tier hands it to `leg_is_clear`; step applies it in `clearance_at`; flat + levelled searches take no reach.
- Evidence: crosscheck table `docs/archive/research/pathfinding-crosscheck-2026-11-20.md` row #7 says the same
  ("radius lives in the step… every route is a route a point could walk"); D306 review records the fix that closed it.
  `git log -S "leg_is_clear"` shows the code existed; commit `aafb740` ("route all movers through the geometric
  solver, delete the tile system") later removed `route_with`/`leg_is_clear`/pyramid entirely — current
  `crates/sim/src/ground.rs:route_with` calls `solve(...)`, no tiers. So D303 was true of the code it described
  and is now moot, not refuted.
- Fixture cited: `radius_route::the_gap_is_planned_for_a_villager_and_not_for_a_trebuchet` (parked `#[ignore]`d).

## D306 — "Tangency is a passage, and the radius reaches the searches" — CONFIRMED

- Bank claim (l.8256): owner measured in HD 2026-11-20 that a packed trebuchet walks a one-tile hole/gap;
  `Ground::fits` = `nearest blocked square − reach >= 0`; flat search + per-radius pyramid; tier 2 runs for any reach.
- Evidence: primary game observation in `docs/reviews/2026-11-20-d306-tangency-is-a-passage.md`
  ("What the screenshots change"); consistent with step rule D290 (depenetration computed, never applied).
  Binary side: `FUN_00840830` inclusive AABB + `+0x266`/`+0x268` inset is what makes tangency possible
  (`docs/archive/research/parity-plan.md:28`, `conformance-sequence.md:102`). Dat side: body-radius classes
  0.20/0.25/0.40/0.50; `config/gamedata/units.toml` carries per-unit `radius` (e.g. line 50).
- Caveat: `Ground::fits`/`pyramid_for` no longer exist post-`aafb740`; D306's mechanism was replaced, its game fact stands.

## D307 — "The stamp resolves at the finest level the obstacle occupies" — CONCLUSION CONFIRMED, BIT-GLOSS REFUTED

- Bank claim (l.8277): `FUN_007f0da0` recurses "only into cells whose second byte has bit `0x20` — 'this cell contains
  an obstacle'" down to level 0, stamping the grown box via `FUN_007ed950` (bit `0x10`).
- Evidence: recorded pseudocode `docs/archive/research/pathfinding-levels-evidence.md:256+` shows
  `if ((b & 0x20) == 0) recurse; else stamp-and-block`. A 2026-11-22 re-read **in that same file** withdraws the
  bank's gloss: bit `0x20` means "this cell is passable", not "contains an obstacle" (blocking an already-blocked
  cell would be a no-op; `FUN_007f1100` and `FUN_007ee760` read the bit the same way). D307's conclusion —
  growth resolved per tile, one-tile gap never washed into 8×8, D306 caveat withdrawn — survives unchanged;
  only the `0x20` label is inverted. Bank row postdates neither; bank text is stale on this point.
- Open item bank names (setter of `0x20`, exact `FUN_007ed950` box test) still open; not verified here (no corpus).

## D308 — "A coarse edge refines to the tile where the passage is, not to a cell centre" — CONFIRMED (read; port verified separately)

- Bank claim (l.8291): `FUN_007eeda0` expands cell to tile span `x << level .. ((x+1) << level) − 1`, scans shared edge
  tile-by-tile along direction bit `bVar1`, returns first interrupted tile + interpolation; waypoints on tile boundaries.
- Evidence: recorded pseudocode `docs/archive/research/pathfinding-levels-evidence.md:§FUN_007eeda0` matches the bank
  nearly verbatim; crosscheck file row #5 independently states the same rule with the same symbol. Port: D322
  (`Levels::refine` emits edge crossings; crosscheck file header notes the one residual: binary scans outward from the
  interpolated point, ours from the low end). 16-tile gate rationale follows directly.
- Not re-read from binary (no corpus); two independent in-repo readers agree.

## D309 — "No pre-search line test has been found; our first tier stays unverified" — UNVERIFIABLE (no contrary evidence)

- Bank claim (l.8309): `FUN_008000d0` goes straight to registry (`FUN_007ef140` → levelled `FUN_007ee980`/flat
  `FUN_008007f0`); `FUN_008a4360`→`FUN_007fdf80` is a list-tail walk, not a line test; `FUN_008a3a20` bracket shows no
  clearance check. Tier 1 (477 ns one-waypoint leg) unverified-as-parity, possibly ours alone.
- Evidence: crosscheck file difference #10 ("unverified — no reader has established whether a clear line short-circuits
  the search") corroborates that the negative result is genuine non-finding, not oversight; parity-plan P14 notes the land
  default really is the levelled search. A negative ("no test exists in functions read") cannot be confirmed without the corpus;
  the bank's prescribed settler (trace caller of `FUN_008000d0`, inspect `FUN_007ffae0` result) is still the right next read.
- Neighbouring OSS: no openage/genie statement on a straight-line shortcut found in-repo for this row.

## D310 — "The avoid list has no 255 cap, and that is left open on purpose" — CONFIRMED

- Full bank title (l.8330): "The avoid list has no 255 cap, and that is left open on purpose."
- Evidence: binary side — `docs/archive/research/movement-in-the-binary.md:260`: ignore list built in `FUN_008007f0`,
  array `local_404`, cap 255, stops filling at `0xfe`. Our side — `standing_bodies` unbounded by decision; the two stated
  reasons (selection order would be ours — id order vs binary's colliders-first — and no witness) are recorded, not smuggled.
  Post-bank measurement D329 (bank l.8842) closed reason 2 empirically: 0–400-body wall probe, routes bit-identical with/without
  truncation; acceptance match's largest live list is 17 entries (D323). Selection-order difference below 255 remains, correctly
  labelled as not-a-cap.
