# 2026-11-22 - D351: a place another body holds is not a place to work from (branch, not main)

The owner, 2026-11-22, on the pile: *"HD definitely doesn't stack up that much. Maybe things like that could happen, but never
in this scale."* This branch is the measured attempt, and it **flips four fixtures that encode the press on purpose** - which is
why it is a branch he judges rather than a commit on `blunt-port`.

## What changed

- **`World::work_position(from, target, mover)`** - the place a worker stands to work a node. The decomp's only "the spot you
  were given is taken" rule is `func_002699A0`: step out by `ext_self + ext_other + 0.01`, one of four sides with an offset
  across that side, fifty tries, **refusing any candidate whose obstruction query is non-zero**; `func_002697C0` runs it for a
  group's children and never writes a child onto a point an earlier child holds. Our ring is that search in a deterministic
  order, and it asked only whether the **ground** was free (`is_free_ground` ignores units on purpose, so a building may be
  placed beside them) - so every man sent to one node was sent to **the same tile centre**. It now refuses a candidate whose
  centre lies inside another body's box, grown by the mover's footprint.
- **The re-seat** (`step.rs`, the gather arm): a worker standing *inside* another body, with `at_node` true and a free ring
  place available, walks to that place instead of taking from the node. Without it the seat is only decided once, at order time,
  and a man ordered while the other was still walking gets the same place - the movement pass never refuses a body (D279's
  port of the binary's epilogue), so he simply stands in him and the gather arm then freezes him there ("a man who is already
  taking does not move").

## Performance

Acceptance scene, `collision_probe` / `pile_probe`, 36,000 ticks, seed 7, release build:

- **deepest overlapping group: 11 -> 9 bodies**;
- **pairs closer than a fifth of a tile: 317,041 -> 274,054 pair-ticks (-13.6%)**;
- **overlapping pair-ticks: 349,824 -> 324,711 (-7.2%)**, on the same 33,536 of 36,000 ticks;
- **most overlapping pairs in one tick: 63 -> 42 (-33%)**;
- re-paths: 24,897 -> 10,703.

The probe itself: **20.8 s** for the 36,000-tick census with the change in, against **11.3 s** for the same probe on
`blunt-port` - the re-seat runs `work_position` (eight candidates, each a path plan) on every gathered tick where a worker
overlaps a friend, and that is where the time goes. Unpriced against a wall that matters yet, and the reason this is a branch
rather than a commit.

**And the transient overlaps rise with it, which is the trade this branch makes.** `pile_probe`'s attribution, same scene:
past **500 -> 454**, neither **164 -> 663**, and every added one is a `Gather` into `Gather` (595 of the 663) - men walking
across each other on their way to a re-seated place, because the movement pass still never refuses a body (D279's port of the
binary's epilogue: the step is taken, the collider is paused, the mover re-plans). So this buys **fewer persistent deep stacks**
(11 -> 9 body groups, -13.6% deep pairs) for **more passing through**, and the passing through is what the three plan ports in
`docs/next-step.md` are for: with routes that do not aim at one point, the men would not have to cross.

New overlaps rise (**877 -> 1,371**), which is the re-seat itself moving men, and the total pair-ticks inspected falls
(17,340,819 -> 14,220,218) because men stand further apart.

The failing fixture that this branch was written against - `one_place_per_man::two_gatherers_of_one_node_are_given_two_places`,
tracked over every tick **both are taking from the node** - read **4,598 raw apart against the 8,000 their footprints need**
before the change and passes after it.

**Acceptance hash: not measured** - the change flips four fixtures, so the scene is not comparable until they are dealt with.

## Stability

**Four fixtures go red, and they are the point of the branch** (all four assert the press as the intended behaviour):

- `a_working_man_is_never_moved` - D246's own fixture, the owner's *"villager is never to be nudged to leave place he is
  working at"*;
- `two_gatherers_on_one_tree_press_in`, `two_woodcutters_on_one_tree_share_its_side_and_press`,
  `ten_men_ask_for_one_bush_and_seven_press_into_it` - D289/D293, the owner's *"drop it and let the crowds jam"*;
- `two_villagers_on_one_node_cannot_take_more_than_it_holds`.

That is the honest shape of the question: **the stacking is not an accident in this engine, it is pinned by tests that encode
his own rulings**, and reversing one of them is his call, not mine.

Suite: **228 result lines, 940 passed, 5 failed, 25 ignored** - read as a set, not a total: the five are the fixtures above and
nothing else.

**What is not measured**: whether the remaining 9-body group is the same mechanism at coarser scale (the plan ports in
`docs/next-step.md` are untouched here), and what the acceptance wall does - the branch is not priced there until the fixtures
are settled.

## The capacity regression the purge found, and what it means for the next attempt

Reading the five reds rather than patching them turned up a defect in **this branch's own rule**:

- `ten_men_ask_for_one_bush_and_seven_press_into_it` now measures **5 of 10 men working the bush against the geometry's 7**.

The cause is the re-seat's *target*: it walks a crowded man to a **ring tile centre**, and a ring tile is 1.0 tile from the
node's centre - 0.5 from its box face - while taking requires the boxes to **touch** (`at_node`, D285). So a man re-seated to a
tile centre is no longer at the node and stops working. The decomp's rule steps out from the crowded *coordinate* by
`ext_a + ext_b + 0.01` (`func_002699A0`), i.e. **along the face he is already on**, not to a grid tile, which is why the original
keeps both the separation and the capacity.

So the next attempt is named and it is smaller than this branch: keep the place test (`work_position`'s body filter, which is
right), and replace the re-seat's *aim* with a step out **along the node's face** - tangent to the contact ring - falling back to
the other side when that side is held, and only then to a ring tile. **Not measured, not attempted here**: the branch is left as
the measurement of why the ring is the wrong target.
