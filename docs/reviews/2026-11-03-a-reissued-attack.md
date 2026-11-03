# Review: a re-issued attack changes nothing

*Step B of session 3: the first half of the owner's item 7 - "soldiers walk up and down instead of consistently
attacking enemy buildings (they strike, then withdraw)".*

## What changed, and why it is the right lever

The base personality re-sends its army every twelve seconds (`[[rule]] do = "attack"`). Two things happened on every one
of those orders, and both are visible on screen as a soldier that does not keep fighting:

1. **`start_attack` set `cooldown_ticks: 0` unconditionally.** A soldier halfway through its reload was reset by the
   order, so the army swung on the *order's* twelve-second beat instead of its own two-second one. In the engine this is
   the opposite of "strikes, then withdraws" - it is *extra* swings - but it is the same defect: the unit's fight is not
   its own.
2. **`start_attack` re-planned the route from wherever the soldier now stood.** Eleven soldiers converging on one
   building each recompute "the nearest tile from which the blow lands" every twelve seconds, and near-ties between two
   stand tiles are exactly how a line shuffles back and forth ("they strike, then withdraw").

Both are answered by one rule: **an identical attack order changes nothing.** If the task is already `Task::Attack` on
the same target with the same resolved damage, reload, range and dead zone, `start_attack` keeps the route and the
cooldown. A different target, or a different resolved fight, starts afresh.

`retaliation` was checked and is **not** the cause: it answers only animals (D71's rule, and the game's - a villager shot
at keeps gathering), so a soldier never turns off its ordered target.

## Tests

| test | what it pins |
|---|---|
| `combat::a_reissued_attack_keeps_the_reload_it_has_run` | the first blow lands at the swing, the re-order lands twenty ticks into the reload, and **two** blows have landed twenty ticks after that - not three |

Watched failing first: *"two blows by tick 80, not three: left 13, right 17"* - the reset version had already swung at
the re-order.

The route half is not separately observable from outside (`path_length` is the same for a re-plan from the same spot to
the same goal), so it is documented rather than pinned by a second test. That is a gap in the coverage, named here.

## Performance

**Acceptance match, seed 7, 36,000 ticks, direct binary:** 21.0 s before (`069d6c7`, hash `4fb503de303691bf`), 26.8 s
with this change and the content below (hash `1d25f5aba9ba1a6e`). The wall clock is not this change's alone: the same
commit adds a gold-gathering rule that walks villagers across the map.

**And the change made the army weaker, which is the measurement worth keeping.** At 36,000 ticks the enemy town centre
stood at **19 hp** before the fix and at **385 hp** after it, with the same content: the bug had been handing the army
free swings by resetting its reload, and a correct reload is slower. The win came back through the AI (a villager put on
gold), not through combat - see the next review.

## Stability - what was attacked

- **A re-order while walking.** The route is kept when the task matches; a soldier pushed off its route by collision
  still recovers through `collide`'s repath, which does not go through `start_attack`.
- **A re-order at a target that has become invalid** (a corpse, a resource, a garrisoned unit, a friendly one) still
  runs `check_attack` first and is refused exactly as before - the identity check is *after* the validation.
- **Two different fights, same numbers** (two identical villagers): the target id is part of the match, so swapping
  targets resets the swing. Pinned by the guard's shape, not by a test - named as the gap.
- **A building re-ordered at itself** (a tower told to attack): `source.kind == Building` skips pathing as before and
  the cooldown is kept on a match; a building's own volley therefore no longer restarts on a re-order either.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
