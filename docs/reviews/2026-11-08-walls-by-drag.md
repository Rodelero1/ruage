# Walls by drag: one bend, and a queue that finally works (D155)

*2026-11-08. The owner: walls go **vertically, horizontally and at 45 degrees**; *first left click starts placement, second click
ends it*; *one corner … could be done*; and his answers: **C** (the view hands the sim a list of tiles), **B** (a blocked
segment is skipped, not fatal, and the preview shows it red), **A** (the engine picks the bend — 45° first, then straight),
**5A** (sim and tests first).*

## What shipped

**`World::wall_line(from, to) -> Vec<(u16, u16)>`** — the geometry, in `world/lines.rs`, with no world and no state, so the
**view** can draw its preview from the same function the command uses. One bend at most: a **45° leg first** (both axes at once,
corner to corner), then the straight leg along the longer axis. Both ends included; a click with no drag is one tile.

**`Command::BuildLine { villager, unit, tiles }`** — the list, in drag order. Each tile is started or queued in turn; a tile that
cannot be built on is **skipped** and the rest of the line still stands, including the part **past** the obstacle. The whole
command fails only when *no* tile could be built, and then with the first refusal.

**`Command` is no longer `Copy`** (`Clone` stays): a line of a hundred segments is handed over once, not copied on every pass.

## The bug this uncovered - and it was not mine

The first version queued segments 2..N through `start_build`, and **only the first segment was ever built**. Instrumenting
`start_build` showed why:

```
start_build at (4,8) busy=false queue_len=0   <- started
start_build at (5,8) busy=true  queue_len=0   <- queued
start_build at (6,8) busy=true  queue_len=1   <- queued
start_build at (7,8) busy=true  queue_len=2   <- queued
start_build at (5,8) busy=false queue_len=2   <- the advance path... queued it AGAIN
```

The queue's advance in `world/step.rs` popped the next site and called `start_build`, which asks *"is there a queue? then queue
this instead"* — so a site popped from a queue of three went straight back to the back of it, and a villager that finished a
building with two more queued **never started either**. That is a **pre-existing engine defect** on D66's path, not something
this feature introduced: it needs **three** queued sites to show, and the existing `build_queue` tests queue two.

Fixed by splitting the immediate path out: `World::start_build_site` starts a site with no regard for a queue, `start_build`
keeps the "queue this instead" rule, and the advance path now calls `start_build_site`. `a_drag_lays_every_segment_of_the_line`
is the regression test: four segments, all four built.

## The tests

`crates/sim/tests/wall_lines.rs`, six tests, written before the behaviour:

```
a_horizontal_drag_is_a_row_and_a_vertical_one_a_column   ok   (pure geometry, passed on arrival - see below)
a_diagonal_drag_touches_at_corners                       ok
a_drag_that_is_neither_goes_diagonal_then_straight       ok
a_single_click_is_one_segment                            ok
a_drag_lays_every_segment_of_the_line                    FAILED -> ok
an_ineligible_tile_is_skipped_and_the_rest_stand         FAILED -> ok
```

The four geometry tests were written expecting the function not to exist, so **RED was a compile error**, not an assertion —
worth stating plainly rather than dressing up as a watched failure. The two behaviour tests failed for real: *"four segments,
not one: [(4, 8)]"* and, after the queue fix, *"the two blocked tiles are skipped, the other four stand: [(4, 8), (5, 8)]"*
(a house is **2×2**, so it ate two tiles of my line and my expected count of three was wrong — my error, corrected).

## Two of my own mistakes, both caught by running tests

1. **`submit_checked` queues; `step` applies.** My helper spawned a villager and immediately ordered a build, and every order was
   refused `MissingEntity`. A given command is not in the world until a tick drains it.
2. **A house is 2×2, not 1×1.** The "skip the blocked tile" test expected a 6-tile drag to lose one tile and build five; it
   loses two and builds four. The test now asserts the segment **past** the house exists, which is the claim that matters.

## Performance

**Inert on the acceptance match**: hash **byte-identical** to before this step, `45596da62c9e3668`, 135 entities, 125 repaths,
116 made-room. No drag happens in that scenario, and `wall_line` is a pure function called once per placement.

**Measured**: the six-test wall suite runs in **1.13 s**; the acceptance run on an idle box took **9,390 / 9,287 ms** for the two
runs recorded under D154, and nothing in this change touches the tick loop except the queue advance, which now calls a
slightly *smaller* function (`start_build_site`) instead of `start_build`. `wall_line` allocates one vector of
`max(|dx|, |dy|) + 1` tiles — for a hundred-tile wall, 101 pairs, allocated once.

## Stability

- **The corrected invariant is the interesting one**: a queue of any depth now advances, because the advance path no longer
  re-queues what it just popped. Before, three or more queued sites deadlocked with the villager idle and the queue full.
- **The line cannot cheat**: every segment goes through `check_build` (price, civ, prerequisites, site, reachability) at the
  moment it starts, so a drag past a razed forest still builds and a drag into a wall still refuses.
- **`EmptyLine`** exists so an empty list is refused by name instead of quietly doing nothing.
- Suite: **749 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.

## Not done, deliberately

- **The UI drag** (the owner chose sim first): the view still has no placement path. `wall_line` is public for it, and the
  preview/red-tile drawing is a separate slice.
- **Wall types**: one role (`palisade_wall`); stone/fortified (`117/119/155`) remain content work.
- **Per-tile refunds**: deleting a queued segment out of the middle of a line is possible (`Delete` on the foundation) but a
  drag cannot yet be *edited* after the second click.
