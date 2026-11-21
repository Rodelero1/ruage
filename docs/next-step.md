# OPEN - the three remaining reds, then the two defects D312 found, then the written-up reads

**Where this branch stands, measured (`blunt-port`, D313).** Two of the six original reds are fixed and one of the follow-up
defects is closed:

- **the two `reseed` rows** (D312): the gather order was refused by the reach test - measured to the plot's *anchor point*
  rather than to its ground (`gap_to_footprint`) - and the fixture's villager had **speed 0**, so a statue was asked to work a plot;
- **the drop-site invariant** (D313): a carrier banked from up to 0.45 tiles because of a clause written before tangency was
  allowed; it is `radius + one step` and nothing looser now.

Suite **924 passed / 3 failed / 21 ignored**. Acceptance match: hash `89ff37d844cfd11f`, 129 entities, 10,026 repaths,
9.28-9.54 s (D313); the state differs from D312's because the stricter bank is a real behaviour change - the economy is smaller at
48,000 ticks, and that is the price of the owner's rule.

## Ready now - no evidence needed

1. **The three reds**, in cost order:
   1. `a_walled_off_wood_is_tried_once_and_then_left_alone` - the `share_failed` book; movement-adjacent, so it may have shifted
      under D306's radius change and D312's reach change. **Start here: re-diagnose it, do not assume its old cause.**
   2. `both_sides_field_an_army` and `the_skirmish_only_refuses_what_the_fight_has_taken` - the two scenario rows, red since before
      this work.
2. **A click that is accepted and does nothing** (D312 finding 1). `submit_checked`'s dry run plans to the node's **point**
   (`orders.rs:57`) and never runs the reach test, so the apply refuses an order the click already accepted, with the `Err` dropped
   at `step`. The fix is for the dry run to ask what the apply will ask; the witness is a click on a plot behind a wall, refused **at
   the click**.
3. **`work_position` chooses a stand by compass order** (D312 finding 2): the ring starts north-west and it skips the tile the man
   already stands on, because `plan_path(from, from)` is `NoPath` - measured, a man beside a plot at `(19, 18)` walked to `(19, 17)`.
   `approach_footprint` (the builder's) already picks the **nearest**; the two should agree. Every gather order goes through it, so it
   wants its own witness and its own review.
4. **The delivered totals per column, both trees** (D313's owed measurement): `headless --report` prints them, and they turn
   "+9% wall time" into a throughput figure for the stricter bank rule.

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
- The reseed machinery (D226, D312) and the delivery rule (D246, D313): both landed and pinned.

## The three reds, verbatim

```
a_walled_off_wood_is_tried_once_and_then_left_alone
both_sides_field_an_army
the_skirmish_only_refuses_what_the_fight_has_taken
```
