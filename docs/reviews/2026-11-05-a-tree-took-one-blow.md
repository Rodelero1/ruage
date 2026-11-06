# A tree took one blow: the chop rate was multiplied where it should have been divided (D129)

*Found in the second external review, in a finding filed as a **performance** complaint about an `f64` in the tick. The cost
was real and small. The arithmetic underneath it was wrong by a factor of 1,600, and no test noticed, because no test ever
asserted how long a tree stands.*

## The defect

`config/economy.toml` says a villager's chop is **1.5 hit points per second** - "3 hit points every 2 seconds", and the file
does the sum itself: *a 20 hp tree falls in 13.3 seconds (seven chops)*. `crates/data/src/economy.rs` turned that into a
per-tick amount like this:

```rust
let per_tick = self.fell_hp_per_second * f64::from(tick_rate_hz.max(1));
```

**Multiplied.** At the shipped 20 Hz that is **30 hit points a tick**; at 40 Hz, 60. A 20 hit point tree therefore fell to the
**first** blow, in one tick - 0.05 s against an intended 13.3 s, a factor of **267** at 20 Hz and 1,600 against the
`hp/tick` the function's own name promises.

Measured before the fix, on the shipped pack:

```
RATE: 1.5 hp/s at 20 Hz -> 30 hp/tick; a 20 hp tree falls in 1 tick(s) = 0.033 s (intended 13.3 s)
RATE: 1.5 hp/s at 40 Hz -> 60 hp/tick; a 20 hp tree falls in 1 tick(s) = 0.008 s (intended 13.3 s)
```

So the whole **felling** mechanic - trees are obstacles until they are chopped down, the owner's rule of 2026-10-30 - has
never happened in a match. Every tree has been dying on contact, and the only visible consequence was that wood arrived a
little sooner than it should.

## Why nothing caught it

`crates/sim/tests/depletion.rs` asks whether a chopped tree stops blocking its tile, and it asks with `amount`, not `hp`:
instant felling makes that test *pass*. No test stated the rate. The `no_float` guard cannot see it either - the float lives
in `ruage-data`, where floats are allowed, and the guard scans the simulation crate.

The review that found it had the right instinct - "a `f64` multiply/ceil per worker per tick inside the simulation hot loop" -
and stopped one question short: *what does it compute?* It is the same shape of miss as the other review's overflow findings:
the mechanism was spotted, the magnitude was not.

## The fix

1. **The rate is converted once, at load**: `Gather::fell_milli_per_second` is the pack's `1.5` as `1_500` thousandths,
   rounded and range-checked in the loader. The tick never sees a float, which is what D9 asked for in the first place.
2. **The remainder is carried**: `World::fell_carry` holds, per chopping node, thousandths of a hit point scaled by the tick
   rate. Each chopping tick adds `fell_milli_per_second`; one whole point is `1_000 * hz` of it. At 20 Hz a 20 hp tree crosses
   that line on tick **267** - 13.35 s, and exactly the seventh chop of "3 every 2 seconds".
3. **The carry is state** and is hashed: it decides which tick the next point comes off, so two runs that disagree about it
   fall the same tree on different ticks.

Three tests were written first and watched fail (`a_tree_is_still_standing_ten_seconds_in` failed on
*"a 20 hp tree must still stand 10 s into a 1.5 hp/s chop, but its hit points are 0"*):

| test | before | after |
|---|---|---|
| still standing 10 s in, and at least 10 points gone | **FAILED** - 0 hp | passes - 5 hp left |
| down within 15 s | passed (it was down at 1 tick) | passes - down at 267 ticks |
| the rate is per second, not per tick | **FAILED** | passes |

The middle test passing before the fix is the point of writing the pair: "down by 15 s" was true for the wrong reason.

## Performance

The change adds one `BTreeMap` lookup per chopping worker per tick and removes an `f64` multiply and `ceil` per chopping
worker per tick. Forty ticks a second with a handful of woodcutters is not a measurable quantity, and the acceptance match
agrees - **12,383 ms median against 12,391 ms** before, the same machine, three interleaved runs each.

The match itself is different, and cannot not be: wood now arrives at the rate the pack describes, so the base personality
builds less inside thirty minutes.

| | acceptance, 36,000 ticks |
|---|---|
| before | hash `e4bdd521acaaf342`, 152 entities, 237 repaths |
| after | hash **`394c4892aac776d2`**, **141** entities, **184** repaths |

That is a **behavioural** change, stated as one - unlike D128, whose hash did not move, this one is supposed to move.

## Stability

| attack | result |
|---|---|
| `fell_hp_per_second = 0` | felling off, as the file documents: `milli_per_second == 0` never touches the carry |
| a negative or absurd rate | refused at load (`> 1_000.0` is not a rate of hit points per second) |
| carry overflow | `saturating_add`; the carry is reduced modulo `1_000 * hz` every tick, so it cannot grow past one point |
| a node removed mid-chop | the carry entry is dropped when nothing is chopping the node |
| two workers on one tree | one carry per **node**, so their two blows add to one tree's felling rather than racing |
| tick rates | the rate is per second at any `hz`: 20 Hz and 40 Hz fell the same tree at the same wall-clock moment, and the test states it |
| determinism | the carry is in the state hash; two runs of the same seed agree |
| the `no_float` guard | green, and now more true than before: the tick's chop path has no float at all |

**702 passed / 0 failed / 19 ignored**; clippy and fmt clean.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
