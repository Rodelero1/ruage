# Prior-art watch

Awareness of open-source AoE2 engines and tooling, on an adaptive schedule, without burning tokens
on projects that have not moved.

## The rule (D14 amendment)

The owner's rule, implemented against **upstream** activity rather than our own calendar — our
counters drift, and a missed tick would silently age the whole list:

| project last touched | look at it |
|---|---|
| within 2 days | every 2 days |
| within a week | weekly |
| quieter than a week | every 2 weeks |

Plus one override that matters more than the table: **a project is due the moment its head commit
changes**, whatever its idle time. That is how a project that has been quiet for a year and then
comes back to life is caught on the day it moves, rather than up to a fortnight later.

## How it runs

- `tools/prior_art/watch.py` holds the watchlist, queries GitHub for each repo's head commit, applies
  the rule, and prints **only** the projects that are due. Empty output is the normal case.
- Cron job `a10883e59b0a` runs the script daily at 09:00 UTC. The script's output is the gate: when
  it is empty the agent replies with one line and does nothing else.
- State (last head sha, last look) lives in `build/prior-art-state.json`, which is gitignored, so the
  watch cannot dirty the tree it is watching. A second state file,
  `build/prior-art-discovery.json`, dates the monthly **discovery sweep** and remembers which
  candidates have already been shown.
- `/opt/data/scripts/prior_art_watch.py` is a **shim** that runs the repo copy. It exists because the
  scheduler requires its scripts in its own directory; it must stay a shim or the two copies drift.

## 2026-10-30 — the watch now discovers, not only diffs

Owner decision: `yanfengliu/aoe2` joins the list, and the gate gains a monthly **discovery sweep**.
The reason is the shape of the old tool, not a new interest: a closed list of twelve can only ever
answer "did a project we already know about move?", so "no new relevant project" was an artefact of
the list rather than a finding. A keyword sweep found a peer project the list had never heard of.

**The sweep.** Once every 30 days the gate queries GitHub's search API (`aoe2 engine`,
`"age of empires" engine`, `age of empires ii reimplementation`, `topic:aoe2`,
`aoe2 deterministic simulation`) and prints what it finds as `DISCOVERY` lines: candidates for the
owner to judge, never report material. Its state is `build/prior-art-discovery.json` (gitignored),
and it advances **only on a complete sweep** — one dead query is named on stderr as
`DISCOVERY FAILURE` and fails the run, exactly as a failed `head_commit` does, so a rate-limited
sweep can never pass as a quiet month. The search endpoint is a separate rate-limit bucket from the
per-repo commit reads (10/min unauth against 60/hour), so the sweep cannot starve the gate.

**What it filters, and why the filters are shaped that way.** Forks, archived repos, repos with no
description (nothing to judge), repos untouched for 90 days, curated lists, SEO drill pages, and our
own account. The query set searches names, descriptions and topics only: measured against live data,
adding `readme` took the candidate list from 35 to 57 without finding one extra project worth
watching — the extra 22 were a "point Claude at any game" mod pack, decompilation directories, a
starred-repos mirror and an emulator.

**Only what was printed is remembered.** The report is capped at eight candidates a month, and the
cap deliberately bites: candidates beyond it are *not* marked seen, so they surface next month rather
than vanishing. The first draft marked everything seen and would have discarded the overflow
permanently.

**Test-first, and what the tests caught.** 45 tests; every behaviour above was watched failing
first. Three defects were found reviewing the change rather than by running it: the overflow bug
above; a description containing a tab or newline forging a column (descriptions come from strangers'
repositories, so their whitespace is flattened); and equal push times reordering the report between
runs (ties now break by name). A fourth was found by the suite itself — the pre-existing
`GateTests`/`FailureTests`/`MixedRunTests` classes call `main()`, so the new sweep made the test
suite a real GitHub client, which showed up as a 7-second run and a written state file. Every class
that runs `main()` now isolates the discovery state and injects a search stub.

**A pre-existing rot fixed on the way.** `test_unchanged_within_interval_is_silent` failed on
2026-10-30 for reasons that had nothing to do with this change: it pinned `NOW = 2026-10-27` in the
test while the gate read the wall clock, so a project "looked at six hours ago" was three days old
and correctly due. `main()` now takes an injectable clock (`main(now=...)`), and the suite pins it.

**What the sweep found on its first real run.** 31 candidates, none of them a rival engine — replay
analysers, a Lua scenario tool, a DE patch reverter, a rating chart. The engine-shaped project it was
built to find was already found by hand the same day; from here on it is the list's job.

## What is watched

Reference engine:
- `SFTtech/openage` — the asset pipeline (D18), the engine/content split, and a determinism posture
  that is a contrast rather than a model.

Deterministic reimplementation and formats:
- `librematch/genie` — active; format readers and a DE-targeting deterministic implementation (D13).
- `Zaydme/openaok` — from-scratch AoK (1999) engine, C++20/SDL2/EnTT, MIT; fixed 16 ms tick, its own pathers, reads `EMPIRES2.DAT` at `VER 5.7`. The nearest sibling to this project, and the one that answers questions about our own dat version.
- `yanfengliu/aoe2` — TypeScript replica on its own sibling engine `yanfengliu/civ-engine` ("AI-native,
  civ-scale simulation"); MIT, and **original art by choice** — no AoE assets — which makes it the
  only watched project working D18's problem from the far side. Ships `design/spec-final.md` as its
  rules target, a coverage figure derived from its own test suite, and an LLM-vs-AI playtest loop
  with a recorded findings log. Came in by the discovery sweep's first principle: it was not on any
  list before 2026-10-30.

Dat modelling we depend on:
- `SiegeEngineers/genie-rs` — **dormant since November 2022**, so it will not answer U15 or U17; kept
  on the list because it is still the cleanest crate-level model of the format.
- `SiegeEngineers/genieutils-py` — an independent reading of the same format.

Asset pipeline (D18):
- `SiegeEngineers/sld-extractor` — DE sprite extraction.
- `SiegeEngineers/WololoKingdoms` — HD-era file conversion (what changes between HD and DE).

Published-value cross-checks for open questions:
- `SiegeEngineers/aoc-reference-data` — updated most days.
- `SiegeEngineers/halfon`, `SiegeEngineers/aoe2techTree` — unit/building/tech references.

Later steps:
- `SiegeEngineers/rms-check`, `SiegeEngineers/rms-doc` — the random map script language.

## What a report must contain

1. **What changed**, one sentence per commit or release.
2. **Divergence**, on the axes that matter: asset pipeline and format (D18), engine/content boundary
   (D17), determinism and tick model, platform targets, language and build, licence. A better
   approach than ours is reported as better, with the cost of adopting it.
3. **Study list**, with pointers specific enough to open.
4. **Never copy** — GPL-3.0 sources are read for ideas; nothing crosses into `crates/`.
5. **Open questions** — any U-number a source now answers.

## The relevance filter

The long-term plan: an open-source engine for **Linux, Windows and possibly macOS**, keeping the Age
of Empires II feel and dynamics, running initially on the original assets and ready for original
ones. A project outside that (a different kind of game, unusable licence, graphics-only toolkit) is
reported only if it teaches something usable anyway — and then the divergence is stated, not implied.

## 2026-10-27 — the watch tool was missing, and now it is not

The daily job failed with `FileNotFoundError: tools/prior_art/watch.py`. The script had never been
committed, and the working copy was gone, so the shim could only fail. Two runs were spent surveying
by hand with state that could not advance.

Fixed, and the fix is the point:

- `tools/prior_art/watch.py` is committed, with `tools/prior_art/test_watch.py` (16 tests; `python3
  -m unittest discover -s tools/prior_art`). The failure mode of a watch is **silence**, so the
  tests pin what "due" means rather than what the code looks like: never-looked, head-moved,
  interval-expired, and the quiet case.
- A failed fetch now **fails the run** (exit 1, `WATCH FAILURE <repo>` on stderr) even when other
  projects are due. Before that, one failed repo was hidden by one due repo — the exact shape of
  silence this tool exists to avoid.
- A failed fetch does **not** advance that project's state, so the next run tries again.
- An unreadable or naive timestamp makes a project due, not cold.
- State keeps `previous_sha` beside `sha`, so a look can always diff against the sha the last
  report saw.
- `Zaydme/openaok` joined the list; it is the only other project reading the same 5.7 dat.

State still advances when the script fetches, not when the report is written. That is a deliberate
trade: if a run dies between the two, the project goes quiet for at most its interval instead of
being reported twice a day. The alternative — ack-on-report — makes a forgotten ack repeat forever,
which is noisier but no safer.

## 2026-10-27 (second pass) — the effect kinds, named twice and confirmed in our own data

All twelve heads are the shas the last look recorded, so nothing was diffed this pass. What follows
came from reading those heads, and one of them answered three open questions.

**Two independent implementations name the same kind table.** `librematch/genie` (DE, AGPL-3.0)
`crates/engine/genie-core/src/technology/effect/mod.rs` has `Kind::SET 0, RESOURCE 1, ENABLE 2,
UPGRADE 3, ADD 4, MULTIPLY 5, RESOURCE_MULTIPLY 6, SPAWN 7, TECH 8`, and `TECH_COST = 0x65` (101),
`TECH_DISABLE = 0x66` (102), `TECH_TIME = 0x67` (103). `Zaydme/openaok` (AoK 1999, MIT)
`src/core/Rules.h:330` names 0–6 and 101–103 identically, and `src/ecs/Effects.cpp` gives the field
roles: `SetAttribute/AddAttribute/MultiplyAttribute` (a=unit, c=attribute, d=value), `EnableUnit`
(a=unit, b=1 on / 0 off), `Resource` (a=resource, b=0 set / 1 add, d=value), `TechCost` (a=tech,
b=resource, c=0 set / 1 add, d=amount), `TechTime` (a=tech), `DisableTech` (d=tech id).

**Our data confirms it without either repo.** Across the 97 kind-101 commands in
`config/gamedata/effects.toml`, `b` takes only 0, 1, 2 and 3 (37/17/1/42 — exactly the four
resources), `c` only 0 or 1 (58/39), and `d` is an amount, including negatives. The effects holding
them are the ones whose published behaviour is cheaper technologies: effect 10 "Lombards Technology
Tree" prices Feudal/Castle/Imperial at 425/680/850 food and then *adds* −200/−157/−250; effect 5
"Magyars Technology Tree" zeroes 67/68/75; effect 3710's `[101, 197, 0, 0, 0.0]` is a free tech.

**U16 closes.** Kind 101 is a tech cost and its `a` is a **tech id**. The census's "84 present in
both tables" is made of techs that happen to share an id with a unit (202 = double-bit axe, and a
unit id); no unit reading survives, because enabling a unit is kind 2. `Meaning::AmbiguousId` should
become a tech reference — test-first, in `crates/data`, against effect 10's five age prices and
effect 5's three zeros.

**U2's open kinds are answered**, subject to the same test: 0 set attribute · 2 enable unit ·
6 multiply resource · 101 tech cost · 102 disable tech (`d` = tech id, which matches U13's finding
independently) · 103 tech time. Kind 3 = upgrade unit and 4/5 = add/multiply attribute were already
pinned, so every kind our data uses is now named.

**New, and not in any U-row: the tens digit is the party.** genie's `Party::of_block(c.kind / 10)`
maps 0 to the player, 1 to mutual allies, 2 to mutual enemies, 3 to neutrals, 4 to Gaia. Our data
confirms it in place: the effect at `effects.toml:3770` carries `[11, 50, 1, -1, 1.0]` beside
`[1, 50, 1, -1, 1.0]` — the same grant to self and allies; `:3656`'s `[15, 82, -1, 13, 1.25]` is a
team-wide gather bonus; `:4028`'s `[11, 3, 1, 13, 500.0]` gives allies 500 gold. So kinds 10–49 are
the six base kinds retargeted at another party. openaok does not implement this at all
(`Effects.cpp`'s `default: break`), so the decoding is genie's, from the DE binary; our data is the
independent check.

**Gap this exposes.** 97 kind-101 and 25 kind-103 commands sit in `effects.toml`'s `commands` and
have no counterpart in `modifiers` — effect 10 carries twenty of them and only its kind-5 multiply
survived the import. The TOML keeps the raw command, so nothing is lost, but the schema has no
`tech_cost`/`tech_time` modifier and the effect interpreter cannot see them.

**U12 is still unanswered for HD.** genie's DE recordings carry `dt` per frame (genie-oracle runs
`frame.dt`; its fixtures use 20 ms), which is a property of DE's recording format, not of HD's
clock. openaok chose 16 ms by hand, commented `~60 Hz`, which is 62.5 Hz and 3.125 ticks per 50 ms
datum — its own second-based values land off the grid ours were chosen to land on (D16). Twenty
hertz stays a choice, and still the cheaper one. openaok's A* is likewise its own design, not
evidence about the original's pathers (U7).

**Divergence worth naming.** openaok targets macOS, Linux and Android, and not Windows. Its engine is
MIT while its build-time data tools link LGPL-3.0 `genieutils` — the boundary D12/D21 draw for us,
working in practice.

**Study list.** genie `docs/ENGINE_SYSTEMS.md` (nineteen systems, tick order, per-folder READMEs);
`crates/engine/genie-core/src/world/world_update.rs` — only the player whose turn it is runs its unit
AIs' full update, and the turn rotates each tick, so AI cadence in the original is per-player-turn,
not per-tick; openaok `src/verify/` — forty one-scenario-per-file checks, `Determinism.cpp` among
them, the shape our adversarial pass keeps reaching for.

**Licence.** genie is AGPL-3.0: read, nothing crosses. openaok is MIT and may be read freely; its
`third_party/genieutils` is LGPL-3.0 and must stay in tools. A field-role table is not
copyrightable, but the code that carries it is — "it was only a small function" is how that line
gets crossed.

## 2026-10-28 — two ports rebuilt around the tick, and a foundation rule we do not have

Four heads moved: `genie` 74331ec6→f30f36dc (38 commits), `openaok` 092ed1ad→d1b237ed (32), and
`SiegeEngineers/halfon` plus `aoe2techtree` to DE update 188329. The census this file relies on
was re-derived rather than trusted: `effects.toml` holds 97 kind-101 and 25 kind-103 commands,
kind 101's `b` is only 0–3 (37/17/1/42) and its `c` 0/1 (58/39), and the class totals are 0:106, 1:84,
2:117, 3:227, 4:1091, 5:576, 6:2, 11:2, 15:1, 102:942. The 2026-10-27 findings hold.

**genie: an object update no longer writes the world.** FD-045 has taken the core apart so that
`Unit::update` cannot touch another object: it writes into `ObjectOutputs` (40 channels, plus 15
unit-AI effects and 51 hand-offs) and `World::step_object` in `world/after_step.rs` applies them
after the update in a fixed order. The new `docs/EFFECTS.md` (137 lines, generated by
`scripts/effects.py`) lists every channel with the routine that emits it and where the caller
applies it: attribute adds, techs done, kills, releases and takes, a counted builder leaving,
remains and refunds, promotions, captured buildings, flare and sight events.

Two things in that tree are worth our attention, and neither is the code. First, `docs/ENGINE_SYSTEMS.md`
documents one tick: the command phase, then Gaia and players 1–8 in turn, each running gates open,
groups, the player's own update, every live object in list order, list removal, explored tiles
paying out, gates shut; after the loop the fog ghosts, the turn passing, the victory check and the
vision masks. List removal is the game's swap-remove, which skips the object that lands in the freed
slot until the next tick. Only the player whose turn it is runs its unit AIs fully.
Second, the file names in that system table are, per the doc, those that "the HD edition's debug
symbols name" (`act_atak.cpp`, `act_list.cpp`). If HD shipped symbols naming its routines, then
names for the update routine and for the pathers are available against the copy we already own —
which is exactly the anchor U12's breakpoint and U7's pather lack. Unconfirmed; one probe on our
own install would settle it.

Determinism, from the same doc and worth borrowing as a property rather than a design: the game
has eleven WELL512a streams, all re-seeded from the frame seed at the start of every tick, so no
draw reaches the next tick; equal elements sort by MSVC introsort because the order of equals
shows; objects update in list order. We have no RNG yet. When one arrives, seeding per tick from
the tick number is the cheap property (not eleven streams, not WELL512a).

U12 is still open, and genie now states why it always was: `tick(dt: u32)` takes the interval from
the recording's frame and no constant exists in the tree. Twenty hertz remains ours by D16.

**Divergence on the tick's side effects.** Deferred writes buy one thing: within a tick no object
observes another's half-finished update, so results depend on the order the game applies channels,
not on the order objects happen to run. We mutate directly (`crates/sim`), and our order is already
fixed by a deterministic list, so determinism is not at stake — this is fidelity. Adopting it would
cost a context plus an applier for behaviours we do not have yet (a kill ordering into refunds, a
promotion, a release). Record the order, borrow the checklist when units first affect each other,
restructure nothing now.

**openaok is now a console game.** A Nintendo Switch homebrew build with its own packager,
on-device network logging and file fetch, art decoded as it comes into view, plus an Android JDK-
21 build fix; releases are marked pre-release. Targets are now macOS, Linux, Android and Switch —
still no Windows. Not our plan, but it is the best evidence yet that an engine on that many
platforms needs the split their `src/hal/` makes: window, per-machine folders, touch and controller
played as a mouse, DRS archives and SLP decoding, texture atlases, audio device, sockets.

**Their asset path is the opposite of D18.** Sprites come straight from the SLPs in
`graphics.drs`, decoded on first sight and cached as atlases; there is no intermediate pack. That is
the cheapest way to run on install art and the least ready for original art. Our asset pack layer
keeps earning its cost.

**A mechanic we lack: a foundation holds no ground until first worked.** Their `docs/VERIFY.md`
`foundations` row: a placed foundation blocks nobody and is nobody's target, is seen only by its
side, and holds no footprint sight until a villager starts on it; delete refunds follow from that.
Our `Walk` grid is terrain only (`crates/sim/src/path.rs`, "terrain does not change during a match
yet"), so we neither block nor diverge yet — but when buildings start to block, the foundation must
not block until worked, or every villager's placing path changes under us.

**Spies/Treason, and a data identity that beats an id.** They find the tech by its effect, not its
number: `src/ecs/Techs.cpp:183` calls a tech Spies when its effect sets dat resource 183 above zero.
Our data agrees without being asked: effect 420 "Spies/Treason" is `[1, 183, 0, -1, 1.0]`, effect
673 "C-Bonus, Hide Enemies" is `[1, 183, 0, 0, 0.0]`, and tech 408 carries 420 at 200 gold,
research_time 1, requires 103, location 82. Their price scales that cost by the enemies' civilian
population (resource 37, clamped to 1–150) and a per-player rate (resource 85); our 5.7.2 dat holds
a flat 200 gold, so if HD scales, it scales in code. Recorded as a mechanic, not adopted: Spies is
far past where the sim is.

**A frame budget that does not touch determinism.** `src/main.cpp` gives the simulation 25 ms a
frame and, past it, drops the accumulator backlog and plays slower instead of spiralling; the
lockstep branch is untouched. They pair that with sight tables kept between ticks, an ally-sight
table, a bucket index for target search, and gates and herds skipped when out of reach. Ours is
tick-driven with no wall clock in the loop, which stays the simpler position.

**halfon and aoe2techtree: DE update 188329, nothing for us.** Both mirrored the balance patch —
elephant archer 873/875 to 70 food and 220/270 hp, a couple of attack values 3→2, a speed 1.5→1.45,
an armour class pair 1→2. Both track DE exclusively, so neither is evidence about 5.7.2; their value
is as a DE patch calendar, not as data (D5 keeps the dat authoritative).

**Registry moved.** `docs/mechanics/README.md` now marks U16 and U2 resolved, with the evidence
pointed back here. The code they unblock (`Meaning` carrying a tech reference; `tech_cost` and
`tech_time` modifiers for the 97 and 25 commands that have none) is still owed, test-first.

**Study list.** `genie` `docs/ENGINE_SYSTEMS.md` ("One tick", "One object's update", the determinism
section) and `docs/EFFECTS.md` as a map of what an update touches; `openaok` `src/ecs/Spies.cpp`
with `src/ecs/Techs.cpp:183` for effect-as-identity, and `docs/VERIFY.md`'s `foundations` row with
`src/ecs/Buildings` for the passability rule.

**Licence.** genie is AGPL-3.0 and its `EFFECTS.md` is generated from *its* ports: the channel list
is a fact about the game, the table, the script and the Rust types carrying it are not ours to
reuse. openaok stays MIT with LGPL-3.0 `genieutils` confined to build-time tools. "It was only a
small function" is how that line gets crossed; write the expression, not the file.

**Correction to the 2026-10-27 entry.** Its `effects.toml` line numbers (`:3770`, `:3656`, `:4028`)
were read before that file was regenerated on 2026-10-28 and no longer point at those commands. By
effect they are 626 "T-Bonus, Free Cartography" (`[11, 50, 1, -1, 1.0]` beside
`[1, 50, 1, -1, 1.0]`), 607 (`[15, 82, -1, 13, 1.25]`) and 669 (`[11, 3, 1, 13, 500.0]`). Cite
effect ids and command tuples, never lines in a generated file.

## 2026-10-29 — genie makes its tick a schedule and its divergence a field; openaok measures reach to the wall

**genie, 48 commits (`f30f36dc..19a3ed54`): the update became a declared schedule.** The steps now
live as `BEFORE_PLAYERS`, `EACH_PLAYER` and `AFTER_PLAYERS` arrays over `WorldStep`/`PlayerStep`
enums in `crates/engine/genie-core/src/world/schedule.rs` — 20 steps, each naming the DE routine it
ports (`FUN_141552990`, `world+0x18c`, slot 0x100) — and `scripts/schedule.py --check` renders them
into `docs/ENGINE_SYSTEMS.md`, so the doc cannot drift from the code. `World::tick_observed` runs the
same steps and shows the world after each one, which is what the new `harness steps` compares against
the game's state at the same point.

The `Context`/`ObjectOutputs` design from last week survives but is **shrinking**: the regenerated
`docs/EFFECTS.md` counts 15 world outputs, 15 unit AI effects and **46 hand-offs, down from 51**,
because stage B moved the hand-offs the game makes *inside* the update to the call site —
`world/lets.rs` lends the object or the world for a call that may meet it, so a kill reaches the
world at the hit, a flare at the volley, a promotion where the master changes, and a gate opens and
shuts within its own update. One such change (a builder's completion) was committed, reverted and
redone, which is the honest signal that the aliasing they were designing around is real.

**The comparison got a type.** `crates/base/genie-diff` derives `Diff`: two values of the same type
compared field by field, each difference carrying its path (`life.hp`, `actions[0].to[1]`,
`garrison.len`) and both sides' text. `#[diff(unsaved)]` marks a field the game's save does not hold;
`#[diff(gap = "why")]` marks a *known* difference, and the gaps are, in their words, "the work list
of what the comparison cannot check yet". Their oracle diffs every saved field of every object after
every step.

Also in this range: `genie-command-codec` reads and writes a recording's command packets as
genie-core's command structs, each command a struct with one `run` and one dispatch, its fields in
newtypes; `ActionKind` and `OrderKind` replace the unit AI's numeric action and order codes;
`scripts/lint_allows.py` fails on any `#[allow]` not registered in `.config/lint-allows.json`.

**openaok, 38 commits (`d1b237ed..c050533c`): it is now the better model of the original's geometry.**

- `58574aa` — **reach is measured between the dat's boxes.** `Combat::BoxOf` gives every entity a box
  (a building its footprint, anything else the box its dat size spans about its middle) and `Gap` is
  the straight distance between the two boxes, zero where they overlap. Every attack uses it now, not
  only the dead zone, and their comment carries a data fact worth having: "the dat's axes are a
  quarter turn from the grid's: its y extent is the grid's x".
- `b005beb` — **units pass over one another; nobody is pushed or slid.** A per-tick `Traffic` survey
  of bodies (`src/ecs/Navigation.h`) lets a route go *round* units standing still, motion ignores
  other units, and a unit stops when it comes up against a body standing on its goal. A crowd sent at
  a job takes side-by-side spots and waits for one to free (most of the 180 lines in
  `WorkerSystem`), checked by a new `src/verify/CrowdCheck.cpp`.
- `cc23f09` garrisoned archers and villagers each add an arrow, their damage a second over the
  building's own; `26a721c` an Ungarrison button; `f38a550` a garrison flag.
- `6c64468` a ship or a trebuchet turns to its target at the dat's turn rate before it shoots.
- `e357f88`, `1fed548`, `e40e1e3` — placement by the original's rules: uneven ground by the hill
  rules, a dock's middle on water with beach beside it, a gate's outline along its wall.
- `c86edbf` — the roadmap gains a seventh step, **The Conquerors**: the expansion's dat, archives,
  strings, scenarios, random map and AI scripts read from the player's own copy, the 1999 game
  unchanged. Same direction as us (install data first), arrived at independently.
- `c050533`, `320983a` — GitHub builds the Android APK and attaches it and the Switch zip to a
  release. Still no Windows target.

**Divergence — reach, and this one is worth acting on.** Ours compares **centre** distance to the
range and pads by both bodies' radii only for `min_range` (`crates/sim/src/lib.rs`, the `in_reach`
branch of `step`). So a skirmisher can hit a town centre from farther out than the original allows,
because the wall is nearer than the middle, and a building's "8 tiles" is eight tiles from its middle
rather than from its wall. The fix is bounded: units already carry `radius`
(`crates/data/src/units.rs:30`) and a square of `2r` is their box; the missing datum is a building's
footprint, which our schema does not have at all — a building is one tile in `Walk` (D54). **Cost:**
emit a footprint per building, measure attacks and approaches box-to-box, re-anchor the tests that
assume centre distance. Do it when buildings stop being one tile, not before.

**Divergence — the crowd at a work goal.** D52 says work goals are *not* exclusive, deliberately:
two villagers must be able to work one tree. openaok now does the opposite for work too — spots,
claimed and waited for. That is closer to what a player sees (villagers fan round a tree rather than
stacking) and it costs exactly the thing D53 declined to build: a claims table with a release on
every order end, plus a deadlock rule for when the last spot is taken. Recorded as a known
simplification; revisit if a probe ever shows villagers piling onto one tree.

**Divergence — routing round units.** Their routes bend around still bodies from a survey taken at
the top of the tick; ours go through anything that is not terrain or a wall (D54). We already keep
the bitset of unit tiles D52 paid for, so the layer is nearly free; the cost is route invalidation
when a body moves mid-route. The gain is crowd scenes only, so: noted, not scheduled.

**Better than ours, and cheap — the verify gate.** `docs/VERIFY.md` documents `--verify 3000`: one
headless run, named scenarios each printing one line, checked line by line, about thirty seconds.
One of those lines is `fixed point`: **no `float`, `double` or `ToFloat()` in the simulation's
sources.** Ours has `f64` in `apply_scalar` and the hp setup; if those are load-time only, a grep
gate over `crates/sim/src` costs one line of CI and catches the first slip into the tick. Worth
checking before asserting it.

**Same answer, better shape — divergence as data.** genie's `#[diff(gap = "why")]` makes every known
divergence an annotated field, so the list cannot rot the way a paragraph can. Adopting the *shape*
for our oracle comparisons (a table of field → why we differ, asserted by a test) costs a small
registry and no AGPL code.

**U12 has a third corroboration, for the wrong build.**
`crates/tools/genie-oracle/src/step_runs/tests.rs:18` writes
`genie_rec::de::Op::Sync { dt: 50 }`: a recording's sync carries its interval in milliseconds and
genie writes 50, as do its core tests ("the forager's whole updates (50 ms each)",
`world/tests.rs:12562`). genie reproduces DE build 158041 bit for bit, so 20 Hz is the step of the
one implementation that would fail if it were wrong — for DE's data, not for our 5.7.2. U12's status
is unchanged in kind and stronger in weight: two measurements, plus the bit-exact port's step.

**U7's shape is now documented, and it is not one pather.** `docs/ENGINE_SYSTEMS.md`'s systems table
names the game's pathing as *the tile pathfinder, the long pather, the short pather, the old hull
pather, the obstruction mesh and the zones* (`path.cpp`, `pathsys.cpp`, `mpathsys.cpp`,
`zone_map.cpp`). openaok independently ships one integer-cost A* per passability layer, flood-filled
regions, an `Approachable` test and an `ApproachPlanner` that spreads standing spots along a
building's sides — and is playable. Read together: exact arrival times would need the original's
stable of pathers, and a single A* with regions and an approach rule is the bounded-scope answer,
now with a peer that took it. Neither says *which* algorithm the original uses, so U7 stays open.

**Worth recording, not new: the unit AI runs for one player per tick.** "Only one player's unit AIs
run their full update in a tick: the player whose turn it is; the other players' only take their
orders and notifications" — pre-existing in `ENGINE_SYSTEMS.md`, read here beside the schedule. It
is the answer to why a per-unit stagger (`combat.aggro_scan_ticks`) is the wrong *shape*: the
original round-robins players, not units. Written into `determinism.md` as a divergence; changing
ours would shift every existing test's timing for something no player can see.

**Study list.** genie: `docs/ENGINE_SYSTEMS.md` (the generated schedule table, then "One object's
update"), `crates/base/genie-diff/README.md` with `src/lib.rs` (`Scope`, `SavedWithGaps`),
`world/schedule.rs` and `world/lets.rs`, `docs/DEBUGGING.md` for `harness steps`. openaok:
`src/pathing/Pathfinder.h` (regions, approachability, `Standing`), `src/ecs/Navigation.h`
(`Traffic`), `src/ecs/systems/MovementSystem.cpp:108` (the pass-over rule in three lines of
comment), `src/ecs/Combat.cpp` `BoxOf`/`Gap`, and `docs/VERIFY.md` as a model for a one-command gate.

**Never copy.** genie is **AGPL-3.0**. The schedule table, the diff derive, the command codec and
`EFFECTS.md` are its expression, generated from its ports; the facts they encode — the update's 20
steps and their order, the 50 ms sync, the effect channels — are facts about the game, to be written
in our own words and our own types. `BoxOf`/`Gap` is nine lines and MIT, and copying it would still
be copying, and would drag a footprint box into our sim before we have decided buildings are more
than one tile. openaok stays MIT with LGPL-3.0 `genieutils` confined to build-time tools. "It was
only a small function" is exactly the sentence this rule exists against.

## How the open-source projects handle "a worker keeps working" (2026-10-29)

*Read for item 9, after the acceptance match stalled with every villager idle. Censused from source, not from memory:
0 A.D. `master`, OpenRA `bleed`, openage `master`.*

**None of them publishes the test item 9 wants.** 0 A.D. tests components (57 files under
`simulation/components/tests/test_*.js`), openage tests units and demos (`openage/testing/testlist.py`), OpenRA tests
traits in NUnit. No project has "play this build order and tabulate the age timings" - balance is a human activity
there, measured with a bot, not with a test. So our scripted-timing approach is unusual, and it is unusual because
the owner asked for exactly that; nothing to copy, and nothing to be embarrassed about either.

**What they do solve is the behaviour, and 0 A.D.'s shape is the one worth taking.** `UnitAI.js` has an explicit
`FINDINGNEWTARGET` state, entered when the supply runs out, and it tries four things in order:

1. **Bank what is in hand first.** If the unit is carrying that resource and the *next* queued order is not a gather
   of the same resource, a `ReturnResource` order is spliced in front of it. A load is never silently abandoned.
2. **Find a neighbour, of the same *specific* type.** `FindNearbyResource` takes a filter, and the state's own filter
   refuses the previous target and keeps the type: *"Don't switch to a different type of huntable animal"* - and for
   meat, the same template. A hunter whose deer dies keeps hunting deer, not boars. The search itself filters on
   can-gather, **visible**, *available to this player*, in a 64-tile radius, with a TODO nobody has taken:
   *"extend this to exclude resources that already have lots of gatherers"*.
3. **Walk back to where the order started and look again**, on the theory that the next resource may be in fog.
4. **Give up at the dropsite, not in the field.** If there is still nothing, it walks to the nearest dropsite
   *"because that's a nice enough place to congregate and idle"* and finishes the order.

**OpenRA answers the same question with an activity that is re-entered every trip**: `Harvester` queues
`FindAndDeliverResources`, so a harvester never holds a target it has finished with.

**openage answers it with data.** `libopenage/gamestate/activity/` is a node graph - start/end nodes, task nodes,
`xor` gates, and events like `wait` - and `doc/code/game_simulation/activity.md` states the motivation in almost the
words of this problem: *"commands do not always translate to a single well-defined action ... branching paths, wait
states for events, triggers and feedback loops"*. "The resource ran out" is an edge in a definition, not a branch in
code.

**And the policy half is a bot, not a script.** 0 A.D.'s Petra keeps `targetNumWorkers` and `supportRatio`, derives
`wantedGatherRates` from its current plans, and each turn calls `pickMostNeededResources` - *"pick the resource which
most needs another worker"*, by the ratio of wanted to current rate. It also carries `lastFailedGather` per resource,
so a resource it could not serve is retried *differently* rather than in a loop. That is the "keep four on wood" a
timestamp script cannot say, and it is a policy over the same `Gather` orders the player uses.

**What we take:** the four-step chain (in that order), the specific-type filter, and a `lastFailedGather` analogue so a
worker that cannot reach anything stands down *visibly* instead of retrying for ever. **What we can do better than
them:** the crowding rule they left as a TODO - a node with N gatherers already on it should not attract an N+1st
while an idle one stands elsewhere. **What we deliberately do not take:** a full activity-graph engine (openage's
answer, and a rewrite of the tick for a benefit our one worker type cannot use yet).

## 2026-10-30 — genie deletes its deferred writes, and the original's tick turns out to be ordered

**genie, 107 commits (`19a3ed54..e7cc7766`): the effect channels are nearly gone, and the change is a
deletion.** `docs/EFFECTS.md`, regenerated by `scripts/effects.py`, now counts **0 world outputs, 14 unit
AI effects and 4 hand-offs** - 15 world outputs, 15 unit AI effects and 46 hand-offs yesterday, down from
51 the day before. `world/context.rs` is gone.

The `Context` had held **21 copies** of world and player values taken when an object's step began (the
world's time, construction rate, the turn's player, every player's attribute 97 and 204, the sight-takeover
flags, the dt in seconds); each now reads through the world (`c.w.world_time`, `c.w.players[p].computer`,
`seconds(c.dt)`), because that is what the game's objects do, and `PlayerFrame` went with them. The
replacement rule is narrower than "no direct writes": a call on **another** object is the world's routine,
at once; only what the update does to **itself** waits. The reason is mechanical - the acting object is out
of its slot for the length of its update, its stand-in findable by no id, so a self-call goes to the `Step`
(`world/step.rs`: the world, the id, dt, tick, the turn, and four small lists) and `World::step_object`
(`world/after_step.rs`) applies it after.

**That leaves the original's tick a sequence of objects, and ours is not one.** A call on another object
taking effect at once, combined with live reads of the world's and the players' fields, means a later
object in a tick sees what an earlier one did *this* tick - including a kill, since a dead object's actions
are torn down in its own life step before its head action would run. Our `Sim::step` (`crates/sim/src/lib.rs`)
does the opposite: one immutably-borrowed pass decides every entity's work against the state the tick
opened with, collecting `deltas`, `strikes`, `shots`, `landings`, `deposits`, and a second pass applies
them, so two units can land killing blows on each other in the same tick and both die, and a unit's order
of business is never affected by what a neighbour did to it earlier in the same tick.

**Three smaller changes of shape.** T3 gives each walking action its own `MoveTo` component holding its own
point (`+0x38`), and `walks_to` can no longer fail - two `unsupported` sites go (829 -> 828), with the
shared `Mover` keeping the point only for the four actions not yet migrated. T14 defines each repeated
literal once (the master type bytes, classes, ids, action states, player attribute indices), replacing
literals rather than aliasing them. T12 makes `World: Send + Sync` with `Spatial` losing its interior
mutability, so worlds can be run in parallel - many worlds, never threads inside one update.

**Panic-freedom became a campaign (FD-043 T11).** An index the input supplies - from a recording, a save,
the `.dat` - stopped being `v[i]`: `orders/` went from 234 index expressions to 1, `world/` 57 to 8,
`unit_ai/` 48 to 8, and every reader crate (`genie-dat`, `genie-save`, `genie-scx`, `genie-rec`, `genie-drs`,
`genie-cpx`) returns an error where it used to `expect` or `unreachable`, with the panic lints switched on.
They state the rule as a style rule: `v[i]` is a hidden `unwrap`, an id survives a list change and a
position does not.

**New in the repo, and the part worth an hour: `CODING_STYLE.md` §12, "Determinism".** The numeric type is
kept field by field; each sum and product is rounded on its own (no fused multiply-add, no `a * b / c`
rewritten as `a * (b / c)`); `wrapping_add` and its kin where the game's integers wrap, because a debug
panics and a release wraps and the two runs then differ; `/` and `%` (truncating, as the game's C++) rather
than `div_euclid`; float to integer through `cvttss2si` (`math::conversions`), not Rust's saturating `as`;
the game's own float functions (`math/fmath/`); its RNG stream in its order; **the game's list order - a
removal swaps the last element into the free place, never an order-preserving removal**; **sorts are MSVC's
unstable introsort** (`math/msvc.rs`), ties as the game's; and **no `HashMap` iteration**, enforced by
clippy's `iter_over_hash_type`. §16 lists the gates (format, clippy, file length, allowance lint, folder
READMEs, effects inventory, schedule, CRAP, tests on Linux/Windows/macOS, `wasm32-unknown-unknown`, doc
build); §15 says a new `#[allow]` fails `scripts/lint_allows.py --check` unless it carries a reason.
`cargo-mutants` moved off every push to a weekly whole-workspace run, 42,000 mutants in 256 shards.

**What this means for us, in three parts.**

- **D67's judgement holds, from the other side.** We wrote that genie's deferred-write schedule was "an
  order to keep, not an architecture to adopt". A year of work later the project that had the architecture
  deleted it, kept the order (the schedule is still 20 steps: 10 `WorldStep`, 10 `PlayerStep`), and replaced
  the channels with "call at once, and take the acting object out of the world while it runs". The idea
  worth keeping is that second half - absence resolves the aliasing a deferred write was working around -
  and we never needed it: our second pass is what a channel is for, and it is one pass, not 46.
- **The ordered tick is a real divergence and it is not our only one.** Our simultaneous-tick rule is
  defensible (it is order-independent by construction, which is a determinism advantage) but it is not the
  game's, and its visible effect is fights: two equal knights can trade blows to the death here and cannot
  there, and which of the two dies first there depends on player and slot order rather than on anything a
  player sees. Recorded in `docs/mechanics/determinism.md` and as **U18**; the cheap next step is one test
  ("a unit killed by an earlier blow still swings") to pin what we actually do, not a refactor of the tick.
- **One lint, verified available, that we do not have.** `clippy::iter_over_hash_type` exists in our
  toolchain (rustc 1.99.1) and fires on a `HashMap` walk, and this workspace has no `[workspace.lints]`
  section at all, so determinism rule 3 ("never by hash-map order") currently rests on review alone. Cost:
  one key, no code change (`crates/` holds no `HashMap` in the sim today).

**Study list.** `crates/engine/genie-core/src/world/step.rs` (the eleven fields, and the doc comment that states
the at-once/after rule in three sentences), `world/after_step.rs`, `objects/unit/update.rs` (the order:
motion, life step, animation, "actions past life", then the action list), `docs/EFFECTS.md`'s header
paragraph, and `CODING_STYLE.md` §12 with §15-16 (lints and gates). For the shift in vocabulary: commit
`94f902f` (the 21 snapshots), `ebaf8ab` (Context -> Step), `7402ee9` (the MoveTo's point), `073b2b1` and
`2b9da9c` (a group command runs as a group command, which we have no groups for).

**Never copy.** genie is **AGPL-3.0**. `CODING_STYLE.md`, the schedule arrays, `EFFECTS.md` and the effect
inventory script are its expression; the determinism section is prose written against its own ports, and it
would be the easiest thing in the repository to lift by accident because it reads like a checklist. What is
transferable is the *facts about the game* it names - lists swap-remove, sorts are MSVC introsort and
unstable, the sim is float and depends on exact FP semantics, the update order is Gaia then players then
objects - and those go into our docs in our words, as ours already do. "It was only a small function" is
the sentence this rule exists against, and a style guide is larger than a small function.
