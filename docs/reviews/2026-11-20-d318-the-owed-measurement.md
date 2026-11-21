# Review: D313's economy figure corrected, and D314's rule isolated (D318)

*Two things, both owed: a **guard fixture** that isolates D314's rule (a gather on a walled node is refused at the order), and the
**delivered-totals measurement** D313 owed - which contradicts what D313 and I said.*

## The correction first, because it is my error

D313's row and review said *"the economy is smaller, and that is the price of the rule"*, with the entity count (141 -> 129) as the only
number behind it and an arithmetic estimate standing in for the measurement it owed. The measurement has now been taken - the same
acceptance match, `--report 600`, on three trees (D312 at `338b9f8`, D313 at `ec9d99c`, today's):

```
                    600 s            1200 s           1800 s            2400 s        units / buildings
D312 (338b9f8)   w  22  f  23     w  17  f  133    w  354  f  340    w  813  f  395       36 / 16
D313 (ec9d99c)   w  17  f   8     w  90  f  227    w 1006  f 1363    w 1831  f 1503       35 / 18
today            w  17  f   8     w  90  f  227    w 1006  f 1363    w 1831  f 1503       35 / 18
```

**The stockpile more than doubles** (wood 813 -> 1,831, food 395 -> 1,503 at 2,400 s), and the entity count fell because **fewer
foundations** are standing around, not because the economy starved: units 36 -> 35, buildings 16 -> 18. The guess about the mechanism was
wrong in *direction* - the carrier that banks at the wall is the one that stops lingering 0.45 tiles out with its route spent.
D313's review has a correction section; the decision row's sentence is superseded by D318 rather than quietly edited.

**The lesson, and it is the one this project keeps relearning**: an entity count is not a throughput figure. `--report` prints the
stockpile at each interval and costs one run.

## The guard fixture

`gather_pick::a_gather_on_a_walled_node_is_refused_when_it_is_ordered` - a villager, a tree behind a palisade, and one
`submit_checked`. D314 pins the same rule through the rule runner, which is a longer path than the rule needs; this asks the order
directly and names the node, so the fog rule (D148) cannot keep the tree out of the picture.

**It bites, and that was checked rather than assumed**: with the dry run's old line restored
(`plan_path_for(villager, position, target.position)`), the fixture fails with

```
the order was accepted by a tree behind a wall - the tick would drop it and the player would see nothing happen: Ok(())
```

and with `gather_route` back in place it passes. Both runs quoted, both watched.

## Performance

- The fixture adds **one test** and runs in 0.11 s inside `gather_pick` (6 tests in 3.3 s); nothing per-tick, nothing in `crates/sim/src`.
- Full workspace suite after the fixture: **928 passed / 0 failed / 21 ignored**, from **927 / 0 / 21** - one more pass, nothing else
  moved in either direction.
- The measurement runs above are release builds of three trees; the acceptance match itself is unchanged at **9.2-9.5 s** (D314's
  figure) since only a test and docs changed.

## Stability

- The fixture names its node **directly** rather than through `nodes_of_slot`, so a fog change can never turn it into a silent skip; it
  also asserts nothing about *which* tree the picker would choose, which is the other fixture's subject.
- The correction touches no code: D313's clause removal stands, its witness stands, and only the prose that described its cost was wrong.
- Known and accepted: the acceptance numbers above come from the **same generated `config/gamedata/`** for all three trees (the
  worktrees carry no generated data, D8), which is correct for comparing engine code and worth stating because it is a shortcut.
