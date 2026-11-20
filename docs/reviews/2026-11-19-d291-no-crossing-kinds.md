# Review: the engine does not cross resource kinds (D291)

*The owner: "How is `retarget::a_forager_whose_bush_is_finished_goes_to_a_farm` handled by the game? The game code itself should never
hardcode such move. That should be handled by AI to repurpose idle villagers. Is it the case?" - it is, and our engine did the
opposite.*

## What changed

`World::nearest_like_in` matched a spent bush (`Resource(food)`) to a **finished farm** (`Building`) on purpose, with a comment
explaining that a forager would otherwise have nowhere to go. That arm is cut, so the engine's next-source search is **same-kind
only** - a spent tree finds the next tree, a spent bush the next bush, a spent farm the next farm.

**Why the game wins:** the original's next-source search is same-kind inside a radius (the binary's radius is the worker's line of
sight plus five tiles, D149; the replay-derived DE model `aoe2mc` uses 8 for trees and 5 for the rest), and moving a man between
resource *types* is the **AI's** work - the game ships `sn-retask-gather-amount`, `sn-max-retask-gather-amount`,
`sn-food-gatherer-percentage`, and a technology whose only job is to improve it, `up-retask-gatherers`. The decomp has no gather code
at all, so it is silent either way.

## Performance

- the cut **removes** work: `nearest_like_in` no longer resolves the farm's resource column (`economy.farm.resource` -> `slot_of`) on
  every spent-node pick, and `slot_of` is a string lookup over the resource table;
- `retarget` (2 tests, including the 3,000-tick forager) runs in **0.94 s**; the collection it belongs to - `gather`, `farm`,
  `reseed`, `retarget`, `acceptance_economy` - is **2.62 s / 34.25 s / 1.45 s / 1.77 s** by file, unchanged in shape from the last
  run;
- the full suite is **890 passed / 17 failed / 21 ignored**, the same red count and the same names as the D289 baseline
  (`/opt/data/cache/scratch/after-d289b.txt` against `after-d291.txt`).

## Stability

**Test-first, in the right direction.** The fixture was re-pointed before the cut and watched failing:
`the engine sent the forager to a farm by itself: Some(Gather { node: EntityId(130), ... })`. It passes after the cut, and its name
now states the rule (`a_forager_whose_bush_is_finished_stands_rather_than_walking_to_a_farm`).

**Consequence the owner should know:** a forager whose patch runs out with no bush in reach now **stands idle** until the player or
the AI re-tasks him. That is the game's behaviour and it needs the AI layer (D17: the engine knows kinds, not particulars) to be
useful at scale - until that layer exists, a match with a spent berry patch leaves men standing. The three economy fixtures that were
already red (`a_depleted_node_sends_the_villager_to_the_next_one_of_its_kind`, and the two `reseed` ones) are unchanged by this cut
and remain in the queue.
