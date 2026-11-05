# Review: a route that goes stale is re-planned, not walked into

*The owner, 2026-11-05: "I still see scout bumping into trees and getting stuck. Can we learn something from
https://github.com/glouw/openempire?"*

## What the engine did, and what `openempire` does

The mover re-plans when a **body** blocks it (D98a: "anything else is non-past and earns one"). It did **not** re-plan when
**ground** blocks it: `collide` refused the step, tried each axis alone, and then returned `(0, 0)` - the unit stood still, for
ever, because nothing ever looked again. A route goes stale for ground reasons all the time: a house is built across it, a unit
is nudged off the line, or a pulled corner clips a tree the geometry thought it had cleared.

`openempire` answers this by **re-planning every unit's path every 25 cycles**, in ten slices a tick, whether or not it can name
a reason - and its boid layer adds a **wall push** that slides a unit off obstacles. Full notes, with the file and constants, are
in `docs/sources.md`.

**What this step took**: the principle, cheaply and deterministically. A ground refusal now asks for a re-plan, at most **two
per route** (`MAX_GROUND_REPLANS`), and the count is in the state hash because it decides whether a third may happen.
**What it did not take**: the unconditional 25-cycle re-plan and the boid wall-push. The first is affordable but unmeasured in
this engine; the second is float steering, which D9 forbids in the tick and which would fight D117's tile-passage rule (a push
that shoves a body through a gap the tiles say is closed is exactly what D117 forbids).

## The test that pinned it

`crates/sim/tests/stall.rs`: a villager walks a clear lane, and **twenty ticks in a house is dropped across its route**. Before
the fix it stopped at `(659850, 810000)` and stayed there for 2,400 ticks - the owner's report, in a test. After the fix it
re-plans and arrives.

## Performance

Whole acceptance match, 36,000 ticks, three runs each, same machine, `HEAD` (`c9077b7`) against this tree:

| | `c9077b7` | this step |
|---|---|---|
| run 1 | 11,784 ms | 13,588 ms |
| run 2 | 12,098 ms | 13,520 ms |
| run 3 | 11,944 ms | 13,396 ms |
| median | **11,944 ms** | **13,520 ms** |

**+13%**, and it is not a free change: the match itself now goes further (`154 -> 152` entities, `211 -> 237` **repaths**, and a
different flow - the lazy-and-stuck villagers are now arriving and doing work). The move is capped at **two extra searches per
route per order**, so the cost per stuck unit is bounded; the 26 extra repaths over a 36,000-tick match are that cap in action.

**The hash moved, as it must**: `4568cef16368bd5a` -> `e4bdd521acaaf342`, still a win (`players=1`).

## Stability

| attack | result |
|---|---|
| a house dropped across a walk | re-plans and arrives (`a_unit_replans_when_its_route_turns_into_a_wall`) |
| a goal that is genuinely walled off | two re-plans, then the unit stands and the engine records a refusal - it does not search every tick, which is the failure mode D98a's "past" rule exists to prevent |
| a route with no goal (a wander) | no re-plan is asked for: the branch requires a goal tile |
| the per-tick budget | the request goes through `pass.repaths`, which is capped by `MAX_REPATHS_PER_TICK` like every other |
| determinism | `stalls` is in the state hash, and nothing in the new path uses a float (`no_float` green) |
| a body repath arriving on a stalled route | leaves the stall count alone: only a ground re-plan spends one |

**Reported, not hidden**: the +13% is a real cost, and the alternative I measured is not a smaller version of the same fix but a
different design - `openempire`'s unconditional timer would cost a search every 25 ticks for every moving unit, which is more
work than this, not less. If the cost ever matters, the honest lever is the 25-cycle timer *instead of* the stall check, and it
should be measured then, not assumed now.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
