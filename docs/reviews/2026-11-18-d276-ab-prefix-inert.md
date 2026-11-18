# Review: A/B - the fallback prefix is inert, so the slide is what costs the match

*Change: on branch `d276-prefix-only` (off `main`), one edit: a re-plan's tile-chain fallback prefixes the body's own tile
centre. Built to separate two changes that `d276-no-slide` bundled. **Not for `main`** - it is an experiment, and its answer is
that it changes nothing there.*

## Performance

`make headless`, seed 7, 48,000 ticks, release, both binaries built from source:

| build | entities | players | hash | repaths (body / ground) |
|---|---|---|---|---|
| `main` | 131 | 1 (won) | `b5292af5a6072d57` | 551 (416 / 135) |
| prefix only | **131** | **1 (won)** | **`b5292af5a6072d57`** | **551 (416 / 135)** |

**0.0 s of difference, every counter identical** - 551 re-paths (416 body, 135 ground), 131 entities, `players=1`, the same
hash. **Bit-identical, to the hash and to every counter.** The acceptance scenario never re-plans through the tile fallback, so the
edit is outside its world - which is exactly what an A/B needs to establish before charging the next change with a cost.

## Stability

**What the experiment settles.** `d276-no-slide` (slide deleted **and** prefix) reads 153 entities, `players=2`, match
unresolved, *"not enough food: 500 needed, 195 in the bank"* from 2,387 s. With the prefix shown inert, **the slide deletion is
what costs the match** - and the shape says why: with no slide, every ground graze becomes a re-plan, the route's cap of two
(`MAX_GROUND_REPLANS`) is spent on the first two, and the man then stands in `WaitGround` for good. A frozen economy that cannot
reach Feudal is not a price, it is a rule disagreeing with itself.

**The correction it forces, and it reverses D276's order:** the slide cannot go before the mechanism that ends a blocked route
- the original's **substituted waypoint** (`mConsecutiveSubstituteCount`, register row 26) and the **uncapped** re-plan. Slice
order becomes: substitution + no cap, then the slide.

**Found and reported:** the earlier `d276-no-slide` measurement charged two changes to one number. That is the flaw this branch
exists to remove, and the honest reading of the earlier table is "unknown split", not "the slide costs 22 entities".
