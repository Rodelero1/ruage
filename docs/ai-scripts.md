# Scripted AI: counters, scouting, and what the engine has to expose (2026-11-01)

The owner's question: the military AI today is "which unit types to train"; can it go further - counter the player's army,
and react in other ways - **with the behaviour in scripts rather than in code**, and testable headless, without cheating.

This note is the design, not the implementation. It exists so the next session extends the same shape instead of
inventing a second one, and so the three forks below get an owner's ruling before any code lands.

## What exists today

**D91/D92/D93 are decided** (`docs/decisions.md`, 2026-11-01) and the first piece is in the tree: `tools/counters.py`
derives damage, damage per second and damage per resource-second between any roles the pack names, from the pack's own
attack/armour classes, with the engine's formula - including the part that is easy to get backwards, that a class the
target does not list contributes **nothing** (`armour_missing_sentinel`). Against a knight the order it prints is
halberdier 0.200 > pikeman 0.133 > spearman 0.089 > militia 0.013 damage per resource-second: derived, not remembered.
That is the table a `[[rule]]` is written from (D91), and with D93 it is a tool rather than an engine dependency.

`crates/sim/src/script.rs` (D75). A `[[rule]]` is `player`, `at_seconds`, `cooldown_seconds`, a `when` block and one
`do` action. Conditions are **all-of** and every field is optional: `villagers_at_least/at_most`, `has_building`,
`no_building`, `age`, `no_tech`, `has_unit`, `housing_at_most`, `stock`. Actions: `train{building, role}`,
`research{building, tech}`, `build{role, where}`, `gather{resource, node}`, `attack{attacker, target}`. Selectors name
**roles**, never ids; a rule that cannot resolve is refused into state, not swallowed.

Two things already exist that this design leans on, and neither is AI code:

- **`damage_between(attacks, armours, rules)`** (`lib.rs`) computes damage from the pack's `(class, amount)` pairs.
  A pikeman's row carries `attacks = [[5,15],[8,15],...]` and a knight's `armours` carry the cavalry class - **the counter
  relation is already in the data**, which is why it can be derived instead of authored.
- **`World::vision(player, tile)`** returns never / remembered / watched, and `explored_mask(player)` hands out the grid.
  Per-player knowledge exists; per-player *memory of what was seen* does not.

## The gap, stated as two missing inputs

Countering is not a new kind of AI behaviour. It is the same rule shape with two inputs it does not have:

1. **A counter relation.** "Which of my units beats which of theirs" is arithmetic over the pack: for units `a` and `b`,
   expected damage of `a` on `b` divided by the resource-seconds `a` costs. The engine already has the arithmetic
   (`damage_between`); what is missing is a *query* that folds it over the candidate set the player can actually build.
   Deriving it is also the only version that survives our own content (D17/D18): a hand-written counter table would
   encode AoE2 in a file that is supposed to describe *any* pack.
2. **Knowledge of the enemy.** A rule today can ask anything about *its own* player. To counter, it must ask what its
   units have **seen** of the enemy's army - and nothing more, or the AI cheats by construction.

## The invariant that keeps it honest

**Rules read a player's memory, never the world's ground truth.** The engine grows a `Memory` per player - counts of
enemy units by role, each with the tick it was last seen, updated only from what that player's units and buildings can
watch - and rule conditions are only allowed to read it. That is the anti-cheat: an unscouted army is *unknown*, so the
counter rule cannot fire, and a stale memory produces a wrong choice, which is what a human player's information
advantage is worth.

It deserves the same treatment as `no_float`: a gate. A test that walks the condition enum and fails on any variant that
reaches into the world's true state is cheap, and without it "no cheating" is a promise.

Determinism rules apply unchanged: the memory updates in entity-id order, it is hashed with the rest of state (it
changes what the world will allow, exactly like `last_refusal`), and it never reads a float.

## The three forks

**Fork 1 - where the counter table comes from.**
*Recommendation:* derive it at load from the pack's `attacks`/`armours` (a `CounterTable` in `crates/data`, keyed by
role, honest about `-1`/unknown by refusing). It then covers every unit in the pack including ones we author, and cannot
drift from the damage code. The alternative - a generated file - is a second source of truth; the alternative after
that - hand-written - is AoE2 knowledge in `config/` that stops being true the moment our own units exist.

**Fork 2 - is enemy knowledge a memory or a view.**
*Recommendation:* memory only (above). *What it costs:* the AI must scout, and scouting needs a `scout` action and a
target chooser for unexplored ground, neither of which exists. Without it the counter rules fire only after the armies
meet, which is still a large step up from "train 8 archers and hope".

**Fork 3 - how much the rule language has to say.**
*Recommendation:* conditions first, one word later. A rule that reacts is:

```toml
[[rule]]
player = 2
cooldown_seconds = 10
when = { at_seconds = 300, enemy_share = { role = "knight", at_least = 0.40 } }
do = { train = { building = "barracks", role = "pikeman" } }
```

Explicit, debuggable in the refusal log, and it needs no new resolution in the engine. The symbolic form
(`do = { train = { counter_to = "their most common" } }`) is tempting and should wait: it hides the choice from the
script, which is the one thing a scripted AI is for. If rule files start repeating eight near-identical blocks, that is
the signal, and the table is already there to back it.

## What else falls out of the same two inputs

Every one of these is "more observed state plus more conditions", not new machinery:

| behaviour | new state it reads | what it needs |
|---|---|---|
| scouting | unexplored tiles, last-seen ticks | a `scout` action and a chooser for the nearest unknown region |
| raid response | enemy units near a building, within N tiles | a `near` clause on `enemy_units`, and `garrison`/`defend` actions |
| army thresholds ("attack at 20") | counts by role, summed | `army_at_least = N` (today `has_unit` is a yes/no) |
| retreat and regroup | own units' health share | `health_share_below`, and a `retreat_to` action |
| rebuild after a raid | buildings that vanished | memory of own losses, `no_building` already covers the trigger |
| market play | prices, stock | `sell`/`buy` actions (the dat carries the prices) |
| tech choice | the enemy's armour classes | the same table, read the other way: the upgrade that beats *their* armour |

## How it gets tested, which is the point

The harness is already the right one: a scenario fields a **fixed enemy composition** (ten knights, no AI), the rules
under test are the ones being changed, and the batch runner plays it across seeds. The assertion is a behaviour, not a
number: *the rules switched to the counter role within N seconds of the enemy composition being seen*, and the negative
control *did not switch before it was seen* - which is the anti-cheat test as much as a correctness test.

## Cost

- `Memory` + its update pass: one pass on the frames that change vision, per player, over visible units. The fog pass
  already exists; this is a counter increment per visible enemy, not a second scan.
- `CounterTable`: a load-time fold over the populated units (1,166 in this pack) times their attack pairs - the same
  order of work as the existing scale/stat derivation, once per match.
- Conditions: one new enum variant per condition, one match arm each. The rule engine is already data-driven, so the
  cost is the tests, not the plumbing.

Nothing here touches presentation, pathing or the tick structure, and none of it has to be built before the next
economy or combat slice.
