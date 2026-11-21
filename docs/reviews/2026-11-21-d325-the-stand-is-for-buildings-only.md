# Review: the stand is chosen for buildings only, and the compass order is not observable for a tree (D325)

*Change: one new fixture, `crates/sim/tests/stand.rs`. No production code. `docs/next-step.md`'s item 1 - "the compass-order stand is a
tidiness fix" - is closed by measurement: the chooser is not on the path a gatherer walks.*

## What was asked

D317's note said `work_position` picks the first free ring tile from the north-west, so a man who must walk to a tree walks a longer first
leg than he needs to. The next-step item was "do it as a route-length change, and measure the acceptance match's total walking rather than
promising a rule".

## What the code does

`work_position` has three call sites, and only one of them moves anybody:

- **`gather.rs:184`** - the pick: *is this node workable at all?* It answers a question, the man does not walk to the answer;
- **`gather.rs:543`** - the gather route: `work_position` is used **only for `EntityKind::Building`** (a farm, a site under
  construction). For a resource the route is planned **to the node's own position** and the man stops when the box gap says he is in
  reach (`settle`, D169; `gap_to_footprint`, D312);
- **`lib.rs:3794` / `3973`** - `can_work` / `can_reach` predicates, again questions rather than routes.

So for **trees, mines, bushes and animals the compass order never decides where a man stands**: it decides only whether the *pick* says
yes, and any free ring tile answers yes. For a **footprint-anchored building** - a farm dropped mid-tile - every ring tile of its anchor is
inside the footprint, so `work_position` answers `None` and the route falls back to the anchor, where `gap_to_footprint` (D312) closes the
last third of a tile.

## The fixture

`stand::a_man_who_comes_from_the_east_works_the_tree_from_the_east` - one tree at (10, 9) on open ground, a villager spawned four tiles due
east, ordered to gather, 600 ticks. It asserts his standing tile is **(tree.x + 1, tree.y)**, the side he walked in from.

**Mutation-proved**: making the resource branch route to `work_position` (the compass-first ring tile) instead of at the node reddens it -
the man then walks round to the north-west. Reverted; the production tree is unchanged.

## Performance

- **No production change**: the acceptance match is `c7191f531bd81b1b` (141 entities, 7,125 repaths, 12.6 s) and the full suite is
  **931 / 0 / 19** against 930 / 0 / 19 - one new test, nothing else moved.
- The item promised a route-length change measured on the acceptance match's total walking. There is nothing to measure: the code the item
  named is not on the walking path.

## Stability

- The fixture is self-contained: its map has one resource, its own town centres, and finds the tree rather than assuming where the loader
  puts it (two of my own scenes failed on exactly that assumption before the third worked - a tree that was at x=9 while the map's resource
  list said x=10, and a `[[map.resource]]` block missing altogether so the map had no trees at all).
- The second assertion (`the man is still on the map`) is a guard against a fixture that passes because its actor vanished.

## What this closes

- **next-step item 1**: "the compass-order stand is a tidiness fix". Measured: for a resource it is not even observable, and for a building
  the ring is either free (a corner-anchored footprint, where the compass order costs at most a two-tile detour on one leg) or unusable
  (a mid-tile anchor, which falls back to `gap_to_footprint`). **Not a defect, and it stays as it is.**
- **D317's reading is confirmed from the other side**: the man in that scene did not walk off his plot because there was nothing to walk
  to - the stand he was sent to was the anchor, and he was already in reach of it.
