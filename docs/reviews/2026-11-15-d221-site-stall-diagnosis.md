# Review: the 6x6 site stall is a blocked villager, not a silent builder

*Change: no production code. The parked fixture `build::a_content_pack_with_our_own_resources_loads_and_spends` gets a
measured `#[ignore]` sentence instead of the wrong one it carried, and `docs/next-step.md` gets the diagnosis and the two
probes. The earlier sentence said "a builder with a live route is asked for no step (step=Idle)" - that is not what happens.*

## Performance

**Nothing is changed, so nothing is measured in cost - what is measured is the stall.** The site sat at
`progress_ticks = 0` for the fixture's whole **2,000-tick window (100 s of game time)**, while `site_is_clear` reported the
same blocker every tick, and the blocked villager's step-aside returned early on each of them. For the record on the cost of
the fix that will follow: one failed step-aside costs one ring of path searches, which is why the "remember the attempt" cache
existed (its own comment cites an acceptance run going from 16 s to 180 s), so the retry has to be a *deadline*, not a
per-tick search.

## Stability

- **The old diagnosis was wrong in the direction that wastes a session.** The builder is not stuck: it walks to `(5, 4)` by
  tick 30 and stands correctly adjacent to the 2x2 footprint. What holds the site is **another villager of our own parked on
  the footprint** - `site_is_clear` reports `standing [(EntityId(4), 1)]`, its `step_aside` is called every tick, and every
  call returns early because the attempt is recorded in `resuming` **even when the search failed**. A single transient
  failure therefore becomes permanent.
- **The retry was implemented, run, and did not fix the fixture - which is how the second defect became visible.** With
  `ASIDE_RETRY_TICKS = 20` the blocked villager did get a route, and then could not take a single step: the builder's standoff
  tile `(5, 4)` is that villager's only exit, and a body standing on the goal makes every step toward it illegal. So the pair is
  the defect: a failed exit that is never retried, and a standoff search that may hand the builder the only tile another unit
  can leave by.
- **Both changes were reverted rather than landed.** The retry is a real improvement and is still unproven against this
  fixture; shipping it alone would have turned one parked test into one parked test plus an unexercised code path. The tree is
  clean at the previous commit's production code, and only the fixture's sentence and the note changed here.

Known and accepted:

- **The fixture stays `#[ignore]`d**, and its sentence now names the real blocker and the tick range where it was observed, so
  the next session starts from the measurement rather than from the guess.
- **`site_is_clear`'s own comment** ("The builder is not told to leave: its own order is the walk to a standoff outside the
  footprint") is the design decision that produced the conflict; the fix must keep the builder's order intact while stopping
  it from taking a tile that is another unit's exit.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
