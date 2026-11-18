# OPEN - the game's pathing, ported in the order D275 sets

**Where things stand (2026-11-18).** The owner ruled: *"Port everything from the game. I want the game to be ported as
accurately as sensible. We can think about optimizations later."* That is **D275**, and it withdraws the freedom that let
D242 and R1a keep a divergence because the match cost more. `docs/behaviour-diff.md` rows 13, 20-23, 25-30, 32, 33 and 36
are the port list; the conformance set is the score - **28 green / 5 red**, target 33/0.

**The queue, in order. Each slice is test-first, measured, reviewed and pushed; none may be refused on cost (D275).**

1. ~~**P0 - read the termination guard.**~~ **DONE 2026-11-18**, and it is not a misreading: the search's budget *is* the
   heuristic. `local_46c` is an expansion counter, and the loop breaks at `expansions > h` (i.e. `> 25 x Euclidean` tiles) or
   `> 20000`, jumping to the **same epilogue** an empty frontier uses - so a budget-exhausted search returns the best it has
   instead of failing, which is the talk's *"otherwise give the closest point path"* in code. Disassembly and addresses:
   `docs/movement-in-the-binary.md`, "The termination guard, settled". **Still unread:** what that epilogue writes when the
   budget runs out (best frontier node, last expanded, or its parent) and the equal-`f` tie-break inside `FUN_00801e90`.
   **P1 carries that read**, because a route's shape depends on the tie-break and the acceptance hash depends on the route.
2. **P1 - the cost model (row 13).** Already read, not yet ported: **16** per cardinal step, the octile table
   (`table[min] + 16*(max-min)`, `FUN_007ffc80`, `DAT_00afd2a0`), **`h = 25 x Euclidean`** - a **weighted A\*, weight ~1.56,
   returning longer routes on purpose** - and the frontier as a **heap** (`FUN_00801e90`) beside the per-tile best-cost array
   with its generation byte (`0x38414`, `0x2db424`, wraps at 240). Prior implementation: `crates/sim/src/path.rs:16-17`
   (`CARDINAL = 10`, `DIAGONAL = 14`, summed at `:112` and `:284`) with an **admissible** estimate. Today: 50.5 -> 8.9 us
   corner-to-corner (D143), 188 searches a match, ~0.04 s of an 11.5 s run - so the reason to port this is *shape*, not speed.
   Fixtures: equal-cost routes tie identically twice; a detour keeps the cost the table says; the acceptance hash moves and
   the diff is explained. Gate: `entities >= 155` / `repaths` near 380 (D242) - **which is itself waiting on the owner**, below.
3. **P2 - plan for the radius (row 21).** The search answers for *"the boundaries of where this unit's radii will go"*;
   ours plans points (`path.rs` has no radius). `a_big_unit_walks_through_a_one_tile_gap` **inverts** under this rule and is
   rewritten to it, with the reason in its header.
4. **P3 - two levels, a parent, and the closest point (rows 26/27/32).** `AddFuturePath` builds a high-level path over a
   low-level one; *"We don't have a parent path, so we're stopping"*; `mConsecutiveSubstituteCount` substitutes an
   unreachable waypoint instead of waiting (row 26); a search that cannot reach answers the closest point, never `None`.
5. **P4 - the walk.** No slide (`D255`/`D256`'s 3-red state is the resume point: point ground + a building's box), the
   waypoint **snapped to and spent**, and the body kept **on the route** leg by leg instead of aiming straight at each
   waypoint every tick (D259/D260). This is where *"no sliding"* finally lands, and it needs P2 first.
6. **P5 - the collision block's vocabulary (rows 20, 22, 23, 25, 29, 30, 33, 36).** `TakeAtRest` goes; the avoid set leaves
   the search and the classifier keeps it; the pause is re-drawn on a fresh collision; `MAX_GROUND_REPLANS` /
   `MAX_REPATHS_PER_TICK` go; the mate set becomes a formation's (re-tested on `isMoving` each tick); the start is re-seeded
   at `2 x radius` outward only; the order's path vocabulary (forced waypoint, teleport-to-waypoint) arrives with it.
7. **P6 - the place model.** A gather place is a point the body **fits** on and an approach is a route it can **walk**
   (D257): `standoff_point` picks by tile and spacing today, and rejecting the illegal places left `Standoff::NoRoom`.

**Not this queue, but measured and kept**: the crowd ring (D271), the click frame (D273/D274), `place_apart` (D267),
`last_moved` (D213) and the stander discs (D211) are the game's own rules and stay as they are.

**Waiting on the owner (a question that must not expire, AGENTS.md):** the queue's acceptance floor is `entities >= 155`;
the control run reads **154 and never resolves**, while the D250 build reads **137 and wins at 2,381 s**. Either the floor
describes a *contested* match and needs its own fixture, or the win is the better outcome and the floor is restated. Every
slice above measures against that floor, so it is the one number that has to be settled first.

**The narrative this replaced** - D254 through D274, the slide, the five failed designs and the traced refusal - is in
`docs/history/next-step-2026-11-18.md` and in the decision rows themselves.
