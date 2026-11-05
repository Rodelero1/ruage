# Adversarial review: the panel, the job names, and the farm changes

*Asked for by the owner (2026-11-05): "Please do complete adversarial review on performance and stability." Scope: the three
slices landed since the last review - **D120** (the card in the bottom bar), **D121** (job names) and **D122** (one farmer a
farm, and the husk). This is an attack pass, not a summary: five defects found, four fixed here, one behavioural rule the owner
had stated and the code had not.*

## The five findings

**F1 - "nearby" was not implemented (behaviour, fixed).** The owner's answer was *"2 C if there is no empty farm nearby,
villager becomes idle"*. The code sent the second farmer to the **nearest free farm anywhere on the map** - the word "nearby"
was in the decision record and not in the search. A player ordering two villagers onto one field could watch the second walk
across the map to a plot he did not mean. Fixed with `[farm] free_farm_radius = 12` (content, tiles) and a test that puts the
only free plot **40 tiles** away and asserts the villager stands: `a_free_farm_far_away_does_not_take_the_second_farmer`.

**F2 - the husk pass was O(n^2) (performance, fixed).** `step_husks` asked `is_expired_farm(entity.id)` for every entity, and
that accessor does a **linear `find`** - for an entity the loop was already holding. Measured on the tick microbenchmark the
morning after D122 landed: **step 5.9 -> 7.4 us (+23%)**. Rewritten to test the entity in hand (`is_expired_farm_ref`), and the
tick is back to **6.0-6.2 us**.

**F3 - the snapshot was O(n^2) as well, and re-derived every card (performance, fixed).** `Snapshot::of` called
`is_expired_farm(entity.id)` per entity - the same linear `find` per entity, now on the **frame** path - and asked `stats_of`
per entity, which clones the unit's attack and armour lists and resolves four fractional values *through their decimal text*.
Fixed by testing the entity in hand and caching the card's numbers per `(owner, unit)` for the life of one snapshot.

**F4 - a husk removed under a villager's task (stability, tested).** The husk is deleted from `entities` while a villager may
still name it: `Task::Return { node }` on the last load home, or `Task::Gather` taken before the farm emptied. The concern is a
dangling id that leaves the villager standing for the rest of the match. Test `a_villager_survives_his_farm_vanishing` runs past
the husk's whole life and asserts he is neither removed with it nor still holding it.

**F5 - two orders in the same tick (stability, tested).** A box selection sends both gather orders before the world steps. If the
occupancy check only looked at *applied* orders it would let both men onto one farm. Test
`two_orders_in_one_tick_still_take_two_farms` submits both orders with no tick between them and gets two different plots.

## Performance, measured on an idle machine

`cargo run --release -p ruage-view --example step_cost` (added by this review), a settled 128-entity match, 400 runs each,
three interleaved pairs, `PRE` = `7ba3b3e` (the card, before the farm slice) against this tree:

| run | snapshot `PRE` | snapshot now | step `PRE` | step now |
|---|---|---|---|---|
| 1 | 111.4 us | 95.1 us | 6.0 us | 6.2 us |
| 2 | 110.9 us | 96.1 us | 6.0 us | 6.2 us |
| 3 | 115.8 us | 95.3 us | 6.1 us | 6.2 us |
| median | **111.4 us** | **95.3 us** | **6.0 us** | **6.2 us** |

- **Snapshot: -14.4%** (111.4 -> 95.3 us). The snapshot is now *cheaper than it was before the card existed*, because the card's
  numbers are computed once per distinct unit instead of once per entity per frame.
- **Tick: +0.2 us**, inside this box's noise, after fixing F2's +1.4 us (+23%).

Whole-match figures, same binaries, three interleaved runs each against `bbaa3b3` with its own `config/`:

| | `bbaa3b3` | `2503625` (D122 as landed) |
|---|---|---|
| match | 10,389 ms | 11,887 ms |
| entities | 148 | 154 |
| repaths | 124 | 211 |

**And my own accounting of that +14% was wrong.** The D122 review said the extra second was "the match doing more work, not the
new passes". Half right: the match *is* bigger (154 against 148 entities, 211 against 124 repaths), **and** the pass itself cost
+1.4 us a tick. The microbenchmark separated what the wall clock could not; the correction is in this review rather than left in
the older one.

## Stability: what was attacked, and what held

| attack | result |
|---|---|
| a second farmer, free farm nearby | redirected; `a_second_farmer_goes_to_a_free_farm` |
| a second farmer, free farm **40 tiles** away | stands idle (F1's test) |
| a second farmer, no free farm | stands idle; `a_second_farmer_with_no_free_farm_becomes_idle` |
| both orders in one tick | two plots, never one crop shared (F5's test) |
| a husk's ground | walked over by a villager; the husk is not an obstacle |
| the husk's clock | removed after 30 s of content time |
| a villager still naming the husk when it goes | survives, is not removed with it, does not keep a dead node (F4's test) |
| replanting an enemy's husk | refused by the view: not a build order |
| a house raised on a spent plot | already covered by `building_passage.rs` and still green |
| a pack with no `husk_seconds` | 30 s default; a pack with no farms never enters the pass |
| a save that lost the husk clock | impossible from this state: `husks` is in the state hash |

## What is still open, named rather than hidden

1. **`nearest_free_farm` is O(entities) per redirected order.** It scans the world for a free plot on every second-farmer
   order. Bounded by how often a player re-tasks farmers, and not on the tick path unless a script does it; measured as part of
   the tick (+0.2 us) and not separated out. If a rules pass ever re-tasks many farmers per tick, this is where to look first.
2. **`is_farm_role` costs a role-name lookup per call**, and the husk pass calls it per entity every fourth tick. Small at 154
   entities (four-tick period, ~40 role names); a cache of farm unit ids, like `herdable`, would remove it entirely.
3. **The husk has no art.** A spent plot draws as the farm it was until it vanishes. This belongs to the asset pack, as the
   portraits do.
4. **Husk removal is state, not a command.** A recording that replays commands therefore depends on `husks` being hashed - which
   it is - but no test replays a match across a husk's expiry yet. The replay suite is the place that should own it.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
