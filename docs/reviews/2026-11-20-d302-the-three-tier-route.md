# Review: the three-tier route, and the hierarchy takes a third off acceptance (D302)

*11d itself: the levelled search is wired in behind the straight line and in front of the flat tile search.*

## The shape

`Ground::route_with` now answers in three tiers, which is the binary's shape plus this engine's own shortcut:

1. **the straight line** - `leg_is_clear` on the leg the body actually walks: 477 ns, one waypoint, no search;
2. **the levelled search** (D299, cached by D301) - only when `avoid` is empty and `reach == 0`, because a coarse cell knows where
   **objects** stand and nothing about bodies. Its chain is legal by construction (every cell open, adjacent centres inside their cells),
   and the **first and last legs are certified** with the same `leg_is_clear` the tier above uses; if either fails the tier is abandoned
   and the flat search answers;
3. **the flat tile search** - unchanged, and still the answer for anything the hierarchy cannot express.

## Test-first, with the failure shown

`coarse_route::a_blocked_walk_is_answered_by_the_hierarchy` was written with the tier **disabled** and failed exactly as it should:

```
waypoint (630000, 570000) is not a level-3 cell centre
```

- the flat search's per-tile polyline is not on the cell lattice. With the tier restored: **5 passed / 0.01 s** - the blocked walk is a
  chain of `n * 8 + 4` centres, and the control (the same walk with no wall) is still **one waypoint**, because the line reaches it.

## Performance

- **the acceptance suite: 22.96 s, against 33.74 s recorded at D286 - a 32% cut**, with the same 31 tests passing and the same red set;
- full workspace suite: **918 passed / 6 failed / 21 ignored**, from 917 / 6 / 21, red set **byte-identical** (`diff` of the sorted names
  reports nothing);
- the tier costs one `coarse_route` (4.8 µs cached) plus one `leg_is_clear` per waypoint on walks where the line already failed - where
  the flat search it replaces costs 28.9 µs on the measured walled case;
- the pyramid is built once per ground change (D301), so the 21.1 µs build is amortised across every route until something is placed or
  removed.

## Stability

**Three failure modes were designed against, not discovered later**:

- a coarse chain **through a wall** - impossible by construction, since a cell is blocked when any tile under it is, and every cell in
  the chain is checked open;
- a **first or last leg** across an object with the body in a cell corner - certified, and the whole tier falls back to the flat search
  rather than walking it;
- a **body-blind route** where the caller asked to keep clear of bodies (`avoid` non-empty, or a non-zero `reach`) - the tier does not
  run at all, so the D281 re-plan path is untouched.

**What is not measured**: a scene where a hundred units plan in one tick, and a 480×480 map (the pyramid there is ~56× the area, ~1.2 ms
once per ground change, unmeasured). Both are named rather than assumed.

**One thing the red-set diff cannot see**: the routes are *different* now for blocked walks - coarser, at 8-tile spacing - so any fixture
that asserted a per-tile polyline would have moved, and none did. That is the suite's coverage on this subject being thin, and it is worth
saying rather than letting a byte-identical red set imply more than it does.
