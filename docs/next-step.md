# OPEN - the skirmish pair, then the two stand/reach follow-ups, then the written-up reads

**Where this branch stands, measured (`blunt-port`, D314).** Four of the six original reds are fixed, and the "click that does
nothing" defect found in D312 is closed:

- **the two `reseed` rows** (D312): the gather was refused by a reach test measured to the plot's *anchor point*
  (`gap_to_footprint`), and the fixture's villager had **speed 0** - a statue was asked to work a plot;
- **the drop-site invariant** (D313): a carrier banked from up to 0.45 tiles because of a clause written before tangency was
  allowed; it is `radius + one step` and nothing looser;
- **the walled-off wood** (D314): the dry run planned at the node's *point* and skipped the reach test, so a click on a walled
  node was accepted and dropped at apply; `gather_route` is now one authority, called by both.

Suite **925 passed / 2 failed / 21 ignored**. Acceptance: hash `89ff37d844cfd11f`, 129 entities, 10,026 repaths, 9.27-9.42 s
(the state matches D313 bit for bit - the stricter bank is the last thing that moved behaviour).

## Ready now - no evidence needed

1. **The skirmish pair** - the last two reds, both in `crates/sim/tests/skirmish.rs`, and **both failure messages point at one cause**:

   ```
   both_sides_field_an_army                     line 91:  player 1 never fielded infantry: the skirmish is one-sided
   the_skirmish_only_refuses_what_the_fight_has_taken  line 147: the skirmish hit a script error: ["player 1 has no unit militia"]
   ```

   The scene is `config/skirmish.toml` played through `Runner::due`; the first test counts entities whose `unit` is
   `economy.role["militia"]` (so that role exists), and the second reports a **step-resolution** error naming "militia" - i.e. the
   script's own unit lookup, not the engine's. **Start there**: read `resolve`'s unit-name path in `crates/script/src/lib.rs`
   (the `train` arm) and check what it maps a name to, then whether `player 1` can legally field militia at all (a prerequisite or
   a civ gate would answer "has no unit" the same way). Nothing else in the suite depends on this scenario.
2. **A unit fixture for D314's rule**: a gather on a node behind a wall is refused **at the order**, not at the tick. Today the rule
   is pinned only through the rule runner (`gather_pick`), which is a longer path than the rule needs.
3. **`work_position` chooses a stand by compass order** (D312 finding 2): the ring starts north-west and it skips the tile the man
   already stands on, because `plan_path(from, from)` is `NoPath` - measured, a man beside a plot at `(19, 18)` walked to
   `(19, 17)`. `approach_footprint` (the builder's) already picks the **nearest**; the two should agree. Every gather order goes
   through it, so it wants its own witness and its own review.
4. **The delivered totals per column, both trees** (owed by D313): `headless --report` prints them, and they turn "+9% wall time"
   into a throughput figure for the stricter bank rule.

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
- The reseed machinery (D226, D312), the delivery rule (D246, D313) and the gather order's dry run (D314): all landed and pinned.

## The two reds, verbatim

```
both_sides_field_an_army
the_skirmish_only_refuses_what_the_fight_has_taken
```
