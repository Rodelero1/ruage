# Review: A - an order carries a point, not a tile centre

*Change: the owner's report that movement is continuous in AoE2 and tiles are for terrain, resources and buildings only.
`Camera::pick_point` is `pick` before the rounding; `Command::WalkTo { unit, to: (Fixed, Fixed) }` names a place;
`Route` gains `end`, so the **last leg** aims at the ordered point instead of the centre of the tile it falls in; the
window's right-click now sends a point (`cursor_point`), and `check_order_at` validates it. `Command::Move` keeps meaning
the centre of a tile, so scenarios and the AI's rules stay readable - one internal path (positions), two ways to say
where.*

## Performance

**The control matters more than the number here, and it holds:** the acceptance match at 36,000 ticks produces the
**identical hash** `a596e97fc9a47ec9` and `players=2`, because nothing in that run issues a point order - the change is
behaviour-neutral until a player clicks. Wall time **12.63 s** for the same 36,000 ticks.

**A number of mine is corrected in the same breath:** the previous slice's review reported "+21% from `stand_near`, 15.3 s
against 12.66 s". This run is 12.63 s on a tree that contains `stand_near`, so that 15.3 s was **measurement noise on this
box**, not a regression, and the earlier figure should not have been quoted as one. The box's between-run spread on this
probe is at least ±20%, which is the same lesson `docs/next-step.md` already carries from the 2026-10 census: a wall-clock
number under 20% on this machine is unresolved unless the change runs per tick.

Per tick the addition is one branch in `next_waypoint` (a length check and an `Option` copy on the last step); nothing new
is allocated, and no search is added.

## Stability

- **A unit ordered to a point inside its own tile used to freeze, and the new test found it.** `avoid` answered "the goal
  tile is taken" by looking at the occupancy map, which contains the mover itself - so every step was refused and the
  villager never left its square (position unchanged after 400 ticks). The map lookup now excludes the mover. This is not
  a point-order quirk: any tile order whose goal *is* the unit's own tile had the same hole.
- **Three mutations, three caught.** (M1) delete the exact-`end` branch → the several-tiles test fails with *"closest
  approach 0 raw"* to the last tile's centre, i.e. the unit walks through the centre exactly, which is the kink this slice
  removes. (M2) count the mover as its own obstacle → the test fails with the unit never moving. (M3) make `pick_point`
  round like `pick` → the view test fails, so the sub-tile position is real and not re-derived by the engine.
- **The straightness is what the test asserts, not the destination.** Both a route ending on the point and one ending on
  the last tile's centre *reach* the point (an empty route falls back to the order's own target), so a destination
  assertion passes on both sides - the mutation proved it. The discriminator is the closest approach to the centre, which
  is how the test was rewritten after that mutation survived.
- **Two ways to say where, one path:** `Move`/`AttackMove`/`Patrol` convert their tile to its centre at the command
  boundary and go through the same `start_walk`, so a crowd's `slot_for` still works on tiles while the clicked point
  survives as the goal for the unit that takes the clicked tile.
- **The exact end survives a repath** (a body in the way does not silently coarsen the destination back to a centre).

Known and accepted: **scenario steps and the AI's rules still name tiles**, so a scripted order lands on a centre. That is
deliberate - a script says "go to 18,20" and a player says "go there" - and if the owner wants scripted points too, the
step schema needs a tolerant deserialiser so existing integer arrays keep loading. The window's point is not exercised by
the suite (no display); the arithmetic it uses is `pick_point`, which is tested.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*

## Addendum: B's premise measured, and B dropped (2026-11-03)

The plan's next step was B - string-pull a route so a clear run is one line. A probe measured the trajectory of an
order to a point five tiles east and two north, on the "open row" the fixtures use, against the straight line it was
sent along: **worst deviation 1.218 tiles**. That number looked like the staircase B was meant to fix, and it is not:
the goal crossed forest, so the route bent round it - the pather behaving correctly. In a corridor that is open there is
nothing to straighten, because eight-way A\* steps diagonally (10/14 costs, octile heuristic), so the polyline through
centres *is* the straight line.

## Performance

No production change; a test and a probe. The acceptance match is unchanged from the slice above (hash
`a596e97fc9a47ec9`, `players=2`, 12.63 s at 36,000 ticks), which is the control that dropping B costs nothing.

## Stability

The probe was written as an assertion (`worst < 0.10`) and **failed**, which is the useful outcome: it says B's premise
is false on open ground and that the deviation it would remove only exists where the route bends. It is deleted rather
than kept as a test that documents a false claim, and the finding is recorded in `docs/next-step.md` so the queue does not
carry a step whose justification has been measured away. What remains of B - cutting corners on bent routes - is a
cosmetic change with a real risk (a chord across a diagonal gap), so it waits for a screen, not for a plan.

