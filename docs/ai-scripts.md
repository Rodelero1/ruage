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

## How a script is written, and why not `.per` (2026-11-01)

**The question:** would our AI scripts be the same as AoE2's, plus some functionality? **No - the same concepts, a
different surface.** The install's own AI files are `.per`: `(defrule (condition...) (condition...) => (action ARG)...)`,
with facts like `(food-amount < 200)`, `(unit-type-count my-unique-unit >= 10)`, nesting via `(or ...)` / `(and ...)`,
and `(set-goal NAME VALUE)` as the only way to remember anything between rules. Ours is TOML:

```toml
[[rule]]
player = 1
cooldown_seconds = 5.0
at_seconds = 120.0
when = { villagers_at_least = 12, no_tech = "loom" }
do = "research"
building = "town_centre"
tech = "loom"
```

Same three parts - player, when, do - and the same re-evaluation with a cooldown. What differs is what the engine is
allowed to be: our rules are **data**, loaded strictly (`deny_unknown_fields`), evaluated by the engine, hashed, and
refused into `last_refusal` when they cannot fire. A `.per` file is a program for the game's own AI VM, and the VM is
the part we are not reimplementing.

**Why not adopt `.per` itself.** Its value is not the parentheses, it is the fact database behind them: hundreds of
facts (`sn-*` strategic numbers, target points, attack groups, per-unit-type counts) bound to AoE2's data model. D17
keeps AoE2 knowledge out of `crates/`, so a `.per` interpreter would either carry that model in the engine or need an
adapter for every fact - and the adapter is the work. The parser and error reporting are the cheap part; the fact layer
is where the weeks go, and the payoff would be running community scripts, which is a different goal from *our* AI being
readable and deterministic.

**What we take from it, concept by concept:**

| AoE2 | ours today | what to add |
|---|---|---|
| `(defconst NAME n)` | `[role]` / `[tech_role]` name the ids | a `[const]` table for numbers, so a rule never quotes one |
| facts (`food-amount`, `unit-type-count`) | `stock`, `has_unit`, `villagers_at_least` | `enemy_share`, `enemy_at_least`, `army_at_least` (D92/D93) |
| `(set-goal)` - the only memory | **nothing** | **goals**: a named integer slot per player, set by a rule and read by `when.goal`. This is the biggest missing piece: without it a rule cannot remember that it already reacted |
| `(enable-timer)` | `cooldown_seconds`, `at_seconds` | a `once = true` rule (the `disable-self` idiom) |
| `sn-*` strategic numbers | nothing | a `[tuning]` table of named numbers, so "aggression 0.6" is content, not a literal in a rule |
| `(or ...)` / `(and ...)` | implicit all-of | `any_of` / `not` when a script actually needs them - not before, because a boolean language in TOML is where readability dies |
| `(train ...)`, `(build ...)`, `(research ...)` | same | same |
| `(attack-now)`, `(set-attack-group)`, `(scout)` | `attack` (one order) | `scout`, `defend`, `retreat` - each an action over state the memory already holds |

**The recommendation in one line:** keep TOML, add **goals** and **tuning numbers** first (they are what makes a script
feel like a program), then the memory conditions (D93), and only then consider `any_of`/`not`. A `.per` importer stays
on the shelf with the price written above it, so nobody re-opens it casually.

## What the original's scripts actually do, measured (2026-11-01)

The install ships its AI as 39 `.per` files, **919 rules**, 262 KB of script. A census (counting facts and actions,
never copying rule bodies) says what shape the *game's own* AI chose, which is the best available lead for ours:

| what | count | reading |
|---|---|---|
| `goal` | 883 | the goal array is the only memory, and it is used constantly |
| `can-research-with-escrow` / `can-research` | 373 / 123 | research is gated on affordability, not on a plan |
| `unit-type-count-total` / `unit-type-count` | 285 / 219 | army size and per-type counts drive decisions |
| `or` / `not` | 279 / 69 | boolean structure **is** used; our all-of `when` cannot express it |
| `building-type-count-total` / `building-type-count` | 110 / 47 | "do I already have one" is a first-class question |
| `can-build` / `can-train` | 51 / 74 | every build and train rule checks feasibility, not just desire |
| `current-age`, `difficulty` | 77, 85 | age gates rules; difficulty is a *fact*, not a multiplier |
| `set-strategic-number`, `up-modify-sn` | 331, 77 | tuning numbers are the main knob |
| `set-goal`, `disable-self` | 133, 71 | memory and "run once" idioms |
| `release-escrow` | 1,531 | every rule that reserves resources releases them; escrow is pervasive |
| `chat-local-to-self` | 527 | the AI logs to itself - the ancestor of our refusal log |

**Six leads, each a measured fact rather than a preference:**

1. **Civ differences are civ-tagged rules inside shared files.** `(civ-selected ...)` appears **15 times** - e.g. the
   economy's general rules carry a Briton-specific rule - rather than a script set per civilisation. The original picked
   the "tag the exception where it lives" shape, not 31 overlays.
2. **Personalities are separate small files** (`Dr3/personality p4 and 5.per`, `p6 and 8`, `p7`) and they mostly set
   strategic numbers. So personality = tuning table; civilisation = tagged rules.
3. **Difficulty is a fact**, gating rules (85 uses), not a handicap multiplier applied behind the AI's back - which is
   the same reason our AI should not read the world's truth.
4. **"Already have one" is the universal build guard**: `building-type-count-total < 1` (8) and `== 0` (14) before a
   `build`. **A barracks a campaign or scenario provides needs no special case** - the guard asks the question that
   covers it. This is the direct answer to the owner's point about provided buildings.
5. **Escrow is an idiom we do not have**: 1,531 `release-escrow` calls, because the AI reserves resources for the rule
   it is about to run and releases them when it does not. Our rules read the stock and hope nothing else spends it. Not
   urgent, but it is why their rules can be written without ordering guards.
6. **Boolean structure is real**: 279 `or`, 69 `not`. A `when` that is only all-of can express their *rules* only by
   duplicating them, which is the maintenance cost the owner is worried about in a different place.

**What that implies for civ-specific AI (not a decision):** the measured shape is *shared rules + civ tags + a tuning
table per personality*, i.e. my families **B + C**, not 31 overlays and not pure derivation. The variation the owner
described - fewer villagers on stone, more on wood, a different unit composition - is exactly a **tuning table** entry;
the things a table cannot say (Saracens have no knight upgrade) are exactly a **tagged rule** entry. The open question
is only how many of each we are willing to maintain.

## Behaviours asked for, and the primitives each one needs (2026-11-01)

The owner named four behaviours. Measured against the grammar that exists - nine conditions (`villagers_at_least`,
`villagers_at_most`, `no_building`, `has_building`, `age`, `no_tech`, `has_unit`, `housing_at_most`, `stock`) and five
actions (`train`, `research`, `build`, `gather`, `attack`) - they split into "nearly free" and "needs a new kind of
thing":

| behaviour | needs | today |
|---|---|---|
| **Low gold: light cavalry instead of knights** | a stock comparison, an alternative rule, and a flag so it does not flap | `stock` exists; **goals are missing**, so nothing remembers that the switch was made and the two rules can alternate every cooldown |
| **Raid towns and villagers, retreat from a larger force** | target selection by class (villager, building), *local* sighting near the raiders, and a retreat order | **all three missing**: `attack` names a role to attack, memory is a global per-player per-type count with no position, and there is no move/retreat action |
| **Light cavalry onto archers, avoid infantry** | attack priority by class, plus disengage | **missing**: `attack{attacker,target}` cannot say "only archers", and nothing can order a withdrawal |
| **Composition that answers what was seen** | enemy shares from memory | memory exists (D92), the *conditions* that read it do not |

So the resource-aware switch is close to free once goals land; raiding and "hit the archers, avoid the pikes" need two
new kinds of primitive - **spatial awareness** and **movement orders** - which no version of the language has.

## The three decisions this leaves

**A. How a script names a unit.**

| option | what it says | cost |
|---|---|---|
| A1 explicit role | `train { role = "hussar" }`; every rule names a unit, per-civ deltas carry the differences | verbose; nothing to resolve; fully visible (D93) |
| A2 capability filter | `train { class = "cavalry", best = true }` resolved against the tech tree | small resolver; fixes "Saracens have no paladin"; still no judgement |
| A3 class + stated mix + metric | `train { class = "cavalry", against = { infantry = 0.6 }, metric = "damage_per_resource_second" }` | a metric argument; mis-ranks raiders, whose value is speed and pierce armour, not damage per cost |
| A4 intent vocabulary | `train { intent = "raider" }` / `anti_archer` / `anti_cavalry` / `generalist` | a fourth authored table (intent -> class, metric, candidate set); names the owner's light-cavalry examples naturally |

The owner's point that Light Cavalry/Hussar are "best cavalry in a sense" is the stress test: they are best by **intent**
(raid, chase archers), not by class or by damage per cost. A1 and A2 cannot express them as "best"; A3 only with an
intent term bolted on; A4 is the only one that says it directly - at the price of an authored intent table.

**B. Is spatial awareness in scope?** Global counts only (no raiding, no retreat - matches the sim as it stands) or local
sighting per group plus a retreat order (unlocks raiding and kiting; costs a new memory structure and a movement action).

**C. One army or groups?** `attack{attacker,target}` implies a single force. Raiding needs a second group with its own
orders, which is group bookkeeping in the world and in the script.

**Ordering that follows from the dependencies:** C and B are what raiding needs; A is independent. So the cheap path is
A1 (or A2) now, B and C when raiding is actually wanted.

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
