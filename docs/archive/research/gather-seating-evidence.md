# Where a working villager stands: what the original actually does

Written 2026-11-19 to crosscheck **Q1** (is our `gather_place` taken list the game's mechanism?) and, because the sources settled
it too, the jam question (**Q2**). The owner's standing rule applies: research first, and if the evidence is inconclusive say
which part is unknown instead of choosing a default.

## What the binary and the decomp can say

- **The decomp (`samuelcaldas/age-of-empires-2-decomp`) says nothing**: it has no gather or work code at all - the unit-action
  files are 16-47-line stubs, as recorded in `docs/prior-art.md`.
- **The corpus (`/opt/data/tools/rea/pseudo-aokhd`) has the gather action as a state machine**, not a seat picker: the only
  function whose strings name `act_gath.cpp` (`FUN_004aaea0`) switches on the action state and calls virtuals; the only one naming
  `tact_gth.cpp` (`FUN_004b90f0`) is the task-level switch. No loop over candidate positions round a resource, no taken list, no
  per-man index was found in either. **This is an absence of evidence, not evidence of absence** - the corpus is 15,465 functions
  read by their log strings, and a seat picker could hide inside a function whose strings name something else.

## What the game's own players report, which is decisive here

Three independent kinds of report, all about the same engine lineage (DE, but the behaviour they describe is the AoK one our
binary is):

1. **Men sent to one resource bump and stand; they are not given private places.** Capoch's build-order guide: *"If you send them
   to different trees within the same tile, they will usually stand still. This will also happen if you send 3 villagers to the
   same tree."*
2. **A blocked gatherer is re-targeted to the nearest usable resource of the same kind** - the "corner tree" trick:
   *"if a villager collides with an object near its target it will retarget to something else ... the second villager gets blocked
   so it retasks to the closest available target"* (`forums.ageofempires.com/t/degradation-of-game-bugs-pathfinding/289465`), and
   *"I think the vills go to nearest available tree if they can't path to an assigned one"*, with *"1 will just go idle, and 1
   will go in a different direction to chop a different tree"* when six are re-tasked at once
   (`r/aoe2`, "Am I imagining this villager pathing bug?"). The `aoe2mc` project's replay-driven model encodes the same rule as a
   `NEXT = 5` tile search for the next resource of the same kind (`lars-his-projects/aoe2mc` issue #4063).
3. **A walk that cannot advance never gives up.** The best-reported bug in the game is exactly that villagers stay wedged:
   *"they keep changing direction simultaneously and stuck blocking each other"*, *"frozen in place"*, and *"instead of
   recognizing that there's no available route and pausing, they continuously try to navigate without actually progressing"*
   (`forums.ageofempires.com/t/villagers-blocking-each-other/78029`).

## What that means for us, stated as the two answers

- **Q1 - the taken list is ours, not the game's.** The original keeps **no** private place per man: it sends every man to the
  resource he was given, the bodies are solid, and a man who cannot get there is **re-targeted to the nearest usable resource of
  the same kind**. Our `standoff_point` taken list produces a *smoother* result than the game has ever had - men fan out and
  nobody ever bumps - so it is a **deliberate difference**, and keeping it is a choice rather than parity.
- **Q2 - the walk stands, the gather re-targets.** Cutting `finish_blocked_orders` and `beside` follows the game (point 3 above:
  the game's own complaint is that wedged villagers never resolve). Cutting the **D204 re-task does not**: point 2 is the game
  re-targeting a blocked gatherer to the nearest usable resource, which is what D204 does. **So D204 stays and the other two go**,
  which is the opposite of what this project assumed an hour before this note was written.

## Which part is still unknown

Whether the original's re-target radius, its "same kind" test and its choice among equally near candidates match our D204
(`STALLED_TICKS = 40`, nearest-like within a search radius, same resource kind). The sources give the *rule* and a *roughly five
tiles* figure from a third-party model, not the constant. That is the next thing to measure in a replay, not to assume.
