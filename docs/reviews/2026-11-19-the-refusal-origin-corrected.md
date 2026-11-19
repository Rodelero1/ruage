# The refusal-origin split, and the hypothesis it refuted

*Measured on `d279-substitute` (`704d9ba`), the layer D279's re-port replaced. Kept as the **record** of a wrong
explanation and one number, not as a to-do: `short.rs` and its tile-chain fallback no longer exist.*

**The claim that was wrong.** The owner was told the re-plan storm meant *our planner and our mover disagree about the same
corner* - two collision predicates, one radius-aware, one not - and that unifying them was the first fix. **The predicate
is already one** (`clearance_at`, `nearest - radius`) and every call site compares it identically (`< 0` refuses,
`>= 0` allows). There was never a second ground predicate. Withdrawn.

**What the call sites say.** The mover's step test is `clearance_at(point, radius) >= 0 && clear(point, radius)`. The
first half is **ground**, the second is **bodies**. So a refusal labelled `geometry` - "the route came out of the geometry
search" - is overwhelmingly *"the ground is fine and a body moved into the leg"*. The label names the route's origin, not
the cause of the refusal, and reading it as the cause produced the wrong explanation.

**The split**, `DBG_ORIGIN=1`, acceptance, `tick=48000`, `entities=153`, `players=2`, `repaths=156,578` (ground 156,433):

| origin of the refused route | lines | share |
|---|---|---|
| `geometry` | 390,239 | 47.5% |
| `tile-chain` (the fallback the game does not have) | 393,558 | 47.9% |
| `none` | 38,025 | 4.6% |

Half and half - which is why deleting the fallback could never be the whole answer, and why the two halves needed two
different fixes.

## Performance

Not a speed measurement: 821,822 probe lines against 156,433 ground re-paths is **5.3 lines per re-path**, and that is
**unexplained** - either the probe fires per waiting body inside a pass rather than per refusal, or one refusal is counted
several times. The **ratio** is trustworthy; the absolute counts are an upper bound.

## Stability

No production change came out of this. It refuted a planned slice ("unify the two predicates") before that slice was
written.
