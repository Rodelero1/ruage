# Review: the cost table read straight from `FUN_007ed280` - two of my notes were wrong (D327)

*Change: **none in behaviour**. `FUN_007ed280` read in full (116 lines) against the two rows D298 called a named gap and the one it called
"a body on the cell". Both readings were wrong, the notes are corrected in place, and `step_cost`'s doc now says what it does not know.*

## What the function actually does

`FUN_007ed280(x, y, node, direction, flag)` is the coarse search's expansion cost. In its own order:

1. out of the search window → refuse; the tile's `0x10` bit already set → already expanded, refuse;
2. **the distance-from-the-edge term** - `(coord - window_lo) + 10` in the direction of travel, or `-100` on the far edge (which the
   search reads as "this is the edge, keep it cheap"). Positive evidence for D298's note that the term belongs to the search, not the table;
3. **10** - open and clear: `FUN_007f11f0(x, y)` and all four orthogonal neighbours return 0, plus the edge term above;
4. **300** - one of those five tiles holds **an object whose type (`+0x16` of the object's `+0xc`) is 8 or 0x20**. Not "a blocked
   neighbour": the test is a scan of each tile's object array for those two type ids;
5. **100 / 300** - the tile's own byte is non-zero and `FUN_00773c80` (bit **3** of the terrain object's `+0x28`) decides which;
6. **600** - `FUN_00773bf0` (bit **6** of the same byte) is set;
7. then, unconditionally over any of the above: if the per-tile byte grid at `[0x4d]` is non-zero, the cost becomes **2000**;
8. `+10` when `param_5` is set (an extra charge the caller decides).

`FUN_0076f430(x, y)` sets the tile's object, which is what makes steps 5 and 6 a property of **the terrain**, not of the mover.

## The two corrections

- **The 100 and 600 rows are terrain flags, not a passability class.** D298 called them "crossable but slow" and "not crossable but the
  search still opens it", needing a cost class our content cannot express. They are actually bit **3** and bit **6** of the terrain object's
  flag byte at `+0x28`: bit 3 promotes a 100 to a 300, bit 6 gives 600. Naming the two bits is a **terrain-table lookup** (the dat's terrain
  entries, or the terrain editor's labels), not a rule to invent - so the open question is smaller and better posed than it was.
- **The 2000 row is not "a body standing on the cell".** It reads a per-tile byte grid `[0x4d]`, which `FUN_007ef3f0` builds and refines as
  a set of per-direction marks over `[0x4c]`/`[0x4d]` (`0x01 << n` per side, compared against neighbours and against the tile's blocked
  byte). Nothing in the two functions ties that grid to standing bodies, and `FUN_00801a20` - the function that *does* look at bodies - is
  in the flat search, not here. So D298's "a body on it costs 2000" is **unsupported** and is corrected in place.
- **And the row is dead code in our engine anyway**: `coarse_route` calls `step_cost(level, nx, ny, false)` - always `false` - so our 2000
  branch has never run. The doc now says so, where it used to imply the row was live.

## Performance

- No behavioural change, so the numbers are the recorded ones: acceptance `c7191f531bd81b1b` (141 entities, 7,125 repaths, 12.5 s), suite
  **931 / 0 / 19**, conformance **21 / 0 / 0**. The change is a doc comment plus two decision-log corrections.
- The read itself cost one pass over a 116-line function and one over `FUN_007ef3f0` (157 lines) - cheap enough that leaving the two rows
  described wrongly for a third day would have been the expensive choice.

## Stability

- The `step_cost` signature keeps `body: bool`. With no caller passing `true` it is a parameter that cannot be wrong, and when the 2000
  trigger is settled (the `[0x4d]` grid read to its writer) it is the one place a change lands.
- **The process point**: D298 recorded a *named gap* - the right shape - but named the wrong mechanism in both directions: it invented a
  passability class that does not exist and attributed the 2000 row to bodies. A gap written from a partial read is still a guess, and this
  one was quoted in `docs/next-step.md` as a decision the owner could take.
