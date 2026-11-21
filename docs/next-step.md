# OPEN - C is landed; the hierarchy is dormant by parity, and the queue is two fixtures, one read and one mirror

**Where this branch stands, measured (`blunt-port`, D315).** All six original reds are fixed and the "click that does nothing"
defect is closed:

- **the two `reseed` rows** (D312): the gather was refused by a reach test measured to the plot's *anchor point*
  (`gap_to_footprint`), and the fixture's villager had **speed 0** - a statue was asked to work a plot;
- **the drop-site invariant** (D313): a carrier banked from up to 0.45 tiles because of a clause written before tangency was
  allowed; it is `radius + one step` and nothing looser;
- **the walled-off wood** (D314): the dry run planned at the node's *point* and skipped the reach test, so a click on a walled
  node was accepted and dropped at apply; `gather_route` is now one authority, called by both;
- **the skirmish pair** (D315): not an engine fault at all - player 2's attack at 230 s killed player 1's barracks builder and froze
  the site at 119 of 1000, so player 1 never fielded militia. Player 2's attack is at 600 s, measured.

**That block is history** - the numbers below it are the current state: **suite 931 / 0 / 19**, acceptance `c7191f531bd81b1b`, 141
entities, 7,125 repaths, 12.6 s.

## C is landed - a standing man is neither shoved nor walked through (D320, D321)

**D320**: `replan_around_bodies` stamps the new route with `self.tick`. In the original a re-plan is a **new path**, so a man who has stood
still since then is *past* and is walked through - without it a stopped man stayed non-past for ever and a walker sank into him (0 raw, 600
re-plans in 600 ticks). Witness: `replan::a_stopped_friend_is_only_walked_through_after_the_replan`, mutation-proved.
**D321**: with the chain complete, D238's seat-shuffle fixture **passes** once its scene is rebuilt (its gatherers were still walking in, so
it measured nothing) - the place book it described was deleted by D293, and the pile it was protecting against does not appear
(`invariants::villagers_never_close_into_one_body` and `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` are green).

**Acceptance now** (`e90fb4c`, release, seed 7, idle): **141 entities, 7,125 repaths, `players=1`, hash `c7191f531bd81b1b`**, 13.0 s at
48,000 ticks - the first half is **7% faster** than before D320 (re-paths 10,026 -> 7,125 overall), and the second half is where the fight
the script could not previously reach now happens. **Suite 930 / 0 / 19.**

## Landed since C, measured (D322-D326)

- **D322 - the refinement matches the binary**: `Levels::refine` emits the **crossing of each shared edge** (first tile of the cell's span
  the step can pass, point on the boundary) instead of cell centres. The 16-tile gate was **tried and kept**: removing it takes an 8-tile
  walk past one tree from 2 points to 7. Acceptance bit-identical (`c7191f531bd81b1b`).
- **D323 - why tier 2 never answers**: 11,042 route calls in a match, 1,677 long, **0** by the hierarchy. `avoid` is not the cause (removing
  the condition changes nothing); the **descent** is - 1,603 of 1,677 long calls have `descent(start) = 0` because a gatherer stands beside
  his resource. `FUN_007ee980`'s head does the same (level 3 first, the **start** point's cell only, stop at bit `0x20`), so this is
  **parity**.
- **D324 - the body avoidance is not an invention**: `FUN_00801a20` is **fail-closed** - a tile is passable only if every body on it is in
  the ignore list, and that list holds only moving members. A stopped body's tile is blocked. Removing our avoidance is 33% faster and
  reddens two fixtures; reverted.
- **D325 - the compass-order stand is not observable**: `work_position` moves anybody only for `EntityKind::Building`; a resource walk is
  planned at the node and stops in reach. New guard `stand::a_man_who_comes_from_the_east_works_the_tree_from_the_east`, mutation-proved.
- **D326 - the conformance row was stale**: the hierarchy row still asserted cell centres after D322. Fixed; **conformance 21 / 0**. A
  `Levels` change must run `make conformance` and read the totals - it is outside the pre-commit gate.

**Suite 931 / 0 / 19.** Acceptance `c7191f531bd81b1b`, 141 entities, 7,125 repaths, 12.6 s.

## Ready now - no evidence needed

1. **An evenly mirrored skirmish** (named by D315): player 2's barracks order is at 70 s where player 1's is at 320 s, and the file's own
   comment explains why (player 1's opening funds the order at 320 s). Making it a mirror is **not** a no-evidence step - it is a designed
   opening that has to be measured for affordability, and the current asymmetry no longer decides a test.
2. **Seven dangling ids** (`D115, D122b, D124, D125b, D126, D136, D190`) - referenced by history files and defined nowhere. `decisions.py`
   reports them; D283-D285 were the same rot and were backfilled from their review files in D324.
3. **Two stale stashes** from earlier sessions (`WIP on main: a7de0b6`, `On main: d207-wip`) - inspect, then drop or apply.

## Ready after one short read each

4. **The straight-line tier** (#10): D309 read `FUN_008000d0`, `FUN_008a4360` and `FUN_008a3a20` and found **no pre-search line test** - so
   our tier 1 is unverified as parity. What settles it is the caller of `FUN_008000d0`: the object that owns the path request, and whether
   anything tests the leg before it. **D322-D323 did not touch this.**
5. **The re-path's ignore list** (#8): D310 read it - we build the avoid list from every live body within a span of the line, **with no cap**
   and in **id order**, where the binary fills colliders first and stops at 255. The witness that would make the cap a step is a wall of 255
   villagers plus a route that must go round the 256th; D310's reason for not adding it (our selection would be ours, not the game's) still
   stands.

## Parked behind D323 - a decision that cannot be measured until the hierarchy answers in a match

6. **Bodies priced at 2000** (#6): the cost table has the row (D298), and it is inert while tier 2 never answers.
7. **A terrain cost class** (#2's 100/600 rows): a `config/` and map-format change, not an engine one - also inert for the same reason.

## What is *not* owed

- Any further work on the levelled search: four levels, the descent, the cost table, the chain and the refinement are landed,
  measured and gated.
- The tile search's parity: octile table, `25 x euclid` budget, 20,000 cap, tie-break and best-settled fallback all match.
- The reseed machinery (D226, D312), the delivery rule (D246, D313), the gather order's dry run (D314) and the skirmish scenario
  (D315): all landed and pinned.
