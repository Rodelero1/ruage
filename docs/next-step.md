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

## Done next: unit availability (2026-10-27, v0.64)

Kind 2 with a positive flag is "make this unit available", and the effect names say so themselves
("Barracks (make avail)", "Castle -- Age Three", "Trebuchet"). 109 units are gated that way by 81 techs
that do nothing else, cost nothing, take no time and have no home building - so the engine treats them as
bookkeeping and applies them as their prerequisites are met (D24). The consequences are visible rather
than theoretical:

- a **castle** cannot be built before the Castle Age, and can the tick after it (tech 137);
- a **trebuchet** cannot be trained before the Imperial Age (tech 256) - trained, not built, so
  `check_train` carries the same rule as `check_build`;
- a **house**, a **villager** and a **spearman** are unaffected, because no tech gates them.

The refusal is `UnitUnavailable`, checked **before** the price: "you cannot have this yet" outranks "you
are poor". Counted from the data rather than guessed: 109 gated units, 81 bookkeeping techs, exactly one
with no requirements (332, the outpost).

Next, in order: the **`b = 0/-1` kind-2 rows** (the Huns' house and four "enable animal" rows) whose
direction is not established - they are the last thing keeping a civ's *negative* availability out of the
model; then **per-player civs**, which is what would let two players be different civilisations and stop
`[start] civ` being a single name; then the **team bonuses** and the per-civ tuning array.

## Done next: the spear line, and the wrong id that was hiding in it (2026-10-27, v0.65)

Your question - spearmen free with the Feudal Age, pikemen and halberdiers researched - is how the data
works, and checking it found our content naming the **militia** as a spearman (D25: 74 is 40 hp,
60 food + 20 gold, and every infantry upgrade starts from it). The line is **93 -> 358 -> 359**: tech 87
"Pike" makes 93 available (free, instant, requires Feudal), 197 upgrades 93 -> 358 at the Castle Age, 429
upgrades both 93 -> 359 and 358 -> 359 at the Imperial Age.

So content now says `militia = 74`, `spearman = 93`, `pikeman = 358`, `halberdier = 359`, and the barracks
trains **militia and spearmen**. Two engine rules made that honest rather than decorative:

- **An upgrade target is not available, it is an upgrade.** `check_train` refuses a unit whose upgrade
  tech is not researched, so a barracks cannot train halberdiers just because the entry lists the line.
- **The line counts for the entry check.** A pikeman matches a `[train.entry]` that names spearmen,
  through the family root - which is the same machinery the buildings use.

The ten-minute scenario now trains **militia**, which is what a Dark Age barracks can make; it does not
age up, and pretending otherwise would have meant hiding the rule it is meant to exercise.

What is left, in order: the `b = 0/-1` kind-2 rows (the Huns' house and four "enable animal" rows),
per-player civs, team bonuses, and the per-civ tuning array. One more thing this step taught: **the
dat's name codes are not identity** - the spear line is evidence for that, and any future role should be
chosen from stats and the upgrade graph rather than from the name.

## Done next: a civ that takes a unit away (2026-10-27, v0.66)

`[2, unit, 0, -1, 0]` is the disable form, and there is exactly one in the table: the Huns' tree takes away
the **house**, which is true of the game. The bar is a civ-level rule that no tech lifts — `UnitBarred`,
naming the civ — and it is **derived on demand** rather than cached, because a cached set would have been
one more field on `World` and pass seven priced that shape.

The four `b = -1` rows are a different animal and are now named as such (D24a): effects called "Enable
sheep/llama/cow/turkey", carried by techs that cost 40 food + 25 wood and take 15 s, with an attribute and
a value in them. Nothing about that reads as enable/disable, so they stay uninterpreted rather than
guessed at.

**The measurement finding outranks the feature (pass thirteen).** The A/B claimed +6% for a change with no
tick path. Chasing it showed the probe has a between-build sensitivity of roughly ±8% at 126 entities, and
that every "~2% residual" since pass seven was that sensitivity — partly because the base worktree ran with
a `units.toml` stripped of `variant_of`. Future passes: compare against the *previous commit* with the
**same** data files, and treat a sub-10% difference in the idle probe as unresolved unless the change runs
per tick.

Next: per-player civs (which is what would let two players be different civilisations), then team bonuses
and the per-civ tuning array.

## Done next: per-player civilisations (2026-10-27, v0.67)

`[start] civs = ["British", "Huns"]` - entry *n* is player *n+1*, the last entry covers any further
player, and a one-entry list still means "everyone plays this one". The two bar lookups now take a player,
and the ids are hashed **per player**, so a replay cannot lose whose civ was which.

The project's own pack naming two civs is the point rather than a flourish: it makes the bar rules a live
case. Player 1 is refused the Hussar (428) and builds houses; player 2 is allowed the tech and cannot
build a house. One test asserts both halves in one world, and it matches on the error *variant*
(`TechBarred` versus `MissingPrerequisite`) rather than on a message, because 428 has a prerequisite chain
that would otherwise make the assertion about the wrong rule.

Next: team bonuses (`team_bonus_effect`, carried and unapplied), then the per-civ 211-number tuning array,
then the remaining effect kinds (0 and 1 modify a named unit's attributes and costs).

## Done next: teams, and a queue that had to be re-ordered (2026-10-27, v0.68)

`[start] teams = [1, 1]`, empty meaning a free-for-all, and `AlliedTarget` refusing an attack on a
teammate - the general case of `FriendlyFire`, which was always a team rule with one player in it. Teams
are hashed only when content names them, so a world without teams hashes exactly as before.

**Team bonuses were the announced next step and were deliberately not taken (D26).** Every sampled
`team_bonus_effect` targets a mechanic this engine lacks: British and Goths modify a building's *work
rate*, French line of sight, Slavs garrison capacity, Teutons a farm resource. Applying them would produce
a step whose tests cannot fail - invisible behaviour - so the effect ids stay carried and unapplied until
the mechanic exists.

That also retired the plan after it: kinds 0 and 1 are the same story. Kind 0's attributes are 19/20/21
(garrison, projectiles, and four hundred rows of it), kind 1's first field is a *resource* id - the farm
food line (Horse Collar 75, Heavy Plow 125, Crop Rotation 175) - and the sim has **zero** mentions of a
farm. The `[farm]` block in `economy.toml` has been parsed and unused for a long time.

So the next honest step is the one this exposed: **farms**, because `[farm]` is dead content, because the
farm upgrade line is the first kind-1 effect with somewhere to land, and because a gatherable building is
the first entity that is neither a resource node nor a producer.

## Done next: farms (2026-10-27, v0.69)

`[farm]` is read at last: `unit = "farm"` and `resource = "food"` say which building yields what, and that
is the extent of the engine's farming knowledge. `gather_source` is the one authority on "what may be
worked here", used by the validator, the order and the trip home; a farm holds `food_per_farm` from the
moment its plot is marked out and cannot be worked until it is finished.

**Three defects fell out, all about where a worker stands.** Arrival was `position == node.position`
(never true for a blocked building tile); the builder walked *onto* its site and finished the job standing
inside the building, from which no route home can be planned; and `return_task` read the stockpile column
off the node's *kind*, so a farm gave no trip home at all and a villager filled its load and then stood
there for the rest of the match. Now: `work_position` picks the neighbouring tile that is free **and
reachable** (a free tile inside a wall cost a scenario step), both ticks accept adjacency, and the column
comes from `gather_source`.

Because positions changed, the ten-minute scenario's `nth` tree choices needed to become
reachability-aware: `nearest_resource` now filters on `World::can_reach`, so the script picks a tree it can
walk to instead of ordering a trip the engine refuses.

Next: farms make the kind-1 farm food line (Horse Collar 75, Heavy Plow 125, Crop Rotation 175)
implementable, so that is the first thing `[farm] depletion` should learn.

## Done next: the farm food line (2026-10-27, v0.70)

Kind 1 is `[1, 36, 1, -1, 75]`: a change to a *resource*, and the earlier reading of that field as a unit id
was wrong. The importer now decodes kind 1 into `resource_modifiers` (its own namespace, next to kind 4/5's
unit attributes), and content names the id: `[farm] food_resource = 36`. The numbers settle the reading -
75/125/175 are the published Horse Collar/Heavy Plow/Crop Rotation bonuses, all additions to resource 36.

A tech that lands adds its food to every plot the player has that still holds some, and every plot built
afterwards starts at the new total; a spent plot stays spent. Two tests pin it (an existing farm 250 -> 325,
a new farm at 325) and a third pins the spent case.

Next: the remaining kind-1 resources are the same shape and mostly need mechanics we lack (Guilds, Banking,
the market), so the honest next step is **unit availability by age** - the tech tree's kinds 6/101/102 -
which is what `[train.entry]` is missing and what the farm gate (tech 216, "Farms (make avail)") already
demonstrates: a free, instant tech whose only job is to make a building available.

## Done next: the tech tree, and why it was unreachable (2026-10-27, v0.71)

Written as tests first, because the machinery looked complete: availability (`unit_available`, kind-2
`b = 1`), upgrades (kind 3), prerequisites, civ bars. Four of the six tests passed immediately - a Dark Age
barracks is buildable, a Dark Age archery range is not, a barracks cannot train a spearman before Feudal,
and it can afterwards.

The fifth failed, and it was not a test bug: **the archery range was unobtainable even in Feudal.** Tech 147
requires 667, which requires 122 "Shadow Barracks -- Age Two" - an effect with no commands - and 638, which
has no effect at all. The dat spells "a barracks exists" as a tech, and nothing ever granted it. The same
mechanism is why every earlier test granted 659 by hand before researching an age: the Feudal Age needs a
*shadow town centre*.

Fixed by extending `availability_techs` to effects that do nothing, not only effects that enable units. The
Feudal Age is now reachable by playing - 500 food at the town centre, no hand-granting - and the sixth test
proves exactly that. Mutation-proved by turning shadow bookkeeping off: the two Feudal-dependent tests fail
and the four Dark Age ones correctly still pass.

**Known simplification (D29):** the shadows are satisfied up front, so an archery range needs no barracks
standing. The building's name is in a display string, not in data the engine should parse.

Next: `[train.entry]` per age is now redundant with availability, so the better next step is **presence** -
a shadow tech that means "you have a barracks" should be granted when the player builds one - which is also
the hook the known simplification replaces.

## Done next: the count in the prerequisites (2026-10-27, v0.72)

The dat's tech carries `min_required_techs` beside `required_tech_ids`, and `ruage-tools` had been
exporting only the ids - so "at least two of these five" was read as "all five" everywhere. Nineteen techs
have `min < count`, and both copies of the gate (the order path and the availability resolver) now count.

The importer clamps a count of zero, or one longer than the list, to "all of them", which is exactly what a
pack written before the field existed meant - so no old pack changes meaning.

**A correction went into D29 as well.** It claimed that an archery range becoming buildable without a
barracks was wrong against the game. It is not: the archery range has no building prerequisite in AoE2, and
the dat says so with an OR-set that contains an always-true marker (638).

**A test that proved nothing, caught by mutation.** The first version of the counting test used the Hand
Cannon tech and passed even with the "all" rule restored, because 285 is granted for free once its own
prerequisites are met. The real test uses a pack of our own - one tech for the order path, one for the
resolver - since no tech in this dat can reach the order path with a count.

## Done next: housing (2026-10-27, v0.73)

`[population]` was the last parsed-and-unused table, and a house was 25 wood and a shape on the map. Now
population is one per live unit, the ceiling is `min(cap_default, finished houses x 5 + finished town
centres x 5)`, and a train order past it is refused with `PopulationCapped` before it is priced.

`town_centre_population = 5` is new content and published fact (a house and a town centre both support
five; a castle's twenty is not modelled). The Huns needed a rule rather than a formula - their tree bars
the house, so a cap that counted houses would pin them at zero - and the exemption is read from the same
bar the engine already checks, giving them `cap_default`.

Three mutations, three precise failures: no check at all fails four tests, counting unfinished houses fails
only the construction test, dropping the Huns exemption fails only the Huns test.

Next: **ranged combat** - `range` is attribute 1 and 177 commands, and every unit is melee today. It is the
largest mechanic left and it touches the tick loop, so it gets the full review budget.

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
