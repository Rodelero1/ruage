# Review: the work a rule can see, and a drop site at the mine (item 4)

*Step C of session 3. The owner's item 4: "villagers should build drop sites near the resource they are mining - a rule
that raises a mining camp beside gold or stone once a villager is working one there". `where = "gold"` already existed;
the **trigger** did not.*

## What changed

1. **`When.working`: a condition on how many villagers work a resource.** `[{ resource = "gold", at_least = 1 }]`, read
   from each unit's gather task **and** its trip-home task, so a worker carrying a load still counts. The resource comes
   from `gather_source`, not from the entity kind, because a farm is a building whose column is content's business - the
   same rule the trip home uses.
2. **`World::workers_on(player, slot)`** is the count behind it.
3. **`World::can_work(from, node)`**: whether a worker can stand somewhere free *and reachable* to work a node - the
   question `Command::Gather` asks. It exists because a weaker question (`can_reach` to the node's own tile) is what made
   a test helper send a worker at a node it could not work 400,000 times.
4. **Content.** A gold rule: when the bank has ≤ 20 gold, send a villager to the gold (measured: 152 *"not enough gold"*
   refusals once the militia line trains, and the share table's 10% of thirty-two villagers never reaches the mine
   because the share pass moves *idle* villagers and a booming base has none). A mining-camp rule: one villager on the
   gold and no camp → a camp beside the nearest gold node.
5. **Placement checks reachability.** `free_tile_near` and `free_tile_near_node` now take the builder and reject a site
   nobody can walk to - the first gold camp this personality ever ordered was placed inside a wood pocket and refused at
   apply with *"no walkable route from (25, 18) to (26, 16): the target is walled off"*.

## Tests

| test | what it pins |
|---|---|
| `rules::a_rule_sees_how_many_villagers_work_a_resource` | the rule does not fire with nobody on gold and does once a villager is ordered to a gold node |
| `gather::can_work_asks_for_somewhere_to_stand` | a gold node in the open is workable; the same node ringed by eight palisade segments is not |

Mutation: the `working` loop short-circuited → *"nobody works the gold yet: left [3], right [0]"*.

## Performance

**Acceptance match, seed 7, 36,000 ticks:** 21.0 s / `4fb503de303691bf` at the end of step A; 29.4 s / `c12bdc86aab31513`
with the gold rule and the camp rule; the extra work is villagers walking to a mine ten tiles out and the camp rule's
placement searches. **And the match is now won inside the standing window**: `players=1` at 36,000 ticks, bank at 1,600 s
69 wood / 26 food / 40 gold, 47 units, 51 buildings.

**The camp itself does not stand, and that is reported rather than dressed up.** The rule fires (one firing), and the
engine's last refusal is a site it could not reach; camps in the dump: **0**. Three things are true at once: the trigger
exists and is tested, the content is written, and this personality sends only one miner at a time so the camp's own
precondition is met only late in a match that ends before the camp pays for its 100 wood. A second run with the
threshold at two produced **no firing at all**, which is how the threshold of one was chosen.

## Stability - what was attacked

- **A worker's node is not its entity kind.** A farm is a building and a carcass is a unit; the column comes from
  `gather_source`, which is the one authority the trip home already uses. A gold count keyed on `EntityKind::Resource`
  alone would miss a farm and count nothing when the food column is the one asked about.
- **The condition must not thrash.** `working` is evaluated only when a rule's cooldown expires (8 s here), and it walks
  the entity list once; it is not on the tick path.
- **Reachability is the expensive test, so it is last** in the placement chain, after the cheap footprint and lane
  checks. A search that asked it first would pay a path search per candidate ring.
- **`can_work` is not wired into `nearest_resource`.** It would ask `work_position`, which runs up to eight path searches
  per candidate, and `nearest_resource` already pays one per candidate; that is a 8x cost on a path that runs on every
  gather order and every share pass. The predicate is public and tested, and the wiring is left for a slice that can
  measure it.
- **Still open, and measured:** the last engine refusal in every run is a villager with no route home to the town centre
  (its own base walls it out), and 12 villagers were idle at the end of one run - the share pass does not re-task a base
  with no idle villagers left to move.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
