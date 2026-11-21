# OPEN - the suite is green; next are the stand choice, the owed fixtures, and the written-up reads

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

**Suite 927 passed / 0 failed / 21 ignored** - fully green. Acceptance: hash `89ff37d844cfd11f`, 129 entities, 10,026 repaths,
9.21-9.54 s.

## Ready now - no evidence needed

1. **A farmer works from where he stands** - two changes, **one witness each**, and the first was built and reverted in D316 because
   its witness still failed with it in:
   1. the stand is the **nearest** free ring tile, the tile he is on included (`work_position`; it walks a compass order from the
      north-west today, and skips his own tile because `plan_path(from, from)` is `NoPath`). Witness: a man beside a plot at
      `(19, 18)` must not walk to `(19, 17)`.
   2. a **building** node is "at" by its **footprint**, not by a one-tile box at its anchor (`at_node` in the gather arm;
      `gap_to_footprint` already computes it). Only with both does the D316 witness pass:
      `a_man_beside_a_plot_stays_where_he_stands_and_works_it` - measured failing at
      `was (780000, 820000), now (785660, 814340)`.
   This is the last piece of the owner's invariant 4 ("a working man does not move"); it holds today only because the settle seats a
   man before he starts taking.
2. **A unit fixture for D314's rule**: a gather on a node behind a wall is refused **at the order**, not at the tick. Today the rule
   is pinned only through the rule runner (`gather_pick`), which is a longer path than the rule needs.
3. **The delivered totals per column, both trees** (owed by D313): `headless --report` prints them, and they turn D313's "+9% wall
   time" into a throughput figure for the stricter bank rule.
4. **An evenly mirrored skirmish** (named by D315): player 2's opening is at 70 s where player 1's is at 320 s. It no longer decides
   a test, but the file calls that section a mirror.

## Ready after one short read each

5. **`FUN_007eeda0`** - refinement of an edge to a **tile on the boundary** (#5): the fix that stops our coarse routes being 8-tile
   spaced, and the one that lets D306's **16-tile gate** go. Recorded in D308 with its port order.
6. **The straight-line tier** (#10): D309 read `FUN_008000d0`, `FUN_008a4360` and `FUN_008a3a20` and found **no pre-search line
   test** - so our tier 1 is unverified as parity. What settles it is the caller of `FUN_008000d0`: the object that owns the path
   request, and whether anything tests the leg before it.
7. **The re-path's ignore list** (#8): four entities, cap 255 at `0x1265`. D310 read it: we build the avoid list from every live body
   within a span of the line, **with no cap** and in **id order**, where the binary fills colliders first and stops at 255. The
   witness that would make the cap a step is a wall of 255 villagers plus a route that must go round the 256th.

## Needs the owner

8. **Bodies priced at 2000** (#6). The game's behaviour is known; the decision is whether a crowd's own routes should stop routing
   round itself. Changes every crowd route.
9. **A terrain cost class** (#2's 100/600 rows): a `config/` and map-format change, not an engine one.

## What is *not* owed

- Any further work on the levelled search: four levels, the descent, the cost table, the chain and the refinement are landed,
  measured and gated.
- The tile search's parity: octile table, `25 x euclid` budget, 20,000 cap, tie-break and best-settled fallback all match.
- The reseed machinery (D226, D312), the delivery rule (D246, D313), the gather order's dry run (D314) and the skirmish scenario
  (D315): all landed and pinned.
