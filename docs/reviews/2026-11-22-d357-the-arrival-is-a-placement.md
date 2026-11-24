# The arrival is a placement: the pile halves, and the re-plans multiply by eight

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Lands the owner's ruling of 2026-11-22** (D356.1)

**The ask:** *"the second man steps to the next free spot near the same tree/mine (the game's own placement rule), and only goes
idle if there is none"* — after *"HD definitely doesn't stack up that much ... never in this scale"*.

## Test first

`crates/sim/tests/one_place_per_man.rs`, carried from the `d351-reseat` branch, was watched failing on this tree:

```
MEASURED the two gatherers 0 raw apart at their closest, against the 8000 their footprints need:
a place another body holds is not a place to send a man
```

Green after the change. Three fixtures that pinned the **old** ruling (D289/D293, *"drop it and let the crowds jam"*) asserted the
opposite and were **restated, not deleted**: `gather_press_in::two_gatherers_on_one_tree_press_in`,
`gather_spread::two_woodcutters_on_one_tree_share_its_side_and_press`, and the press clause of
`gather_continuous::ten_men_ask_for_one_bush_and_seven_press_into_it`. Each now demands **at least 0.40 of a tile** between two
men on one node, quotes the ruling that reversed it, and says in its own header that the assertion was inverted.

## What the rule is, in the decomp's shape

`func_002699A0` (`unit_action_resolve.cpp`) picks a **side**, places the candidate `ext_a + ext_b + 0.01` out from it with an
offset **along** that side, and refuses any candidate whose obstruction query is non-zero — fifty tries. Ours is the same shape:
the node's box grown by the man's own radius is his contact surface, its four faces are the four sides, and the seat is offset
**along a face** by the pair's radii plus a hundredth. `func_002697C0` calls the child's own **set-position**, so the separation is
written, not stepped: a step is refused by the very body it separates from (measured: a man held a seat **7,000 raw** away and
never moved, his delta refused on every tick).

Two shapes were built and thrown away first, each for a measured reason: a radial step pulled back onto *distance* left the man
**0.024 tiles** off the node's face (so he stopped taking — the same capacity defect D351 hit), and snapping that step per axis
lost the tangential offset it had just made.

**Who yields**: the man who is not working (D246's rule 4 is absolute — measured, the first version moved a working man
**7,835 raw**), and when both work, the binary's right of way: the lower id keeps the spot (`008a80ff cmp eax,esi / jle`).

## Performance

Acceptance scene, 48,000 ticks, seed 7, three runs each:

| | wall (median) | entities | re-paths | outcome |
|---|---|---|---|---|
| before (D353) | **10.10 s** | 124 | **12,847** | draw (`players=2`) |
| after | **24.83 s** | **153** | **138,588** | **win** (`players=1`) |
| after, rule switched off | 9.93 s | 124 | 12,847 | draw |

The re-path figure is the cost: **10.8×** for **1.23×** the entities, and the wall time **2.5×**. The cause is named, not guessed:
the two control runs above differ *only* in whether the seat fires, so the work is in the rule — men that used to share one point now
stand a body's width apart and **stamp twice as many tiles as blockers**, and every walker whose route crosses a node's ring re-plans
where it used to pass. The plan's body test is a per-tile stamp (D296–D310's port), so the price is paid at plan time, not in the
arrival.

The pile, measured on the same seed (`collision_probe`, 36,000 ticks):

| | overlapping pair-ticks | deeper than half a footprint | new overlaps/tick | deepest |
|---|---|---|---|---|
| before | **144,534** (0.91%) | 129,512 | 888 | 8,000 raw |
| after | **74,717** (0.48%) | 70,499 | 437 | **8,000 raw** |

**Halved, not eliminated**: the deepest pair is still a full footprint apart, which is a pair the seat did not reach — a `Return`ing
carrier walking home into another, not an arrival at a node. That is the next slice's first question, and the number that says it is
still open.

## Stability

- Suite run recorded below; the three restated fixtures are the only behavioural reversals, and each names the ruling it follows.
- No panic, no hang, no refusal-count change on the engine side: the acceptance run's last refusal is the same `NoPath` on a
  walled-off target as before.
- **The trade is deliberate and is the owner's own priority** (D275: *"fidelity first, and cost is a later question"*): the rule is
  the game's, it halves the stacking and it turns a draw into a win, and the plan-time cost is now the **top performance item** in
  `docs/next-step.md` rather than a reason to keep the jam.
- **My own errors, in order**: a radial seat that left men 0.024 tiles off the face (caught by the capacity fixture); a per-axis
  snap that undid its own separation (caught by the same fixture); an inverted right-of-way comparison (`friend < mover` where the
  mover was the one who should keep his place) that made every seat a no-op — found by tracing `AT_NODE ... held=true seat=None`.
