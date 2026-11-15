# Review: the minimum-1 rule is once per attack

*Change: documentation only - `docs/behaviour-diff.md`'s damage row moves from "same arithmetic, floor unproven" to **`same`**, on the strength of three published
sources (one of them the HD 2013 guide), and D228 records the three latent differences it leaves behind. **No simulation code changed.***

## Performance

**Zero**: `git diff --stat` on `crates/` is empty, so the gate's numbers stand - 191 binaries, 874 passed, 0 failed, 20 ignored. The cost was one web search and
three readings; the thing it buys is a **rule** where there was an open question, which is the cheapest possible form of a closed debt.

## Stability

- **The claim is now a rule with named sources, not a belief.** The Armor class page's step 6, the Attack page's formula and the **Age of Empires II (2013)** guide
  agree on the same shape: each class's difference is floored at zero, the classes are **summed**, and the `max(1, ...)` is applied to the sum **after all
  multipliers**. The binary neither contradicts nor proves it (the clamp is in a virtual method's caller), and the register says exactly that rather than implying
  more.
- **The alternative reading is named, so it cannot come back as a discovery.** A per-slot clamp would have made a fully-stopped attack land for **3**; it is excluded
  by all three sources, and `combat::no_attack_never_floors_below_the_minimum` would have caught it in the engine.
- **Three latent differences are recorded with the reason each cannot show today**: negative class damage (D216 refuses every unexplained negative attack value, so
  none reaches the arithmetic), bonus-damage resistance (nothing in the 5.7 effect table names such a modifier), and the floor's position relative to
  `elevation_factor` (every height is zero, so the multiply is by 1.0 - but the slice that makes elevation real will find the note).

Known and accepted:

- **The sources are community documentation, not the engine**, and the row says which evidence is which: published arithmetic for the rule, a decompiled accessor
  for the shape, and "not located" for the clamp itself.
- **The elevation ordering is written down and not fixed.** Fixing it would move `minimum` out of `damage_between` and after the multiply, which cannot be tested
  while `elevation_factor` is identically 1.0 - an unproven change, which this branch has refused twice for the same reason.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
