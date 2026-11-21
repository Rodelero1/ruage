# Review: the click asks what the tick will ask (D314)

*Change: `Command::Gather`'s dry run no longer keeps its own plan. The route and the reach test moved into
`World::gather_route`, one copy called by both the dry run and `start_gather_with`. A gather order on a node behind a wall is now
refused **at the click** instead of being accepted and silently dropped at the tick.*

## What was wrong

The dry run planned to the node's **point** and never asked the reach question (D279's test), while the apply planned to the
**work stand** and asked it. So the two could disagree, and when they did the click won and the player lost: `submit_checked`
answered `Ok`, `apply` returned `Err`, and `step` threw the error away. Measured on `gather_pick`'s walled block:

```
candidate EntityId(11) at (13,6) workable=true
submit_checked on the nearest candidate: Ok(())
after 40 ticks: Some("Idle") (idle means the apply refused it)
```

A node behind a palisade, a click accepted, a villager standing idle for ever. The same defect is what made
`a_walled_off_wood_is_tried_once_and_then_left_alone` red: that fixture's first claim is *"the picker must take the nearest node
and let the engine refuse it"*, and it reads the refusals the runner collected - of which there were none, because the refusal
happened where nobody could see it.

## What landed

- `World::gather_route(villager, node) -> Result<Vec<(Fixed, Fixed)>, SimError>` (`crates/sim/src/world/gather.rs`): the work
  stand, the plan, and the footprint-box reach test, in one place;
- `start_gather_with` spends that route and keeps its `share_failed` book, unchanged;
- the dry run calls the same function (`orders.rs`), so a click and a tick cannot answer differently any more.

## Test-first

The witness is `gather_pick::a_walled_off_wood_is_tried_once_and_then_left_alone` - **watched failing before the change** with
`the picker must take the nearest node and let the engine refuse it; it refused nothing, so it pre-filtered`, and green after,
**with the fixture untouched**. The diagnosis above came from a probe of the same scene, not from the fixture.

**Not done, and it names a limit:** no *new* fixture isolates the rule ("a gather on a walled node is refused at the order, not at
the tick"). The existing one pins it through the rule runner, which is a longer path than a unit test needs. Owed.

## Performance

- **Acceptance match, release, seed 7, 48,000 ticks**: **9.42 s** and **9.27 s** against **9.28 s / 9.54 s** on `ec9d99c` -
  within run-to-run noise, **unresolved** rather than a win.
- The final state is **bit-identical**: hash `89ff37d844cfd11f`, 129 entities, 10,026 repaths on both trees. So **no gather order
  in the acceptance match is a walled one at the click** - the branch that changed fires zero times there, and the fixture is
  the only evidence that it fires at all. That is the honest scope of this measurement.
- What it can cost where it does fire: the dry run now asks the same question the apply will, which for a **building** node
  includes `work_position` and its up-to-eight candidate stands. That is per **order** (a click), not per candidate pick - the
  picker's own path is untouched and still spends no search per candidate (D138).
- Full workspace suite: **925 passed / 2 failed / 21 ignored** against **924 / 3 / 21**; the red set lost
  `a_walled_off_wood_is_tried_once_and_then_left_alone` and **nothing joined**. Remaining: the skirmish pair.
- clippy clean at `-D warnings`, `fmt --check` clean.

## Stability

- **One authority, and it is the apply's.** The dry run can no longer be *more permissive* than the tick, which is the whole class
  of bug: an order that the tick would refuse is now refused at the order, with the error handed to the caller.
- The `share_failed` book is still written **only** by the apply (the dry run is read-only), so a refused click does not poison the
  picker's memory - the refusal the rule sees is the one the next real order produces.
- Whatever `gather_route` returns is now spent by `start_gather_with` without re-planning, so a plan cannot be computed twice and
  disagree with itself between two calls in the same tick.
- Known and accepted: the reach test still uses `2 * scale` as its tolerance (D312), and the walk still ends at a tile centre
  (D169's settle closes the last third). Neither is touched here.

**A defect of my own, stated:** the first version of the extraction left `let from = …` behind in `start_gather_with` unused, and
the build said so - the mechanical half of moving a block. Caught by `cargo build`'s warning, fixed before the suite.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged with the
change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
