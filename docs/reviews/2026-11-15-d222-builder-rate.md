# Review: hands are not linear - n builders add (n + 2)/3 of one man's work

*Change: building speed with several hands now follows the game's own `time = 3t / (n + 2)`. `finish_construction` groups the
tick's builders **by site** and each site gains `1000 × (n + 2)` thousandths of a builder-tick, with the remainder under a
whole tick carried in `Construction::carry`; a single hand is unchanged to the tick. D222.*

## Performance

**Two integer divisions and one grouping of a handful of entries per working site per tick**, in place of one addition per
builder - and the `carry` field is four bytes per foundation. The user-visible effect is the number that matters: a house of
**25 s** with one hand takes **18.75 s (375 ticks)** with two and **15 s (300 ticks)** with three, i.e. the second hand buys
**25%** of the time and the third another **20%**, never 50%. Measured against the thing it replaces rather than guessed: the
suite's own rate fixture reads **60 / 80 / 100** builder-ticks over a 60-tick window for one, two and three hands, where the
linear engine read **60 / 120 / 180**; the single-hand case is bit-identical (D218's Spanish fixture still reads 385
builder-ticks, and the 500-tick house fixture still finishes on 500). No wall-clock claim is made, because a match's build
time is not where this could show: the change moves *when* a building appears, not how much work a tick does.

## Stability

- **The rate is counted once per site, not once per builder.** The old code called `work_one_tick` per builder and each call
  added a tick, which is the linearity. Grouping first means the count cannot be inflated by a man whose `Build` task points
  at the same site twice, and a man who leaves stops being counted on the tick he leaves rather than at the end of a batch.
- **The remainder is state, and it is in the state hash.** `Construction::carry` is `< 3000` by construction, so it cannot
  grow without bound, and it is the only thing that makes two hands land exactly on 375 rather than on 374 or 376. A replay
  that omitted it would drift by a tick per three ticks of two-hand work - which is why it was put in the entity, not in a
  side table.
- **The fixture measures the rate, not the finish time, and it caught a second defect while doing so.** A finish-time test with
  three hands passed a *linear* engine's numbers when the third villager arrived late; measuring a 60-tick window with the
  hands already placed removed that flattery, and the second run of the fixture failed at `66` instead of `60` for one hand
  until every hand was placed touching the footprint - i.e. the measurement was wrong before the engine was.
- **What it does not touch.** `total_ticks` keeps its meaning (single-builder ticks, divided by the construction rate from
  D218), so every stored number in a save or a fixture reads the same as before, and the Spanish and British fixtures needed no
  edit.

Known and accepted:

- **The formula is AoC/DE-documented, not 5.7-verified.** Three sources agree and the HD edition is the AoC engine, but a save
  with one house and two builders is the check that would make it ours rather than theirs; that probe is recorded in the
  decision rather than assumed away.
- **The stalled 6×6 site is untouched.** Its cause is a parked villager and a spot claim, not the rate; the owner's rule for it
  (a free space one diameter wide, claimed like a gatherer's place, next site in the queue when there is none) is the next
  slice and is written up in `docs/next-step.md`.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
