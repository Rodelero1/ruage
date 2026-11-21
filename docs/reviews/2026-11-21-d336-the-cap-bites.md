# D336 - P15: the 20,000 cap's row can fail now

**Crosscheck first.** `FUN_008007f0:198`, read again: `if (((DAT_00aad420 != -1) && (iVar20 < local_46c)) || (20000 < local_46c))
break;`. Two clauses, two budgets: the estimate (`iVar20`, 25 x euclid) and the hard cap (20,000). The cap is a *separate*
rule and the only one that binds when the estimate is larger than it, which needs `euclid > 800` - a map past ~1,130 tiles a
side. The old row walked a 480 x 480 map, whose estimate is 16,850 and which spends 642 expansions, so no mutation of the cap
could redden it (measured in D334: `EXPANSION_CAP = u32::MAX` left all four rows green).

**The change (conformance only, no engine line).** The row now builds a 1,200 x 1,200 map with the goal sealed in a 3x3 ring,
asserts that the scene's own estimate exceeds the cap (so the scene cannot silently stop testing the rule), and asserts
`expansions == 20_000` exactly. Green on the engine as it stands.

**Mutation-proved.** `EXPANSION_CAP = u32::MAX`:
*"the cap is the budget on this map: 42300 of 20,000 (estimate 42300)"* - left 42,300, right 20,000. The row fails on the
count, which is what the old row could not do. Cap restored, row green.

## Performance

Four budget rows in **0.10 s** (debug, `--test budget`), against 0.01 s before: the new scene spends 20,000 expansions, so it is
the row that costs the set anything. 1.44 M tiles of `best`/`from` per call (~17 MB), which is why the map is 1,200 and not
larger.

## Stability

- Determinism: the search is deterministic, and the count is asserted exactly, so a different expansion order fails loudly
  rather than drifting.
- The estimate is asserted `> 20_000` before the search runs, so the scene fails on a *changed estimate* too - the one thing the
  old row's arithmetic duplicated is now a precondition rather than the assertion.
- Attacked: the cap (mutation above), the scene (the estimate precondition), and the answer (the route ends short of a sealed
  goal).
