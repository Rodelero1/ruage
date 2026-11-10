# Two workers, one spot (D181)

*2026-11-08. The owner, after D180: **"Jumping and flicker is gone. However there is visible overlap of villagers. And occasionally they appear to be
foraging while the circle is not touching the square with bush."** Two complaints. One is fixed here. The other is measured, explained, and **left
open** - because the obvious fix broke six tests in a way I have not explained, and shipping that would be worse than shipping nothing.*

## The overlap: two workers claimed the same standoff

`standoff_point` (D171) fans workers round a node and passes over a side that is **held** - and "held" meant *somebody's position is exactly this
point*. Two villagers walking in from different sides are not at it yet, so both pass the test, both walk there, and both arrive: the measurement was

```
villagers 9502 and 9503 stood 0.00 tiles apart - their 0.4-wide bodies overlap on screen
```

That is the photograph: four villagers on one berry bush with their squares on top of each other.

**The fix is two lines of arithmetic and one list**: "held" now means *closer than the width of two bodies*, and the sides chosen **earlier in the same
decision pass** count as taken - which is the only way the second worker can learn about the first before either has arrived. The list is `work.settles`
itself, so no new state and nothing to keep in step.

Test: `gather_contact::villagers_on_one_bush_do_not_overlap_each_other`, four villagers on one bush, every tick for 400 ticks, minimum gap against
`0.36` of a tile. Red at **0.00**, green after.

## The distance: measured, explained, **not fixed**

```
the villager took food while 0.92 tiles from the bush; the bodies touch at 0.70
```

A worker takes from its node whenever `at_node` is true, and `at_node` also means *standing beside the node's tile* - up to 1.41 tiles from its centre.
So a villager walking in takes food across the air, which is the second photograph.

**Why it is not fixed yet, in one number**: the gather arm asks `reach_raw`, which answers the **unit-to-unit** question and gives the worker's own
radius alone when the node is a map tile with no unit of its own - **0.2** - while `standoff_point` adds the node's content radius and gets **0.7**.
Two answers to one question, in the same arm of the same match. Gating the take on `touching` with the right sum (0.7) makes this test pass and turns
**six others red**, among them `the_villager_walks_to_the_node_and_then_takes_from_it`: the worker reaches its tree, `standing` is true, the tree keeps
20 hit points and never yields - and I have not explained that yet. Guessing at it would have shipped a woodcutter that cannot cut.

So the test is `#[ignore]`d **with its measurement and this paragraph above it**, and the next step is named: one `contact_raw(worker, node)` used by
both the standoff and the gate.

## Acceptance

| | D180 | D181 |
|---|---|---|
| entities | 154 | **160** |
| repaths | 297 | **254** |
| made-room | 76 | 112 |
| hash | `834efe7c5efd3c62` | **`009e5cfe81b182ae`** |

## Performance

**Measured**: the gate ran in **416,450 ms** (`gate106.log`); the new test is **0.8 s** and costs one pass over `work.settles` (at most a handful of
entries) per candidate side. Acceptance wall time is unchanged.

## Stability

- **The fix adds no state**: the taken list is the same `Vec` the settle pass consumes.
- **One test was badly edited and caught by the gate, not by me**: an `#[ignore]` inserted mid-`///` block turned the doc comment into code and the
  file did not compile (`FMT=1 TEST_EXIT=101` in `gate105.log`). Fixed and re-run; recorded rather than hidden.
- Suite: **786 passed / 0 failed / 21 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
