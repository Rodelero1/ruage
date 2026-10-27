# The effect interpreter, priced from the data

Written 2026-10-26, after the resource enum and the unit roles became content (SPEC v0.54).
**Kind 3 landed the same day** (v0.55): research plus "every unit `a` becomes unit `b`". This file
keeps the census that decided the order of work and records what is left - the question it answers
is not "what would be nice" but **what each increment costs and what it buys**, measured against
`config/gamedata/effects.toml` rather than remembered.

## Done: kind 3 (81 of 93 carrying effects are *only* kind 3)

`Command::Research` + a per-player researched set + `[3, from, to]` applied as a health-fraction
preserving swap, with one order per tech in flight at a time (`TechInProgress`). The tech the tests use is Man-at-Arms (tech 222, effect 211, `74 → 75`, 100 food +
40 gold, 40 s), identified by cost and time rather than by id. Not done, and named: prerequisites
(`required`) are loaded and *not* enforced - tech 222 requires tech 101, the Feudal Age, whose own
effect is a kind-102 gating command - a unit already inside a production queue is not upgraded, and
`location` reads as "any building" when it is `<= 0`, which the data leaves ambiguous: 458 techs
carry `-1` and 136 carry `0`, while only 13 name a barracks explicitly.

## The census, re-measured

`config/gamedata/effects.toml` (generated from the user's own dat):

| | |
|---|---|
| effects | **798** |
| commands | **3,306** |
| kind 4 (add) | **1,091** |
| kind 5 (multiply) | **576** |
| kinds 4 + 5 | **1,667 — 50.4% of every command in the game's data** |
| effects made *only* of kinds 4 and 5 | **270** |
| next largest kind | 102 (tech-tree gating), 942 |

Attributes those 1,667 commands touch, most common first: **9 attack (321), 0 hp (220), 8 armour
(200), 1 range (177)**, then 23 (152), 12 (127), 13 gather rate (117), 100 (88), 5 speed (47),
10 (45). The `class` field (`b`) is `-1` — "every class" — in **1,229 of 1,667**, and the target
unit (`a`) is `-1` — "every unit" — in **440**.

## Done next: kinds 4 and 5, for attack and armour (2026-10-26, v0.56)

The importer now decodes the packing, so an effect carries `modifiers` beside its raw `commands`, and
the engine applies **attack and armour** changes where damage is resolved: additions before
multiplications, in researched-tech order, filtered by the unit id or class the modifier names.
Checked against published numbers: Forging (+1 melee attack, blow 4 -> 5), Scale Mail (+1 melee and
+1 pierce armour, blow 4 -> 3), and a class filter that keeps Forging off a class-4 villager.

Done since, in a second slice (v0.57): **hit points, speed, gather rate and carry**.

Two rules were needed, and both were decisions rather than code:

- **Health and speed are state, so a research grants them to units that already exist.** A villager at
  25 health becomes 40 when Loom lands (and one wounded to 10 becomes 25 — the grant is health, not a
  reset). Masonry is the same shape: a house at 550 becomes 605, one at 100 becomes 155.
- **Gather rate and carry capacity are resolved when an order lands.** A load already in hand keeps
  the numbers it started with, so Wheelbarrow's +25% carry changes what the *next* trip carries and
  not the load being carried now. This is why the interpreter needed no re-pricing pass over
  in-flight tasks.

The maximum is not stored per entity: it is derived (`max_hp(player, unit)` = the table's value with
the researching player's modifiers), which keeps `Entity` unchanged — the previous slice learned what
a wider `Entity` costs.

Two defects the review of this slice found, and what is left of them:

- **Speed and gather-rate additions were applied in the wrong unit** (fixed, tested). The data has nine
  speed additions and two gather-rate additions in tiles/work per second, while the engine holds those
  as fixed-point numbers at `scale`; the code added the raw value, so a +0.35 tiles/s modifier moved the
  unit by 0.35 scale units. `scale_fixed` now scales an addition before adding it, and
  `a_speed_addition_arrives_in_tiles_per_second_and_not_in_scale_units` pins it (a free, instant tech
  that gives a 1.2-tile scout +0.35, so the speed must read 31000 at scale 20000).
- **A build site was quoted the table's health, not the player's** (fixed **and now tested**). A house
  ordered after a health tech finished at 550 rather than 605, because `check_build` copied `stats.hp`.
  Both halves are closed, and the tests exist because the test debt was paid by building the content the
  test needed rather than by lowering the claim: a purpose-built pair of tables (a free, 1-second tech
  that doubles building health) removes the race against the pack's 50-second Masonry, and the scenario
  runner supplies the legal build site the seeded fixture would not.
  - `a_house_ordered_after_a_health_tech_is_quoted_the_new_maximum` - the order-time quote.
  - `a_house_finished_after_a_health_tech_reaches_the_new_maximum` - and the same defect one layer
    down: a site already going up when the tech lands used to finish at the maximum it was quoted,
    because `Construction` stores that number and nothing refreshed it. `finish_research` now updates
    `construction.max_hp` alongside the entity's health.

**`[[train.entry]]` stopped quoting ids** in the same pass: `building = "town_centre"`,
`units = ["villager"]`, resolved through `[role]` at load, with an unknown role refused by name and an
integer refused outright — accepting both spellings is how the numbering creeps back in. A role the
table lacks is an error the author sees at load, not a building that quietly makes the wrong unit.

## Done next: tech-tree gating, and the age-up identity defect it uncovered (2026-10-27, v0.60)

**Prerequisites are enforced.** Every id a tech lists in `required` is a *tech* id - all 614 entries
across the table resolve to techs, even where the number also names a building (tech 50 is Masonry, unit
50 is a farm) - so the gate is "has this player researched it", refused as `MissingPrerequisite` and
named with the pack's `[tech_role]` word when it has one. `World::grant_tech` completes a research
instantly for a world that begins later in the game, and it goes through `finish_research` so a granted
tech applies exactly like a paid one.

**That gate could not be tested without confronting an age-up.** The Feudal Age effect carries 14 kind-3
commands that *replace building ids*: town centre 109 -> 71, barracks 12 -> 498, blacksmith 103 -> 105,
house 70 -> 463. The engine applied them literally, so after an age-up:

- villagers could no longer deposit anything (the drop list names 109, the town centre was 71),
- Forging was refused at a "not a blacksmith" (the blacksmith was 105),
- `[train.entry]` no longer matched a barracks (12 vs 498).

Three separate breakages from one mechanism, all in a real match, none in any test because no test had
granted an age. The fix is a **family**: `ruage-tools` derives `variant_of` from the kind-3 pairs (190
units have a parent; the dat's own `base_id` points at itself and is useless), `Units::root` walks it,
and the four places that name a building - drop sites, research location, training, and the scenario's
role search - compare families rather than ids. Mutation-proved: restoring `unit == wanted` in
`is_family` fails `an_age_up_does_not_take_the_blacksmith_away`.

**What is still not modelled, and is the natural next increment**: a match does not *start* with the
Dark Age tech granted - a scenario's first step has to research it (it is free, instant, and legal at
any finished building), and the tests do it with `grant_tech`. Content should say `[start] techs =
["dark_age"]` and seeding should grant it, which is the same place kind-6 tech-tree effects (what a
building may train, per age) will need to land.

## Done next: a match opens with its starting techs (2026-10-27, v0.61)

`[start] techs = ["dark_age"]` and `[tech_role] dark_age = 104`, granted by the engine when the world
is seeded *or* when the research tables are loaded - either order, because a headless runner and a
scenario runner call them the other way round from each other. The point is not bookkeeping: every tech
in the table lists 104 as a prerequisite, so the previous state of the world ("nothing researched") was
one in which **research was impossible**. Tests had been granting it by hand, which is exactly the
smell: a test doing what the game should.

Two fixtures had to learn the same thing, and their failure was informative - a purpose-built tech
table with no 104 made `load_research` fail with `MissingTech { id: 104 }`, which is the right answer:
content that says "start with dark_age" and tables that cannot supply it are inconsistent, and the
loader says so rather than starting a match that silently cannot research.

**Still missing from "what a match begins with": resources.** Every player starts with nothing, so the
ten-minute script has to gather before it can build and cannot afford a 50-gold tech at all. `[start]
stock = { wood = 200, food = 200, gold = 100, stone = 200 }` (the standard opening) is the same shape of
content, but it will move every test that asserts "the player starts broke", so it wants its own step.

## Done next: the standard opening stockpile (2026-10-27, v0.62)

`[start] stock = { food = 200, wood = 200, gold = 100, stone = 200 }`, resolved through the pack's own
column names and applied once. With it, the project scenario can research Loom at t=0 (50 gold) *and*
still afford its spearman, so **the ten-minute script now demonstrates the interpreter end to end**
instead of demonstrating a queue.

The churn was the interesting part, because every test that said "the player starts broke" was
asserting a *content* fact as an engine rule:

- three gather tests now count from `opening` rather than from zero;
- `a_build_without_the_resources_is_refused` and `research_is_refused_without_the_money` ask for a pack
  with `start.stock` cleared, which is how content says "this match opens broke";
- the two own-pack tests (five own resources) had to clear the project's stock too, and their failure
  was the mechanism working: `UnknownResourceName { name: "food" }`, because a grain/iron/salt pack
  cannot start with 200 food;
- the build-time test lost an assertion that measured wall-clock ticks between "arrival" (inferred from
  position) and the finish, which moved by one because the villager no longer walks to a node first. It
  now asserts the rule - the house needs exactly 500 **builder-ticks**, the last of which is the one
  that finishes it - and says why the wall-clock version was a fixture artefact.

What is left of "what a match begins with" is nothing; the next increment is the **tech tree proper**:
kind-6/kind-101/102 commands decide what a building may train *per age*, which is the other half of what
`[train.entry]` means today (it says a barracks trains spearmen, and nothing says a Feudal barracks
cannot train something else).

## Done next: the civ tech tree as a restriction list (2026-10-27, v0.63)

The dat gives every civilisation a "technology tree" effect whose commands are all kind 102 with nothing
but an id (`[102, -1, -1, -1, 428]`) — no state field, so the file does not say whether an entry permits
or forbids. That ambiguity was the last fork worth asking about, and the data answered it (D23):

- **no civ's list contains anything every civ has** — Loom, the ages, Wheelbarrow and Forging appear in
  **0 of 32** lists, which a permission list could not survive;
- **Britain's 31 entries are its real omissions** — the Hussar (428), the Eagle line (384, 434), the
  Siege Onager (320), the Paladin (265).

So a civ's tree is a restriction list. `civs.toml` gains a loader; `[start] civ = "British"` names the
civilisation by the table's own word; `check_research` refuses a barred tech **before** the prerequisites
(a tech you can never have should say so, not ask for an age) with a refusal that names the civ; and the
civ id is hashed, because it changes what the world will allow. Mutation-proved: disabling `barred_by`
fails `a_barred_tech_is_refused_by_the_civilisation` with the prerequisite message instead.

What that leaves, in the order it now makes sense:

- **unit availability (kind 101 and 6)** — a civ's tree also governs which units and buildings exist for
  it, and those commands name an id that is *sometimes a unit and sometimes a tech* (U16), so each needs
  the same evidence treatment the tree effect just had;
- **per-player civs** — one civ for the match is honest but crude; `[start] civ` should become a list;
- **team bonuses** (`team_bonus_effect`, carried and unapplied) and the 211-number per-civ `resources`
  array, both of which need names before they mean anything.

Still not interpreted, and counted: **range (177 commands)**, plus the `other:`/`unpacked:`
attributes the importer could not name and the effect kinds beyond 3/4/5. Range waits for a reason to
exist: resolution is melee-only, so a range modifier would be code no test could distinguish from a
comment. After that: `required` gating (a tech that needs its prerequisite), and the tech-tree effects
(kind 6 enables/disables) that decide what a building may train.

## Why this is the right next increment and not a bigger one

- **Half the data, two verbs.** Kinds 4 and 5 are "add" and "multiply", and both were pinned by
  published results, not by intuition: Masonry is `[5, -1, 3, 0, 1.1]` (buildings gain 1.1× hp;
  an *addition* of 1.1 hp would be a no-op), Loom is `[4, -1, 4, 0, 15]` (+15 hp to class 4
  civilians). 270 effects contain nothing else, so they are fully unlocked by this one interpreter.
- **The engine can already receive them.** Attack and armour are resolved **when an order lands**
  (`check_attack`), not per tick, and hp is plain state. A per-player modifier table therefore
  changes what a *future* swing resolves to and what an hp total is - no new per-entity fields, and
  the modifier set belongs in the state hash beside `stock`.
- **It does not need the other 11 kinds.** Kind 102 (942 commands) is civ tech-tree gating: which
  techs a civ may research. That is a *permission* question, and it can stay unimplemented while a
  scenario researches by naming the tech directly.

## What the increment actually is

1. `Command::Research { building, tech }` — refuse in a fixed order (not a building, not finished,
   already researched, cannot afford), charge, and start a tick timer from the tech's own
   `research_time`, mirroring `Command::Train`.
2. A per-player `BTreeSet<u32>` of researched techs, hashed; upgrading a unit is *not* a state
   mutation, it is a resolution-time fact.
3. The interpreter: for each command in the tech's effect, apply add or multiply to (hp, attack,
   armour, range) for a target unit, a target class, or everything.
4. The external check: **Loom gives a villager 40 hp and Masonry gives a house 605** (550 × 1.1),
   then a spearman-vs-villager swing that lands where the published numbers say it does.

## What it explicitly does not settle

- Kind 3 (unit upgrade, 227) — a different mechanism (replace the unit id).
- Kind 2 (117) and kind 0 (106) — unresolved meanings, named in `docs/mechanics/effects.md` rather
  than guessed.
- Kind 101 (97) — U16: 13 of its ids are techs, not units, so it cannot be typed by the id.
- Anything needing a renderer.

## What the refactor before it bought

The two previous steps (resources and unit roles as content) were not preparation for its own sake:
the finish line `a_content_pack_with_our_own_resources_loads_and_spends` is a *green test* that an
original pack can price and build, the tick's cost is unchanged within noise (2.26-2.36 µs/tick at
126 entities, 27-28 ms per ten-minute match), and the four-hour run still reproduces its hash. The
effect interpreter inherits that shape directly: a tech's cost is a `Vec<Cost>` that already names
its resources, and a scenario researches a tech by **role**, both of which would have been dat ids
two commits ago.
