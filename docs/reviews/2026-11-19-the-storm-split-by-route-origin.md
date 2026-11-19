# The re-plan storm, split by where the refused route came from - and the hypothesis it refutes

*Measured on `d279-substitute` (`704d9ba`: the substitution slice, one ground cap removed, slide gone), acceptance
scenario, `DBG_ORIGIN=1` instrumenting every refusal with the **origin** of the route it was handed. The number it
explains is the one the register calls unexplained: **149,254 ground re-paths** (`docs/pathfinding-gap-register.md` §5,
step 1's probe).*

## The claim this withdraws

The re-plan storm was explained to the owner as **our planner and our mover disagreeing about the same corner** - two
collision predicates, one radius-aware and one not - with "unify the predicate" as the first fix. **The predicate is
already one**: every ground test goes through a single function (`clearance_at`, i.e. `nearest - radius`), and every call
site compares it the same way (`< 0` refuses, `>= 0` allows). There was never a second ground predicate to unify, and the
slice that would have unified them is dropped.

## What the call sites actually say

The mover's step test is `clearance_at(point, radius) >= 0 && clear(point, radius)`. The first half is **ground**; the
second half is **bodies**. So a refusal labelled `geometry` - "the route it was handed came out of the geometry search" -
is overwhelmingly *"the ground is fine and a body moved into the leg"*. The label names the route's **origin**, not the
**cause** of the refusal, and reading it as the cause is what produced the wrong explanation.

## The split (acceptance, tick 48,000: 153 entities, `players=2`, 156,578 re-paths, 156,433 of them ground)

| origin of the refused route | lines | share |
|---|---|---|
| `geometry` | 390,239 | 47.5% |
| `tile-chain` - the fallback the game does not have | 393,558 | 47.9% |
| `none` | 38,025 | 4.6% |

**Half and half**, and the two halves need two different fixes:

- the **tile-chain** half is the fallback itself - rows 1 and 26 of the register, and the game's rule is *"expand the
  search area once, otherwise give the closest point path"*, never a chain of tile corners;
- the **geometry** half is bodies - and the game's answer is the collision block's epilogue: **re-plan round the
  collider's tiles**, pause the higher id 0.25-1.50 s, and write no position.

So deleting the fallback answers only ~48%, and the other ~48% is not a planner defect at all.

## Performance

Not a speed measurement. **821,822 probe lines against 156,433 ground re-paths is 5.3 lines per re-path**, and that
inflation is **unexplained** - either the probe fires per waiting body inside a pass rather than per refusal, or one
refusal is counted several times. Until that is resolved the **ratio** is trustworthy and the **absolute counts are an
upper bound**; the shares above are the finding, the tallies are not a census.

## Stability

No production change. The result of the slice is that a planned slice ("unify the two predicates") was **cancelled before
it was written**, which is the cheapest possible place to lose it.
