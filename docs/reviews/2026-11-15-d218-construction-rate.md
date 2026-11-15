# Review: the Spanish builder bonus, and a debt whose evidence was a word

*Change: a player's construction rate is dat resource 195, named by content (`[build] rate_resource = 195`) and read from the
player's researched effects **and** the civ's tech-tree effect - the Spanish set it to **1.3** there, and no civ's own row
states it (all hold 0, so the base is 1.0). `check_build` divides the site's builder-ticks by it and rounds up. The same slice
found that the tree's "tech bars do nothing" debt is **false**. D218.*

## Performance

**No per-tick work, and the acceptance match is bit-for-bit unchanged**: `hash=beb6c66f58e4045c`, `entities=154`,
`repaths=277` at 24,000 ticks, against `beb6c66f58e4045c` / 154 / 277 measured immediately before the change - the script's
players are not Spanish, so the fixture is what holds this number, not the match. The same script ran in **19.8 s** on a box at
load 2.29, which is quoted as the instrument rather than as a claim: an identical hash means identical work, and the change
cannot be inside a tick it does not touch. `construction_rate` runs **once per build
order** (a few hundred a match) and walks one effect's command list plus the researched techs' lists; a build order already
resolves a unit, a price and a site check, so this is a rounding error on a cold path. No timing claim is made from the
acceptance wall clock, which was not measured under a quiet machine.

## Stability

- **The rate is read where the game keeps it, and the "not stated" case is the dangerous one.** Every civ's `resources[195]` is
  **0.0**; taking that at face value as a multiplier would freeze every site for every civ. The function therefore starts at
  **1.0** and only takes a stated row, and the test asserts a Briton's house is still **500** builder-ticks while a Spaniard's
  is **385** - so the default and the override are both pinned by the same fixture.
- **Rounding is a decision, not a detail.** 500 / 1.3 = 384.6: 384 ticks reach 499.2 builder-ticks and the site would need a
  385th, so the total rounds **up**. The fixture's number (385) is the rounding rule written down; a `floor` would have
  produced 384 and a house that never quite finishes on its last tick.
- **A debt in the tree was wrong, and only reading the code showed it.** The row said "`DisableTech` is parsed and never
  consulted - a civ's tech-tree bars do nothing", and it was reported to the owner as one of the five sharpest items. It is
  false: `barred_by` reads the kind-102 rows of that same effect and `check_research` refuses with `TechBarred` before
  prerequisites, pinned since D23 by `a_barred_tech_is_refused_by_the_civilisation`. The row's evidence was `grep DisableTech`,
  a word the code never uses. **A grep for a name is not evidence about a mechanism** - the mirror image of this project's
  existing rule, and the first time the rule cost credibility outside the repo.
- **One effect, two halves.** The Spanish tech-tree effect carries both the kind-1 rate row and the kind-102 bars, so this
  slice could have "discovered" the bars as missing while implementing the rate beside them. The test that would have caught a
  regression in either is now the same fixture's neighbourhood: build ticks for the rate, `TechBarred` for the bars.

Known and accepted:

- **`rate_resource` is content, not an engine constant.** A pack that names no resource keeps 1.0 everywhere, which is exactly
  what the hand-written test packs get.
- **Repair speed is not scaled by this rate.** `[build] repair_hp_per_second` is a different rule; whether the original scales
  repair by the same resource is unmeasured, and nothing here pretends otherwise.
- **The tree's `T5`/`DisableTech` row is corrected, not deleted**: it now says what is true (kind-102 rows are applied) and
  cites the two places that apply and test them.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
