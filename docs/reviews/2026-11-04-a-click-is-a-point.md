# Review: a click is a point

*Change (D118, the owner: "I don't want units to walk to the centre of the square. That can walk wherever not blocked"). The
view rounded every right click to a **tile** and sent `Command::Move { to: (tile) }`, so an order could only ever end at the
tile's middle (D114) or, before that, on its corner (the M3a bug). The click point was computed and discarded: `Camera::pick_point`
gives a continuous position and `Camera::pick` rounds it.*

**Now:** a ground click sends `Command::WalkTo { to: (Fixed, Fixed) }`, the point under the cursor. The engine already carried
that form end to end — `start_walk` takes a point, the planner aims at points, and `tile_of(point)` is the goal tile for the
ground test — so this is a view change plus the `Snapshot.scale` field that makes the tile→engine-units conversion honest
(the pack owns the scale; a hard-coded 20,000 in the view would be a second answer).

`Command::Move` (a tile) stays for scripts and the AI, and keeps `tile_centre`: a scenario that names "tile 5,4" has to name
a point somewhere, and the middle is the sane one. **The ground rule is untouched** — "wherever not blocked" is the tile rule
(D117), and a click on a blocked tile still means "come as close as you can".

## Performance

No per-tick cost: the change moves a conversion from the input path (one `pick_point`, one multiply) into the order. The
acceptance match is script-driven and does not click, so its hash and timing are untouched by construction; the view's suite
runs in **4.1 s** (ten tests, one of them the new click test).

## Stability

| attack | result |
|---|---|
| a click inside a tile but far from its middle | the order carries the point: `WalkTo` at (440625, 440208) where the tile's centre is (440000, 440000) — the red test's own numbers |
| the existing ground-click test | re-pointed: it now asserts the order names **the clicked tile** (the invariant it owns) and leaves "it is a point" to the new test |
| a click on a blocked tile | unchanged: "come as close as you can" — the planner's goal-tile exemption and the mover's standoff |
| a script's tile order | unchanged: `Move` still means the middle of the tile |

**One field added to `Snapshot` (`scale`) breakes nothing that reads it** — six test fixtures gained `scale: 20_000` in the
same commit, which is the sort of mechanical edit that is easy to get wrong by hand and was done by pattern with the whole
view suite re-run afterwards.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
