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
- `Divy1211/AoE2DE_UGC_Guide` — RM scripting and data modding, and it states it covers older versions.
  The only candidate from the first discovery sweep (2026-10-31) worth adding: the rest were replay
  viewers, a LAN server, an iPad port and a rating chart.

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

## Do trees have hit points, and is there a felling phase? (2026-10-30)

Read from the dat (`empires2_x2_p1.dat`, civilisation 0), because the owner remembers trees with hit points that fall when
they are cut down:

| unit id | what | `hit_points` |
|---|---|---|
| 349, 350, 351, 413, 414 | forest trees | **20** |
| 411 | a tree variant | **30** |
| 354 | a small tree/plant | **25** |
| 59 (forage bush), 66 (gold), 102 (stone) | resource nodes that are not trees | **0** |

So the owner's memory is right about the **data**: trees carry hit points and the number varies (20/25/30), while every
other resource node carries none. What the data cannot say is whether those hit points drive a **felling phase** before
gathering: nothing in the dat expresses "cut down, then harvest the fallen trunk", and in Age of Empires II the wood
arrives as the villager chops, with the tree disappearing when its wood reaches zero.

Our engine currently matches the second reading (wood per chop) *except* for the half that the parked tests in
`crates/sim/tests/depletion.rs` describe: an emptied tree keeps its entity and its blocked tile, when the game removes the
object and frees the ground.

## Farms and the villager upgrades: what the data says (2026-10-30)

Read from our own generated tables (`config/gamedata/effects.toml`, `techs.toml`) and the dat.

**Farm capacity is a resource modifier on resource 36** (the pack's `[farm] food_resource`), and there are exactly four
in the game:

| effect | name | resource 36 |
|---|---|---|
| 14 | **Horse Collar** | **+75** |
| 13 | **Heavy Plow** | **+125** |
| 12 | **Crop Rotation** | **+175** |
| 402 | Chinese Team Bonus | +45 |

The engine already reads resource 36 for `farm_food_total(player)`, so the farm-upgrade line needs no new data - only the
rule for *when* an upgraded total applies (a new farm, or the one already standing).

**A farm is worked by standing on it and gathering at a rate** - `[farm] rate = 0.4` per second, `use_villager_rate =
false` in content, with the mechanism's unresolved note already recorded there. Farms are walkable, pinned by
`crates/sim/tests/walkable.rs`.

**Not yet verifiable from our tables:** the speed and gather-rate techs (Wheelbarrow, Hand Cart, Double-Bit Axe, Bow Saw,
Two-Man Saw). `effects.toml` names only a handful of modifier attributes - `attack`, `gather_rate`, `carry`, armour - and
**`speed` does not appear at all**, so either the dat encodes those upgrades under attribute ids our generator has no name
for, or the effects carry them as unit *commands* rather than modifiers. This is the same extractor gap that made every
unit's `attacks` list empty; it needs the attribute-name table completed before any of those numbers can be quoted.

## 2026-10-31 — genie makes every id a type; openaok puts the computer players in and lays the blows apart

**genie, 87 commits (`e7cc7766..10af9143`), and the theme has one name: T9.** Nearly every commit in the range
turns one meaning that had been an integer into its own type: `ObjectId`, `MasterId`, `TechId`, `PlayerId`,
`GroupId`, `AttributeId`, `MasterKind`, `MasterAttribute`, `ActionType`, `ClassId`, `OrderTargetType`,
`AttackCategory`, `GatherGroup`, `NotifyKind`, `ObstructionType`, `ObstructionClass`, `FormationKind`,
`EventKind`, `PlayerType`, `Relation`/`UnitRelation`; and on the map `Point`, `GroundOffset`, `Rect`,
`Segment`, `GridSize`, `Tile`, `TileOffset`, `TileRect`, `Radii`, `Footprint`, `Axis`. `ObjectId::raw` and
`impl ObjectKey for i32` are deleted - the raw value survives only at outgoing boundaries (the codec's encoder,
notify parameters, report lines, sort keys, sums of the game's integers). `CODING_STYLE.md` §5 states the rules:
one sweep per type, the raw value only at the boundary, an enum for each of the game's unions of an integer and
a flag (`Issuer`, `NearestAttacker`, `UnloadPick`, `GroupTarget`), and "parse, don't validate".

**And the sweep found a bug no one had asked for.** `Entry`'s derived `Default` gave a production-queue slot
`master: None`, which the game writes as −1; but a slot that the buffer's growth adds is *zeroed*, and the save
holds `master 0` there. `oracle_farm`'s comparison of an Empire Wars start against the game's own farms failed
on the empty slots. The fix is a written-out `Entry::default` (master 0, technology 0, no costs, button 0), and
the lesson generalises past genie: **`None` and zeroed memory are different values, and comparing against the
original's own output is the only thing that tells you which one it meant.** We hold several `Option`s whose
`None` stands for the game's −1 (`target`, `drop_site`, `inside`); nothing in `crates/sim` pins what a *saved*
world has in a slot the game would have zeroed.

**The combat read got a name.** `combat/victim_reads.rs` (new, 175 lines) is "what a hit reads of its victim":
`victim_of` (the master's values and the unit's own, the player's elevation attributes, the terrain defence of
its tile, the game's `mode_680`) and `victim_in` (both directions of alliance, whether the victim's place is a
guess, its place in its tile's list, a building's annexes). `victims.rs` gave up 246 lines to it. It is a
read-model, not a snapshot: it reads at the moment the hit lands.

**Nothing under `world/` moved.** `world/step.rs`, `after_step.rs` and `objects/unit/update.rs` carry no change
in this range, so yesterday's finding stands as written (U18: the original's tick is a sequence of objects, ours
is simultaneous).

**openaok, 104 commits (`c050533c..49d4c9b7`): the computer players, then a performance pass.** About forty
commits are AI, and not toy AI: walls on perimeters with other buildings kept off the line, attack groups made,
filled, gathered on a leader and sent once the enemy is known, the game's target evaluation (each known enemy
thing scored out of 100 for how soon the group would destroy it and how near it is, economic buildings after the
rest, walls last), Monks fetching relics, fishing ships and warship groups, allied computer players sharing what
they find and answering calls for help, difficulty scaling (the Hardest is given 500 of each resource with every
age), spies and Treason. `src/ai/` gains `Knowledge`, `Relics`, `Walls`, `Costs`. Then about thirty commits of
*measured* optimisation behind a `--perf` flag and a stress scenario (eight computer players, giant map, Imperial
Age, armies and fleets, ~3,000 units and buildings): **about a third of a millisecond a tick, ~100 ns an
entity**. Our own probe is 15-19 ns/entity at 2,126 entities with no AI, no route load and no rendering, so their
figure is a full-game budget to hold ours against later, not a target now.

The techniques worth opening: one packed sort key per unit in the tick's spatial survey, then **re-sorting last
tick's order instead of sorting afresh** (nearly-sorted, so insertion); a landmark-based A* estimate (terrain
landmarks costed with the map, re-costed only when terrain changes what a layer may cross) so the search no
longer floods the land behind water and cliffs; a search budget tied to the route's length ("a chase into a
crowd walks as far as the search got and plans again"); and a square root that starts from the hardware's and is
corrected to the exact integer floor, "the same result on every machine, many times faster".

**Two combat-timing facts, both new.** "A blow begun lands on its target" - the swing remembers its victim, so a
scout between two wolves wounds the one it swings at. And **volleys are spread on purpose**: soldiers set on the
same enemy begin their first blow up to six tenths of a second apart, so a line of archers does not loose every
arrow on one instant. That second one is a recorded deviation from the original (`docs/IMPROVEMENTS.md`), and it
is the other answer to the problem our simultaneous tick leaves open: they kept an ordered 60 Hz tick and broke
the simultaneity by hand; we broke the order and left the simultaneity.

**Their tick is 60 Hz and has always been fixed point.** `docs/ARCHITECTURE.md`: 32.32 fixed point on a fixed
16 ms tick (the README says 60 Hz), where genie's DE port is 50 ms. Two projects that both claim the original's
feel, at 20 Hz and at 60, is the strongest evidence yet that **the tick rate is a quantisation choice rather
than a fidelity requirement** - which is also what our scenario format already assumes (D16: seconds, never
ticks).

**Platform: they now ship Windows**, which leaves no host we name and not this one. MinGW-w64 build, SDL
included as `<SDL2/...>`, Winsock linked, runtimes linked in, `windows/build.sh` laying out the zip,
`OpenAoK.bat` preparing the data and starting the game, CI publishing on a tag; the data tools gained UTF-8 and
plain newlines so Windows prepares the same files. Their data preparation is a **build-time** step - unpack the
CD's cabinet, dump the dat, play from a folder - where ours is D81's runtime chain resolved by id. Simpler for
them; no mod can override an id for them.

**Verification is a product in itself.** `--verify` builds its own world from a fixed seed in about forty
scenarios and compares each printed line: a golden world hash over 3,000 ticks, "no `float`, `double` or
`ToFloat()`" in `src/ecs`, `src/pathfinding`, `src/physics`, `src/ai`, a lockstep host and client over loopback
*with a desync caught on purpose*, and one scenario per mechanic - `foundations` alone covers refunds, footprint
sight, overlapping sites, hill rules and a half-ashore dock. Our `crates/sim/tests/no_float.rs` came from this
gate; the inventory is a checklist to measure our own coverage against, not a file to port.

**yanfengliu/aoe2, first look (`61758598`), and it is not what the watchlist assumed.** A TypeScript browser
AoE2-style prototype on a *sibling* engine, `yanfengliu/civ-engine` (MIT, "an AI-native, AI-first game engine
built for civilization scale simulation games"), rendered by another sibling package (`voxel`) in Three.js, with
all art procedural and original, bit-exact replays, and an LLM-agent playtest campaign loop whose bundles replay
in the app. Its own commits this month are "canon sync" against a fleet governance repository, not engine work,
so the repository worth reading is `civ-engine` - and it is TypeScript. Keep it on the list for one reason: it
is the only project here that took D18's B-case first, and its answer to the cost of original assets is a
renderer, not an engine.

**aoc-reference-data:** one commit, five lines in `data/players.yaml`. Nothing.

**Study list.** `crates/engine/genie-core/src/combat/victim_reads.rs` (the two functions and their doc comments,
175 lines); commit `4feb5099` (a default that was −1 where the game's memory was 0, and the oracle that said
so); `CODING_STYLE.md` §5 in full (the boundary rule and the union-enums) after `62afd442`; openaok's
`docs/ARCHITECTURE.md` (especially "Passability per terrain restriction" - one passability layer per terrain
set, each with its connected regions, the asking player's key in the upper bits so a gate opens to its owner and
allies) and `docs/VERIFY.md`'s table; `src/core/Costs.h` with `src/verify/Stress.cpp` for the per-system budget.

**Never copy.** genie is **AGPL-3.0**. The newtype list is the tempting part and it is the dangerous part: the
*decision* (which meanings deserve types) is a design fact anyone would arrive at, but the names, the boundary
rule, the union-enums and the commit-by-commit order of the sweep are their expression, and §5 is a style guide,
not a small function. openaok is **MIT**, which makes it licence-compatible and therefore the *easier* mistake:
its dat reader is not. `THIRD_PARTY_NOTICES.md` names `genieutils` as **LGPL-3.0** (used only by the data
tools, with GPL-3.0 beside it), and the Indeo 5 decoder as LGPL-2.1 - so "openaok is MIT, I can lift its `.dat`
parsing" is wrong twice over. `yanfengliu/*` is MIT throughout and the cleanest of the four, which still does
not make its TypeScript a design we can copy into a Rust sim.

## 2026-11-01 — genie ends the newtype sweep with one real fix; openaok decides where a unit stands

**genie, 130 commits (`10af9143..2f0ac827`; 612 files, +51,688 / -24,762), and 129 of them begin `T9:`.** The
sweep continues to the end of the type list rather than changing course: the clock and every timer
(`WorldTime`, `Millis`, `Seconds`), the map's `Tile`/`TileRect`/`GridSize`/`Distance`/`FixedPoint`, the pathers'
`NodeIndex`/`Cell`/`CellEdges`/`RayFraction`/`WalkResult`, the states (`ObjectState` and one enum per action:
`AttackState`, `GatherState`, `BuildState`, ...), `EffectCommand` by kind, `TechType`, `CivId`, `ClassMask`,
`ClassAmount` of a `DamageClass`, `AttackPriority`, `UnitStance`, and on the player's side `PlayerSet` and
`VisionBits` as `bitflags`. `CODING_STYLE.md` gained the bitflags rule that this half of the sweep needed: a set
of bits is a `bitflags` type, the bits that have a meaning are named, `const _ = !0;` so no operation can drop one,
and a word read from the save is built with `from_bits_retain`, never `from_bits_truncate`. The codec and the
oracle moved with it (39 files of `genie-oracle`, the command codec's encoder), which is what "the raw value only
at the boundary" costs when the boundary is a save format.

**One commit is not a rename, and it is the one to read: `d9280ea5`.** `Player::resources` (attributes 0-3) and
`Player::construction_rate` (attribute 195) were two copies of one thing, filled from the save's attribute array at
import. Every add and store wrote 0-3 into `resources`, so `attributes[0..4]` went stale; nothing wrote
`construction_rate` at all after the import, so **a change of attribute 195 never reached a builder's work or the
planner**. `Player::attributes` is now the one column (player+0x100, count at +0x90), `resources()` and
`construction_rate()` derive their four values and 195 where they are read, and the store, escrow, tribute,
treason, BUILD cost, repair payment and the forced replay all read it. A cost in an attribute past the four now
pays from that attribute, which closes their "a cost in a resource other than the four" gap. The lesson is the same
shape as last week's `Entry` default, seen from the other side: last week a *derived* default was wrong for a
zeroed slot; this week a *copied* field was stale because two places held one meaning.

**Two more defects the sweep surfaced, both derived defaults.** `5d11a8ea`: the dying building's annex test read
sprite 0 - the old i16 field's derived default - where `SpriteId`'s `None` now stands; it reads the master's
destruction sprite, which is the game's path, and the note carries a dat fact for the asset pack: **111 of the
447 buildings have no destruction sprite.** `ab8d99fa`: `GameVector` and `BVector` were one meaning written twice
in the game's (x, height, y) order, and are now one type.

**What that costs us, measured.** `crates/sim` holds no player attribute column at all: stockpiles are fixed rows
of `MAX_RESOURCES` and there is no 195. Two measurements against our own dat, because genie's column is DE's:

- **No effect in 5.7.2 touches attribute 195.** Of 798 effects and 3,306 commands, the decoded modifiers name 25
  distinct attributes and 195 is not among them; no raw command of kind 0/4/5 carries `a = 195` either. So 5.7's
  builders-work-faster bonus (the Spanish one, if 5.7 has it) **cannot come from the dat**, and our content pack
  has to carry it as an override - the same shape as the other hardcoded civ bonuses. Verify by oracle before
  writing it: build the same barracks with a Spanish and a British villager on the same ground, and time it.
- **The per-civ `resources` array is 211 long in all 32 rows, holds 30 non-zero values, and the 31 playable civs
  differ from each other in exactly one slot: index 34, where the Aztecs have `2.0` and everyone else `0.0`.**
  (Gaia differs from all of them at 36, 62, 63, 78 - `175`, `20`, `32000`, `0.3` - which are the playable civs'
  values.) Index 195 is `0.0` in every row, Gaia included. That is U8 shrunk from "190 entries to map" to "one
  index with a civ-level difference and four entries that are the same for everyone", and it agrees with U13:
  civ identity is in `tech_tree_effect_id`, not in this array.

**openaok, 13 commits (`49d4c9b7..6c9e64c5`; 25 files, +395 / -46): the last of the computer players, and where a
unit stands.** Two clusters matter to us, and the first lands on this week's work (D85's garrisoned recruit, D76's
footprints).

**`src/pathfinding/ApproachPlanner.h/.cpp` - `FindStandingSpot`.** Each caller asks for itself, from where it is:
the planner walks the target's perimeter at a `standoff` (the worker's reach plus its own half-width), starts at
the point of the caller's own face nearest to it, and steps outward +1, -1, +2, -2 so the answer is always the
nearest usable spot. Two passes: the caller's own face at quarter-worker grain, then the whole perimeter by whole
worker-widths - "a crowd fills the wall in front of it before anyone is sent round a corner". Three rules here
are worth more than the code:

- **A walker counts by its destination, never its position**, so the taken list does not change under a replan and
  a villager on its way to a wall does not keep losing the spot it was given.
- **Each taken spot is tested against that body's own room**, not one fixed clearance: `8e2cef5` changed
  `gap < clearance` to `gap < taken_room[i]` so a large body and a small one can each stand as close as they
  actually allow.
- **The spot is a function of the asker and the taken list, not a reservation in the world.** There is no
  placement pass to keep in sync; the same inputs give the same spot on every machine, which is why it survives
  their determinism gate.

`4b7ab96` extends the same planner to soldiers set on a building, spread along its walls; `d3d9857` adds
`KeepLane`, the march: a group of three or more on one far target has each unit's waypoints shifted sideways by
its lateral offset from the group's centroid, **tapered to zero within 6 tiles of the target**, and each shifted
point is kept only if it is passable and clear of bodies for that unit's own radius - so a group goes round a
corner abreast instead of in file, and closes up as it arrives.

**The search budget became a tick budget.** `AStar::Searched()` returns a running count of expansions (incremented
in both `FindPath` and `FindRoute`), and `src/ecs/Combat.cpp` stops a tick's chase replanning at either 8 plans or
**6,000 tiles expanded past the first plan** - so an army ordered at one far target replans over a few ticks
instead of stalling the one. Last week the budget was per search (a route's own length); this is the same idea
applied to the tick's total, and it is deterministic because expansions are. It is the cheapest answer we have
seen to "pathing is fine per unit and ruinous for an army", and it costs one counter.

**Their determinism gate moved with the behaviour, visibly.** The golden hash in `tools/check_determinism.sh` went
`7627c023a5957619` -> `221acd07913e73ac`, i.e. a behaviour change is a deliberate edit to the gate rather than a
tolerated drift, and a new `walkers` scenario runs four computer players for ten minutes on Arabia *and* Islands
and fails if a villager stands still on its way to work for ten seconds. Population limits reach 1,000 and the
computer players play above 200 by scaling their plan for 200, which is a content-side answer to a limit we have
in the engine as a constant.

**Divergence that matters.** Placement is the live one. They decide a spot by a deterministic scan of a target's
perimeter with a taken list, at the moment the order is given; we decide on the map (footprint masks, and garrison
when the tile is blocked). Theirs has no global placement pass at all and no cost when nothing moves; ours has to
answer "who else is coming" where they simply carry it in the request. Adopting it would mean a per-order spot
pick plus a per-target taken list, and it would replace D85's garrison fallback with a weaker one only when the
whole perimeter is full - which is the behaviour the game appears to have. Everything else stands as recorded: the
tick is still 16 ms fixed point, the engine still knows the dat's particulars (D17 unchanged on both sides), and
neither project has moved on assets (D18).

**Study list.** `src/pathfinding/ApproachPlanner.h` (the `ApproachRequest` comment block names every input and why)
with `FindStandingSpot` below it - about 120 lines in total, and the only thing this week that would change a
design of ours; `src/ecs/Combat.cpp` around `kChaseSearchPerPass` and `KeepLane`; genie's `d9280ea5` commit body
(one column, and the list of every reader that had to move); `5d11a8ea` for the destruction-sprite count;
`CODING_STYLE.md`'s bitflags rule after this range.

**Never copy.** genie is **AGPL-3.0** (and now carries `LICENSE-GPL-3.0` beside it) - the one-column fix is a
*model* fact we can re-derive, not a diff to port, and the sweep's value is the list of meanings, which is
theirs. openaok is **MIT**, and this is exactly the week the "it is only a small function" thought arrives: 120
lines of perimeter scan is the most liftable-looking code either project has produced. The rules above (destination
not position, a body's own room, the spot a function of asker and taken list) are what we would take; the file is
not, because its `standoff`/`clearance` numbers and its two-pass grain are the game's feel arrived at by their own
tuning, and copying them would give us their tuning without their evidence.

## 2026-11-02 — the guide that names the numbers, and U19 falls

**The smallest of the three moved the most.** `Divy1211/AoE2DE_UGC_Guide` (`bda6dec6`, first look) is not an engine: it is
a GPL-3.0 prose guide to the game's own tables, written for scenario designers and data modders, and it carries what the
question register has been asking since 2026-10-23 — **names for the numbers**.

- `docs/general/resources/resources.md` (4,483 lines) is the **player-resource table**: every id from 0 upward named,
  with its per-civ defaults. This is the array `civs.toml` holds — 4 population headroom, 6 current age, 23/24/25 the
  Castle/Imperial/Feudal age techs, 36 farm food amount (**default 175**, Chinese 220, "what horse collar etc.
  modify"), 84 starting villagers, 88 fish trap food, **195 construction rate modifier — 0 default, 1.3 Spanish**.
- `docs/general/attributes/attributes.md` (1,662 lines) is the **unit attribute table**, and its 195 is a *different*
  195: "Initiated Tech Id". Two namespaces, one number.
- `docs/general/damage_calculation.md` lists all **38 damage classes** by name and id (0 Wonder, 1 Infantry, 3 Base
  Pierce, 4 Base Melee, 11 All Buildings except Port, 13 Stone Defense, 22 Walls & Gates, 36 Heroes and Kings, 37
  Hussite Wagons) — U3's table. Its own foreword says it is outdated on the formula, and it claims a per-unit **Base
  Armour of 1000**, which is a claim to test rather than to adopt.
- Its RMS, scenario-binary and AI pages are empty stubs, so it says nothing about `world-rms.md`.

Believed only after a check against our own file: resource 23/24/25 = **102/103/101** (Castle/Imperial/Feudal age techs),
36 = **175**, 84 = **3**, 88 = **700** — every one exactly the value in `config/gamedata/civs.toml`. `Zaydme/openaok`'s
`Core::Rules::Res` constants name the same indices independently (36 farm food, 84 starting villagers, 88 fish trap, 23/24/25
the age techs), so the table is confirmed by three sources with no disagreement.

**U19 is answered, and the last report was wrong about it.** The Spanish bonus *is* in the dat, as a **resource**, not an
attribute: effect **446 "Spanish Technology Tree"** carries `[1, 195, 0, -1, 1.3]` — kind 1 (resource), resource 195,
`b = 0` (set), value 1.3. The guide says what that means — "builder work rate multiplier", and "the actual work rate for
builders is given by `construction_rate_mod * builder.default_work_rate`" — and lists 0 default, 1.3 Spanish. The
2026-11-01 census said "no effect in 5.7.2 touches attribute 195" and concluded the bonus "cannot come from the dat". The
census was true of kinds 0/4/5 and I did not look at kind 1, which is where the answer was; attribute 195 is Initiated
Tech Id and resource 195 is the construction rate. A search named after a number finds the wrong namespace twice.

**And the same command shape gives the farm its base, which we have 75 too high.** Resource 36 is 175 in every playable
civ's row, and the farm techs are kind-1 *adds* to it — Horse Collar `[1, 36, 1, -1, 75]`, Heavy Plow +125, Crop Rotation
+175, Chinese team bonus +45 — so 175 + 75 = **250**, the published figure, which is where our `food_per_farm = 250` came
from. `Sim::farm_food_total` then adds the techs *on top of* 250, so a Horse Collar farm here holds **325 where the game
holds 250**, and a Chinese one 295 where the game holds 220. U20.

**genie, 84 commits (`2f0ac827..0c133548`; 801 files, +47,054 / -23,509).** Four streams, and the one that carries a lesson
for us is again a defect rather than a rename.

- **`93293d63`: `Player::computer` was a copy of `Player::kind` (player+0x10) that the import never set.** genie-oracle
  wrote `false` for every player, so every test of the game's `player+0x10 == 3` answered "human" for a computer player.
  The field is gone, `PlayerType::is_computer` asks the type, and 160 of 14,110 recordings now stop at update 1 on
  computer branches they do not model — the branch set had been invisible. Third week, same shape: a copied field with no
  writer (week 1 a derived default, week 2 two copies of one column, this week a copy nothing filled).
- **T4, the objects' "parts".** The world gained one table by object id (`World::index`, as the game's own `world+0x168`
  does), and the per-object layers left the row for `World::parts` — actions, building, combat, the mover, the unit AI.
  An object taken out of its player's list for its own update is not findable in the index while it is out — the game's
  own behaviour, ported — which is one more piece of the sequential-order picture U18 is about. Every step claims "bit
  for bit" against a recording.
- **`3e13b4df`: a unit's top speed is read from the sprite it holds, not from its master.** A spawned deer ran at 1.4003
  because the mover kept the master's *running* sprite's speed; the game reads slot 65 off the sprite the object actually
  holds (0.737 for that deer). Our sim reads one speed per unit and never re-reads it, which is right for a walker and
  wrong for anything that changes sprite.
- **`bd106a43`: a building lets go its links to objects past state 6**, and the long pather now asks the gate on its own
  tile whether it lets this player through (unlocked, both alliances, a transform-capable master) instead of walking round
  it. Found by a diverging recording, then ported from the game's function.
- `a7628788` makes the sprite table a dense column by sprite id; T15 adds an RFC 430 casing gate; T9b finishes the newtype
  sweep into the player and the action states.

**openaok, 83 commits (`6c9e64c5..7e50543e`; 239 files, +27,399 / -1,697).** Three clusters.

- **Platforms: the iOS port merged, and the docs now name six targets** — Windows, macOS, Linux, Android, iPhone/iPad and
  the Switch, from one `src/` tree, SDL2 behind a HAL, the original's 800x600/1024x768/1280x1024 frame drawn to an
  off-screen texture and letterboxed into the window (nearest-neighbour at whole-number scales, filtered otherwise).
- **The engine documented itself: 43 markdown pages under `docs/`, a VitePress site.** `docs/engine/pathfinding.md` (508
  lines), `economy.md`, `combat.md`, `data.md`, `determinism.md`, `rendering/*`, `platforms.md`,
  `development/performance.md`. This is the first public cost model for "a cheap A* that feels like AoE2", and it is the
  study item of the week.
- **Farms are worked on the field, spot by spot.** A farmer stands *on* the plot, works one spot for one unit of food's
  time, then steps a tile or so to another spot and works again — gathering while it steps, stopping only for the trip to
  the drop site. The spot is a function of the farmer, the farm and the count of spots so far, so it needs no random
  stream. A farm's food is not its own storage but its owner's **resource 36** at the moment it is laid, which is how a
  farm tech reaches only farms laid after it. Also new: a load is taken only to a *finished* drop site, a spent farm
  leaves the dead farm unit that is rebuilt by right-clicking it, a fish trap leaves its own dead unit for Fishing Ships.

**Divergence.** Unchanged on D17 (both still put the dat's particulars inside the engine — genie by porting the game's
structs and offsets, openaok by a build-time `rules.json`; neither has moved toward our four layers) and on D18 (no
original art from either; their player-colour work is a renderer fact, below). What is new:

- **Platform is now a real difference and their answer is better than our absence of one.** Six targets including phones
  and a console, from one source tree, with the interface laid out in the original's pixel space and scaled at the end.
  That is exactly the shape D18's asset pack wants — sprite refs stay at original resolution and a different screen is a
  scale factor, not a second layout. Cost to us: a HAL boundary and a fixed frame, which are meaningful decisions and
  cannot be judged before a renderer exists. Record it, do not adopt it yet.
- **U7: still one integer A*, and the published cost model is complete.** `kStraight 10`, `kDiagonal 14`,
  `kBesideClosed 4`, `kPastStanding 40`; octile distance raised by 8 per-layer landmark distance fields (corners and
  edge middles, measured at load); short routes budgeted `256 + 64 x` their length, others 20,000 expansions; ties
  broken by one 64-bit key. New this week: **shared walks** (a route planned this tick for the same point, on the same
  ground, from within 2 tiles, is copied rather than searched), **marching** (a group is left out of the standing list it
  would plan around), and **a crowd over 32 tiles at the goal costs nothing to cross** — the route goes straight in and
  units stop against whoever they meet, which is the opposite of our "go inside / garrison" (D85/D76). Nothing in the
  recorded decision changes; the shared walk is the one piece we do not have and could use.
- **Determinism.** Their gate is our `no_float.rs` (a script that greps simulation code for float literals) plus a golden
  world hash; their tick states the rule "where an outcome depends on which entity goes first, collect them and sort by
  id". genie's T4 gives the mechanism behind U18's "sequence, not a snapshot" — an object is out of the world's id table
  while it updates — but no project yet says whether a unit killed earlier in a tick still swings.
- **Asset pipeline.** openaok bakes each player's colour into shared growing texture pages so a screen full of sprites
  draws in a few batches, and indexes graphics through a table built once. Both are "entity+state+owner → one sprite id in
  a dense table" facts, which is the asset pack's core; neither is a dat format decision that touches us.

**Study list.** The guide's `docs/general/resources/resources.md` entries for 34 ("Unused Resource 034 … known to sometimes
have some values" — exactly where the Aztecs' lone `2.0` sits), 36, 62, 63, 78, 195 and 196, and its
`docs/general/damage_calculation.md` class table for U3; openaok's `docs/engine/economy.md` ("Its food" and "On the
field", about 60 lines), `docs/engine/pathfinding.md` ("Who stands in the way", "Units sent together", "The budget" —
about 150 lines) and `docs/engine/platforms.md` for the frame model; genie's `3e13b4df`, `bd106a43` and `a7628788` commit
bodies.

**Never copy.** The UGC guide is **GPL-3.0**, and it is new to this list precisely because a docs repository feels
harmless: the ids and the names are facts, its prose and the arrangement of its tables are not. Take the id, write our own
sentence, cite the page. genie is AGPL-3.0; openaok is MIT but its dat reader is `genieutils`, LGPL-3.0. This week's "it is
only a small function" candidate is openaok's `Farmland()`, about twenty lines — and what we want from it is the *rule*
(the storage entry names which resource line a farm draws on; the owner's table gives the amount), not the function.

**Open questions.**

- **U19 closed** — with the correction above. What is left is ours, not the data's: `build` progress counts builder-ticks
  and never reads resource 195, and a civ's `tech_tree_effect` is read only for its kind-2 bars, so the 1.3 is loaded and
  unused. Two slices, test-first: the tech-tree effect's kind-1 sets applied at match start, and `build` progress
  multiplied by resource 195 (0 meaning 1.0). Test: a Spanish barracks reaches 100% in 1.3x fewer working ticks than a
  British one, on the same ground.
- **U20 new** — the farm food base is 175 (dat, resource 36), not 250; the three techs add to it and the Chinese team bonus
  adds 45. `farm_food_total` adds the techs on top of the published 250, so Horse Collar gives 325 where the game gives
  250. The guide names the resource, openaok reads it the same way, and our own file agrees on the base; the fix is to
  read the base from content per civ rather than from a constant.
- **U8 answered** — the 211-entry array is the player-resource table, named entry by entry, and the 31 playable civs agree
  on every value except index 34, which the community table itself calls "unused but known to sometimes have some values".
  Civ identity is not here, again (U13). The one action left is to decide whether the content pack carries the names.
- **U3 answered** by the class table (0–37); the "Base Armour of 1000" claim needs an oracle test before it enters
  `combat.md`.
- **U18 unchanged, third week**, now with the id-table detail. **U7 updated**: shared walks, and a crowd at the goal is
  crossed rather than gone round.
