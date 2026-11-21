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

## In flight - C: a standing man is neither shoved nor walked through (owner, 2026-11-21)

**Landed (D320): the past chain's missing link.** `replan_around_bodies` now stamps the new route with `self.tick`; in the original a
re-plan is a **new path**, so a man who has stood still since then is *past* and is walked through. Without it a stopped man stayed
non-past for ever and a walker sank into him (0 raw, 600 re-plans in 600 ticks). Witness: `replan::a_stopped_friend_is_only_walked_through_after_the_replan`,
off ignore and re-specified to the chain. Acceptance: re-paths **10,026 -> 7,125** at 48,000 ticks, first half **7% faster**, full match
**40% slower** with **`players=1`** - the fight now resolves, and the gap is entirely in the combat half.

**Next in this slice: re-measure D238 with the chain in place.** `overlap::a_passer_by_does_not_move_the_seated_gatherers` is still
`#[ignore]`d and now fails its own precondition (*"no gatherer was at work beside the node"*), so it measures nothing. Its verdict - that
the seat shuffle is **load-bearing** because a walker walks through a seated man - was measured **before** the pause, the past rule and
D281's "a stopped body blocks" existed, and before D320 completed the chain. Rebuild the fixture so it seats a man who is actually taking
(`progress > 0`), watch it fail, then re-run D238's three refuted variants (ignore walkers; ignore non-ring bodies; release a seat only
for a body at rest) against today's numbers. **Gate**: acceptance entities and re-paths against the D320 baseline (**141 / 7,125**), the
suite red set unchanged, and `invariants::villagers_never_close_into_one_body` still green - that invariant is what the shuffle was
protecting.

## Ready now - no evidence needed

1. **The compass-order stand is a *tidiness* fix, not a rule fix** (D317, which withdraws D316's reading). `work_position` picks the
   first free ring tile from the north-west rather than the nearest, so a man who must walk to a tree or a plot walks a longer first
   leg than he needs to - measured: a man a tile west of a centre-anchored plot is sent to its north-west corner instead of its west
   side. He works the node once he arrives either way, and **a man already at the node is not moved**, which is the owner's invariant
   4 and is kept. If it is worth doing, do it as a **route-length** change with `approach_footprint` (the builder's "nearest" answer)
   as the model, and measure the acceptance match's total walking rather than promising a rule.
2. **A unit fixture for D314's rule**: a gather on a node behind a wall is refused **at the order**, not at the tick. Today the rule
   is pinned only through the rule runner (`gather_pick`), which is a longer path than the rule needs.
3. *(Done in D318 - the owed measurement was taken: the stricter bank rule **doubles** the stockpile at 2,400 s, wood 813 -> 1,831 and
   food 395 -> 1,503; D313's "smaller economy" was my error and is corrected.)*
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
