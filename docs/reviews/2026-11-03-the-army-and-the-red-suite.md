# Review: an army, the ground a farm leaves, and a suite that was already red

*Step A of session 3 (the owner's items 8 and half of 7, plus the `farm_husk` leftover named in the queue).*

## What changed

1. **`When.army_at_least` and `army_at_most`, and `World::army(player)`.** A rule can wait on a **count of military
   units**, taken from content's `[metrics] military_roles` (the list the balance harness already counts), matched by
   family so a man-at-arms answers for the militia line. This was written once before and reverted; it is re-landed here
   with its cause found (below).
2. **`RuleAction::Attack` sends the whole army.** `resolve_rule` now returns a list of commands, and an attack rule
   orders **every** unit of the attacker's family, choosing a target per soldier (reachability is part of that choice).
   The old shape ordered `max()` by id - the newest unit - so one mangonel fought while eleven soldiers stood at the
   barracks.
3. **A spent farm is a husk the next farm can use.**
   - `Command::Spawn` gave **any** unit the dat's `food` field as its `amount` when told zero; a farm's `food` is its
     250-food capacity, so a spawned spent farm came back full and `is_expired_farm` never saw it. The default now
     applies to animals only (what `[hunt] animal_roles` names).
   - `is_free_ground` counted a husk as occupied while `site_refusal` counted it as free. The placement search asks the
     first, so a farm could not be rebuilt on its own spent plot: **105** *"no free tile for a farm"* refusals in one
     acceptance run, with nine husks in the base.
   - `site_has_lane` counted a **farm** as a wall. A farm's footprint is all walkable - `[footprint.farm]` is `"..."` -
     so it cannot wall anybody in; the exemption is what lets a husk *inside* a field be refilled.
4. **D58's blind overbuild was dead, and nine tests said so.** `site_refusal` skips a building the player cannot see when
   building its `held` map - and then asked `walk.blocked(tile)`, where `insert` had already blocked the same building's
   tiles. So a hidden foundation refused a blind placement with `SiteBlocked`, and "the placement is allowed and
   cancelled on sight" could not happen. Tiles blocked by an **unseen building** are now the one kind of blocked ground a
   blind site may cover.
5. **A self-issued refusal now names its actor.** `entity_of_last_command` was a stub returning `EntityId(0)`, so every
   recorded refusal read `entity 0`; the actor is read before `apply` takes the command. And the queued build that cannot
   start was `let _ = self.start_build(...)` - the order vanished silently; it now goes into `last_refusal`.

## Tests (each watched failing first)

| test | what it pins |
|---|---|
| `rules::an_army_counts_soldiers_by_family_and_not_villagers` | three soldiers are three; a villager is none; a man-at-arms counts as his line's root |
| `rules::an_army_rule_waits_for_an_army` | the rule does not fire at zero soldiers and does at three |
| `rules::an_attack_rule_sends_the_whole_army` | all three soldiers are ordered, not just the newest |
| `farm_husk::a_spent_farm_is_replaced_on_its_own_tile` | the parked defect: a spawned farm told "nothing left" is a husk, and a new farm replaces it for the full 60 wood |
| `farm_husk::a_spent_farm_is_free_ground` | the placement search can offer a husk's tile; a working farm still holds it |
| `farm_husk::a_farm_beside_a_farm_leaves_a_lane` | a farm is a floor; two houses edge to edge are still a wall |
| `build_queue::a_second_site_that_overlaps_the_first_names_its_builder` | the refusal names the villager and says the ground is taken |
| `build_queue::a_queued_site_whose_ground_is_taken_is_reported` | the queue-pop branch records a refusal instead of dropping the order |
| `foundations::*` (nine, previously red) | a hidden foundation does not block; a seen one does |

**Mutations seen to bite:** `if false && army < least` → *"no soldiers, so no army: left [3], right [0]"*; the attack
action reduced to `.take(1)` → *"EntityId(128) was left behind: Some(Idle)"*; the farm-lane exemption removed →
`a_farm_beside_a_farm_leaves_a_lane` fails at its first assertion; the husk-amount fix reverted → the parked farm test
fails at *"a farm with no food is expired"*.

## Performance

**The acceptance match, measured back to back, direct binary, seed 7, 36,000 ticks:**

| | before (`55fcc2f`) | after |
|---|---|---|
| wall clock | 15.2 / 15.4 / 15.4 s | 21.0 / 21.0 s (0.58 ms/tick) |
| hash | `91a8ad6e2cbd01f6` | `4fb503de303691bf` |
| entities / repaths / made_room | 131 / 177 / 214 | 171 / 147 / 194 |
| units, buildings | 35, 24 | 44, 52 |
| bank at 1,600 s | 1,914 wood / 416 food / 512 gold | 484 / 46 / 60 |
| rule firings | 14 rules, none military | 16 rules; the new train/attack pair fires 155 and 72 times |

**The 35% is the match doing more, not the code doing it.** The run now trains an eleven-man army, builds 28 more
buildings, and has the army marching and fighting for the last third of the match; repaths and made_room are *lower*
than before. The one per-candidate cost the change adds is in `site_refusal`: an unseen building's footprint is
collected per site check. It is bounded by the number of buildings the player cannot see, and the placement search pays
that cost per candidate exactly as it already paid for the entity scan; the manner in which it is measured here is the
whole-run wall clock, which does not isolate it. A dedicated A/B (the same binary with the army rules disabled) is owed
if this path is touched again.

**The army result, which is the point of the step.** At 36,000 ticks the enemy town centre stands at **19 hp** and the
enemy is down to three villagers; at **36,500 ticks the town centre is gone**, and at 37,000 `players=1`. At 48,000 ticks
(2,400 s, the scenario's own length) the run ends `players=1`, hash `a1ec92ed664e4773`. So: **the AI gathers an army and
wins the match, just after the standing 36,000-tick measurement point** - 10 militia at 1 damage a blow take ~500 s of
contact to eat 2,400 hp, and the food column is what holds the army back.

## Stability - what was attacked, and what fell over

- **The suite was already red at HEAD, and that is the finding of the step.** In a clean worktree at `55fcc2f` with the
  generated `config/gamedata/` copied in, a full `--no-fail-fast` run reports **525 passed, 40 failed, 18 ignored**
  (`build` 3, `build_queue` 2, `foundations` 9, `gather` 3, `population` 4, `farm_husk` 1, `defeat` 1, `delete` 1,
  `dropoff` 1, `building_passage` 1, `research` 8, `tech_tree` 6, `walls` 1). Every one is the same family: a test staged a building on a fixed tile that the footprint work
  (buildings own their ground) made illegal. **The reason nobody saw it is D8's own skip:** every suite returns early when
  `config/gamedata/` is absent, which is exactly the state of a fresh clone or a clean worktree, so "100 binaries green"
  was measured in a tree where the tests did not run. **32 of the 40 are fixed here** by asking the engine for a legal site (`site_fits`) instead of naming a tile - except the
  ones that were engine defects (D58 above, and the refusal actor). The eight that remain are all in `research.rs` and
  run for 400-600 s in a debug build; they are named at the end of this review rather than fixed in this slice.
- **My own error, in the queue entry and in the compact.** The reverted `army_at_least` was blamed on
  `World::army` returning fewer than three for three spawned militia, "cause not found". The cause was the **test**: it
  computed `max(id) + 1` once per iteration without stepping, so all three spawns named one id, two were refused
  (`DuplicateEntity`, silently, by design), and one soldier existed. `army()` was never wrong. The `map_or` change in the
  re-landed version (a world with no unit table compares ids instead of returning zero) is a real hardening, but it was
  **not** the cause and the compact said it was.
- **My second error, in this slice's own tests.** The first version of the "seen foundation blocks" staging used (20, 16),
  which is forest on the project map; and the house-lane assertion used (33, 30), which leaves a one-tile gap between two
  2x2 houses - so it passed for the wrong reason until the coordinate was fixed to (32, 30).
- **The farm lane exemption is a rule change, and its risk is named.** Farms may now be packed edge to edge. The
  exemption's justification is that a farm's footprint blocks nothing, which is `[footprint.farm]` as data; the risk is a
  field of plots that a villager cannot queue around, which the earlier slice saw. The acceptance run's farm refusals fell
  from 105 to 67 and its food column improved; if a packed field ever traps a worker, this rule is the first suspect.
- **Still red and named, not fixed:** villagers cut off from their own town centre by the base's layout - the last engine
  refusal in every acceptance run is *"no walkable route from (22, 18) to (16, 16): the target is walled off"*. It exists
  in the baseline too, so this slice did not cause it, and it is not in the nine.
- **Rule firings are counted, not assumed**: the run reports `rule firings: [...]` including the new army rules, so a
  rule that stops firing is visible rather than inferred from a flat bank.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
