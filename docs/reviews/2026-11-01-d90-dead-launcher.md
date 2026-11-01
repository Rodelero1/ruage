# D90: a missile from a dead launcher does no damage

*2026-11-01. The owner's second combat rule: "For projectiles also cause zero damage once launcher died in original game.
If trebuchet goes missile and dies, missile causes 0 damage." Adversarial review of the change.*

## What changed

The `landings` loop in `World::step` checks its **shooter** before resolving damage. If the launcher is gone (a death sweeps
the entity) or is still present at or below zero (killed earlier in the same tick), the landing does nothing: no damage, no
blast, no retaliation, no meat. The arrow still flies and still disappears where it lands — it is an entity with its own
lifetime — it simply carries a blow worth nothing.

## Performance

One `find` per landed arrow, i.e. a linear scan of the world on the ticks where something lands. Measured on the idle probe
(no combat, therefore no landings): unchanged at 25.3 / 23.7 / 24.0 / 23.3 / 23.8 ns per entity per tick at 126→2,126
entities, best-of-3. A building's twelve-arrow volley on a 3,000-entity map pays at most ~36,000 comparisons on the landing
tick — microseconds, and rare enough that caching the lookups would cost more than it saves. That is the price of the rule
and it is stated rather than hidden.

## Stability — what was attacked

1. **Both kinds of dead.** A launcher killed earlier *in the same tick* is still in the entity list with `hp <= 0` (the
   sweep runs at the end of the tick), while one killed on a previous tick is simply absent. The guard uses
   `is_none_or(|e| e.hp <= 0)`, so it covers both; checking only `is_some()` would have let the same-tick case through and
   the test above would still pass — the failure mode would be a corpse's arrow landing on the tick its owner fell.
2. **The blast goes with it.** A trebuchet's stone that lands after its launcher died does no splash either, because the
   whole landing is skipped rather than just the aimed hit. "Causes 0 damage" is the owner's wording, and a splash is
   damage.
3. **Nothing else in the landing is skipped by accident.** The arrow is still removed (it is already in `landed_arrows`
   before this loop), the target is still pushed to the eject list so a building that took no damage is evaluated exactly
   as before, and no state is added, so the hash is untouched.
4. **Entity ids are never reused** (`insert` takes `max + 1`), so a dead shooter cannot be confused with a live entity that
   inherited its id — the one way this check could silently hit the wrong unit.
5. **A building's volley follows the same rule**: every arrow carries the building as its shooter, so razing a tower stops
   the volley already in the air. That is the same rule, not a special case.

## What this does NOT settle

- **Whether the *original* stops the arrow or merely zeroes its damage.** The owner's rule is about the damage; we keep the
  arrow flying because that is what the engine already modelled. The difference is invisible unless something else reads a
  landing — and nothing does today.
- **Whether a hit that does 0 damage should still count as a blow** for kill credit or retaliation. It does not, and that
  is the deliberate reading of "causes 0 damage": no blow happened.

## Tests

One new test (`a_missile_from_a_dead_launcher_does_no_damage` in `crates/sim/tests/blow_order.rs`), watched failing first
with the villager on 21 health instead of 25 — the arrow had landed for full damage. `ballistics`, `combat` and `blow_order`
stay green, and the acceptance match's hash is unchanged.
