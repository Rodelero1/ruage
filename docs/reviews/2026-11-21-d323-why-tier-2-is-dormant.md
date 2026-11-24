# Review: why tier 2 is dormant - the descent is anchored on the start cell, and the binary says so (D323)

*Change: **none in `crates/sim/src`.** This note records a measurement made while trying to make D322's refined hierarchy pay for itself in the
acceptance match, and the binary read that explains it. Instrumentation was added, used, and removed; the tree is the `87ca435` state.*

## The question

D322 made the refinement match `FUN_007eeda0` and measured the acceptance match **bit-identical**. Instrumented over 48,000 ticks:
11,042 route calls, **1,677** of them ≥ 16 tiles, **0** answered by the hierarchy. Two candidate explanations were on the table - the
`avoid.is_empty()` condition, and the coarse search's level - and only one of them is real.

## What was measured

- **`avoid` is not the cause.** Dropping `avoid.is_empty()` from the tier-2 condition (keeping everything else) leaves the match **byte for
  byte the same**: `c7191f531bd81b1b`, 141 entities, 7,125 repaths, 12.70 / 12.88 s against 12.64 / 12.70 s. The condition was restored: it
  is the conservative reading and it changes nothing measurable either way.
- **The descent is the cause, and it is anchored on the start.** Per long call (`DBGDESC`): **1,603 of 1,677** have `descent(start) = 0`
  while the goal's descent is 1-3. The routes come back at **level 0 with 21-50 cells** - tile-level chains - which `coarse.level >= 2`
  rejects. `reach` is 4000 raw (0.2 tiles) throughout.
- **Why the start's descent is 0**: the starts are gatherers standing beside their resource. A level-3 cell is 8x8 tiles and the rule is
  "blocked if **any** tile under it is blocked" (D303's stamping), so the cell that holds a villager *and his forest* is blocked; likewise
  every coarser cell above it. The descent walks down to the tile and stops.

## What the binary does

`FUN_007ee980`'s head (157 lines, `pathsys` - the levelled search), read against our `Levels::descent`:

```
iVar7 = 3;
while (true) {
    iVar4 = (int)param_2 >> (iVar7 & 0x1f);        // x cell at this level
    iVar6 = (int)param_3 >> (iVar7 & 0x1f);        // y cell
    if ((iVar7 == 0) ||
        ((*(byte*)(*(int*)(*(int*)(*in_ECX + iVar7*4) + iVar4*4) + 1 + iVar6*2) & 0x20) != 0))
        break;                                      // this level's cell is occupied: stop here
    iVar7--;
}
```

It starts at level 3, indexes **the start point's own cell** (`param_2`, `param_3` - the first two parameters after the handle), and stops
descending at the first level whose cell carries bit **`0x20`**; level 0 always stops it. **The goal is not consulted at all.** That is the
same rule as ours: `descent(from).min(descent(to))` is our shape, but since only the start can be the binding one in the binary,
`descent(from)` alone is what the game has. Our `.min()` is harmless (the goal's descent is never smaller in practice - 0 cases out of
1,677 in the match) and is left as is, with this note as the reason it is not a divergence worth churning.

**So tier 2's dormancy is parity, not a defect.** The game's hierarchy is for walks that begin on open ground; a villager who is standing
beside the tree he is about to chop is pitched to tile level in the original too, and the fixture `descent::a_wall_in_the_coarse_cell_drops_
the_start_one_level` and `descent::a_one_tile_corridor_starts_at_level_zero` already pin exactly this.

## Performance

- No production code changed. The acceptance match is `c7191f531bd81b1b`; the suite is the same **930 / 0 / 19** D322 committed.
- The instrumentation itself cost measurable time (two `eprintln!`s per call on 11,042 calls) and is gone; nothing of it is in the tree.

## Stability

- Instrumentation was reverted from backups in two steps and both times `git diff` showed **only** the two D322 files; the tree is clean
  apart from the docs this note adds.

## What this closes, and what it does not

- **Closed**: "the refinement should make the hierarchy answer short walks". It does not, and the reason is the binary's own descent rule -
  D308's gate note ("until the refinement matches") is superseded by a better reason, already recorded in D322.
- **Left open, and it is the bigger question**: our `standing_bodies` **adds** standing bodies to the search as `Clear` obstacles, and the
  binary has **no such input** - its list is an *ignore* list that makes bodies **passable** (`FUN_00801a20`: a tile held only by path-ignore
  units is free ground), and the collision epilogue's answer to a standing body is pause + repath, not avoidance. That is the next thing to
  measure, and it is a behaviour the owner will see, so it is a fork rather than a step.
