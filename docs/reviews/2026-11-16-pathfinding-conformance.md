# Review: the original's pathing rules as a separate set - and the defect writing them down found

**Change.** `crates/conformance` (new, D247/D249, `docs/pathfinding-conformance.md`): 33 tests that assert the original's
pathfinding and obstacle-avoidance rules against this engine's planners. Sources: `samuelcaldas/age-of-empires-2-decomp`'s
`src/zone/` modules for the shape, HD 5.7's own sites (`FUN_007ffd10`, `FUN_007f9d80`, `FUN_007fb220`) for the numbers.
**No test is `#[ignore]`d**: 28 green, 5 red - three rules of the game we do not keep (U25 a-c) and one defect of ours whose
fix was measured and reverted (U25 d). The crate is its own workspace, so the reds cannot fail the gate. Plus a
`make conformance` target, `docs/pathfinding-conformance.md`, D249, a SPEC row and queue item R10. **No engine code changed.**

## Performance

**Measured, and the honest shape of it is that this change cannot move a tick.** 33 tests in **0.185 s** wall, warm, debug,
on this box; no single binary above **0.01 s**; cold compile **1.8 s** (0.7 s with `ruage-sim` warm). Nothing in `crates/sim`,
`crates/data` or `config/` was touched - the diff is the new crate, the root manifest's `exclude`, the Makefile and docs - so
the acceptance figure is identical by construction, and I do not claim a tick measurement I did not take.

The number that is a property of the *subject*: `nearest_reachable`, corrected, costs **143 us a call on a 64x64 map** and
**1.1-3.2 ms on 200x200** (release; the box was shared, hence the range), because the rule needs a sweep of the reachable
region. That is fallback-path only - an order whose own plan failed - and it is not why the fix was reverted.

The other number of the subject, corrected in this pass: `short_range` declines a window past 40 tiles, and
`a_window_beyond_the_bound_is_declined` pins both sides of it - a 30-tile walk solves, a 58-tile walk declines. `short.rs:29-36`
states 40 tiles a side; the fixture shows the bound is the endpoints' box plus the 3-tile margin, so the first size that
declines is 35 tiles apart, not 41. That correction is in the test's comment.

## Stability

**What was attacked, and what the attacks found - including two of my own errors and one engine defect, measured twice.**

**Attack 1 - the suite's own arithmetic.** The first run panicked inside the suite: `attempt to multiply with overflow` at
`crates/conformance/src/lib.rs:106`, because the segment-to-point distance squared the cross product of raw-unit deltas
(800,000 per 40 tiles -> 6.4e11, squared past `i64`). Fixed by computing in `i128` and clamping the answer back. A helper that
overflows in debug and silently wraps in release is the worst shape for a measuring instrument.

**Attack 2 - my analytic costs were wrong, twice, and the tests said so.** `a_wall_is_paid_for_at_its_gap` first asserted
**104** for a route whose true cost is **88** (I counted the legs wrong: 1 diagonal + 3 cardinal each way = 44, not 52).
`a_reversed_search_reaches_the_other_end` first asserted the octile bound on a map whose block sits **on** the diagonal;
measured **144** against the **126** bound, because a block on the line must be paid for. Both were my arithmetic, not the
engine's.

**Attack 3 - the fixtures are not vacuous.** Removing the two blocked corners from `a_diagonal_may_not_cut_between_two_blocked_tiles`'s
map reddens it (the direct diagonal becomes legal and the route costs 14, the assertion demands more); restoring them turns it
green. A mutation of `path.rs` - deleting the both-corners test outright - was **not run**: another agent session was compiling
and running the `ruage-sim` suites in the same tree, and a temporary edit to `path.rs` is the "half-applied change" this
tree's rules forbid. It is owed on a quiet tree.

**Attack 4 - determinism.** `an_equal_cost_choice_is_made_identically_every_time` runs the same search nine times;
`a_route_does_not_depend_on_earlier_searches` interleaves forty other searches, including a failing one, between two identical
calls and demands the same route. Both green - the frontier's ordering is doing what `path.rs:8-9` claims.

**ATTACK 5 - the defect: `nearest_reachable` answers one step from the start, not the tile nearest the click.** On empty
ground from (1,1) toward (10,10) the answer is **(2,2)** with a one-step route; from (1,7) toward a wood's middle it is
**(2,6)**. Its frontier is popped best-first by distance to the click, but every tile it pushes is already free, so the first
pop is a neighbour of the start and it returns before the wavefront has moved. Its own doc comment and D134's *"nearest legal
standing point"* say the opposite. **Why the world suite never saw it**: `click_into_the_woods.rs` spawns the villager **one
tile** from the wood (1,4 against a wood at x = 3), so its assertion - "a legal tile within one tile of the block" - is
satisfied by the bug's own answer.

**The fix was built, measured and reverted, and that is the step's real finding.** Sweeping the reachable region answers the
tile nearest the click - (25,19) for a target at (25,17) - and the acceptance match goes from **155 entities and a win**
(`45b5bd2`, hash `8cf433ad4e0205f4`, 497 repaths) to **125 entities and no win** (hash `0f0a37dc86bf1002`, 451 repaths). Both
runs: same worktree, same generated data, seed 7, 48,000 ticks, release, **one file** different. The mechanism is measured,
not inferred: an env-gated trace shows the acceptance script makes exactly **two** `nearest_reachable` calls, both
`(26,13) -> (25,17)`, and the sweep sends that villager six tiles instead of one; a villager diverted at 1,303 s loses the win.
Landing the rule therefore means retuning `config/acceptance.toml` - ours to retune, since the game has no one-step answer -
and that is a ruling for the owner (U25 d), not a defect to land quietly. Filed as two red tests in `findings.rs` and queue
item **R10**.

Known and accepted: the set says nothing about the step layer's collision rules (the pause, the past filter, the mate
exemption) - those are the movement register's business. The decomp repo carries **no licence**, so nothing here is a copy of
its expressions: each test states a rule in our words and measures our API, and its addresses are AoC/PS2, so numbers come
from HD's own binary. Its C bodies are reconstructions of PS2 assembly (at least one loops `w` for both axes), so a claim
enters the table only where the masks, the node layout or a first-party source agrees.
