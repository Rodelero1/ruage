# Review: the two reseed reds were a statue, and the reach was measured to the wrong thing (D312)

*Change: `start_gather`'s "is this node reachable?" test (D279) measured the route's last point against the node's
**position**; it now measures the gap to the node's **footprint** (`World::gap_to_footprint`, `crates/sim/src/world/sites.rs`).
Plus a fixture repair: `crates/sim/tests/reseed.rs`'s `spawn` helper gave a villager `speed: 0`, so the two reseed rows were
measuring a statue.*

## What was actually wrong

D311 split the fault in two candidates - the reseed arm never reached, or the placement refused - and **both were wrong**.
The instrumented run (`DBG` prints on `start_gather` entry, its plan, and its result at apply time) said:

```
DBG plan stage: from=(19, 18) to=(20, 18) kind=Some(Building)
DBG planned: Ok((1, Some((Fixed(380000), Fixed(340000)))))
DBG apply gather EntityId(130)->EntityId(131): Err(NoPath { from: (19, 18), to: (20, 18) })
```

1. **The gather was refused**, so the plot never ran out and nothing ever asked for a reseed.
2. The refusal was the **reach test**, not the placement. A route's points are tile corners, a node's position is wherever
   `Command::Spawn`, a scenario or a save put it, and a farm is 3x3: for a plot anchored at the middle of a tile the first free
   ring tile reads **1.5 tiles away on each axis** - 4.5 tile-squared against a 2-tile reach - so a villager could not be sent
   to a plot standing in the open three tiles away.

The **number** is unchanged. The *measure* was wrong, which is the FORM-of-a-rule case: the 2-tile reach stays and the thing
it is measured to is fixed.

Then, with the order accepted, the second fault showed up in the gather arm's own log:

```
DBG arm t=3 w=EntityId(130) pos=(390000,370000) target=(410000,370000) node_r=10000 step=0 gap_sq=36000000 at_node=false path=Some(1)
```

`step = 0`: the fixture's villager **cannot walk**. He stands 0.3 tiles short of the plot's box for 1,200 ticks, so he never
takes, the plot never empties, and no reseed is ever asked for. The reseed machinery - queue, toggle, husk, replacement on the
same ground - was complete the whole time. Both fixtures go green with **every assertion untouched** once the helper resolves
the man's speed from content, which is what `farm.rs` already did.

## Test-first

The witness is new and was watched failing (`crates/sim/tests/farm.rs`):

```
a_man_sent_to_a_plot_anchored_at_a_tiles_middle_works_it
  before: the man walks over and takes food out of it: 1000000 -> 1000000
  after:  ok
```

It names the rule rather than the scene: a plot spawned at a tile's **middle** - which is exactly what `Command::Spawn` is
handed - with a man three tiles away on open ground, the order taken and food out of it.

## Performance

- **Acceptance match, release, seed 7, 48,000 ticks**: **8.54 s** on this tree; the parent (`373902f`) ran **8.53 s** and
  **8.81 s** over two runs. Sub-10% and inside run-to-run noise: **unresolved**, not a win.
- The final state hash is **identical on both trees** - `3f6c802027ff40fb`, 141 entities, 6031 repaths - so the looser reach
  **fires zero times** in the acceptance scenario. That is the bit-identical proof that the change is inert there, and it is
  also the honest limit of the measurement: the branch it changes is never taken in that match.
- What it costs where it does fire: one box gap per gather **order** (four integer multiplications, two comparisons), on a
  path that already plans a route. It cannot move a per-tick figure, because nothing per-tick reads it.
- Full workspace suite: **923 passed / 4 failed / 21 ignored** against **920 / 6 / 21** before; the red set went from six to
  four and the `diff` is exactly the two `reseed` rows leaving. **No red joined.** Three of the increase are the two
  repaired rows plus the new witness.
- clippy clean at `-D warnings` (workspace, all targets); `cargo fmt --check` clean.

## Stability

**What was attacked, and what pins each.**

- *The measure.* Five shapes gone through: a centre-anchored plot (refused before, accepted now), a corner-anchored one
  (`farm.rs`'s existing five rows, unchanged and green), a resource node (no `unit`, so no footprint - it answers as its own
  point, which is the one-tile node it is), an animal node (footprint 1x1, gap 0 when the man is on it), and a node behind a
  wall (still refused: the search's best settled point is far from the **box** as well as from the point). The new witness
  pins the first; `a_second_farmer_goes_to_a_free_farm`, `a_free_farm_far_away_does_not_take_the_second_farmer` and
  `a_villager_survives_his_farm_vanishing` pin the rest.
- *The statue.* Both `reseed` rows now run a man at content's 0.8 tiles a second and reach the plot, empty it, husk it and
  raise the replacement (`t=169` and `t=193` in the probe, the plot at amount 0 and a new farm at 175 food, construction 0).
  `without_a_queue_or_the_toggle_nothing_is_reseeded` still passes, which is what keeps the pair from being satisfied by
  "reseed always happens".

**Defects found, including my own.**

1. **D311's conclusion was wrong.** Its evidence (the queue paid, the toggle remembered, the neighbours green) was right; its
   inference - that the fault was in the reseed arm or in the placement - was not, and it pointed the next session at the wrong
   file for a whole step. Withdrawn here rather than left standing.
2. **The fixture gave a villager speed 0** (`reseed.rs`'s helper, mine). A man who cannot walk is not a scene the game can
   produce, and nothing in the suite noticed because every assertion before the placement only read the queue, the toggle and
   the stockpile.
3. **`submit_checked` accepts what the apply refuses** - found and **not fixed**. Its dry run plans to the node's *point*
   (`crates/sim/src/world/orders.rs:57`) and never runs the reach test, so both of these orders were answered `Ok` and then
   dropped at `step`, the `Err` discarded. A player's click that is accepted and does nothing is a defect of its own; the shape
   of the fix is for the dry run to ask the same question the apply will, and it needs its own witness (a click on a plot
   behind a wall refused *at the click*).
4. **`work_position` picks a stand by compass order, not by proximity** - found and **not fixed**. Its ring starts north-west,
   and it skips the tile the man is already standing on because `plan_path(from, from)` is `NoPath`; so a villager beside a plot
   is walked to the corner before he starts, where the game leaves him where he stands. Measured in the probe: a man at
   `(19, 18)` sent to `(19, 17)`. A separate change with its own witness; `approach_footprint` (the builder's version) already
   does it right, so the two should agree.

Known and accepted: the reach is still a fixed **2-tile** number rather than the game's box gap against a range of zero - a
labelled deviation, and the reason it is 2 and not 1 is that our routes end at tile centres while the game's end on the
tolerance. The ring-order wart above is priced as its own step, not folded in here.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged with
the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
