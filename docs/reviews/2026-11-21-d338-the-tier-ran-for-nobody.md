# D338 - P14: the tier's `avoid` gate is gone, and the gate that actually binds is the descent

**Crosscheck first.** Three sources, and they agree. *The binary*: `FUN_007ee980` - the levelled search's own body - calls
`FUN_007fd880`, `FUN_007ed5c0`, `FUN_007f0be0`, `FUN_007ede40`, `FUN_007edb80`, `FUN_007eeda0`, `FUN_007ee760` and
`FUN_007ee600`, and **no stander inserter**: `FUN_007f9d80`'s only callers are `FUN_007f96b0` and `FUN_007fbab0`, reached from
`FUN_007fb940` -> `FUN_007f9f10`/`FUN_007fa8a0` -> `FUN_007f7460`, not the levelled search. *openage's* pathfinding note, in
words: the high-level pathfinder "uses mip-mapping ... **It ignores moving objects, e.g. units or wildlife**, though it takes
static objects like buildings into account". *The first-party talk*: units "bump into others, stop and repath around". So the
coarse tier runs in a crowd and ignores bodies, and our `avoid.is_empty()` gate was the opposite rule.

**The prior implementation.** `Ground::route_with` (`ground.rs:556-652`) tried the pyramid only when the avoid list was empty
(`:595`), on walks of 16 tiles or more (`:592`) whose descent level was at least 2 (`:598`), certifying each refined leg with
`&[]` (`:606-618`). D334 measured what that meant: **121 long walks in one acceptance match, 121 of them carrying a stander,
0 coarse searches**; lifting the gate then answered 6 with the hash unchanged.

**Landed:** the gate is gone - the tier is asked on every long walk, bodies or not. The leg certification keeps `&[]`
(terrain and static objects only), because the coarse search's blindness to bodies is the game's, and a chain that walks
through a friend is answered the game's way: the step refuses, the collider joins the plan's avoid list, the route is re-made.

**Witness, red first** - `crates/sim/tests/tier_ignores_bodies.rs`, through the order path (`Command::Move` ->
`World::route_points`), not `Levels` directly, so the gate is on its path:

> *"a long walk with one stander near it must come from the hierarchy, whose waypoints are cell-edge crossings; every interior
> waypoint here is a tile centre, so the flat search answered: [(730000, 970000), (1290000, 810000)]"*

Green after: the route's interior waypoints sit on a tile boundary (`n * scale`), which is where the refinement puts a
cell-edge crossing (D322) and where no tile-centre route can go.

## The finding: the avoid gate was not what kept the tier off

The slice added a counter (`Ground::tier_answers`, instrumentation like `plan_count`) and a probe
(`crates/sim/tests/tier_probe.rs`, ignored, release). One acceptance match, seed 7, 48,000 ticks:

| configuration | long walks | no chain | level < 2 | answered | routes | re-paths | hash |
|---|---|---|---|---|---|---|---|
| **floor 16, level >= 2 (landed)** | 185 | 58 | **127** | **0** | 61,276 | 41,473 | `527bf4ebb9520fa0` |
| floor 16, level >= 1 | 130 | 62 | 68 | 13 | 80,117 | 61,563 | `69a548722252b27e` |
| floor 16, level >= 0 | 182 | 57 | 125 | 108 | 80,267 | 59,771 | `d700295118db1114` |
| floor 8, level >= 2 | 35,351 | 227 | 35,117 | 7 | 50,665 | 36,220 | `2e4e7c4e306a2d67` |
| floor 0, level >= 2 | 40,757 | 1,613 | 38,974 | 7 | 50,665 | 36,220 | `2e4e7c4e306a2d67` |

So the tier answers **0 of 61,276** routes in the acceptance match even with the gate gone, and the binding clause is the
**descent floor** (`coarse.level >= 2`, D303): 127 of 185 long walks start on a tile whose 4-tile cell is not usable - the
acceptance map is wooded - so the flat search answers them, which is also the game's answer at level 0.

**The 16-tile floor re-tested, and kept.** Lowering it to 8 (and to 0, which answers the same 7 routes - the effective floor is
between 8 and 16) does not widen the tier's reach much and rewrites the match (hash `2e4e…`, 36,220 re-paths against 41,473).
The floor is ours, not the binary's (D306: a walk past one tree became a 31-point staircase at short range), and the reason it
still earns its place is that **the refinement for short spans is the unread half** - sequence step 8 (the scan direction) is
what would make a short chain cheap. Keep 16; re-test after step 8.

## Performance

Same tree, same pack, seed 7, 48,000 ticks, release, idle box.

- acceptance hash **`527bf4ebb9520fa0`**, re-paths **41,473**, entities **132**, routes planned **61,276** - every one
  identical to D337's landed build, so the match is bit-identical: the tier answering 0 routes moves nothing here.
- wall, six runs of the landed build: **15.85, 15.88, 15.90, 16.03, 16.05, 17.86 s**, median **15.96 s**. D337 quoted
  15.70 s for the same hash in a previous session; the box's own spread over three consecutive runs is **2 s**
  (17.86 to 15.85), so the 0.26 s is not attributed to the change and is not a claim.
- **What the change cost and what it no longer costs.** Asking the pyramid on every long walk put **185 coarse searches**
  into a match that answers none of them. Because `coarse_route` derives its level as the lower of the two ends' descents, the
  level is knowable without searching: the descent is now checked first, so **127 of the 185 searches are skipped** and the
  change costs 58 searches a match. The wall cannot resolve the difference (it is inside the spread above); the count is what
  the optimisation is worth.
- The probe (`tier_probe`, release) itself takes 16.0 s for 48,000 ticks, which is the match's cost, not the counter's.

## Stability

- Suite: **216 result lines, 935 passed, 0 failed, 21 ignored** (the count before this slice was 214 / 934 / 20; the two new
  ones are the witness and the ignored probe). Read from the exit line of a `--workspace --no-fail-fast` run, never a grep
  mid-run. Conformance **21 / 0 / 0** across 9 binaries.
- Clippy `--all-targets -D warnings` clean; `cargo fmt --check` clean.
- The fixture was watched failing on the **un-cut** tree and green after.
- Attacked: the tier's gate (the new fixture), the certification's body blindness (measured as 0 answers, above), the two
  levers that could hide the effect (level floor and distance floor, each moved alone), and determinism (hash identical across
  three runs).

## The AI review (OCR, D209), and what was taken

`make ai-review` read 4 files, 4 comments, and **cut two for budget** (`crates/sim/src/lib.rs`, `crates/sim/tests/tier_probe.rs`)
- both are the counter and its probe, read by hand instead. Three findings were accepted and one was accepted by reversing a
decision I had just made:

1. **Accepted (medium, real).** `pyramid_for` and both descents ran before the `long_walk` test, so every non-straight route paid
   for a pyramid build on a cold cache. Moved inside the arm; the short-circuit now only runs for long walks.
2. **Accepted (low).** `coarse.level >= 2` was redundant once the descent is checked first - `coarse_route` derives exactly
   that minimum (`levels.rs:162`), so the check is one spelling of one condition, and it was dropped.
3. **Accepted, and it reversed the commit's earlier form (medium).** Certifying the chain with `&[]` broke `route_avoiding`'s
   published promise: a re-plan told to keep clear of a body could be handed a chain through it, and D320's "a collider that
   has not moved since the plan is past" would then let the mover walk through the man he was told to avoid. The certification
   takes `avoid` again. Measured: **0 answers either way** in the acceptance match, so the promise costs nothing.
4. **Accepted (low).** The fixture's failure message blamed the flat search for an empty interior when a one-point route would
   have produced none; it now asserts the chain first and says which case it is.

**Refused, with the reason:** none. The one finding that argued the tier should ignore bodies altogether is the rule for the
*search* (the binary's), not for our `route_avoiding` contract - the two are different promises and the review read them as one.
