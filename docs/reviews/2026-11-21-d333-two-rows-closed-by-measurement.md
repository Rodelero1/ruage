# Review: two rows closed by measurement, one by a read - and the queue is now reads and a project (D333)

*Change: `docs/parity-plan.md` only. P9 marked **DONE** (its two halves were cut before this directive), P6 marked **NOMINAL (measured)**,
P3 re-stated from the function it names and marked **blocked on one read**. No code.*

## P9 was already done, and the register did not know

The row named two rules of the owner's own: arrival "beside a held goal" (D200) and a crowd ring point for a group move (D271). Neither
exists in the tree - `beside` went with D292 and D271 was withdrawn by D289, both at his instruction - and `grep` finds no `crowd_ring_point`
at all. So the "forks forfeited" row was work already done, filed as work to do, which is the third time this register has sent the next step
somewhere already visited.

## P6 is nominal, and that is measured rather than assumed

The row claimed the ground's per-tile `occupants` count differs from the game's per-object radius pairs. Every object this content carries is
a **whole tile wide**:

- all four resource kinds in `config/resources.toml` have `radius = 0.5`, which is exactly a tile's half-extent;
- buildings take the dat's radius, which is `1.0`, `1.5` or `2.0` for the ones that block ground.

A tile-granular index therefore **is** the object's box for this content, and the deviation is nominal. It becomes live the day an object
narrower than its tile exists - which the row now says, so the next reader knows what would reopen it rather than rediscovering it.

## P3 is blocked on one number, and the read narrowed it

`FUN_00840830` read in full (from `0x840830` down the collision arm):

- the overlap test is an **inclusive AABB** - `DAT_00ddeee0 <= max_x && min_x <= DAT_00ddeee4`, and the same in y - so **touching counts**,
  which is the `<=` the register quoted;
- and it is applied to a box that is **shrunk on all four sides** by the float at `+0x268` when the flag at `+0x266` is set
  (`min += d`, `max -= d`, all four coordinates).

That shrunk box is what makes D306 possible: a body whose nominal radius is half a tile still fits a one-tile gap because the query box is
inset. Landing the inclusive test **without** the inset would forbid every one-tile gap - contradicting the owner's own measurement - so P3's
step is "inclusive test plus inset", and the inset's formula is not in the corpus. Its writers are per-object fields on half a dozen unrelated
structs, and the query object's own setter has not been identified.

**So our `object_depth > 0` is the same rule stated from the other side**: refusing penetration into a box is refusing touching of a box inset
by a hair. That is a real equivalence, not a fudge, and the row says it in those words.

## Performance

- No code changed: acceptance `e248486298910e28` (127 entities, 1,727 re-paths, 10.2 s), suite **933 / 0 / 19**, conformance **21 / 0 / 0**,
  all as committed in D332.
- The two checks cost: one `grep` for each cut rule, one `grep -c` over `config/resources.toml`, and one function read (about 70 lines of the
  decompiled `FUN_00840830`). Cheap next to a refactor started on a false premise.

## Stability

- Nothing executable changed.
- **The pattern this closes**: three rows of this register were already done (P1, P2, P5 earlier; P9 now) and two were mis-stated (P3's
  ruling, P6's width). The register is now checked by symbol whenever it is used as a spec, and its status column is the thing to read first.
- **What is left of "remove all deviations" is not code**: P3 needs one number, P12 needs one mask, and P11's third pathfinder is a project.
  Everything else in the register is either done or nominal, and each of those is now said with the measurement that says it.
