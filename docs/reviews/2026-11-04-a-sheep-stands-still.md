# Review: a sheep stands still, because its owner moves it

*Change (D116, the owner's call): **herding is dropped.** A gathered herdable used to walk itself to the nearest drop site
with the villager trailing behind (D70) — the `herds` pass planned the animal's route, re-task'd it every tick and dropped
the villager's own route so it stepped at where the animal was. Now a gathered animal is worked **where it stands**, moving
livestock is an ordinary order, and the shepherd's knife is the only thing the gather order still does by itself.

The owner's words: *"Is it about sheep being herded by shepherd? If yes, I'd like to drop this feature. Sheep are more
likely to be found by scout and player and AI are more likely to direct sheep to the drop site themselves."*

**What was removed, precisely:** the drop-site lookup, the `walk_towards(animal, drop_pos)`, and the `self.paths.remove(&villager)`
that made the follower step at a moving target. **What was kept:** everything about a herdable that is not movement —
ownership by sight, the one-blow kill, the shepherd's-knife task name, and the slaughter → body → resume-gather chain.

**The trigger moved, and that is the interesting part.** The strike used to fire when the *animal reached the drop site*.
With the animal no longer walking, it fires when the **villager is within reach of the animal** — and a strict
`distance^2 <= reach^2` never fired, because a body that walks at another stops a step short of the standoff by
construction. Measured: the villager came to rest at **10,399** raw units where the standoff is **10,000**, and stood beside
its sheep for the rest of the match. The rule is now "reach **plus one tick's step**, or the animal's own tile" — the same
"a radius, not an equality" rule the rest of the engine already uses for arrival (C2).

## Performance

The acceptance match is **unchanged**, three runs: 36,000 ticks, `players=1`, hash `82efabd6f614c798`, 148 entities, 10.39 /
10.42 / 10.77 s — identical to the hash before the change, because that scenario gathers no herdable. That is the honest
reading of a behaviour change that the benchmark cannot see: it is local to one task, and the benchmark does not exercise it.

Where it was exercised, the work went **down**: a gathering shepherd no longer costs a `nearest_drop_site` walk (units ×
drop sites) plus a route plan for the animal plus a `paths.remove` every tick. Nothing was added to any per-tick path that a
map without livestock executes.

## Stability

| attack | result |
|---|---|
| restore the herding pass | `a_gathered_sheep_stands_still` fails: the sheep walks from (440000, 440000) to (360001, 341501), i.e. toward the town centre — the exact behaviour the owner asked to remove |
| strict `squared <= reach^2` for the strike | the shepherd never strikes: measured 10,399 against a 10,000 standoff, `Task::Gather` held for 2,000 ticks |
| a sheep on the town centre's solid core (`centre + 1`) | `a_live_animal_is_a_gather_source` fails — and had been passing only because a body's *centre* could stand on the core's corner. Fixture moved to `centre + 2`, a free tile in the yard |
| a sheep ordered to walk | `a_sheep_walks_where_its_owner_points` passes: livestock takes orders like any other body, which is what the player and the AI now do instead of the animal steering itself |

**Tests retired, with their rule:** `a_sheep_stops_at_the_drop_site` measured an animal arriving at a building — the
behaviour that no longer exists. It is deleted rather than bent into something else, and the new
`a_sheep_walks_where_its_owner_points` covers what replaces it. `the_villager_follows_the_sheep` survives, re-worded: it was
"trails a sheep that is walking home", it is now "reaches the node it was ordered to work", and its tolerance tightened from
6 tiles to 2 because there is no longer a moving target to trail.

**One consequence worth naming:** the *only* remaining moving-target gather is a player who walks a sheep away from the
shepherd mid-gather. That is the case that blocked M3c — `next_waypoint(id).unwrap_or(target)` walks blind at a spent route
— and dropping herding takes the automatic version of it out of the acceptance path. The blind-walk hazard is still real
(a player can order it, and deer may one day flee), but it is no longer in the way of the shape rule.

## Known and accepted

- **No flee, no lure.** Nothing else in this engine moves on its own in response to a gatherer, so "the node walked away"
  is now only ever a player's order.
- The economy comment in `config/economy.toml` says what `herdable_roles` now means (livestock: sight-owned, one blow,
  knife) instead of describing herding.

---

*`tools/review_check.py` looks for `## Performance` with a number and `## Stability` with content; both are above.*
