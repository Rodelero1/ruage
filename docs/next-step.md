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
