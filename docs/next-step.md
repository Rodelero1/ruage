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

## Done next: ranged combat (2026-10-27, v0.74)

`range` - attribute 1, above zero on 277 of 1,166 units - is real. A unit with reach strikes from anywhere
inside it and walks only until it is inside; a unit with none keeps the adjacency rule it always had
(diagonals included). One comparison on squared distance in engine units, so no square root and no
trigonometry in the tick. Range rides on the task, resolved at order time like damage and reload.

That made the archer upgrade line visible for the first time: Fletching, Bodkin Arrow and Bracer are
`range +1` on class 0, and the test is behavioural - an archer five tiles out walks a tile without
Fletching and stands still with it.

Mutation-proved twice: forcing the melee branch fails all three ranged tests, and dropping the range
modifiers fails exactly the Fletching one.

**Still not modelled, and now named:** projectiles (a shot is instant damage at range, not an entity), line
of sight (a unit shoots anything in reach it can path to, walls included), and buildings that shoot (the
town centre's arrows need garrisoning).

Next, in the order I would take them: **training queue** (one slot per building today, and a crowd stalls
even that), **projectiles as entities** if the visual/gameplay difference matters, then **kind 0** - which
is per-unit attribute changes and needs garrison and LOS to land.

## Done next: the training queue (2026-10-27, v0.75)

`[train] queue_length = 5` is content now, and one slot per building is gone. The head of the queue stays on
`Entity.training` - so the tick loop keeps its fast path and `Entity` does not grow - and the rest waits in
`World.queued`, a side table by building, the same shape as `paths` and `Units::variants`.

An order is paid when it is placed; a finished unit hands the building to the next, which starts counting
from the create time it was priced at; and the population gate counts committed units, so a queue cannot
slip past housing.

**The finding worth carrying forward:** with `command_apply = "next_tick"`, `submit_checked` validates
against state where the previous orders have not landed, so five orders in one tick all saw an empty queue
and all passed. The checks now count pending `Train` commands as well. Immediate-apply mode would have
hidden this completely, which is an argument for keeping the suite on `next_tick`.

Mutation run also caught a weak test of my own: the population-cap-queue test placed two orders that were
still pending, so it passed with the committed count deleted. It steps once now.

Next: **projectiles as entities** (a shot is instant damage at range today), or **kind 0** once garrison and
line of sight exist, or **per-civ tuning** (the 211-number array, still unread).

## Corrected: the ceiling pauses, it does not refuse (2026-10-27, v0.76)

The housing step shipped the ceiling as a refusal. It is not one: a queue may hold more than the player can
support, and when the ceiling is reached **training pauses** - the job keeps its remaining ticks and waits
for a unit to die or a house to finish. `check_train` no longer looks at population, `PopulationCapped` is
gone, and the production pass asks once per player with a building in production whether they are full.

The price of the wrong rule was the tests: five of them asserted the refusal and all five had to be
rewritten. And one of the new ones - "pauses the clock" - still passes with the pause removed, because a
crowd of villagers around a town centre gives a finished unit nowhere to stand; the mutation is caught by
the queue-drain test and by the house and death tests instead.

Next: **projectiles as entities**, which was the instruction this correction interrupted.

## Done next: projectiles as entities (2026-10-27, v0.77)

A shot is an entity now: it flies at `[combat] projectile_speed`, carries the damage resolved when the order
was given, and lands where it was **aimed** rather than on the unit it was aimed at - so an arrow whose
target dies on the way finds nothing and does not retarget. It costs no population and cannot be attacked
(`NotATarget`). Speed zero keeps the old instant behaviour, so old packs are unaffected.

Two shapes of the tick dictated the code, and both were found by breaking them: the entity list is
index-stable while positions are applied (so shots are recorded during the strike pass and created after
movement), and `step` already had a `spent` vector for consumed waypoints (so the arrow list had to be
named something else, or arrows would never be removed).

Mutation-proved: forcing shots back to instant fails both projectile tests.

**Known and recorded:** projectile ids are "one past the highest", an O(n) scan per shot - consistent with
how trained units are spawned, and the first thing to fix if arrow-heavy fights ever matter in a profile.

Next: **garrison and line of sight**, which is what kind 0 needs before any of its 106 commands can mean
anything, or **the per-civ 211-number array**, still unread.

## Done next: garrison, and buildings that fight (2026-10-27, v0.78)

`garrison` and the buildings' own `attack`/`range` were both parsed and unused. Now a unit goes inside
(`Command::Garrison`) with the building's own capacity as the limit, must be standing on it or beside it, and
cannot act or be attacked while inside; ungarrisoning lets it out where it stands; and **a garrison dies with
its shelter** rather than pouring out of a razed town centre. A town centre shoots because a building is an
armed entity that cannot walk: reach from `range`, damage from its attack array, shot as a projectile.

Two details worth keeping: `start_attack` skips pathing for a building (pathing from a blocked tile refused
every shot), and that guard must test the *kind*, not the speed - a stationary unit in a test still wants its
route, which one existing test proved by breaking.

Mutation-proved three ways, and one of them found a hole: removing the capacity check fails the full-building
test, letting a garrison survive its shelter fails the razing test, and letting garrisoned units be attacked
was **not** caught by anything until a test was added for it.

**Line of sight stays out, with the reason written down:** 844 units carry `los`, but nothing asks - no fog,
no scouting, no untargetable-when-unseen rule. It waits for a consumer.

Next: **kind 0** now has garrison capacity to act on (attribute 21), or **the per-civ 211-number array**,
still unread.

## Corrected: ejection at 20%, and volleys that count (2026-10-27, v0.79)

D36 got two rules wrong, and both are now the published ones.

**A garrison is ejected, not killed**, when its building falls to 20% of its hit points or below - unharmed,
where it stands - and the building takes nobody in until repaired above that. Content:
`[combat] garrison_eject_fraction = 0.2`, checked where damage lands rather than in a per-tick scan.

**A building's extra arrows are dps arithmetic:** `floor(sum of garrisoned units' dps / building dps)`, where
dps is the **pierce** attack over the reload, divided once at the end. The qualifier is the damage class, and
our own data settles it - a Throwing Axeman (281) and a Gbeto (1013) are ranged with *melee* damage and add
nothing, while archers, cavalry archers and hand cannoneers add.

**Not modelled, and named:** each extra arrow should carry the garrisoned unit's attack (only the first is the
building's), and a garrisoned villager should count as a fixed 2.5 dps. Today every arrow in a volley carries
the building's damage and villagers contribute nothing.

Mutation-proved: removing the ejection fails the ejection test, forcing one arrow fails the archer test, and
counting melee damage as pierce fails the axeman test. That last one also showed the axeman test was passing
for a different reason than its name suggested.

## Corrected again: production buildings do not take outsiders (2026-10-27, v0.80)

A second correction to D36, from the same source. The barracks, archery range, stable, siege workshop and dock
hold **only what they have just trained** (by rallying to themselves) and never take a unit back in; town
centres, towers and castles take units from outside. The data draws the line itself - those five carry a
garrison capacity of ten and **no attack array**, the three shelters carry both - so the rule is "a building
that can fight takes outsiders in" rather than a hard-coded list of ids.

`CannotGarrisonHere` says so. **Not modelled, and named:** the other half (a trained unit kept inside by
rallying to its own building) needs rally points, which this engine does not have - so today a barracks holds
nobody, which is right for a production building.

**Also in this commit:** the ejection and volley corrections above, and the correction to the Throwing Axeman
test that had been passing off the pierce filter rather than the range check.

Next: **rally points** (which is what makes production-building garrison usable, and what a scenario wants
anyway), then kind 0.

## Done next: rally points (2026-10-27, v0.81)

`Command::Rally` with two behaviours: `Some(tile)` sends the finished unit there (`Task::Move`), `None` clears
it, and `Some` of the *building's own tile* keeps the unit **inside** - which is the half of the
production-building rule D38 had to leave out. `RallyPoint::Inside` is derived from clicking the building, so no
second command is needed, and the state lives in a side table (`World.rallies`) that is hashed.

Off-map rallies are refused (`OffMap`); an *unreachable* rally is not, and the unit simply stands where it
appeared - recorded in D39 as a gap rather than assumed.

Mutation-proved both ways: ignoring the inside arm fails the "stays inside" test, ignoring the tile arm fails the
"walks there" test.

Next: **kind 0's 106 commands** (garrison capacity is now a real attribute to modify), or the **per-civ
211-number array**, still unread.

## Corrected: rallies that gather, and rallies you cannot reach (2026-10-27, v0.82)

Two corrections to D39, both from the user.

**Unreachable is not a refusal:** a rally into a walled pocket now sends the unit as close as the map allows -
the tile if there is a route, otherwise the first reachable tile on each ring around it, ten rings out - instead
of leaving it at home. **A rally onto a resource is a gathering order:** the node is chosen by the tile, through
the ordinary gather path, so a villager rallied onto stone with wood on three sides gathers the stone (proven on
a purpose-built map, including that the stone's amount falls) and never substitutes the nearest tree.

Units that cannot work the node, and nodes that cannot be reached, fall back to walking as close as possible.

Mutation-proved: making the rally never gather fails the stone test, and removing the ring search fails the
pocket test.

## Done next: kind 0, and population (2026-10-27, v0.83)

Kind 0 is the third namespace of the effect command (`set` against kind 4's `add` and kind 5's `multiply`),
106 commands, now decoded. **Population (attribute 21) is consumed**: a building supports what content gives its
role unless a `set` modifier replaces it, and the ceiling sums that per finished building - so the Slavs' team
bonus (military buildings support five) works, proven against a British control that gains nothing.

The other named attributes are decoded and **not** consumed, each because the mechanic does not exist yet:
**accuracy 11** (Thumb Ring), **ballistics 19** (leading a shot), **minimum range 20** (Andean Sling), **extra
projectiles 102** (Yasama - a tech-granted volley, unlike the garrison's). Two attributes stay unnamed
(`other:22`, `other:105`) because nothing in the data pins them.

Next: **ballistics**, the one of those four with a mechanism already in place (projectiles fly to an aim
position, so leading a moving target is a change to where the aim is, not new machinery).

## Done next: ballistics (2026-10-27, v0.84)

Two halves, because either alone is nothing: an arrow **lands where it was aimed** and hurts the target only if
the target is still within half a tile of that point (before this, every shot hit), and **Ballistics** (attribute
19, set by a tech) aims at the intercept - the time of flight iterated three times along the target's motion,
in integer arithmetic including an integer square root.

The target's motion is read from the step its task would take this tick, because `Entity.velocity` is a
spawn-time field that movement never updates.

Mutation-proved: removing the landing check makes a runner take arrows without Ballistics; disabling the lead
makes the Ballistics case miss. The anchors do not move, and **do not cover this path** - nothing in the scripted
scenario fires an arrow.

Next candidates: **accuracy 11** (the missing chance-to-hit roll, which is what a missed arrow's randomness would
need), **minimum range 20** (a siege engine that cannot shoot what is next to it), or the **per-civ 211-number
array**, still the largest unread thing in the dat.

## Done next: accuracy (2026-10-28, v0.85)

Attribute 11 is consumed. **Accuracy is the chance a shot is aimed true**: the base comes from the dat (archer
80, hand cannoneer 65, every building that shoots 100), a tech can set it (Thumb Ring 100), the roll is a hash
of the tick and the shooter's id so replays agree, and a failed roll moves the aim point `accuracy_scatter`
(2 tiles, authored) in one of eight directions. The landing rule from the previous step turns that into a miss,
so accuracy, ballistics and the landing rule compose without any of them knowing about the others.

Mutation-proved: never consulting accuracy fails the test, and making every roll pass fails it too. Two fixtures
pin the scatter off where the archers shoot at immobile targets - a stated fixture choice, not a gap in coverage.

Next candidates: **minimum range 20** (a siege engine that cannot shoot what stands next to it), or the
**per-civ 211-number array**, still the largest unread thing in the dat.

## Done next: minimum range (2026-10-28, v0.86)

`min_range` is imported (it was in the dat all along, beside `max_range`) and consumed: **a unit cannot strike
inside its own dead zone**, and **the attacker backs away** until the shot is legal. 46 of 1,166 units have a
dead zone - mangonel 3, bombard cannon 5, trebuchet 4, dock 2 - so the rule is data, not a siege list; an archer's
is 0 and fires from where a mangonel cannot. Attribute 20 (`set`, Andean Sling) moves it.

The number is resolved at order time like damage, reload and range, and it is hashed.

Mutation-proved: ignoring the dead zone makes the mangonel fire from inside it. Not modelled: blast damage, and
the retreat does not check terrain - a mangonel backed into a wall keeps trying to step into it.

Next: **blast damage** would complete the siege engine (a shot that hurts a group), or the **per-civ 211-number
array**, still the largest unread thing in the dat.

## Corrected: `set` modifiers, the skirmisher's dead zone (2026-10-28, v0.87)

A review found two real defects behind D44's claim, both fixed test-first (D44a):

- **`set` modifiers were dropped** by `scale_fixed`, so Andean Sling (seven `set 0`) and the Koreans' team bonus
  (three `set 1`, mangonel/onager/siege onager 3 -> 1) did nothing. Population and accuracy had handled `Set`
  themselves, which is exactly why the gap went unseen.
- **A one-tile dead zone could never bite** on a tile grid, because the comparison was centre to centre. It is
  now measured to the target's **edge** (`min_range + both radii`), so a skirmisher's neighbour is genuinely too
  close. Radii were already imported.

The dead zone is therefore not siege-only: the skirmisher (7), slinger (185) and genitour line have one tile of
it, taken away by Andean Sling.

Next: **blast damage** would complete the siege engine, or the **per-civ 211-number array**, still the largest
unread thing in the dat.

## Done next: `overrides.toml`, and Murder Holes confirmed (2026-10-28, v0.88)

**The override layer now exists** (D45). `"units.<id>.<field>"` in `config/overrides.toml` replaces a unit's
value after loading; unsupported paths are refused by name rather than ignored; every replacement is logged with
its old and new form; and the file joins the pack's handshake hash when present. Proven end to end by a test that
overrides a tower's `min_range` to zero and watches it shoot what stands against it.

**Murder Holes needed no code.** Towers and castles carry `min_range` 1, and the tech is an `add -1` — which the
modifier path already applies — so the test is the proof rather than the feature: a tower cannot shoot an
adjacent unit before it, and can after.

Next: **`techs`/`effects` overrides** (the other half of the configuration surface the user described), or
**blast damage**, or the **per-civ 211-number array**.

Still not interpreted, and counted: **kind 0 (106 commands), kind 6 (2), kind 11 (2), kind 15 (1),
kind 101 (97, undecidable namespace), kind 103 (25)***, plus the `other:`/`unpacked:`
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

## After the prototype list: what the skirmish cannot settle

`config/skirmish.toml` (step 30) scripts both sides and they fight, but **it does not produce a decided match**.
Militia do ~3 damage a hit against a 2,400 hp town centre, so a fifteen-minute script ends with two armies
grinding rather than a result. Two ways out, and the owner should pick one before more time is spent on either:

1. ~~**Siege in the content**~~ - **done for the mangonel** (D62): Castle Age 102, siege workshop 49 and mangonel 280
   are roles, the train edge exists, and a mangonel damages a building in a test. What remains is the *economy*: a
   scripted opening that reaches Castle inside fifteen minutes (villagers from tick 0, farms, ten-plus on food) and
   the acceptance test that breaks a town centre with it. A ram is still unbuilt and would be the cheaper second
   siege unit (attack class 20 against buildings, no dead zone). The boom script now also has to satisfy D65:
   two Dark Age buildings for the Feudal Age, two Feudal buildings (blacksmith + market is cheapest) for Castle,
   and the workshop's blacksmith prerequisite (D63).
2. **A second victory rule** - a time limit, a score, or "destroy all buildings" instead of the seat. Cheaper, but
   it changes what the engine promises rather than what the content contains.

Until one is chosen, the honest statement is: the engine can be *played* to a fight, not yet *won*.

## Other named gaps, in the order they would be felt

- **No gates** (D54): walls block their owner too, because pathing is one grid.
- **No unit pushing** (D52): overlap is possible, and a unit already sharing a tile is never moved aside.
- **Fog without occlusion** (D50): sight is a disc, not a line through terrain.
- **The HUD is data, not pixels, and stays that way** (D57; owner's decision, 2026-10-28): no bitmap font. `Hud`
  remains a struct with tests; if the screen ever needs to show a number, colour bars are the cheap half and a
  font is not on the list.
- **Selection has no control groups or hotkeys** (list step 25, not in this batch).
- **The overbuild race is tested, with one branch statically unreachable** (D61): `foundations.rs` covers first
  builder wins, an untouched foundation losing to it, and a blind placement cancelled on sight. The "already
  started" clause inside `cancel_overbuilt` cannot be reached, because a builder that can work the tile must
  have seen it first and the sight rule cancels the placement before work begins. It stays as the rule reads;
  flipping it changes no test, and D61 says so.
- ~~**A very slow unit walks the wrong way**~~ - **fixed** (D64): a builder that finished left its `velocity` set, so
  an idle body slid at `speed / tick_rate` for ever. Work *is* adjacency-gated (the "work at a distance" reading was
  wrong), and a slow move order arrives correctly; the bug was velocity, not pathing. Three tests now hold it.
- **Overbuilding a town-centre foundation is now a refund, not a loss** (D59 + D61): the seat rule is gone, a
  cancelled foundation is paid back in full, and the town centre is class 3 like any other building, so losing
  one costs time and a walk rather than the match. The old sharp edge (a blind house ending the game) cannot
  happen any more, which is worth knowing before anyone "fixes" it back.

## The defeat rule (D59, applied 2026-10-28 - kept here as the reasoning, not as a gap)

Our D46 says a match ends when a player's **town-centre family** is gone. The original game says something else,
twice, in the manual:

> "To defeat a player, you must destroy all units and buildings but the following: towers, walls, gates, relics,
> Fishing Ships, Fish Traps, Transport Ships, Trade Cogs, Trade Carts, Farms, and sheep."

So, in Age of Kings/Conquerors/DE:

- A player is **eliminated when they have no units and no buildings left** - and the listed things (walls, gates,
  towers, farms, trade and fishing ships, fish traps, sheep, relics) do not have to be destroyed, so a player left
  with only those counts as defeated.
- The **town centre is not special.** Losing it does not lose the match: any villager can rebuild it, and any other
  building keeps you alive. "Military conquest" is the only condition that always applies, whatever victory
  setting the lobby chose (Standard adds Wonder and Relic victories, plus Time/Score/Last Man Standing).

The mismatch is not cosmetic. Under D46 a player whose town centre falls has lost even with twenty villagers and a
barracks across the map, and cannot rebuild. Under the original rule the same player is alive and rebuilding -
which is also what makes a razed base a setback rather than a defeat.

**Applied as D59** - the seat rule is gone, replaced by the original's elimination:
a player is out when they have no living unit and no standing building that *qualifies* - and what qualifies is
a **class**, not a role: `[conquest] surviving_classes = [3]` (D59b). "Walls and farms do not count" turns out to
be the same statement as "towers are class 52 and walls class 27". D46's "refuse orders after the match is over"
and the draw case are kept. **The grace window stays out** (owner's decision, 2026-10-28): it is a DE client
behaviour, not part of the manual's rule.
- **Defeat uses classes, so the D59a gap is closed** (D59b): `surviving_classes = [3]` is observed by five
  tests. What is *not* yet exercised is the manual's unit half - sheep, livestock, ships, trade carts are
  excluded there and the pack has none of them. When it does, add a class list for units and the test "a sheep
  does not keep a player alive".

## From the prior-art digest of 2026-10-29 (triaged, not yet done)

The daily watch found four things worth acting on eventually and one worth acting on now (the fixed-point
gate, which is D67 and is done).

- **Reach is measured between boxes, not centres.** `openaok` compares the dat's boxes for every attack:
  building to its footprint, unit to its size box. We compare centre distance and pad by both radii for
  `min_range` only, so a skirmisher can hit a town centre from further out than the original, and a
  building's "8 tiles" is eight tiles from its middle rather than from its wall. For **units** this is nearly
  free - we already emit `radius`, and a box is `2r` on each axis. For **buildings** it needs a footprint,
  and a building is one tile (D54). Do the unit half when reach is next touched; the building half when
  buildings stop being one tile.
- **Crowd at a work goal.** `openaok` now claims and waits for spots at work as well as at move goals; D52
  says work goals are deliberately *not* exclusive. Theirs is nearer what a player sees and costs exactly
  what D53 declined: a claims table released on every order end. Recorded; not adopted.
- **Routing round still bodies.** Their pather bends round units from a per-tick survey; ours goes through
  everything except terrain and walls. We already keep the unit-tile bitset, so the layer is nearly free;
  the cost is invalidating routes when a body moves. Crowd scenes only.
- **`genie`'s declared schedule.** Twenty steps in `BEFORE_PLAYERS`/`EACH_PLAYER`/`AFTER_PLAYERS`, each
  naming the DE routine it ports, with deferred writes so a tick's result never depends on which object ran
  first. Our order is already fixed and deterministic, so this is fidelity, not correctness: keep the
  checklist, restructure nothing until units start affecting each other. Also: the original's unit AI runs
  for **one player per tick**, rotating - our per-unit aggro stagger is the wrong shape, and it shows only
  in large fights.
- **Foundations block nobody until worked.** Confirms D61's shape from the other direction: when buildings
  start to block paths, a foundation must not block until a villager first works it, or every placing
  villager's route changes the moment the site goes down.

## The animal slice, in three increments (D69, started 2026-10-29)

1. **Importer** - *done*: `food` from `unit.resources` type 0, `dead_unit` from `dead_unit_id`, `decay_rate`
   from `resource_decay_rate`. Pinned against sheep 100 / boar 340 / deer 140 and the three body ids.
2. **Herding** - next: a sheep with no villager near it stands still; a villager within a few tiles makes it
   walk **toward the nearest drop site of the villager's player (mill or town centre)**, not toward the
   villager, and it stops when it arrives. Keyed on the animal classes (9, 10, 58) or a content role - never
   on `food > 0`, which five plants and food piles also satisfy.
3. **Retaliation and the carcass** - a struck boar turns on whoever struck it (it already carries the attack);
   a dead animal leaves its `dead_unit`, and that body can be gathered for food until empty. A body that does
   not exist must be a lookup that finds nothing, not an error: 640 records name 768, which no civ has.

### The herding rule, complete (owner, 2026-10-29)

Two corrections in a row, and the second changes the shape of the increment:

1. *"villager is herding sheep towards closest mill or town centre. So sheep do not close distance to him but
   move in the direction of mill or town centre."* The sheep's destination is the **nearest drop site of the
   villager's player**, not the villager.
2. *"Also the villager follows the sheep herding them towards the gathering point."* The villager **trails the
   sheep**, so the pair travels to the drop site together.

So herding is **not a separate order**: it is what gathering from a live animal *is*. A villager sent to gather
a sheep keeps its gather task; the sheep steps toward the drop site; the villager follows it because gathering
requires adjacency. The loop ends when the sheep reaches the drop site, and gathering continues there until the
sheep is empty - at which point it dies into its `dead_unit` (D69) and the villager gathers from the body if
there is anything left.

Two consequences to get right, both testable:

- **The villager must re-path when the node moves.** Every other gather source is static, so the tick loop
  never had to notice a node moving; a herded sheep moves under the villager's feet.
- **The villager is not idle while following.** It carries on gathering whenever it is adjacent, so the walk
  and the work interleave rather than alternating wholesale.

### Who is allowed to make meat (owner, 2026-10-29)

*"Also, sheep or any other food animal provides no food if killed by anybody else than villager."*

So the meat is a property of **how the animal died**, not only of what it was. The rule, as it will be
implemented: when an animal dies, its `dead_unit` becomes edible **only if the killer was a villager** - a
villager of any player, including an enemy's, because the rule names the role and not the side. Killed by a
soldier, an arrow, a tower or another animal, the body is still a body, and there is nothing in it.

**Not verified against the game, and said plainly:** my recollection is that in AoE2 a carcass can be gathered
whoever killed it, and that the "killed by a non-villager means no food" rule is the *first* game's. The
owner's rule is what gets built - it is his game to specify - but it is exactly the sort of thing to check
against HD when someone has the game open, so it is written here with the doubt attached rather than as
settled fact.

**Where it belongs in the code:** the death path already knows who dealt the last blow, because the outcome
names the killer. The rule is one content switch - `[hunt] meat_requires_villager` - consulted when the corpse
is created, so the engine asks content rather than knowing that a villager is special (D17). Testable in one
assertion: the same sheep, killed by a soldier and by a villager, leaves bodies that differ in food and not in
identity.

### Where increment two actually stands (2026-10-29)

**In place and green:** `[hunt] herdable_roles = ["sheep"]` in `economy.toml`, the `sheep` role (594), and
`Hunt` in the economy schema (raw + runtime + mapping). Four tests in `crates/sim/tests/animals.rs` say what
herding means; they are **`#[ignore]`d** because the engine half is not built, and the ignore note lists why.

**Not built, and the three reasons the tests gave:**

1. **`gather_source` refuses a unit.** Sending a villager to a sheep fails with `NotAResource { id }`: the
   function accepts a resource node or a farm building, and nothing else. The fix is content-shaped - a live
   animal of a `[hunt] herdable_roles` family yields `[hunt] resource` - but it is a change to the one
   function every gather order passes through, so it wants its own slice rather than a patch bolted on.
2. **A placed animal carries no meat.** `Command::Spawn` takes an explicit `amount`, and a sheep placed with
   `amount: 0` is worth nothing to gather. Either the spawn derives `amount` from the unit's `food` (D69), or
   whoever places animals must pass it; the first keeps the pack as the single source of the number.
3. **Nothing walks the animal.** The herd pass is not written: while a villager gathers a herdable animal, the
   animal should be sent toward the nearest drop site of that villager's player, and the villager's cached
   route dropped so it follows the moving node instead of a stale path to where the sheep used to be.

**One unexplained observation, recorded rather than forgotten:** in the "a sheep nobody is gathering stands
still" test the sheep moved **0.025 of a tile** over 600 ticks with no order given - the same *shape* as the
"very slow unit walks the wrong way" bug that turned out to be a leftover velocity (D64). It may be the same
class of defect in a path nothing else exercises yet; it is worth a look before the herd pass is written,
because a standing-still test cannot be written against a body that creeps.

### Conversion: how a herdable changes hands (verified, 2026-10-29)

Asked by the owner: *"sheep captured by another player can be recaptured if no unit is nearby (please verify how
and conditions)"*. Verified against the game's own manual and the wikis; the rule is **line of sight**, not
adjacency, and it is immediate:

- *"Sheep start the game neutral (gray) and unowned. As soon as a sheep enters the line of sight of a unit, the
  sheep is under control of that player. You can steal ownership of another player's sheep if it comes within
  your unit's line of sight and there are no other players' units near the sheep."* (AoE2 manual, sheep entry)
- *"if the herdable animal subsequently gets in contact with another player's unit without a unit of their
  current owner nearby, they will immediately fall under the other player's control."* (Food, Fandom)
- *"If more than one civilization has units near an animal, the animal will be owned by the player who has the
  most units nearby."* (Herdable animal, Fandom)
- **Celts** ignore the "no enemy units near" condition entirely. A civ bonus, out of scope until civs decide
  anything mechanical (D63 is the one place a civ has mattered so far).

**What that means for the engine**, and why it is bigger than it looks: the trigger is our **vision layer**
(D50), not proximity - a tile a player's unit can see is a tile whose herdables that player takes, provided the
current owner has no unit seeing it too. We already have `visible_to`/`unit_sees`, so the test is nearly free;
what is new is that **ownership of an existing entity changes**, which nothing does today: every ownership
change so far is a kill or a spawn. That wants its own decision on what happens to a villager *gathering* a
sheep that changes hands mid-load, and whether the meat already gathered stays with the gatherer.

### Step one of the animal slice: a live animal is a gather source (done, 2026-10-29)

`gather_source` accepted a resource node or a farm building and refused everything else, so a villager sent to
a sheep got `NotAResource` before it walked a single tile. It now also accepts a **living unit whose family is
an animal**, with the resource named by `[hunt] resource` and the amount carried by the animal itself.

Content gained the roles it needed (`sheep` 594, `boar` 48, `deer` 65) and a second list, because herding and
edibility are two questions: `animal_roles` is everything worth meat (both kinds), `herdable_roles` is the
subset that walks to a drop site. `Hunt` carries `animal_roles` and `resource`.

Two tests, both RED first: a villager sent to a sheep is accepted and ends up carrying meat; and a sheep placed
with no amount is worth the pack's **100** rather than zero.

**Adversarial review of this step:**

- `meat_of` is applied to *any* unit the pack gives food to, and five are not animals - the forage bush 59 and
  its twin, a food pile 262, a deer's meat pile 333, an oddity 544. Harmless today (a placed bush carrying an
  amount gathers nothing: `gather_source` gates on `animals`, not on food) but it means "lists `food`" and
  "is an animal" must never be conflated later.
- The check is on the **order**, not the tick, so this costs one set lookup per gather order and nothing per
  frame.
- `animals` is derived at seed from config and is not hashed, like `conquest_classes`: it decides what orders
  are *refused*, and a replay runs with the same pack.
- **Left for the carcass step, deliberately:** when a sheep's meat runs out the villager stands down and the
  sheep stays on the map alive at zero amount. That is wrong in the game and right for now, because killing it
  is the carcass rule's job (step three), not this one's.

### Step two: herding (done, 2026-10-29)

`[hunt] herdable_roles` resolves to families at seed (`World.herdable`). While a villager gathers one, the
animal is walked to that player's nearest drop site and the villager's cached route is dropped so it follows.
All six animal tests pass: an unworked sheep stands still, a gathered one walks toward the town centre, the
villager follows it rather than watching, a sheep stops on arrival, a live animal is a gather source, and a
placed sheep is worth 100.

**Two things the tests corrected in me:** my "villager follows" assertion compared the wrong two distances
(the villager starts *at* the town centre, so "closer than it started" is nearly zero) - it now asserts the
villager moved and stays within six tiles of the sheep; and the first implementation re-found the gather node
per gatherer per tick, an O(units) scan that the arm had already done - the node's unit is read once now.

**Still open:** the sheep is not killed when its meat runs out (carcass rule, step three), a boar never
retaliates (step three), and an animal never changes hands (conversion, step four - needs the owner's ruling on
what a gatherer's load does when its sheep is stolen).

### Step three, re-shaped by the slaughter rule (2026-10-29)

The owner's correction - meat implies death, and the killing blow lands **on arrival at the drop site** - means
step one's shape needs reworking, not just extending: a villager currently gathers from a *live* sheep, which
the rule forbids. The corrected sequence, and what each part costs:

1. **Slaughter on arrival**: when the herded animal reaches its drop site, the villager kills it. Cheapest
   faithful form is a transformation in place - the entity's `unit` becomes its `dead_unit`, its health goes to
   zero, its meat amount stays, and the gatherer's task stays valid because the id did not change.
2. **Gathering the body**: `gather_source` must accept a carcass (no health, meat in `amount`), which is
   content-named - `[hunt] carcass_roles` for 595, 356 and 43 - rather than inferred from "no health".
3. **Retaliation**: a struck animal turns on whoever struck it. Independent of the slaughter work and done
   first.
4. **Only a villager makes meat**: the killer decides whether the body holds food.
5. **Conversion** (step four): line of sight, immediate, and now with a clear window - the animal must be
   **alive**, which means during the walk and before the killing blow.

### Retaliation: attempted, parked, and what the attempt proved (2026-10-29)

The first attempt is reverted and its tests are `#[ignore]`d. Three things it established, all worth more than
the reverted code:

- **A boar parked beside the town centre is shot by the town centre.** The blow that lands is an *arrow*, and an
  arrow lands with its damage but not with its shooter - `Task::Fly` carries no attacker - so an animal cannot
  retaliate against a building at all today. My test was never testing melee.
- **Moving the beast to eight tiles changed nothing**, so either the town centre reaches further than I assumed
  or the blow came from elsewhere again. Either way the test's premise was unproven, which is exactly why it
  should not have been written that way.
- **The melee hook never fired**: `retaliate` was called from the melee strike path, its debug print never
  appeared, and yet the boar lost health - so the damage took a path I had not identified.

**What to do instead, next time:** write the test with two units and no buildings within range, so the attacker
is unambiguous; then find which of the four `hp -=` sites in the tick actually delivers a melee blow, with a
print at each, before writing the reaction. The mechanic is still wanted - a boar that does not fight back is
scenery - but the ordering was wrong: identify the damage path first, write the test second.

### The retaliation test's premise, proved wrong by instrumentation (2026-10-29)

Owner's rule, which the instrumentation then confirmed on screen: *"If Town Centre kills the boar, it doesn't leave
the carcass with food. Villager has to be the one delivering last blow."*

Printing at both damage sites and running the parked test settled it: the boar was struck **only** by arrows -

```
ARROW target=EntityId(127) dmg=3   (six times)
MELEE ...                          (never)
```

so the town centre shot it, exactly as the owner said it would, and the villager **never landed a single blow** in
two thousand ticks despite being ordered to attack at eight tiles' distance. Two consequences:

1. The retaliation test was invalid from the start, for the reason now visible rather than guessed: the only
   attacker was a building, and an arrow carries no attacker.
2. **A villager ordered to attack an animal may not strike at all.** That is either a pathing problem (it never
   arrived) or a targeting rule (player 0 is not a valid victim for a villager) - and it matters, because the
   owner's rule makes the *villager's own blow* the thing that decides whether a carcass holds food. Find out which
   before building the slaughter: if a villager cannot strike an animal, the slaughter has to be an act of its own
   rather than a very small fight.

**The damage path, for the record:** melee blows land in the `strikes` loop (`victim.hp -= landed`, which knows its
attacker); arrows land in the `landings` loop (`victim.hp -= damage`, which does **not** - `Task::Fly` carries the
target, not the shooter). Anything that needs to know who killed something must either take the melee path or the
arrow must start carrying its archer.

### Meat is decided by the last blow; hunters shoot, shepherds cut (owner, 2026-10-29)

Two refinements, both narrowing what the earlier notes guessed at:

- *"If soldiers hurt the boar, but villager makes the last hit, then carcass does give the meat."* So nothing about the
  **history** of the wound matters - soldiers may soften it, arrows may stick in it, and a villager's last blow still
  leaves a carcass worth 340. Only the **last** blow counts, which means the engine has to record who dealt *that* blow
  and nothing else.
- *"Hunter villager also use the bow while shepherd villager uses the knife."* A hunter variant attacks at **range**, a
  shepherd at melee. That is the likely explanation for a villager never landing a blow on a boar at eight tiles: an
  attack order with a ranged variant resolves as a **shot**, and a shot lands without carrying its archer - so the
  victim is hurt by an arrow nobody can name, and the variant that is supposed to make meat is the one whose blow the
  engine cannot attribute. This is now the first thing to check, before any carcass code is written.

### Boar luring: what the video is, and what we can and cannot take from it (2026-10-29)

The owner's link (`DkFD8bhtFkI`) is **"How to lure a boar in AoE2 shown in 45 seconds or less (garrison method)"** by
Hjörleif. Its transcript is **disabled**, so the narration could not be read; the title, the description's companion
links (a "Viper method" and a "basic method"), and general knowledge are what this rests on, and it is labelled as
such rather than presented as the video's content.

The **garrison method**, in the terms the engine will need: a villager is sent to hit the boar once (or simply come
close) so that it takes the villager as its target; the villager then walks back toward the town centre with the boar
following; at the town centre the villager **garrisons**, and the boar - having lost its target - stops in place,
where the remaining villagers finish it. The trick is that the boar keeps chasing only while it has something to
chase, so the garrison ends the chase exactly where you want the killing to happen.

Our engine has every part of that except the last: an animal takes no target of its own (retaliation is parked), and
garrisoning exists. A boar lure is therefore a scenario script that a **retaliation** rule makes work - which is a
second, practical reason it is not optional.

And the owner's remark that the **factory AI never lures the boar in HD**: our acceptance scenario must script the
lure explicitly. Nothing in the AI will do it, and a fast-castle script that ignores the boar is leaving 340 food
standing in the forest.

### The acceptance budget: 30 minutes of game time (owner, 2026-10-29)

*"Can you speed up the simulations instead and then add extra 10 minutes?"* - yes, and the two halves are independent:

- **Speed is already free.** The simulation is headless and deterministic: the Dark Age probe played **20 minutes of
  game time in about ten seconds** of wall clock. Nothing in the acceptance test waits on real time; a longer match
  costs seconds, not minutes. There is no speed knob to turn, and none is needed.
- **The budget is a game-time decision, and it is now 30:00**, up from 20:00. The measured reason: a berries-and-farms
  economy reaches Feudal at about 14:30 and Castle does not fit inside twenty minutes without the sheep and boar the
  researched openings rely on. Ten more minutes of game time is ten more seconds of test runtime and removes the
  pressure that was forcing the economy to be faster than it is.

### The boar's leash, and where it goes back to (owner, 2026-10-29)

*"Boar can also lose interest in chasing the hunter and not follow him to the town centre and returns where he was
before. So another shot with arrow is needed before boar flees too far. I usually wait until 2 arrows are shot and then
try to flee."*

So an animal's chase is not a commitment, and the garrison method is not a trick that works from any distance:

- it follows **as long as it is interested**, and interest expires - by distance from where it started, or by time
  spent chasing, or both;
- when interest expires it **returns to its origin**, which is a place it remembers, so an animal needs a home
  position in its own entity state (it has none today);
- **each new blow renews the interest**, which is why a second arrow lands before the hunter runs: the arrow is not
  aimed at damage, it is aimed at keeping the boar following.

Three states, then, not two: **grazing at home -> chasing a target -> returning home**. Built testably as: a struck
animal walks toward its attacker; break line or walk far enough and it walks back to where it began; a second blow
turns it round again.

### Priorities, weighted towards what a simulation can settle (owner, 2026-10-29)

The owner's aim, in his words: *"I intend to keep system, that allows simulations ... when adding new civilizations, it
would be helpful to use the simulations for game balancing."* That changes what "next" means: work that a headless run
can measure comes before work that only a person can see.

**Simulation-testable, and wanted:**

1. **The slaughter and the carcass** - in flight. Last blow decides the meat; the body is gathered; the animal becomes
   its `dead_unit` in place. Every assertion is arithmetic.
2. **The animal's chase and leash** - the three states above. Timeable, distance-measurable, deterministic.
3. **A scripted opponent and a scenario format** - steps with times, so a match is reproducible. This is the thing the
   balance goal actually needs: *"at 2:10 build the mill, at 4:30 send two villagers to wood"* is a build order that
   can be run for every civ.
4. **A batch runner with metrics** - N scenarios x M seeds, headless, emitting a table: time to Feudal, time to Castle,
   food/wood/gold per minute, villagers lost, first military unit, match outcome. Without this, balance is opinion.
5. **The acceptance match** - fast castle, siege workshop, mangonel, raze the town centre, `Victory { winner: 1 }`
   inside 30:00. It is the one test that says the whole economy-to-army chain works.

**Not yet simulation-testable, and why:** there is no AI (only scripts), civs are data that nothing branches on beyond
one building prerequisite (D63), and the market, trade and full counter triangle are absent. Balance work on those
would be measuring a game we have not built yet.

**The order that follows:** 1 and 2 are small and both close a hole I opened; 3 and 4 are the balance engine and are
the first things that pay back the owner's stated goal; 5 is the acceptance test that uses 3.

### Step three, part one: the slaughter (done, 2026-10-29)

A herded animal that reaches its drop site becomes its body **in place** (unit 595/356/43, no health, meat
intact) and the villager gathers from it. Content names the bodies; the sweep is taught to keep them. Seven animal
tests pass.

**Adversarial review of this change:**

- **A per-tick table clone, found and removed.** The first sweep change cloned the entire unit table once per
  tick to answer "is this a body"; the set now holds ids *and* families (six entries) so it costs a lookup.
- **`hp == 0` no longer means "delete" for these units** - a rule the rest of the engine has to know. Anything
  that treats zero health as gone (target selection, blast victims) already skips them, which is right: a body is
  not a target.
- **Still missing, and named:** an animal killed by anything but this slaughter is swept away with nothing left.
  The last-blow carcass needs the killer, which melee knows and arrows do not - the same fact that parks
  retaliation.
- **Open questions kept:** a body belongs to Gaia, so anyone's villager can work an enemy's carcass (probably
  right, unverified); a body never decays (`decay_rate` is imported and unread); and a body emptied of meat stays
  on the map for ever.

### Corrected: the shepherd kills, then there is a corpse (owner, 2026-10-29)

The transform-on-arrival in increment three has the right body and the wrong trigger. It must become:

1. the herded animal arrives at the drop site;
2. the villager **attacks** it - the shepherd's knife, melee - and the animal **dies of the damage**;
3. **death** leaves the body: `dead_unit`, meat intact if the last blow was a villager's, empty if it was anyone
   else's;
4. the villager goes on gathering from the body, which is why the body should keep the entity's id.

**Order of work, because 2 is blocked:** establish that a villager's melee blow actually lands on an animal
(the instrumentation showed no `MELEE` line at all, only `ARROW dmg=3` - which the owner's hunter/shepherd split
predicts: a hunter variant attacks at range, and a shot lands without its archer). Then the death path learns its
killer, then the body is made at death rather than at arrival.

## Item 1 - the damage path, retaliation, and the boar's leash (2026-10-29)

*This section is the compact for the increment; the review is appended to it when the code lands.*

### Verified before writing anything (and it corrects the previous session's diagnosis)

- **`start_attack` was never called, so "its silent refusal" was a misdiagnosis.** `retaliate` runs only on the
  *instant* melee path (`lib.rs`, the `strikes` loop's `victim.hp -= landed`), and no blow has taken that path:
  `combat.toml` sets `projectile_speed = 10.0`, and the loop turns **any** blow into an arrow whenever
  `flight > 0 && source.0 != aim`. For a non-Ballistics attacker `aim` is the victim's own position, which differs
  from the attacker's for every melee blow - so a villager's knife, a spear and a militiaman's sword all become
  arrows in the air. The data says the same from the other side: villager 83 has `range = 0.0, reload = 2.0`, and
  `check_attack` is happy with it.
- **Consequence, and it is one defect with three faces:** melee damage lands in the `landings` loop, which carries
  no attacker, so (a) `retaliate` never fires, (b) the last blow cannot be attributed, and (c) the previous
  session's "no `MELEE` line" was the whole tick, not one fixture.
- **The knife test that passes is passing through the arrow path.** `a_villagers_knife_lands_on_a_sheep` asserts the
  sheep lost health, and it did - by a projectile.

### The fix, in two halves that are separately testable

1. **Melee lands in place.** A blow is a projectile only when the attacker's *resolved* range is above zero. Melee
   damage applies on the tick it is swung, in the `strikes` loop, which is the path that knows `(attacker, target)`.
2. **An arrow carries its archer.** `Task::Fly` gains the shooter, the landing carries it, and the landing applies
   damage *and* the reaction - so ranged damage is attributable too.

Then retaliation is reachable at all, and the leash has something to hang off.

### The leash: three states, and where the numbers come from

**Verified:** the dat carries **no leash field** for animals (`BOARX` 48 has `los = 4.0`, `speed = 0.8`, no
distance or duration of interest), so the leash cannot be derived from the pack.
**Assumed, and content-authored with provenance:** an animal chases until it is `leash_tiles` from the place it was
first struck, or until `leash_seconds` have passed since the last blow - each blow renews the clock. Both live in
`[hunt]`. These are the knobs balance work will tune; they are judgement calls and are marked as such rather than
presented as game numbers.

### Tests this increment owes

- `a_struck_boar_turns_on_whoever_struck_it` (parked `#[ignore]` today) - now reachable, so un-ignored.
- a melee blow never puts a projectile in the world (the fix's own control).
- an archer's arrow names its archer: a boar shot from range turns on the shooter.
- the leash: struck -> chases -> out of interest -> walks back to where it started; a second blow turns it round.
- the refusal record: `retaliate`'s swallowed error is readable instead of invisible.

### Item 1, review: what the change costs, and what it nearly broke (2026-10-29)

**Landed:** melee lands on the tick it is swung; `Task::Fly` carries its `shooter`; retaliation works on both
damage paths; the animal's leash is built (three states, home recorded at the first blow, each blow renewing the
clock); a swallowed reaction refusal is recorded and readable. Tests: 8 in `retaliation.rs`, five of them written
and watched failing first (`a_melee_blow_never_becomes_an_arrow` reported **3 arrows** during a knife fight).

**Performance.** The per-tick cost is unchanged in shape: one `matches!` on an already-loaded task per blow, and
the leash pass returns immediately when `World.leash` is empty - which is every world where nobody has struck an
animal. The `landings` vector is one struct wider (a shooter id); it was invented here not at all. No new table
walk in the tick: the leash numbers are resolved at seed, and the shooter rides on the arrow rather than being
looked up. **Not measured with the A/B probe yet** - the honest statement is "no new per-entity work", and the
probe is owed before anyone claims a number.

**Stability.** The state hash gained the leash (only when entries exist) and the shooter on an arrow, so a replay
that lost either diverges visibly rather than silently. The anchors move for exactly that reason in any world that
strikes an animal; the scenario/probe hashes must be re-recorded when next they are run.

**Three defects the review found in my own work, all fixed:**

1. **`leash` was left out of the bookkeeping tables.** A dead animal would have left its entry behind for ever, and
   `bookkeeping_entries` - which exists precisely to make that visible - would not have noticed, because it did not
   count the table. Added to both `bookkeeping_entries` and `bookkeeping_touches`, and to the removal sweep.
2. **The no-float gate went red.** The first version converted `10.0` seconds with `f64::from(tick_rate)` inside
   `seed_from`, in `lib.rs` - a float in the sim crate's tick-adjacent code, which is the exact thing the gate
   exists to refuse. Fixed by making both numbers **whole seconds and whole tiles in content** (`u32`), so the
   conversion is integer arithmetic and there is no value a scale could round.
3. **Two fixture bugs, mine.** A `Move` to a hand-picked `(2, 2)` is `NoPath` on this seeded map, and an archer
   spawned ten tiles out stood somewhere unreachable and refused its attack order at apply time. Both now use a
   position the test has already proved reachable (the villager's own start tile), and the archer sits at
   (12, 1) - recorded here so the next fixture does not repeat either.

**What is still not right, named rather than glossed:** blast damage retaliates (the shooter is known now) but a
*body* cannot, and nothing stops a boar chasing into a wall - it plans a route and gives up only if there is none.
The leash numbers are judgement calls, not game numbers; a balance run is what will tune them.

## Item 2 - the last blow leaves the body (2026-10-29)

*Compact before the code.*

**Verified:** every animal names its body in the pack (`dead_unit`: sheep 594 -> 595, boar 48 -> 356, deer 65 ->
43), and the body's own record is a plain unit (`SHEEP_D` class 11, `hp = 0`, `food = 0`, `decay_rate = 1.0`). A
blow now carries its attacker on **both** damage paths (D71), which is the fact this item was waiting for. The
sweep already keeps an entity whose unit id is one of `[hunt] carcass_roles`, so a body that exists is not swept.

**Assumed from the owner, and only from him:** the **last blow** decides the meat - soldiers may soften the animal
and arrows may stick in it, and a villager's final hit still leaves 340 on a boar; anything else's last blow leaves
a body with nothing in it. The dat has no field for this.

**Unknown, and content's to answer:** *which* roles count as "a villager". That is a role list, not a hard-coded
id: `[hunt] butcher_roles = ["villager"]`, resolved to unit families at seed like every other content list.

**What is built:** death by damage promotes the animal **in place** to its body - same id, no health, meat kept
only when the killer is a butcher, task idle, no motion - because the gatherer's task names the id and must stay
valid across the blow (D70). The killer is recorded per tick (`last_blow`, a `Vec` of `(victim, attacker)` filled
by both damage sites), because the blow that kills is always the blow of the tick it dies on; nothing is carried
between ticks and nothing new is hashed.

**Tests this owes:** a villager's last blow leaves the meat; a soldier's last blow leaves an empty body; and a
soldier who *softens* the boar does not take the meat once a villager finishes it.

## Item 3 - the shepherd kills, and meat only comes out of a body (2026-10-29)

*Compact before the code.*

**The owner's correction, verbatim:** *"No, shepherd has to kill the animal. It doesn't change to corpse itself."*
So arriving at the drop site is not the kill. What is built today is the right body with the wrong trigger: the
herding pass promotes the animal the moment it reaches the drop site (a corpse without a blow).

**And a second thing the same rule forbids, which the code does today:** a villager gathering a **live** animal
takes meat out of it (`take_amount` reads the node's `amount`, and a live animal carries its meat). *Meat implies
death* (the owner's rule), so a live animal must yield **nothing** while it is being herded; the work during the
walk is following, not taking.

**The shape:** on arrival at the drop site the villager is **ordered to attack** the animal (the ordinary combat
path, so the blow carries its attacker and the last-blow rule already decides the meat), the animal dies of that
damage, and death promotes it to its body in place (item 2). Then the gather order **resumes on the same id** -
recorded in a side table `slaughter: villager -> animal` - because the villager's original `Gather` task was
replaced by the fight and the body is what it should be working.

**Numbers, verified from the pack:** a sheep is 7 hp and a villager's blow is 3 damage at a 2 s reload, so three
blows (six seconds) kill it. The villager stands adjacent, which is where gathering put it.

**Tests this owes:** a live animal yields no meat (a villager beside a boar carries nothing while it lives); the
shepherd strikes before the body exists (the villager's task is an attack on the sheep, and the sheep's health falls
while it is still a sheep); and the gathering resumes on the body once the animal is dead.

### Item 2, review: what the body cost (2026-10-29)

**Landed:** death by damage promotes an animal in place to its `dead_unit`; the last blow decides the meat
(`[hunt] butcher_roles`); the killer is collected per tick from both damage paths.

**Performance.** The first version walked **every entity every tick** to find the dead - the same shape as the
per-tick unit-table clone the previous session's review caught. It now runs only on a tick where blows landed, and
only over this tick's *victims*: one lookup per corpse, nothing on a quiet tick, and nothing added to `Entity`.

**Stability.** Nothing new is hashed: the killer record is a local `Vec` in `step`, empty in any world where nobody
is fighting. The promoted body keeps its id, so every task that names it stays valid - which is the property the
gatherer depends on.

**The defect the review found in my own change, fixed test-first:** with the body now surviving the sweep, "the
target is gone" stopped being how a fight with an animal ends, so an army would have swung at a corpse for the rest
of the match. `an_attacker_stops_swinging_once_its_quarry_is_a_body` failed before the fix (the attack phase now
requires a target with health) and passes after it.

**The trap in the fixtures, recorded because it will recur:** `Command::Spawn`'s `amount` is in **whole units** and
scales itself, so a fixture passing `340 * scale` builds a 340-food boar worth 340 x scale x scale. Both new
fixtures had it; the assertion caught it at 136,000,000,000 against 6,800,000.

### Item 3, review: the slaughter is a real kill (2026-10-29)

**Landed:** on arrival the villager attacks; the animal dies of the blow; the gather order resumes on the body;
a live animal yields nothing.

**Performance.** No per-tick scan: the herding pass already visits the workers that are gathering an animal, and the
resume pass costs one map walk that is empty unless a slaughter is in flight. `take_amount` gained one set lookup
(the animal id is now in `animals` alongside its family precisely so this costs no table read).

**Stability.** `slaughter` is hashed (it decides what the villager does next), joins the bookkeeping tables, and is
dropped in both directions when either side dies - a dead villager cannot leave its animal behind, and a dead animal
cannot leave a stale resume behind. No new `Entity` field.

**Two of my own defects, both caught by running rather than by reading:**

1. **The animals fixture never loaded `combat.toml`**, so `check_attack` refused every slaughter with `NoTables` and
   the villager stood beside a living sheep for 2,000 ticks. Item 1's refusal record named it on the first run.
2. **A test assertion that was too strict**: it demanded "the villager is swinging *and* the animal is alive" at
   every tick the task was an attack - but the attack task lingers one tick past the killing blow, so it fired on
   the tick the body appeared. The assertion now has to see both facts on the *same* tick, which is what the rule
   means.

**Named as still open:** the villager's first blow lands the moment it is ordered (it is already adjacent), so the
shepherd spends six seconds beside a sheep; a real one would not need to wait, but the reload is the data's, and
three blows is what a 7-hp sheep needs.

## Item 4 - a body rots at the rate the pack gives it (2026-10-29)

*Compact before the code.*

**Verified, and from two directions.** `decay_rate` is imported per unit and unread. The numbers are
**food per second**: `BOARX` 0.4, `SHEEP` 0.25, `DEERX` 0.25 in our own pack, and two independent community accounts
give the same figures for the game - *"Boar decays at 0.4 Food per second, Deer and Sheep at 0.25"*, and *"sheep and
deer decay at a rate of 15 food per minute, while boars decay at 24 food per minute"* (15/60 = 0.25, 24/60 = 0.4).
The pack agreeing with both to the digit is what settles the unit of the field.

**A trap this closes:** the *body's* own record carries `decay_rate = 1.0` for all three bodies (`SHEEP_D`,
`BOARX_D`, `DEERX_D`), which is not a rot rate at all - it is the same 1.0 on every corpse, while the living
animal's value is the one that differs per species and matches the published numbers. So the body inherits the
**living animal's** rate, resolved once at seed.

**What is built:** at promotion the body's rot rate is fixed (`rots: body -> raw per tick`), and a per-tick pass takes
it out of `amount` until the body is empty, at which point it leaves the map - which closes the "an emptied body
stays for ever" hole as a side effect rather than as a separate item.

**Cost:** the pass returns immediately unless a body is rotting, and it walks only bodies. Nothing new is hashed: the
rate is derived from the unit id, which is state already.

**Tests this owes:** ten seconds of rot on a boar's body is exactly four food (0.4 x 10); and a body rotted to
nothing leaves the map.

### Item 4, review: rot, and the tick a body was born hungry (2026-10-29)

**Landed:** a butchered body rots at its species' `decay_rate` (food per second, resolved to raw units per tick at
death); a body that rots to nothing leaves the map and leaves nothing in the side tables.

**Performance.** No new pass: the rot is applied **inside the sweep that already walks every entity once per tick**,
with `rots` taken out with `mem::take` so the closure borrows one field rather than two. `rots` is empty unless a
body is rotting, and its per-entity cost is one `BTreeMap::get` - bounded by bodies, not by the map.

**Stability.** Nothing new is hashed: the rate is derived from the unit id, which is state already. `rots` joins the
bookkeeping tables so a rotted body's cleanup is testable (`bookkeeping_touches` is asserted after the body leaves).

**Three of my own defects, all found by running:**

1. **A body lost exactly one tick of food as it appeared** - `the_last_blow_of_a_villager_leaves_the_meat` failed at
   6,799,600 against 6,800,000, which is precisely one tick of a boar's rot. Promotion happens before the sweep in
   the same tick, so a body made this tick was already rotting in it. Fixed by collecting the new rot entries during
   promotion and inserting them **after** the sweep: bodies rot from the tick after they are made.
2. **The `retain` closure cannot mutate**, so the first rot attempt could not decrement anything without a second
   walk of the world; `retain_mut` plus a taken map is the shape that keeps it to one pass.
3. **Non-butchered bodies still have no lifetime** - a soldier's kill leaves a body with no meat and no rot entry,
   so it stands there for ever. That is item 6's "emptied body" hole, still open, and now named in one place.

## Item 5 - a herdable changes hands by line of sight (2026-10-29)

*Compact before the code.*

**The rule is the manual's, quoted in D70 and re-checked there:**

> *"Sheep start the game neutral and unowned. As soon as a sheep enters the line of sight of a unit, the sheep is
> under control of that player. You can steal ownership of another player's sheep if it comes within your unit's
> line of sight and there are no other players' units near the sheep."*

So: the trigger is **vision** (a *unit's* LOS, not a building's), it is **immediate** (no timer, no adjacency), only
**herdables** convert, and ties go to whoever has **more units near**. The Celts' exemption is a civ bonus and out of
scope while civs decide nothing mechanical.

**Content answers "which animals".** `[hunt] herdable_roles` already names the animals that **walk**; the manual's
sentence and the walking are the same set of animals in this pack, so the list does double duty rather than gaining a
second list that could disagree with it. Recorded as a decision, not a coincidence.

**What the engine must know cheaply.** Each unit's `los` is a pack value, so it is squared and resolved at seed
(`sight`), and the herdable *entities* are tracked in a set filled at seed and on spawn - both so the tick can ask
"who sees this sheep" without walking the unit table.

**What changes when an animal changes hands:** its owner, and nothing else - but the *old* owner's gatherer was
following it, so any task naming it as a gather node is stood down and its own route is dropped: it belongs to
somebody else now and walks to nobody's drop site. That is the "window for stealing is the walk" the chicken-and-egg
earlier slices kept pointing at, and the meat is not in question because meat only exists after death.

**Tests this owes:** an unowned sheep joins the first player whose unit sees it; an owned one is **not** stolen while
its owner still sees it; it **is** stolen when its owner cannot; a tie between two stealers goes to the side with
more units near it; and a boar - food, but not herdable - never changes hands.

### Item 5, review: line of sight, and a sheep that kept itself (2026-10-29)

**Landed:** herdables change hands by a **unit's** line of sight; the owner's own sight protects its animal; a tie
goes to the side with more units near it and then to the lower player number; a boar never converts; and the animal
that changes hands drops its route and stands its old herder down.

**Performance.** The pass is one walk of the units times the herdables alive - and returns immediately when no
herdable stands, which is every world whose pack names none. The sight reach is `los` squared, resolved at seed, so
the tick does no table read and no square root. `sight` is a map over the unit table (~1,389 entries); the
alternative was a table read per unit per tick, and the map is paid once at seed. **Not measured with the A/B probe**
- the honest claim is "O(units x herdables) per tick, zero when there are no herdables".

**Stability.** Ownership is state, and it is hashed through the entity. Nothing new is carried in `Entity`. The
herdable set joins the bookkeeping tables and is pruned with the entity.

**Three defects, all mine, all found by running:**

1. **A sheep is not a witness.** Animals have a line of sight of their own, and the first version counted them as
   seers - so an owned sheep saw itself and could never be stolen. The debug print showed `seen = [1, 0, ...]`, which
   is the signature. Animals are now excluded from the seer set.
2. **The contested test could not test the tie-break.** Its counts were two against one, so the mutation that
   reverses the comparison (`>` to `>=`) passed it untouched. A second test with two watchers on **each** side was
   added, and that mutation now fails it. The first test is kept: it pins the count rule, the second pins the
   ordering rule, and neither implies the other.
3. **A fixture ordering trap worth knowing:** ownership is decided the instant anybody sees the animal, so placing
   the sheep before the thieves lets the first thief own it and then its own sight protects it. The tie-break test
   places the watchers first and the animal last. It cost one failing test to see, and it is recorded in the file.

## Item 6 - three known-wrong answers, paid off (2026-10-29)

*Compact before the code.*

**(a) `settle_blind_sites` runs an O(n) scan every tick.** It has a guard, but the guard *is* the scan ("is anything
under construction?"). A maintained set of foundations makes the guard a set lookup, and a `debug_assert!` that the
set agrees with the world keeps the maintenance honest - drift in a maintained set is exactly the failure a
behaviour test cannot see.

**(b) Two count assertions in the data tests follow the pack.** `pack.rs` re-asserts the dat's 801 techs and 798
effects. That is the *dat* test's business (`techs.rs`, where pinning the census is the point); in a pack test it
fails the day a civ is added, for a reason that has nothing to do with packing. Replaced with a relationship: the
pack's tables are the ones its files hold, compared against a direct load of the same file.

**(c) A body with no meat had no lifetime.** A soldier's or a tower's kill leaves a body with nothing in it and - as
of item 4 - no rot entry either, so it stood on the map for the rest of the match. The lifetime is **derived from the
pack**, not invented: a body lies as long as its species' *full* carcass would take to rot (meat / decay rate - 850 s
for a boar, 400 s for a sheep), whether or not it holds meat.

**Tests this owes:** a soldier's kill still leaves a body that can be seen at a minute, and is gone after the span
the pack implies.

### Item 6, review: the three known-wrong answers (2026-10-29)

**(a) The blind-site guard is a set, not a scan.** `settle_blind_sites` asked "is anything under construction?" by
walking every entity, every tick, and the answer was "no" in every match that was not being built in. `foundations`
is maintained where a construction starts, where it finishes and where the entity dies, and a `debug_assert!`
compares it with the world on every tick it is non-empty - so the drift a maintained set invites fails in debug
instead of silently skipping a rule.

**Measured, interleaved against a `HEAD` worktree** (2,000 finished buildings, no foundations, release, 1,000 ticks
each): **68.5-70.1 µs/tick on this tree against 71.5-72.0 on `HEAD`**, and **the state hash is identical**
(`7a7f41250fdd3ab8`) - so the change is behaviour-neutral and worth 2-4% in a world that never builds anything. It is
an O(n) -> O(1) change, so the payoff grows with the world; at 2,000 entities it is real but small.

**(b) The pack's counts became a relationship.** `pack.rs` asserted the dat's 801 techs and 798 effects; it now
compares the pack's tables against a direct load of the same files, which is what a *pack* test can prove (it loaded
the files it says it loaded) and which survives a pack that grows. The dat's census stays pinned in `techs.rs`, where
the number is the subject rather than a side effect.

**(c) An empty body has a lifetime, derived rather than invented.** A soldier's kill leaves a carcass with nothing in
it and - after item 4 - no rot to work with, so it stood on the map for the rest of the match. Every body now carries
an expiry tick as well as a rot rate: **its species' own meat divided by its species' own rot rate** (850 s for a
boar, 400 s for a sheep), so a pack whose animals change changes the lifetime with them. Nothing was invented, and
the test asserts both halves - present at a minute, gone inside eighteen.
