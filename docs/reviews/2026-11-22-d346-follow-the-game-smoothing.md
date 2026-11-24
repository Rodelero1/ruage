# 2026-11-22 - D346: follow the game - the chain is smoothed, and the tier answers

The change: the tier's floor becomes the binary's (nothing refused above level 0), the polyline goes through the original's
own smoothing pass (`FUN_008acf20`, `move_obj.cpp:0xf56` -> `0xf69`), the per-leg certification loop is replaced by that pass,
and bodies leave the tier's geometry (they were never in the binary's). `Ground::smooth` is the port;
`tier_smoothing::the_levelled_route_is_smoothed_before_it_is_handed_over` is its witness; the two D345 witnesses are un-parked
from `docs/parked/` and pass unchanged.

## Performance

**Acceptance match, seed 7, 48,000 ticks, release, median of three, back to back on an idle box:**

| tree | wall | band | hash | entities | re-paths | player 2 out |
|---|---|---|---|---|---|---|
| before (floor 2, no pass) | 12.18 s | 11.94 / 12.18 / 12.21 | `1e1fcc3cf2b05a4c` | 133 | 36,567 | ~2,350 s |
| after (floor 1 + smoothing) | **11.70 s** | 11.62 / 11.70 / 11.77 | `28aa6caa2747f9db` | 130 | **17,743** | ~1,977 s |

**4% faster and re-paths nearly halved.** The one number that moves the wrong way is entities: 130 against 133, and player 2
leaves ~370 s earlier, so the match itself changed - the routes are the game's now, and the game's routes lose three units'
worth of fights. Both are reported rather than picked.

**The tier's own census** (`tier_probe`, release, 48,000 ticks):

- answers **16 of 31,256** routes (31,256 because the match diverges; the 50,022 figure was the older tree);
- **194 raw waypoints -> 57 smoothed** - the pass removes 71% of the crossings, which is what its log line claims;
- on the **same 16 walks**, tier **57 waypoints / 5,460,841 raw** against the flat search's **76 / 5,251,895** - the tier now
  hands over **fewer** waypoints than the flat search, at **+4.0%** ground (before the pass it was 172 against 67, +9.8%).

So the pass closed the waypoint gap entirely and half the length gap. What is left is the coarse grid's dogleg against an
octile route at tile resolution - the next slice, not this one.

**Cost of the pass:** one `chain_leg` call per candidate/anchor pair on routes of 10-20 points, i.e. tens per long walk, and
only for walks the tier answers (16 in 48,000 ticks). It cannot show up in the wall above, which improved for a different
reason (fewer re-paths).

## Stability

Attacked, and what each attack found:

- **Index arithmetic of the port**: the binary's delete-then-retry means the candidate index stays put while the array shifts.
  Getting that wrong either loses a corner (a route through a wall) or loops. The witness is the raw chain printed above
  (10 points, including a 4-tile overshoot past the goal) collapsing to corners, plus `tier_ignores_bodies` and the two
  un-parked witnesses, all green.
- **A route the pass cannot improve**: if every segment is blocked the pass keeps every waypoint - the loop's `else` branch -
  so a walled zig-zag is returned unchanged, not truncated. `descent`, `coarse_route` and `route_shape` unchanged.
- **A one-point chain**: `smooth` returns early below three points, so a two-point answer cannot lose a point.
- **Bodies**: removed from the tier's geometry. That is the binary's rule (no stander inserter in `FUN_007ee980`,
  `FUN_00840c60` called with flags `0x40`, and the ignore-list branch off), and the step's past/bump/re-plan (D320) is what
  answers a body; `moving_bodies`, `body_boxes`, `evasion` and `gather_ring` all pass, and the acceptance re-path count
  *fell* rather than rose.
- **The floor**: `descent == 0` still falls through to the flat search, so the open-field walks are untouched; the change is
  bounded to walks whose 2-tile cell is passable.
- **Determinism**: the pass removes points by index, no map iteration, no float; `no_float`, `state`, `reseed` pass and the
  acceptance hash is stable across three runs.

**My errors, in the same breath:**

1. D345's "the tier's routes lose" was measured against a polyline **without the game's smoothing pass** - the cost I
   attributed to the mechanism was the cost of a step I had not ported. The row says so and this one supersedes it.
2. The first smoothing fixture asserted "no three collinear points", which is not exactly true of integer interpolation and
   would have been a brittle test; replaced with a bound on the count, with the un-smoothed count quoted in the message.
3. `docs/parked/` was created in D345 and is now empty - the witnesses are back in `crates/sim/tests/`, and the park/un-park
   cycle cost two commits. Parking a rule the binary plainly has was the wrong call; "follow the game" was already the
   owner's standing instruction.

**Not attacked**: the level-0 search (the one gate left, named in `docs/next-step.md`); whether `FUN_00840c60` with flags
`0x40` includes *units* in the segment test (its ignore-list branch is off, so if it does, the pass would keep slightly more
waypoints near bodies - second order, and unread).
