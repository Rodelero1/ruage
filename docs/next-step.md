# OPEN - the tier and the body box are the game's (D338, D339); next is waypoint substitution

**Where this stands (D339, `blunt-port`):** suite **217 result lines, 936 passed, 0 failed, 21 ignored**; conformance
**21 / 0 / 0**; acceptance `36ceb342bd94acc9`, 134 entities, **both players alive at 48,000**, 40,127 re-paths, **15.93 s** median
of three. The *order* to 100% conformance is **`docs/conformance-sequence.md`** - read it first; this file is the immediate step
and the outstanding forks.

**Landed in this session:** sequence steps 1-4. P15 (the cap's witness, D336), P13 (the key is `g + octile`, D337), P14 (D338 -
the tier's avoid gate is gone; **the clause that actually binds is D303's descent floor**, 127 of 185 long walks), P16 (D339 - a
body collides as a box; the match changes and the probe's pocket census does not fall to zero, because those pockets are
*exempt* pairs).

## The next step: 5 - waypoint substitution (P17)

**Prior implementation: none.** `grep -r substit crates/` finds a comment and nothing else. What happens today: a mover whose
waypoint cannot be reached re-plans (`world/motion.rs`, the collision arm around `:571`, `replan_around_bodies`), and the re-plan
keeps returning the same waypoint, so the man re-plans for ever - a stuck unit with a live order. Pinned by `stall`, `stuck`,
`replan`, `immediate_replan`, `right_of_way`; none of them asserts a waypoint being **skipped**.

**The rule, read** (`FUN_008a4360`, `move_obj.cpp:0x1203`, HD 5.7):

- `iVar6 == 1` -> *"We will reach our current waypoint next time, setting cIncrementWaypointNextUpdate"* - the advance is
  deferred by one update.
- `else if (2 < in_ECX[0x52])` -> *"We won't reach our waypoint either this or the next update, but we have a
  mConsecutiveSubstituteCount count of %d, faking it"*, and then the **same** block as the reachable case: `FUN_007ff650(0,1)` /
  `(1,1)`, i.e. the waypoint is advanced anyway.
- So the counter is at `+0x148` (`0x52 * 4`), the threshold is **> 2** - the **third consecutive** failure substitutes - and
  "faking it" is the game's own name for what we are about to implement.

**Still unread:** where the counter is incremented and cleared (`iVar6`'s producer) and what `FUN_007ff650`'s two flags do
besides advancing. One read each; the witness needs the first.

**Witness owed:** a walk whose waypoint is blocked re-plans and then **moves past** it. Today it re-plans for ever; the fixture
must show the third consecutive failed approach advancing the waypoint instead of re-planning again.

## Also ready, independent of that

- **A waiting walker is an obstacle** (step 6): `standing_bodies` skips any body with a route; the binary's test is velocity zero
  (`FUN_007f9d80:45`, `[vtbl+0x2a0] == 0`).
- **The 300 row** (step 7) and **the refinement's scan direction** (step 8): both reads done, each a small slice. Step 8 is also
  what would let the 16-tile floor fall (D338's re-test).
- **The level floor** (D303): the clause that keeps the tier off in a wooded map. Route-wide; needs its own A/B.
- **The mate rule (step 11) is already the game's**, and the sequence doc is stale on it: `stamp_group` (`world/step.rs:110-144`)
  gives mates only to **military** units and a villager is never one, with the owner's rule of 2026-11-16 quoted in the comment.
  The evidence: openage ("villagers ... will not be part of any formation"), the talk ("formations allow for some friendly
  overlap"), the bug threads (villagers blocking each other), and the collision block's own per-body **ignore list**
  (`FUN_00840830:43-54` - count at `+0x24c`, ids at `+0x254`). What is left is a *formation model* for military units - a gap of
  its own, not a villager-crowd problem.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
