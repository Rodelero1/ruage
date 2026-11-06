# No pushing allowed: the head-on overlap, closed (D131 A)

*2026-11-06. The owner chose **A** — "I think A should be it", and his own memory that *"narrow passes with 2 units passing
in opposing directions were a problem"* in the original. The defect: two head-on villagers came **628 raw units** inside the
**8,000** their two radii need, measured at tick 66 of the one-tile corridor.*

## What was wrong

Two rules handed a body its **full** step into another body's space:

- **rule 4** — a moving body on the mover's own tile: wait, or give way;
- **rule 9** — the mutual block: whichever the pass visits second takes its whole step.

"Give way" was the anti-deadlock mechanism, and it worked: it also stepped straight through the invariant. The step is
**1,000 raw** (0.05 tile); the pair sat at exactly `reach` (8,372 was the contact distance in the fixture, 8,000 the sum of
the radii), so one full step ended **628** inside.

## The fix

`World::bound_to_reach(here, other, step, reach)`: the largest fraction of the step that ends at or outside `reach`, found
by **halving** — the full step, then 1/2, 1/4, 1/8, 1/16, 1/32 — five integer probes, no `sqrt`, no division by anything
the compiler has to round. If even 1/32 does not fit, the step is zero and the unit waits.

**It is scoped, and the scoping is the interesting part.**

### Two players' units only

Bounding **every** pair — friends included — broke the match, measured:

| | baseline | bound on every pair | bound on hostile pairs |
|---|---|---|---|
| entities at 1,800 s | 141 | **164** | 141 |
| players alive | **1** | **2** | 1 |
| refusal lines | 574 | **1,021** | 574 |
| reactions | 188 repaths, 187 made-room | 16 / 28 | 188 / 187 |
| hash | `f328ebc8d4d34b78` | `9a28aea697b97e4f` | **`f328ebc8d4d34b78`** |

The every-pair version is not "safer": it is a different game in which nobody finishes anybody, because a base's own
villagers cannot get past each other. The deck says which rule is intended — page 9, *"Formations allow for some friendly
overlap"* — so the bar is between units of **two different non-neutral players**, and friends keep brushing past.

### A body at rest is not bounded

First attempt bounded rule 8 too (the blocker is not moving). **Seven tests failed**, all gather and rally:

```
a_gatherer_stands_beside_the_node_and_not_on_it
the villager ended at (15, 15) for a node at (20, 15): it must stand beside it, not on it
```

The cause is arithmetic, not intent: a resource node carries **no radius**, so `reach` falls back to `self.scale` — a whole
tile. Bounding against that forbade a villager from standing beside the tree it had been sent to chop, and it stopped five
tiles out on the node's boundary circle. Rules 4 and 9 are about two bodies that both want the ground; a tree never yields.

## The test, and what it now says

`crates/sim/tests/footprint.rs`, renamed from `a_head_on_pair_in_a_one_tile_corridor_passes` to
`a_head_on_pair_in_a_one_tile_corridor_never_overlaps` — the old name asserted the behaviour the owner has now ruled out,
and a test that keeps an overruled name is a lie in the suite. It also spawns the pair as **two players'** villagers, which
is the case the rule is about.

**RED, watched** (`cargo test -p ruage-sim --test footprint`):

```
thread 'a_head_on_pair_in_a_one_tile_corridor_never_overlaps' panicked at crates/sim/tests/footprint.rs:169:9:
the pair came 7372 raw units apart on tick 66, inside the 8000 their footprints need
test result: FAILED. 2 passed; 1 failed; 0 ignored
```

**GREEN**: 3 passed. Two assertions, both needed: the invariant for 2,000 ticks, **and** that the pair actually met
(`closest <= reach + half a tile`) — without the second, a pair that walked away from each other would satisfy the first for
free, which is the D130 defect in another costume.

## What A costs, stated plainly

The pair **no longer gets past each other** in a one-tile corridor: they stop nose to nose. That is the game's documented
behaviour (*"units bump into others, stop and repath around"*), and it is what the owner remembers being a problem in the
original — so it is fidelity, not a regression. In the open, the repath around the blocker is the exit and it is unchanged.

## Performance

- **Acceptance match, 36,000 ticks, seed 7**: hash `f328ebc8d4d34b78` — **identical to the baseline**, which is the proof
  that the scoped rule changes nothing in the shipped scenario. 141 entities, 188 repaths, 187 made-room, 574 refusal lines,
  all equal to the baseline run taken on the same machine minutes earlier.
- **Wall clock**: **11.7 s** against **10.6 s** for the baseline (three runs of each ranged 10.6-11.7 s; the difference is
  inside that spread, and the rule cannot add work — `bound_to_reach` runs only when a hostile pair is already blocked).
- **The probe itself**: at most six integer multiply-compares per blocked hostile step, and only in the branch that used to
  return the full step. No allocation, no `sqrt`, no float.

## Stability

- **It cannot loop or panic.** Five halvings, arithmetic on `i64` raw positions; the largest value is a squared gap over a
  map 64 tiles wide (`(64 * 20_000)^2 = 1.6e12`), four orders inside `i64`.
- **The deadlock the rule could have introduced is bounded by design**: at the boundary every probe fails and the step is
  zero, which is a **wait**, not a lock — the unit keeps its order, rule 7 has already asked for a repath around the blocker,
  and the pair separates the moment either one is ordered anywhere else or the blocker moves.
- **Degenerate cases**: a pair already inside each other is untouched (rule 3 takes the step, "a body already overlapped is
  not made worse"), and `a_pair_standing_inside_each_other_can_separate` still passes.
- **Scope is enforced by the entity player**, so a neutral body (player 0) and a same-player friend can never be bounded by
  accident: a bug there would show up as the acceptance hash moving, and it did not.

**719 passed / 0 failed / 19 ignored** (the corridor test is no longer ignored); clippy `-D warnings` clean; `cargo fmt
--check` clean.
