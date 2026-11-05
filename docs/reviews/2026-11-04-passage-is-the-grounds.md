# Review: passage is a property of the ground, not of the body

*Decision (D117), from the owner's rule, given while I was three attempts deep into making the ground obey a body's radius:
**passage is a property of tiles.** Fully occupied tiles that touch — even at a corner — leave no passage at all, for
anyone; tiles that do not touch leave passage for everyone, whatever the size of the unit's circle and however many units
are pushing. A formation that cannot fit squashes and goes through.*

**What that means for this engine, verified rather than assumed:** it already does this. Ground passability is
`Walk::blocked` per tile — a body's circle is never consulted — and the diagonal rule ("two trees touching at a corner leave
no gap") has been pinned by `two_trees_touching_at_a_corner_leave_no_gap` since the passage slice. The body's radius governs
`collide` between **bodies**, which is where AoE2 spends it too.

So the slice is a *test* and a *decision*, not a code change: `a_big_unit_walks_through_a_one_tile_gap` places a
`trebuchet_packed` — radius **0.5**, i.e. a half-tile circle, the biggest land body in the pack — and walks it through the
one-tile gap between two houses. It passes at HEAD, unchanged, which is the point: the rule was already true.

**And the rule kills the slice I was about to land.** M3c would have judged the ground by a body's disc. In a one-tile gap a
0.5-radius disc *touches both walls*, so `ground_overlap` is zero at the centre line and any deviation makes it worse than
zero — the body could only ever move exactly along the centre line, and the first drift would jam it. That is the arithmetic
that produced M3c's eight stuck workers, and it is the arithmetic the owner's rule says not to do: **a body that cannot fit
is squeezed, not refused.** Three sessions went into that model; this one ends it.

## Performance

No production code changed. The full suite runs **663 passed / 0 failed / 19 ignored**, one test more than at `d2dce1c`,
and the acceptance match is untouched by construction — the same single `Move` fixture as the neighbouring doorway test,
2,400 ticks, about 20 ms of test time.

The decision removes work rather than adding it: the disc-vs-ground check (`ground_overlap`, called once per step per
moving body) is not landed, so the per-tick movement cost stays where it is today.

## Stability

| attack | result |
|---|---|
| a half-tile circle through a one-tile gap | passes at HEAD, unchanged — the gap is a gap |
| the same body, gap closed to a diagonal corner touch | blocked by the tile rule and by `two_trees_touching_at_a_corner_leave_no_gap`; the rule does not consult size, so the *big* body is covered by the same test as the villager |
| judge the ground by the disc (M3c) | refused at the gap's mouth: the disc touches both walls, so every step that is not exactly along the centre line increases the overlap. Not landed, and now explicitly out of scope |

**What I got wrong, so it is on the record:** I spent three sessions building a model the owner's rule says is not AoE2's,
and I never asked the question that would have settled it in one line — *does a unit's circle decide whether it can pass?*
The measurements that "blocked" M3c (538 raw units inside a wall, eight stuck workers, an 8.7-tile shortfall) were all real,
but they were symptoms of a rule the engine does not have. The right move was to ask before the third attempt, not after.

## Known and accepted

- **A drawn body may overlap a wall.** With tile passability and a circle for bodies, a unit standing in the tile beside a
  wall has its circle crossing into the wall's tile. That is AoE2's picture too, and the owner asked for passage, not for
  visual separation.
- **The planner still expands by the body's radius** (`plan_path_for`). It does not *refuse* a one-tile gap, because the
  expansion is exact and touching is allowed — but it keeps routes a body's width from walls, which makes a walk look tidy and
  costs nothing measurable. If a future unit's radius ever exceeds half a tile, this becomes wrong and must become
  `radius.min(scale/2)` or a plain follower margin; recorded here rather than discovered later.
- **M3c is cancelled**, not deferred: the prototype stays at `/opt/data/cache/m3-findings/` for the record, and
  `docs/next-step.md` says why so the next session does not re-open it.

---

*`tools/review_check.py` looks for `## Performance` with a number and `## Stability` with content; both are above.*
