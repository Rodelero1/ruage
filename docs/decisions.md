# Decisions

One row per decision, with the date, the reasoning, and what it commits us to. Decisions are
not revisited silently — if one is reversed, add a new row rather than editing the old.

| # | Date | Decision | Rationale | Commits us to |
|---|---|---|---|---|
| **D1** | 2026-10-23 | **Answered: G1 — "generally similar behaviour, sensibly attainable."** Own deterministic engine; numbers exact where the dat is exact, tolerance-banded where behaviour is only observable through play. **G2 rejected** (after asking the owner directly what it would cost): no oracle here can show a per-tick trajectory, the original runs on x87 with 80-bit intermediates, and bit-exactness would mean transliterating machine code rather than writing an engine. **G1+ was considered and not adopted** — no per-subsystem exactness budget is committed. | Owner's words: "I want generally similar behaviour, but sensibly attainable". The 13 saves validate constants and outcomes, not trajectories (`docs/oracle.md`); the binary is 32-bit x86 with heavy x87 use (crude census: ~69k x87 escape bytes vs ~26k SSE `mov*` in `.text`), so exact-match work would not be *engine* work. Reaching for G2 also has a legal cost: a bit-exact clone is a derivative work, not interoperability. | Exactness where it is cheap and free (dat values, fixed-point per-tick displacement, our own replay hashes); tolerances for behaviour, written down in §6.1. A subsystem may still be matched more tightly later as an optional experiment, but that is not part of the goal. |
| **D2** | 2026-10-23 | **Answered: file-based oracle.** No driven game, no Windows migration. | 13 `.aoe2spgame` files already present, all inflate (raw DEFLATE, `VER 9.4`). Saves are full state dumps. | Snapshot-based verification where cheap; save parser left partial (step 2b). |
| **D3** | 2026-10-23 | **Answered: start with one civ — British — and add civs later.** British is whatever the dat lists at civ index 1 (index 0 is Gaia); the name is not a preference. Scope of step 5 is British vs British. | Owner's decision. Measured the same day (U13): the unit tables of all 31 non-Gaia civs differ in 5 rows out of ~36,000, so the cost of a second civ is a tech-disable-list join, not a data model change. Doing one civ first buys a running game sooner without assuming a one-civ design. | The data model stays per-civ (deltas are real, if tiny); the loader must not hardcode "British"; `gamedata/` keeps emitting civ 1 as canonical, and `tech_tree_effect_id` (type-102 disable lists) becomes the path to more civs. |
| **D4** | 2026-10-23 | **Answered: B — publish the engine and tooling; keep the spec and the mechanics internal.** Condition set by the owner: "as long as the game can be compiled and ran" — a stranger with their own install must be able to clone, build and run it. Not published: `docs/mechanics/`, SPEC §3/§4 (hashes, string VAs, install paths), `docs/oracle.md`, `config/gamedata/` (already excluded by D8). | Owner's decision after the four options were costed. Three regimes separate the material: our own work (no exposure), published community facts (facts are not copyrightable; only wiki prose is CC-BY-SA), substantial extraction from the dat (Database Directive 96/9/EC Art. 7), and binary-derived material (Art. 6). Nothing in `crates/` came from the last category, so the safe set is large. Publishing now, before any decompiler output exists, is also the clean-room record — after step 7 the history can no longer prove it. | A licence split (`crates/` MIT/Apache, `tools/` GPL-3.0 to match the parser it imports); engine code must not cite internal docs; one provenance line per module. **Blocking gap:** a public clone cannot currently produce `config/gamedata/` — see the publishability checklist in SPEC §6.2. |
| **D5** | 2026-10-23 | **The dat is authoritative for all numeric values.** Where the dat and community documentation disagree, the dat wins. | Stated by the project owner. The dat is the artefact the game actually shipped; wiki numbers describe DE and are derived, not primary. | Every number in `docs/mechanics/` that conflicts with a wiki gets a dat-side value and a note (U11 is the first). |
| **D6** | 2026-10-23 | **Game data lives in `config/` as generated TOML, and the engine reads TOML — not the `.dat` — at runtime.** Nothing numeric is hardcoded. Hand edits go in `config/overrides.toml`; loading is strict (unknown or missing key = startup failure). | Project owner's requirement, stated explicitly. The generated layer makes the data reproducible and diffable; the override layer keeps experiments alive across regeneration; strict loading stops typos from silently doing nothing. | `gen_config.py` must stay runnable and `MANIFEST.toml` must be checked at startup for staleness. If **D4** lands on "publishable", `config/gamedata/` is gitignored and generated at first run — it is derived game content. |
| **D7** | 2026-10-23 | **TDD is mandatory.** No production code without a failing test first. The failing run must be recorded in the session before the implementation exists. Scope: engine crates + shipped tooling; throwaway analysis probes in `build/tools/` are exempt. Coverage is behaviour-based with **no percentage gate**. | Project owner's requirement. A recorded failure is the only thing that distinguishes TDD from tests-after, which pass immediately and therefore prove nothing. A coverage % can be raised without testing a behaviour. | Code found without a prior failing test is **deleted**, not retro-fitted. Every cycle report names the test and shows the command that saw it fail. Full rules: SPEC §11. |
| **D8** | 2026-10-23 | **No game data, and nothing derived from it, is committed.** `Age2HD/`, `config/gamedata/`, `build/`, saves and assets are gitignored; a pre-commit hook refuses them. | Project owner is porting their own lawful copy and wants to comply with EU law. Decompilation for interoperability is permitted (Software Directive 2009/24/EC Art. 6) but redistribution of what it yields is not, and the dat is a substantial extraction under Database Directive 96/9/EC Art. 7. | A clone regenerates the data locally with `tools/genconfig`; docs quote only illustrative values. Full policy: `docs/licensing.md`. |
| **D10** | 2026-10-23 | **The scale is derived from two measured grains, and config invariants are enforced rather than declared.** Rules: `scale % 10^dat_value_decimals == 0` (no dat value is ever rounded) and `scale % (tick_rate_hz × 10^per_tick_decimals) == 0` (no per-tick motion ever drifts). Measured: `dat_value_decimals = 4` (`work_rate = 0.0111` on units 110 and 1021), `per_tick_decimals = 3` (speed `0.737`). Supersedes D9's "must divide a power of ten" clause, which was arithmetically inverted — it accepted 1, 2, 4, 5 and 8 (none of which can represent a single dat value) and rejected 300. | Adversarial review, 2026-10-23: the old check let a config load with `schema = 99`, `floats_in_sim_state = true`, `max_entities = 0`, one RNG stream, or 60 Hz paired with a 20 kHz scale — under which 34 of 57 speeds lose per-tick exactness. An invariant that lives only in prose is not an invariant. | `config/sim.toml` carries both grains; `crates/data/tests/corpus.rs` sweeps the whole generated corpus and fails if a dat value ever needs more places. Change `tick_rate_hz` and the scale must move with it (60 Hz → 60000). |
| **D9** | 2026-10-23 | **The fixed-point domain is simulation state only.** *(The "divide a power of ten" clause is superseded by D10.)* Values that enter state are `Fixed` (i64 at `sim.fixed_point_scale`); load-time constants that never enter state (pathing weights, heuristic costs) stay f64. The scale must divide a power of ten. | "No floats in sim state" is the determinism rule; it does not forbid f64 in a config file or a heuristic weight. Rounding such values into the fixed domain silently changes behaviour (1.41421356 has no exact representation at scale 20000), so the boundary has to be explicit rather than incidental. | `Fixed::parse` refuses sub-precision instead of rounding. *(Amended by D10: the scale rule is now two derived divisibility checks, not "divides a power of ten".)* Enforced by tests in `crates/fixed` and `crates/data`. |

| **D11** | 2026-10-23 | **Gathering data shape (review finding): `genconfig` emits drop sites, and the task→variant mapping is a hand-written config table.** Per-unit drop sites come from `task_info.drop_site_unit_ids` (562 lumber camp, 584 mining camp); the seven villager variants stay addressable by unit id (VMLUM 123 = 0.39, VMGLD 579 = 0.38, VMMIN 124 = 0.36, VMSHE 592 = 0.33, VMFOR 120 = 0.31, VMHUN 122 = 0.41, VMFIS 56 = 0.43). The one fact the dat cannot supply for `VER 5.7.2` — which task selects which variant — lives in `economy.toml`, because D6 forbids it in code. | Adversarial review, 2026-10-23: `economy.toml` claimed `rate_from = "dat"` while the generated config carried neither the mapping nor any drop-site field, and `VMBAS` carries its *build* rate (1.0) in `work_rate`, so the naive wiring runs gathering 2.5x too fast. Owner chose this shape over deferring gathering. | `genconfig` grows a drop-site field (test-first, TDD applies); `economy.toml` grows a 7-row variant table; a test asserts every variant id resolves and that no gather rate is read from a base villager. |

| **D12** | 2026-10-23 | **The generator lives in its own repository, and any parser fork lives with it.** *(Amended: no fork is currently needed — see below.)* `tools/genconfig` and a fork of `Divy1211/GenieDatParser` (GPL-3.0) move to a separate repo; the engine repo stays MIT/Apache, pure Rust, and never imports Python. A fork keeps the GPL (which is allowed and expected) and the licence boundary becomes a repository boundary rather than a directory convention. | Owner's suggestion, after the GPL framing was corrected: a fork will be needed, so keep it where it can be GPL without questions. Evidence that changes are needed: at `VER 5.7.2` the parser raises `VersionError` for `tasks` (the per-task list) and cannot read `unique_unit_effect_ids` or `name2` — the three gaps U13 and D11 ran into. | The engine consumes `config/` (TOML) only. A public clone gets the engine and instructions; the data comes from the generator repo, run against the user's own install. Splitting is not urgent: it is a publish-time step, not a code change today. |

**D12 amendment (2026-10-23, same day).** The "fork is likely" reasoning was wrong on the evidence:
the per-task lists *are* readable via `dat.unit_data.tasks` (412 populated; the `VersionError` came
from a different, version-gated accessor), and `unique_unit_effect_ids` / `name2` are DE-5.9-only
fields that do not exist in a 5.7.2 file, so no fork could supply them. The separate-repository
decision stands on its own merits (licence boundary, and the generator is Python), but it is not
currently motivated by a required patch.

**D11 amendment (2026-10-23, same day).** The task → variant mapping was recorded as a fact "the
dat cannot supply". It can: each variant's gathering task declares its `resource_in` (1 wood,
2 stone, 3 gold, 0 food; 15/16/17 are pseudo-resources, U14). So the mapping is derivable, and the
hand-written table in `economy.toml` becomes a cross-check. `genconfig` should emit the task lists
before step 5 rather than after.

| **D13** | 2026-10-23 | **Adopt `librematch/genie`'s format crates inside the tooling repository; do not relicense the engine, and do not port their engine core.** Concretely: `genie-dat` (read/write `.dat`), `genie-rec` (recorded games), `genie-save` (saves), `genie-scx`, `genie-drs` — all GPL-3.0 — are candidates for the generator/tooling repo that D12 already puts outside the engine. The engine keeps MIT/Apache and the TOML boundary (D6). Their `genie-core` is AGPL-3.0, is a port of **DE build 158041**, and keeps f32 arithmetic to match it — a reference for our step 6 module map, not a drop-in. | Surveyed 2026-10-23. `genie-dat` is already one of the two implementations that cross-validated our step 1, so the adoption risk is low. `genie-save` could revive the 13-save oracle that step 2b abandoned as unparseable. Taking format crates costs GPL-3.0 only in the tooling repo; taking `genie-core` would mean AGPL-3.0 *and* a DE target we are not building. | Tooling repo gains a real reader instead of a Python parser plus a fork question; engine unchanged. Revisit only if we decide to read the dat at runtime and drop D6 — that would force the engine to GPL-3.0. |

| **D14** | 2026-10-23 | **Keep Ruage on HD 5.7 under G1; borrow narrowly; watch the field.** Concretely: (1) target HD, because it is frozen and a G1 engine's only asset is "the numbers are right today"; (2) adopt `librematch/genie`'s **format crates** only (D13) — never their engine core, because a nine-day-old repo with one author is a bus factor, and a narrow interface stays replaceable; (3) rebuild the step-6 backlog from the game's own subsystem split, as seen in their module list (§7.1); (4) offer HD-specific findings upstream (5.7 task lists, type-102 tech trees, the two decimal grains) so the relationship is two-way; (5) review prior art periodically and record what changed. | Owner accepted the recommendation. The user's intuition that DE would be "too ambitious" is right about *bit-exactness* and wrong about the version: under G1, DE is a data target, not a mountain — what makes it costly is that it patches, so the target and the oracle both move. HD is frozen, which is worth more than the extra civs. | No engine relicense; no dependency on their engine; a standing review of the field. A pivot to DE, or to linking a GPL reader at runtime (which would end D6), needs a new decision — not drift. |

| **D15** | 2026-10-23 | **Step-5 scope, decided: hand-authored map, scripted opponent, grid A* pathing.** The map is a 64×64 authored in `config/` (forests, berries, gold, two town centres) — not an `.rms` and not a `.scx`. The opponent is a fixed command list on a tick schedule, which doubles as our replay format. Pathing is A* on the static grid with the weights already in `movement.toml`. `.rms` (122 scripts), `.scx` (49 scenarios) and `.per` AI (74 scripts) are **future work, explicitly out of the slice** — each is a language or format implementation, not a slice feature. | Owner's decisions on the three blockers. Each "future" item was priced: an RMS interpreter is a language implementation, `.per` likewise, and `.scx` needs a binary reader — none of them makes a villager gather wood. The slice's job is to prove the loop, not the content pipeline. | Step 5 is headless, one civ (D3), British vs British (D13 terminology: our own map, our own opponent). Adding any of the three future sources is a new decision, not a refactor. |
| **D16** | 2026-10-25 | **The tick rate is a per-match setting, defaulting to 20 Hz; nothing in the engine may assume it, and mod content is expressed in seconds, never ticks.** A match carries its `tick_rate_hz` *and* `fixed_point_scale` in the handshake and in the replay header, so two peers running different rates refuse the match rather than desync. Revisit only when multiplayer exists and input latency can be measured. | Owner's question ("are there benefits to a higher rate if we need no savegame compatibility and no clone?"), answered by pricing it. Higher rates buy three things and none of them is fidelity: (1) **input latency** — an order arrives up to one tick late, 50 ms at 20 Hz versus 16.7 ms at 60 Hz; (2) **tunnelling** — a unit at 8.8 tiles/s moves 0.44 tiles per tick at 20 Hz and 0.147 at 60, which changes what the collision resolver has to catch; (3) **off-grid quantisation** for modded values that do not sit on the dat's 50 ms grid. Against that: CPU is linear (a 10-minute game is 12k ticks at 20 Hz, 36k at 60), lockstep delay is counted in turns so the same wall-clock latency buffers 3x the turns, and every §6.1 band tightens. Crucially the dat's own values already sit on a 50 ms grid (20 reload values, all multiples of 0.05), so 20 Hz represents them **exactly**; a higher rate buys resolution the data does not contain. Compatibility is not an argument either way here — no savegame interop is needed, and multiplayer is same-engine — so the clock should be chosen for cost, not imitation. | `sim.toml` remains the source of truth; a match handshake must compare rate and scale; mods and `config/` express durations in seconds and the engine converts (D6). Changing the default later is a config edit plus a CHANGELOG entry, not a migration — `minimum_scale()` derives the scale. The modding work the owner actually wants (wider civ differences, more civs, new art) lives in the effects/tech interpreter and the per-civ delta model, which are clock-independent. |

## Prior art: `librematch/genie` (surveyed 2026-10-23)

A deterministic Rust reimplementation of the **DE** engine, created **2026-10-15**, pushed the day
of this survey, one visible author plus agent-assisted commits, 7 stars, 0 open issues. It is much
further along than that age suggests, because `genie-core` is not written to a spec — its own
module documentation says it "reproduces build 158041 **bit for bit**", keeps "the game's f32 sums
and products, each rounded on its own (no fused multiply-add)", and mirrors "the decompiled
routines' math names, formulas, flag fields and parameters, assignment order". Its subsystems are
the game's: `abilities, attack, best_target, build, checkpoint, damage, delete, diplomacy, effect,
explore, facing, farm_queue, garrison, gather, gather_search, ghost, group, …`.

Three things follow for us:

1. **G2 is not hypothetical.** A peer project is doing the thing we priced and rejected, with an
   oracle harness (`genie-oracle`) and per-tick comparison against a running game. If the owner
   ever reopens G1, this repo is the worked example — and note it contradicts D9: under G2 you must
   keep f32 and the game's instruction-level conversions. Fixed-point is only right under G1.
2. **Their evidence is unverified by anyone but its author.** Nine days old, 7 stars, no external
   review. "Runs to 8000 clean" is a claim in a commit message, not a measurement we made.
3. **What we should take is the file-format work, not the simulation.** `genie-dat` already appears
   in our own step-1 cross-validation, and `genie-save` is the parser we gave up on.

## D7 applied: the generator rewrite

`build/tools/gen_config.py` existed before D7 and had no tests. Rather than grandfather it,
it was **deleted** and rewritten as `tools/genconfig/` — 17 tests, each written and watched
failing before the code that satisfies it. Behaviour was proven preserved by stripping
comments from both versions' output and diffing: identical across all four files
(22,158 / 5,609 / 3,194 / 194 data lines).

Two bugs that TDD caught during the rewrite, both of which the untested script had shipped:
a duplicate `schema` key (invalid TOML) and `d` values emitted as the string `"1.2"` instead
of a float.

## D5 applied: the farming rate (U11)

Under D5 the farming rate comes from the dat, and the dat offers **two** values for one
action:

| value | where | rate |
|---|---|---|
| farmer villager variant (`VMFAR`, 259) | `task_info.work_rate` | **0.530** |
| farm building (`FARM`, 50) | `task_info.work_rate` | **0.400** |

Both have `work_value1 = 1.0`, so neither is scaled down by the task record — checked
explicitly, since a `× work_value1` composition would have reconciled the two cleanly. It
does not.

**Resolution taken: the effective farm gather rate is 0.400 — the farm building's own
`work_rate`.**

**The construction hypothesis was tested and is falsified.** It is the natural reading, so it
is recorded here rather than just discarded:

| test | result |
|---|---|
| Do all buildings carry a `work_rate`? | No — **`HOUS` has `work_rate = 0.0`** yet builds fine in 25 s |
| Is build time separately in the dat? | Yes — `creation_info` holds 15 s for farms, 25 s for houses, 50 s for barracks |
| Would a 0.4 multiplier make farms build *slower* than houses? | Yes — but farms (15 s) build *faster* than houses (25 s) |

So `work_rate` cannot be construction speed: the build number lives elsewhere, and the one
building with a zero rate is buildable.

**What the farm's 0.400 appears to be.** `FARM` is a resource *container*: its two tasks are
`action 21` / `action 121` with `resource_in = 16` (food) and `resource_out = -1` — it holds
food, it does not produce it. Its `work_rate` is therefore plausibly a throughput limit on
extracting that resource. That reading is supported by farms being the only resource-container
building with a rate below 1.0 — **but `FTRAP` (fish trap) has `work_rate = 1.0`**, so the
mechanism is not uniform across containers, and the interpretation stays a hypothesis.

**What this costs if it is wrong:** 250 food on a farm empties in 625 s at 0.400 versus 472 s
at 0.530 versus ~784 s at the published 0.319. That is a 25–65% spread in the single most
important economy number in the game.

**How to settle it later, cheaply:** two saves 60 s apart with a known number of villagers on
farms. Food delta / (villagers × 60) is the rate, to three decimals. That is one scenario and
one arithmetic step, and it needs no code.

**Implementation rule until then:** the farm rate must be a *single named constant* loaded
from the dat (`FARM.task_info.work_rate`), never inlined, so a re-measure changes one number.

---

## D17 — Four layers, and only one of them is about Age of Empires

**Decided 2026-10-25.** The engine must be able to ship a game that is not Age of Empires II,
without being rewritten.

| layer | owns | may know about AoE2? |
|---|---|---|
| **engine** (`crates/`) | fixed-point determinism, tick loop, commands and replays, pathing, entity model, damage arithmetic, effect verbs | **no** — it knows *kinds* of things, never particular ones |
| **content pack** (`config/`, our schema) | resources, units, buildings, techs, effects, map, scenario — in our vocabulary | carries AoE2's *values* today; nothing about them is typed into the engine |
| **importer** (tooling repo, D12) | dat → content pack, plus the dat's semantics (packed attack, class ids, effect kinds) | **all of the AoE2 knowledge lives here** |
| **asset pack** | entity + state → sprite reference | today points at the install's graphics; later at ours |

The rule that follows: **any number or id that means something only because of AoE2 stays outside
`crates/`**. Pinning "kind 5 multiplies" and "attack is packed `class * 256 + amount`" is importer
work; the engine receives "add 1 to attack class 4".

Consequences already visible in the code:

- ~~`ResourceKind` is a fixed enum and stockpiles are `[[u32; ResourceKind::COUNT]; 2]` — four
  resources and two players in the *type*.~~ **Paid down 2026-10-26** (`docs/plan-resource-deletion.md`):
  resources are content (`config/resources.toml`), a node carries a column, costs name their
  resource by reference id *or* by name, and the stockpile's width is the engine's `MAX_RESOURCES`
  with the pack's count checked at load. Proven by `a_content_pack_with_our_own_resources_loads_and_spends`.
  The player count was already data by D20.
- `script.rs` names `109` and `12`; the scenario format should name roles from the content pack.
  (**Still open**: the resource half of this is done - a step names a resource and the pack resolves
  it - but building and training still quote dat ids.)
- `Attribute::from_id` knowing 0/1/8/9 is fine *in the importer*, not in the engine.

## D18 — Target shape: A now, B-capable; assets swappable from the start

**Decided 2026-10-25.** The owner's words: keep the vibe and dynamics; start with the original
graphics, tech trees and stats; be ready to replace the graphics; move towards our own content as far
as the cost allows.

- **(a) now.** Reference content is the dat's numbers and the install's art. That is what makes the
  work verifiable: every mechanic has a known-good value to compare against.
- **(b) scheduled, not blocked.** Own resources, own tech tree, own stats — the engine must not need
  a change to accept them. That is what D17 buys.
- **(c) not planned.** Divergent rules (new resource set, ages replaced) would still be content, and
  the same schema should carry them; nothing is designed against that hope.

**Graphics readiness is a data question before it is a rendering question.** The dat carries a sprite
table — `slp_id`, `layer`, `force_player_color`, `color_table`, `num_deltas`, `bounding_box`,
`sound_id` — with animation deltas and player-colour flags, so an entity → sprite binding can be
**generated** by the importer rather than hand-authored. Two consequences:

1. the engine addresses visuals by a *content id*, never by a file name, so a replacement pack is a
   different table with the same shape;
2. sprite geometry the simulation needs (radius, footprint) is already in the unit table, so
   **swapping art cannot change gameplay** — which is the test to write when the first renderer
   lands.

**Not yet done, deliberately:** there is no renderer. The headless run is the harness, and a renderer
is a large piece of work that buys nothing until the simulation is worth watching. "Ready for
replacement" is therefore written as a schema and a boundary, not as a graphics engine.

**Verified / not verified.** The sprite fields above are read from `GenieDatParser`'s model at
`VER 5.7.2`. The unit → sprite *link* and how the install stores the numbered graphic resources
(DRS, and whether `slp_id` indexes it directly) are **not** checked yet — that is the first task of
the asset-pack work, not an assumption to build on.

---

## D14 (amended 2026-10-25) - openage is the primary prior-art source

The weekly watch (Monday 09:00 UTC) now leads with **SFTtech/openage** rather than treating the field
evenly, because it is the one project that has already paid the costs we are about to pay: a
from-scratch AoE2 engine with its own asset pipeline.

Four specific things to learn there, in order of usefulness to us:

1. **Their asset pipeline (D18).** How the install's SLP/DRS graphics are converted into openage's
   own formats, and how palettes, player colour, frame anchors and animation timing are handled.
   This is our asset-pack layer, and it is the difference between "graphics replaceable" being a
   schema and being a rewrite.
2. **Their engine/content split** - the *nyan* data format - against D17. Where is their boundary
   better than ours, and where does it force engine changes that ours avoids?
3. **Their determinism and networking posture, as a contrast and not a model.** openage's time
   subsystem is a wall-clock event queue with a 50 ms update cap, not a fixed tick (recorded under
   U12). Our tick loop is not negotiable, so this is the one place where being similar would be a
   mistake.
4. **Effect and tech modelling**, against what is pinned (kind 102 disables a tech, kind 4 adds an
   attribute, kind 5 multiplies) and what is not (U16's ambiguous `a` field, U17's negative packed
   attack).

**Licence rule, unchanged and not negotiable:** openage and the genie tooling are GPL-3.0. Reading
for ideas is fine; no code enters `crates/`, which is MIT/Apache. "It was only a small function" is
the sentence that creates the problem, and the watch is asked to flag anything that looks like it.

Still watched at lower priority: `librematch/genie` (format readers, exact-DE target - D13),
`SiegeEngineers/genie-rs` (dat modelling; check whether U15's `create_time` and U17's negative
packed values are now modelled there), and any new deterministic-simulation writeups.

The watch checks against our open questions rather than in the abstract: it is asked to name any
U-number a source now answers, and to say "nothing changed" when nothing did.

---

## D19 - A match's content is identified by a hash, and a mismatch refuses the match

**Decided 2026-10-25.** Two players in a LAN game must be running the same content. If they are not,
the simulation diverges within a minute and the network is blamed for it.

`ContentPack::load(config_dir)` reads every file a match depends on - `sim.toml`, `economy.toml`,
`map.toml`, `gamedata/units.toml`, `combat.toml`, `resources.toml` - and produces one **content hash**
(FNV-1a over each file's bytes, mixed with its name, in a fixed order).

**The hash is over file bytes, not parsed values.** That is deliberate:

- it is what "are we running the same data" actually means, and no field the loader happens to ignore
  can slip past it;
- it needs no stable serialisation of every struct, which would be a second source of divergence;
- its cost is accepted: editing a comment changes the hash, so a modded pack must be re-published
  rather than tweaked in place. For a shared pack, that is the correct behaviour.

`differences(&other)` names the files that disagree, so a handshake can say *which* file rather than
"different hash" - the difference between a five-second fix and an evening.

**Where it goes:** the match handshake and the replay header, alongside `tick_rate_hz` and
`fixed_point_scale` (D16). A peer whose hash differs is refused at the door. A replay whose hash
differs from the running content is refused the same way, because it would silently simulate
something else.

**Not yet wired into a handshake** - there is no networking yet. The pack and its identity exist so
that the first network code cannot be written without them.

---

## D20 - Eight sides, so four-versus-four

**Decided 2026-10-25.** The engine supports **at most 8 players** (4v4), not 2.

Stockpiles were `[[u32; ResourceKind::COUNT]; 2]` - two players in the *type*. They are now
`[...; MAX_PLAYERS]` with `MAX_PLAYERS = 8`, and `Command::Spawn` refuses a ninth side in both the dry
run and the apply, so the failure is a named `PlayerOutOfRange` rather than a panic on the array.

Why eight and not two: the cost is paid once during this refactor, while the array is a constant;
a later widening is paid in every state hash, every replay and every test that pins one. Four-versus-
four is also the real ceiling for the reference game, so it is the number content can be written
against.

**Player 0 stays neutral** - resource nodes live there - which is why the bound is a ceiling and not
a range: `1..=8` for sides, `0` for "nobody".

## D4 (resolved 2026-10-25) - publish early, tooling first

The repository goes **public**, and soon rather than after the game is playable. The owner's
long-term plan is an open-source engine, so the provenance decisions are cheaper to make now than to
retrofit.

Order, because it is forced:

1. **Split the generator out first** (D12). **Done 2026-10-26** — `Rodelero1/ruage-tools`, and this
   repository's history was rewritten so the parser never appears in it. The original reasoning:
   `tools/genconfig` imports the GPL-3.0 `GenieDatParser`;
   while those share a repository, publishing publishes the GPL dependency and its obligations with
   it.
2. Then the engine repository is publishable as it stands: MIT/Apache `crates/`, no game data (D8),
   no GPL code, and a `docs/mechanics/` folder whose provenance is documented per file.

**Consequence for how work is written from now on:** anything added under `crates/` must be
defensible as original work informed by data and published specification, never as a transcription of
someone else's implementation - which D17 already requires for a different reason (the engine must
not know AoE2 particulars).

## Map and pack identity (2026-10-25)

The map is part of the content pack hash today. When map variety arrives - different maps between the
same two players without republishing a pack - the map becomes a **per-match** field hashed
separately, beside `tick_rate_hz` and `fixed_point_scale`. Not before: one identity fewer is one
desync cause fewer, and nothing about the current single authored map needs variety.

---

## D21 - Licence policy: permissive engine, attribution for art, nothing borrowed from GPL code

**Decided 2026-10-25**, after surveying the field (see `docs/licensing.md`).

Every comparable engine is GPL-3.0 (`openage`, `OpenRA`, `OpenRCT2`, `openblack`, `genie-rs`,
`WololoKingdoms`) or AGPL-3.0 (`librematch/genie`). Our MIT/Apache dual is therefore an outlier, and
it stays: permissive code can be taken into GPL projects, packaged by distributions and read by
modding tools, while the reverse is not possible. The compatibility runs one way and this is the
direction that costs nothing.

Three artefacts, three answers:

- **engine and content schema**: MIT OR Apache-2.0, as now. A modder must be able to copy a schema
  format without a licence question.
- **original art and audio**: CC-BY-SA 4.0 with per-asset attribution when they exist. Art needs
  attribution, not source disclosure, and the engine already addresses visuals by content id (D18).
- **game data**: never shipped (D8). It is not ours to license.

**D13 is corrected**: `librematch/genie` is AGPL-3.0, so adopting its readers in the tooling repo is
withdrawn - the network clause on the one component that reads the game's files is the strongest
obligation available. `GenieDatParser` (GPL-3.0, local, separate repository) is the less restrictive
choice and remains it.

**`sld-extractor` declares no licence.** No licence is not permission: default copyright applies, so
it may be read for what it does and never copied from.

---

## D22 - Bring your own copy: the game requires a legally acquired original

**Decided 2026-10-25.** Ruage does not distribute the game's data or art. It runs against an
installation the player already owns, and continues to require that installation until replacement
content exists for every piece it uses.

This is the standard posture for interoperable engines - ScummVM, OpenRA, OpenTTD's original-graphics
option - and it is a *posture*, not a licence: it reduces exposure by never redistributing, and it
does not claim Microsoft's permission.

### What it binds, concretely

1. **Nothing derived from the installation may enter the repository.** Already D8 for data; the same
   rule now covers anything an asset extractor produces. The asset pack is generated on the player's
   machine, into a cache directory, and `config/gamedata/`, `MANIFEST.toml` and any sprite cache stay
   gitignored.
2. **A generated content pack is not shareable.** This is the consequence worth stating loudly: a
   player who sends a friend their pack is redistributing game data, which is precisely what this
   decision exists to avoid. The LAN story is therefore *each player imports from their own
   installation* - and D19's hash is what makes that safe, because two imports of the same version
   produce the same identity. A mismatch refuses the match rather than accepting a shared pack.
3. **Replacement is tracked per piece, not all at once.** D18's asset pack entry gains a provenance
   field (`original` vs `ours`), and the content pack records which of its pieces still depend on the
   installation. The long-term goal - a standalone game - is then a query, not a rewrite: "how many
   entries are still `original`?"
4. **First run says so plainly.** The importer takes the path to the player's installation and refuses
   with a message naming what it looked for and which version it accepts (dat 5.7.2 currently). No
   path is guessed, no download is offered, and no default is baked into a CLI (D6).
5. **The product is not named after the game.** Trademark is untouched by any of the above.

### What it does not fix

It does not make redistribution of assets lawful, and it does not permit shipping a "convenience" pack
"for testing". The line is the same one D21 draws: expression we did not create is never ours to
distribute, whatever the intent.

## D23 - A civ's tech-tree effect lists what the civ *cannot* have

The dat encodes each civilisation's tech tree as an effect whose commands are all **kind 102**, one per
restricted id, with every selector set to -1 and the id in the value: `[102, -1, -1, -1, 428]`. There is
no state field, so the file does not say whether an entry permits or forbids. The direction was settled
from the data rather than assumed:

- **No civ's list contains anything every civ has.** Loom (22), the Feudal/Castle/Imperial ages (101,
  102, 103), Wheelbarrow (213) and Forging (67) appear in **0 of 32** lists. A list of permissions would
  have to contain them.
- **Britain's 31 entries are its real omissions.** 428 upgrades the scout into the **Hussar**; 384 and
  434 are the **Eagle Warrior** line; 320 upgrades into the **Siege Onager**; 265 upgrades the knight
  into the **Paladin**. Britain has none of those, and the entries are far too few (31-44 per civ, 97
  distinct ids in total) to be a permission list, which would run to hundreds.

So a civ's tree effect is a **restriction list**, the engine reads it as such, and the refusal says whose
rule it is ("not available to the British") rather than pretending to be a generic gate. Barred techs are
checked **before** prerequisites: a tech the player can never have should say so, not ask for an age.

`team_bonus_effect` is carried and not applied; the per-civ `resources` array (211 numbers of tuning) is
carried uninterpreted, because naming its entries from the values alone would be guessing.

## D24 - A unit exists once a tech "makes it available", and those techs are bookkeeping

Kind 2 is the availability command: `[2, unit, flag, -1, 0]`. The effects that carry it name themselves
- "Barracks (make avail)", "Watch Tower (make avail)", "Castle -- Age Three", "Trebuchet", "Pike" - so
the reading needs less argument than D23's did, and the census backs it: **109 units are named by a
positive kind-2 row, and 81 techs exist only to name them**. Each of those techs costs nothing, takes no
time, has no home building, and does nothing but kind-2 rows; every one of them requires an age (59 of
them also require tech 266, the "a castle exists" marker). Exactly one has no requirements at all: 332
"Outpost (make avail)", which is available from the first second.

Two rules follow, and both are content-driven rather than hard-coded:

1. **A unit is gated if some tech makes it available.** A gated unit cannot be built or trained until a
   tech that carries a positive kind-2 row for it has been researched. A unit no tech gates - villager,
   house, spearman - is available always, which is what stops the rule from becoming a blanket lock.
2. **A tech that costs nothing, takes no time, has no home building and does nothing but make units
   available is bookkeeping, and the engine applies it as soon as its prerequisites are met.** That is
   why aging up makes the barracks, watch tower, pike and castle appear without the player spending a
   click on an invisible tech, and why the rule needs no per-age table in content.

Not interpreted: the four `b = 0/-1` rows (the Huns' house, and four "Enable sheep/llama/cow/turkey"
rows with an attribute in `c`). Their direction is not established by anything in the data, and guessing
would show up as a unit a civilisation mysteriously cannot build. They are recorded here as a known gap
rather than silently read as "disable".

## D25 - A unit's identity comes from its stats and the upgrade graph, not from its name code

The spear question ("spearman, then a researched pikeman, then a researched halberdier") exposed a wrong
id in our content: `[role] spearman = 74`, where 74 is the **militia**. Two independent facts say so and
the name field says otherwise:

- **Stats.** 74 has 40 hp, 60 food + 20 gold, 21 s - published militia numbers. A spearman is 45 hp and
  35 food + 25 wood, which is unit 93.
- **The upgrade graph.** Every infantry upgrade starts at 74: `[3, 74, 75]` "Man-at-Arms", `[3, 74, 77]`
  "Long Swordsman", `[3, 74, 473]` "Two-Handed Swordsman", `[3, 74, 567]` "Champion". Nothing upgrades
  74 into a spear unit.
- **The name field** calls 74 "SPRMN" and 93 "PKEMN" - exactly one ahead of what those ids are.

So the rule for content: **trust the stats and the graph.** The name code is an abbreviation, right for
most units and wrong for this line, and a name is not evidence. The spear line is **93 → 358 → 359**,
with 87 "Pike" (free, instant, requires Feudal) making 93 available, 197 upgrading 93 → 358 at the Castle
Age, and 429 upgrading *both* 93 → 359 and 358 → 359 at the Imperial Age.

A second rule fell out of the same chain and is worth stating: **an upgrade target is not "available", it
is an upgrade.** `[train.entry]` names the line (a barracks trains militia and spearmen), the family root
means a pikeman matches that entry, and `check_train` refuses a unit whose upgrade tech has not been
researched - otherwise a Dark Age barracks could train halberdiers the moment someone wrote them into the
entry list.

### D24a - The zero flag is "disabled", and the negative ones are not that

`[2, unit, 0, -1, 0]` takes a unit away: one row in the whole table, in the Huns' tree, for the house -
and the Huns cannot build houses. That is enough evidence for the zero flag, and the engine reads it as a
civ-level bar that no tech can lift (`UnitBarred`, "not available to the Huns").

The four `b = -1` rows are **not** that. They live in effects called "Enable sheep/llamaged/cow/turkey",
each carried by a tech that *costs* 40 food + 25 wood and takes 15 s (555 and its siblings), and each
carries an attribute in `c` and a value (0.85) that no enable/disable reading accounts for. Whatever they
are - a scenario-tool leftover, or a herding special case - they are not a match mechanic, and reading
them as "disable" would be inventing a rule. They stay uninterpreted and named here.

## D26 - Teams are content, and team bonuses wait for the mechanics their effects touch

Combat already knew one rule that is really a team rule: `FriendlyFire` refuses a target belonging to the
attacker. That is the one-player case of "allies are not targets", so the general rule went in with
content behind it:

- `[start] teams = [1, 1]` names a team per player (entry *n* is player *n+1*, the last entry covers the
  rest). **Empty means a free-for-all** - every player is their own team - so no pack has to say
  "nobody has allies", and the eight existing combat tests keep the meaning they had.
- The resolved teams are hashed **only when content named them**, so a world without teams hashes exactly
  as it did before the feature existed.

**Team bonuses are deliberately not applied yet.** Each civ has a `team_bonus_effect`, and every one
sampled targets a mechanic this engine does not have: British `[5, 87, -1, 13, 1.2]` is an archery range's
work rate, Goths the barracks' work rate, French `[4, 569, -1, 1, 2]` is line of sight, Slavs
`[0, 12, -1, 21, 5]` is garrison capacity, Teutons `[1, 77, 1, -1, 2]` is a farm resource. Applying them
today would be invisible - a step with tests that cannot fail - so the effect ids stay carried and
unapplied until the mechanics land, and that is cheaper than pretending.

The same reasoning retires the "implement kinds 0 and 1 next" plan: kind 0's attributes are 19/20/21 (four
hundred rows of garrison and projectile behaviour) and kind 1's are resource ids including farm food, and
the sim has **zero** mentions of a farm. Both need mechanics before they need an interpreter.

## D27 - A farm is a building content calls a farm, and workers stand beside what they work

`[farm]` sat in `economy.toml` for months with nothing reading it. It now names two things the engine
cannot know by itself:

    [farm]
    unit = "farm"        # a role, resolved to unit 50
    resource = "food"    # the column a worked farm fills

That is the whole of the engine's knowledge about farms. `gather_source` returns a resource name and an
optional pace for **any** entity: a resource node names its column, a building does if content says it is
the farm. `use_villager_rate` picks who sets the pace - the building's own work rate or the farmer
variant's - which is a data question, so content holds the answer and the comment records that the 0.4/0.53
reading is still unresolved.

A farm is worth `food_per_farm` food from the moment its plot is marked out, and it is unworkable until
finished (`BuildingNotReady`). The plot *is* the budget: a half-built farm destroyed takes its food with it,
and a spent farm stays as a building that yields nothing, like a bare tree.

**Two bugs came out of this, and both were about standing.** The gather tick decided a worker had arrived
when `position == node.position` - true for a tree you stand on, never for a building, whose own tile is
blocked. And the builder had always walked *onto* its site, finishing the job standing inside the
building: from there no route home can be planned, because the tile it stands on is blocked. So:

- `work_position` finds the neighbouring tile that is both free **and reachable** (a free tile inside a
  wall is no place to stand - that cost a scenario step), in a fixed scan order;
- both the gather and the build ticks accept adjacency as arrival, and refuse to walk into the building;
- `return_task` used to read the stockpile column off the node's *kind*, which returns `None` for a
  building - so a villager on a farm filled its load and then stood there forever. It now asks the same
  authority the order asked (`gather_source`).

The column was briefly carried in `Task::Gather` instead. That was reverted: `Task` is 32 bytes either
way, so the field cost nothing, but it also bought nothing, and one authority is better than two.

## D28 - Farm food is a resource the dat numbers, so content names the number

Horse Collar, Heavy Plow and Crop Rotation are kind-1 commands: `[1, 36, 1, -1, 75]`. Read as units they
look absurd (a bombard cannon's price?), and the earlier notes in this repository read the field as a unit
id. It is a **resource** id, and the numbers settle it: 75, 125, 175 are the published food-per-farm
bonuses of the three farm upgrades, and they are all additions to resource 36.

So the importer decodes kind 1 into its own table (`resource_modifiers`), the engine carries it, and
content says which id means "food per farm":

    [farm]
    food_resource = 36

That is the third appearance of the same shape (D17): the dat has a namespace with no names in it, the
importer refuses to invent names, and the content pack supplies the meaning. With no `food_resource`,
farm food simply never changes.

**Existing plots hear about an upgrade; spent plots do not.** A plot's food is a budget, so a tech that
adds 75 adds it to every plot the player has *that still has food in it* - the game's farm upgrades do
benefit a farm already standing. A plot that has given everything stays at zero: its food was eaten, and
an upgrade that refilled it would be inventing food. Modes other than 0 (set) and 1 (add) exist once
(`-1`, effect 605) and are carried without interpretation, like `other:` attributes.

## D29 - The dat spells "a barracks exists" as a tech, and the engine has to grant it

The Feudal building tree was unreachable, and the reason is one of the dat's odder habits. The archery
range is made available by tech 147, which requires **101 (Feudal) and 667**. Tech 667 requires **122 and
638**. Tech 122 is called "Shadow Barracks -- Age Two" and has an effect with **no commands at all**; 638
has no effect whatsoever. Nothing would ever research them, so 147 could never apply, and the archery range
could never be built - in a game where it is an ordinary Feudal building.

The pattern is systematic: 194 techs in this dat have no effect, and a further set have an effect that does
nothing. The Feudal Age itself needs 659, which needs 120 (a shadow town centre) and 638 - which is why
every test until now granted 659 by hand before researching an age. That hand-granting was the smell.

So `availability_techs` now includes techs that cost nothing, take no time, have no location, **and** whose
effect is either absent or does nothing except make units available. They are granted as soon as their own
prerequisites are met, exactly like the availability techs already were. Two consequences:

- the Feudal Age is reachable by playing (500 food at the town centre) with no help from a test;
- the archery range, stable and everything else behind the shadow chain arrive with it.

**Correction (same day): this is not a simplification after all.** The first version of this note claimed
that an archery range becoming buildable "without a barracks standing" was wrong against the game. Two
things say otherwise. In AoE2 the archery range's only prerequisite *is* the Feudal Age - it has no
building prerequisite - and the dat agrees: 667 requires `[122, 638]` with **min 1**, and 638 is a marker
with no requirements, so that half of the gate is vacuous **by design**. The barracks shadow is in an OR
set with an always-true marker, which is how this dat spells "no building needed". The hand-granting that
used to be necessary was the real defect, and it is gone. See D30 for the field that made the OR visible.

## D30 - A tech's prerequisites are "at least N of these", and the engine was reading "all"

The dat's tech record carries **two** fields: `required_tech_ids` (up to six) and `min_required_techs`.
`ruage-tools` exported the first and dropped the second, so every prerequisite in the engine read as
"all of them". The dat means "at least this many":

| tech | requires | min | meaning |
|---|---|---|---|
| 120 Shadow Node, Age Two | 110, 258, 108, 122, 282 | 2 | any two Dark Age buildings |
| 135 Shadow Node, Age Three | 132, 106, 130, 128 | 2 | any two Castle buildings |
| 147 ARRG (make avail) | 101, 667 | 2 | Feudal **and** (barracks or marker) |
| 659 (used by the Feudal Age) | 120, 638 | 1 | node **or** marker |
| 85 Hand Cannon | 103, 47, 285 | 2 | Imperial and (Chemistry or free-Chemistry) |

Nineteen techs in the table have `min < count`. Both copies of the gate - the order path in
`check_research` and the availability resolver - now count satisfied prerequisites and compare to
`min_required`. Content that says nothing gets "all of them", which is what a pack written before the
field existed meant, so no old pack changes meaning.

**What this quietly shows:** the outcome rarely changes, which is why "all" survived this long. In the
cases that matter the OR-sets contain an always-true marker (638, 110), so demanding everything happened to
land on the same answer. The rule was wrong even where it did not yet bite.

**And a test that proved nothing.** The first version of this test used the Hand Cannon tech, on the theory
that "all three" would make a hand cannoneer unobtainable. The mutation check - restoring the "all" rule and
running the suite - showed it passing anyway: 285 ("C-Bonus, Free Chemistry") is granted for free once its
own prerequisites are met, so all three were satisfied regardless. The test that counts now uses a pack of
our own, one tech per gate copy, because no tech in this dat can reach the order path with a count.

## D31 - Housing, and the exemption that has to come from the tree

`[population]` was the last parsed-and-unused table: `cap_default = 200` and `house_population = 5` had
been validated at load for months while the sim never read them. Housing is now real:

- population is the number of live units a player has, one each - the dat carries no per-unit cost, and
  neither does the game;
- the ceiling is `min(cap_default, houses x house_population + town_centers x town_center_population)`,
  counted from **finished** buildings only: a house supports its five when it is finished, not when the
  site is marked out;
- a train order past the ceiling is refused with `PopulationCapped`, before any price is quoted.

`town_center_population = 5` is new content, and it is published fact rather than invention: a house and a
town centre both support five, a castle twenty (the manual and the AoE2 DB agree; the Chinese +5 is a civ
bonus, not a rule). A castle is not modelled, so a pack that wants its twenty says so when castles land.

**The Huns are the reason this needed a rule rather than a formula.** Their tech tree takes the house away
(`[2, 70, 0, -1, 0]`), so a cap that counts houses leaves them at zero population for the whole match. The
exemption is therefore read from the *same* content the bar comes from: if the player's civilisation bars
the house, no house arithmetic applies and the ceiling is `cap_default`. No new field, no invented rule,
and a pack with no civs has no exemption because it has no bars.

Two smaller findings, both from making the tests behave: a trained unit whose every neighbouring tile is
taken **waits inside its building** rather than being lost, so a crowd around a town centre stalls training
- that is the engine being careful, and the test helper now says so instead of asserting an arrival that
cannot happen. And the population fixture had to load the **effect tables** for the Huns' exemption to
exist at all: a bar is read from the civ's tech-tree effect, so a world without effects has no exemptions.

## D32 - Reach is a distance, and it is resolved when the order lands

`range` is attribute 1, in tiles, above zero on 277 of the 1,166 units - and until now every one of them
closed to adjacency and fought like a spearman. Combat now asks one question with two answers:

- a unit with reach strikes from anywhere inside it, and walks only until it is inside;
- a unit with none strikes from the next tile, diagonals included, which is the rule it always had.

Both are one comparison on squared distance in engine units, so there is no square root in the tick and no
trigonometry to disagree with itself across platforms. Range is carried on the task, resolved at order time
like damage and reload, so a tech that lands mid-fight does not extend a shot already in the air - the same
reasoning as the gather rate in D-v0.58.

**Range is the fourth attribute to ride the existing modifier path**, and the one that finally makes the
upgrade line visible: Fletching, Bodkin Arrow and Bracer are `range +1` on class 0 (archers), and they now
actually move the archer's reach rather than being carried and ignored. The test is behavioural rather than
numeric: an archer five tiles from its target walks a tile without Fletching and does not move with it.

**Not modelled, and named rather than implied:** projectiles (an arrow is instant damage at range, not an
entity), line of sight (a ranged unit can shoot anything within reach it has a path to, walls included),
and buildings that shoot (the town centre's arrows need garrisoning, which is a mechanic of its own).

## D33 - A queue is a side table, and the validator has to count orders it cannot see applied

One slot per building was a slice, not the game: a real barracks holds five. `[train] queue_length = 5` is
now content, the head of the queue stays exactly where it was (`Entity.training`, so the tick loop keeps its
fast path and `Entity` does not grow), and everything behind it waits in `World.queued`, a side table keyed
by building - the same shape as `paths` and `Units::variants`, and for the same reason (pass seven).

Three rules come with it:

- an order is **paid when it is placed**, which is what makes a queue a commitment rather than a wish;
- a finished unit hands the building to the next one, which starts counting from the create time it was
  priced at;
- the population gate counts **committed** units - the one being made plus those behind it - so a queue
  cannot be used to slip past a housing ceiling.

**The subtlety that cost an hour, and is worth the paragraph.** `command_apply = "next_tick"` means an order
joins `World.queue` and is applied on the next step - so `submit_checked` validates against state in which
the *previous* orders have not landed yet. Thrown at a five-deep queue, five orders placed in one tick all
passed validation, because each saw an empty queue. The checks now count pending `Train` commands for the
same building as well, and separately count pending orders per player for the population gate. The
immediate-apply mode would have hidden this entirely, which is a good argument for keeping the suite on
`next_tick`.

A weak test fell out of the mutation run as well: `queued_units_count_against_the_cap` placed two orders and
expected the third to be refused, and it passed even with the committed count deleted - because the two were
still *pending*, so the pending-orders rule caught it instead. It now steps once, asserts the head is
running, and fails when the committed count is removed, which is the state it was written for.

## D34 - The ceiling pauses training; it never refuses the order (correcting D31's refusal)

Housing went in with the ceiling refusing orders: `PopulationCapped` before the price was quoted, and
committed units counted so a queue could not slip past. **That was wrong, and the correction is the rule
the game has:** a queue may hold more units than the player can support, and when the ceiling is reached
**training pauses** - the job keeps its remaining ticks and waits for a unit to die or a house to finish.
Refusing the order made the ceiling a wall you cannot queue behind, which is not what a city-builder does
with an idle barracks.

So `check_train` no longer looks at population at all, `PopulationCapped` is gone, and the production pass
asks once per player with a building in production whether that player is at their ceiling; a capped player's
jobs are skipped, which leaves their countdown untouched. When a slot frees the same job continues.

Two things this cost, both worth naming. First, "committed units count against the cap" and its two helpers
existed only to serve the refusal, and both were deleted rather than left as dead code. Second, the tests
that asserted `PopulationCapped` were *five* and all had to be rewritten to the new shape - which is the real
price of a wrong rule: the tests cement it.

**A weak test of my own, again.** `a_full_player_holds_the_order_and_pauses_the_clock` asserts that nothing
arrives and that the countdown does not move - and it still passes with the pause removed, because a crowd of
five villagers around a town centre leaves a finished unit nowhere to stand, so "nothing arrived" is true for
the wrong reason. The mutation *is* caught, by `a_queue_can_exceed_the_ceiling_and_drains_later` (which
expects five alive after eight thousand ticks) and by the two house/death tests. Recorded rather than
quietly kept: a test whose failure mode is masked by a second rule is worth less than it looks.

## D35 - A shot is an entity, and the tick's shape says where it may be created

Ranged combat landed instant damage at range (D32). A shot is now an entity: it flies at
`[combat] projectile_speed` tiles per second, carries the damage resolved when the order was given, and
applies it where it was **aimed** - a position, not the target - so an arrow at a unit that walks or dies
first finds nothing and does not pick another. The kind is `EntityKind::Projectile`; it is not a unit, so it
costs no population and cannot be attacked (`NotATarget`). Speed zero means instant damage, which is what
every pack written before this meant.

**Two shapes of the tick dictated the implementation, and both were discovered by breaking them.**

1. The entity list is **index-stable for the length of a tick**: the movement pass holds indices into it and
   applies positions afterwards, with a fallback by id and a panic when even that fails. Creating an arrow
   inside the strike pass shifted every later index and the panic fired inside `step`, in tests that never
   mentioned projectiles. So shots are *recorded* in the strike pass and created once movement has been
   applied - the same discipline as `finished`, `switches` and `buried`.
2. There was already a `spent` vector in `step` (waypoints consumed), so the obvious name for landed arrows
   bound to the wrong one and arrows were never removed. It is `landed_arrows` now.

The projectile id is "one past the highest", which is an O(n) scan per shot. Training spawns do the same, so
this is consistent rather than new, and it is recorded in `docs/performance.md` as the thing to fix first if
arrow-heavy fights ever show up in a profile.

## D36 - Garrison is data, buildings fight, and line of sight still has no consumer

Every unit carries `garrison` in the dat (a town centre 15, a castle 20, a barracks 10) and 55 buildings
have a real capacity; town centres, towers and castles also carry an `attack` array and a `range`. Both were
parsed and unused. Now:

- `Command::Garrison { unit, building }` puts a unit inside, `Command::Ungarrison { building }` lets it out.
  The capacity is the **building's own field**, so the only numbers are the data's. The unit must be standing
  on the building or beside it (`TooFar` otherwise) - a garrison is not a teleport.
- A garrisoned unit still exists: it costs population (the game charges for it), it cannot act
  (`Garrisoned`), it cannot be attacked (`NotATarget` - the building is the target), and it is not on the
  map for movement.
- **A garrison dies with its shelter.** Ejecting everyone onto the rubble would let a razed town centre
  disgorge fifteen villagers into a fight, which is generous in a way the game is not.
- A **building fights like a unit that cannot walk**: the reach is its `range`, the damage its attack array,
  and the shot is a projectile (D35). `start_attack` skips pathing for a building - pathing from a blocked
  tile refused every shot a town centre wanted to take - and the guard is the *kind*, not the speed, because
  a stationary unit in a test still wants its route.

**Line of sight is deliberately still out.** `los` (844 of 1,166 units) would let the engine answer "what can
this player see", but nothing asks: there is no fog, no scouting rule, and no targeting rule that hides
anything. A visibility model with no consumer is the mistake D26 recorded, so it waits for one - a UI
overlay, a scenario condition, or a rule that makes unseen units untargetable.

## D37 - Correcting D36: a garrison is ejected at 20%, and a building's volley is dps arithmetic

Two rules went in wrong in D36 and are corrected here, on published sources and the user's statement of
them.

**Ejection, not death.** D36 said a garrison dies with its shelter. It does not: a building **ejects** its
garrison once it falls to **20% of its hit points or below**, and refuses a new garrison until it is repaired
above that. The units come out unharmed and land where they already stand - the game does not punish a
player for a building taking damage. (Rams and siege towers are the exception: only destruction ejects them,
and this engine has no garrisonable ram yet.) The fraction is content:
`[combat] garrison_eject_fraction = 0.2`, with the source named beside it, and the check sits where damage is
applied rather than in a per-tick scan.

**Extra arrows are dps arithmetic, and the qualifier is the damage class.** The published rule:

    extra arrows = floor( sum of garrisoned units' dps / building dps )

where a dps is the **pierce (class 3) attack over the reload**. Two consequences worth stating: the division
happens **once at the end** (not per unit), and only units that deal *pierce* damage from range contribute -
which is why a Throwing Axeman, a Mameluke or a Gbeto adds nothing despite being ranged. That qualifier is
data rather than lore: our own table shows the axeman (281) and the Gbeto (1013) as ranged with a **melee
(class 4)** attack and no pierce, while archers, cavalry archers and hand cannoneers carry pierce.

**Two things deliberately not modelled yet, both recorded rather than papered over:** the published rules have
each extra arrow transmit the *garrisoned unit's* attack (only the first arrow is the building's), and a
garrisoned villager counts as a fixed 2.5 dps whatever the villager's own numbers are. Today every arrow in a
volley carries the building's damage, and villagers contribute their own (zero) pierce dps. Both are visible
differences from the game and neither is guessed at.

**A test that passed for the wrong reason, found by mutating the wrong thing.** The first Throwing Axeman test
survived removing the range qualifier, because an axeman has no pierce damage to contribute anyway - the
discard happened in the dps function, not the range check. Mutating the *damage class* filter instead fails it,
which is what that test actually pins.

## D38 - A production building holds only what it has just trained

D36 let any building with a garrison capacity take any unit standing beside it. That is wrong for the five
buildings that exist to *make* units: the barracks, archery range, stable, siege workshop and dock accept only
a unit they have just trained - in the game by putting the rally flag on the building itself - and a unit that
has walked out can never come back. Town centres, towers and castles take units in from outside.

**The data draws the line without help from a list of names.** Those five buildings each carry a garrison
capacity of ten and **no attack array**; the three shelters each carry a capacity *and* an attack array with a
range. So the rule is "a building that can fight takes outsiders in", which is the same split, derived rather
than enumerated, and it keeps the engine free of a hard-coded set of building ids (D17).

The refusal is `CannotGarrisonHere`, and its message says what the rule is rather than what the code did.

**One half of the rule is not modelled yet, and is named:** a freshly trained unit can be kept inside its
production building by rallying to the building itself. This engine has no rally points at all - a trained unit
always appears on the neighbouring tile - so today a barracks holds nobody. That is the correct behaviour for a
production building under this rule, and the other half arrives with rally points, which is a step of its own.

## D39 - A rally point is a place, and rallying to the building means staying inside

The engine had no rally points at all: a trained unit appeared on a neighbouring tile and stood there, whatever
the player wanted. `Command::Rally { building, point }` adds them, with two behaviours and one default:

- `Some(tile)` - the unit walks there (`Task::Move`) and stops. `None` clears the rally and restores the
  default, which is where it appeared. A tile off the map is refused (`OffMap`); a tile with a tree on it is
  fine, because a rally point is somewhere to walk to, not somewhere to stand.
- `Some` of the **building's own tile** - the unit stays inside (`Task::Garrisoned`). That is how the game
  spells "keep them home": you right-click the building itself, and the `RallyPoint::Inside` state is derived
  from that click rather than needing a second command. This is the half of D38 that needed rally points: a
  production building can now hold what it has just trained, and it still never takes an outsider.

The state is a side table (`World.rallies`, by building) - most buildings have no rally, and `Entity` is walked
every tick - and it is hashed, because where a building sends its units is state a replay must keep.

`Check` runs at order time: the building exists and is a building, the tile is on the map. What is *not* checked
is whether the rally can be reached - a unit ordered to walk somewhere walled off simply stands where it
appeared, which is the same shape as a villager whose node is gone. Recorded rather than silently assumed: a
failure to path is not reported to the player yet.

## D40 - A rally point is a place, a resource, or a pocket you cannot enter

D39 treated a rally tile as "walk to exactly this point or stay home", which is wrong twice over, on the
user's correction:

- **Unreachable is not a refusal.** A rally into a walled pocket or the far side of a forest means *go as close
  as you can*. `walk_towards` tries the tile, then each ring around it outwards (ten rings), taking the first
  tile a route exists to, and switches the unit to `Move` there. It stays where it appeared only if nothing at
  all is reachable - which is now the rare case rather than the default.
- **A rally onto a resource is a gathering order.** If a resource node stands on the rally tile, the finished
  unit is given a gather order for *that* node through the ordinary path (`start_gather`), which is why the
  answer to "rallied onto gold surrounded by wood" is the gold or nothing: the node is chosen by the tile, not
  by what is nearest, so a tree next door is never substituted. The test builds exactly that map - stone with
  wood on three sides - and asserts the task names the stone and the stone's amount falls.

Two fallbacks, both deliberate: a unit that cannot *work* the node (a spearman rallied onto a tree) and a node
it cannot reach both fall back to walking as close as possible, because a rally is an instruction about a place
first and a job second.

**What is still not reported:** a rally that cannot be reached at all leaves the unit where it appeared, and
nothing tells the player. That is D39's gap, narrowed but not closed.

**Provenance, asked directly: does the game refuse a rally on a cliff?** No - it accepts the click. Nothing in
the series wiki, the DE notes or the support forums describes a rally point being *rejected*; the flag is set by
"right-clicking on the map or mini map" and units "automatically travel to the Gather Point"
(ageofempires.fandom.com, *Gather Point*), which is only meaningful if the point can be somewhere the unit
cannot stand. Where a game in the series *does* validate, the wiki says so plainly: AoE IV's rally "cannot be
set on their own associated building" and not on enemy units. AoE II has no such list, and it does allow a rally
on the building itself - DE later added an explicit Alt+hotkey to do exactly that, which is D39's `Inside`. The
"as close as possible" half is corroborated by DE's own pathfinding note that a villager returns to work "from
the closest point to the gather point" (update 58259), and the resource half by a DE bug report whose expected
behaviour is that a gather point on a resource has the villager work *that* resource (forums.ageofempires.com,
build 14512201) - the reported defect being that a mined-out node sends her somewhere near herself instead of
near the gather point.

So: **accept the placement, send the unit as close as it can get**, which is what D40 does. The one refusal we
keep is off the map, and no mouse can produce that click. Current limits of the comparison, stated: our terrain
is only grass and forest (no cliffs, water or elevation yet), so a cliff rally is a forest rally today and takes
the same ring search.

## D41 - Kind 0 sets a unit's attribute, and population is the first one the engine consumes

The dat's effect commands have three namespaces with one layout, `[kind, unit, class, attribute, value]`:
kind 0 **sets**, 4 **adds**, 5 **multiplies**. Only 4 and 5 pack the value for attack and armour, which is why
the same attribute arrives as `1025.0` in one and `5.0` in another. The importer already decoded 4 and 5; kind 0
was carried raw and unread, all 106 commands of it.

It is now decoded (importer `ATTRIBUTE_NAMES` grew, `_modifier` accepts kind 0 with `op = "set"`), and the
attributes are named only where a tech says what they mean:

- **21 population** - the Slavs' team bonus sets twelve military buildings to 5; the Chinese town centre to 10.
- **11 accuracy** - Thumb Ring sets 100, City Rights 87.
- **19 ballistics** - ballistics sets a flag on every bow, gun and galley; Fire Ships auto-ballistics.
- **20 minimum range** - Andean Sling sets a slinger's to 0, the Koreans' team bonus to 1.
- **102 extra projectiles** - Yasama sets 3 on towers.

Two stay `other:<id>` (22 on City Rights, 105 on Mercenaries): nothing in the techs pins them, and a guess is
worse than a hole.

**What the engine consumes: population, and only population.** A building's provided population is now
`population_provided(player, unit)`: what content gives the role (house 5, town centre 5), unless a `set`
modifier replaces it. The ceiling sums that per finished building instead of counting role instances, which is
the shape a team bonus needs - a barracks supports nothing by content and five by the Slavs'.

**Applying a team bonus needed a new path.** A team bonus is an **effect no tech researches**, so
`modifiers_for` now reads two sources: the effects of researched techs, and a per-player `bonus_effects` list
filled from the civ's `team_bonus_effect` at `load_civs`. It is derived from `civ_by_player` and not hashed
separately.

Mutation-proved both new paths: ignoring `set` fails the Slavs test, and never granting the team bonus fails it
too. The British case in the same test is the control - a British barracks supports nothing, so the five is the
bonus and not the building.

Not yet consumed: accuracy, ballistics, minimum range, extra projectiles. Each is a real mechanic the engine
does not have (leading a shot, a minimum range for siege, extra arrows from a tech rather than a garrison), and
they are counted in `docs/next-step.md` rather than left as an implied "done".

## D42 - An arrow lands where it was aimed, and Ballistics aims further ahead

D35 gave projectiles a flight but not a way to miss: the arrow landed at its aim point and then damaged the
target wherever the target had got to. That made every shot a hit, and it left the ballistics flag (attribute
19, D41) with nothing to change. Both halves are now there:

- **A hit needs the target to be where the arrow landed.** On landing, the target takes the damage only if it is
  within **half a tile** of the aim point - the space a unit occupies - and the arrow leaves the world either
  way. Half a tile rather than a whole one because an arrow's last step is half a tile at the content's
  projectile speed, so a wider tolerance would let a stopped arrow count as a hit on a target it never reached.
- **Ballistics leads the shot.** With attribute 19 set on the shooter's unit, the aim is the intercept, not the
  target: iterate the time of flight (`distance / projectile_step`, three passes), each pass moving the aim
  along the target's motion for that many ticks. Integer arithmetic throughout, including an integer square
  root, so two replays aim at the same point.

The target's motion is **read, not stored**: `Entity.velocity` is a spawn-time field never updated by movement,
so `motion_of` asks what step the target's own task would take this tick. That keeps the aim honest without
adding a per-tick write to a hot struct.

**What this changes about existing behaviour**, stated because it is a real change and not an addition: a shot
at a moving unit can now miss. The scenario anchors do not move because nothing in them fires an arrow, which
also means they do not exercise this path - the ballistics tests are the whole coverage.

Mutation-proved both ways: removing the landing check makes the runner take arrows without Ballistics (the test
fails), and disabling the lead makes the Ballistics case miss.

Not modelled: a shot can still hit the ground and vanish with no feedback, there is no chance-to-hit roll (the
game has an accuracy attribute, 11, now decoded but unconsumed), and a missed arrow does not collide with
whatever happens to be standing at the landing point.

## D43 - Accuracy is a chance to aim true, and a failed roll scatters the shot

Attribute 11 has been decoded since D41 (Thumb Ring sets an archer to 100, City Rights to 87) and unconsumed.
It is consumed here, and the data supplies the base: an archer is 80, a hand cannoneer 65, a crossbowman
lower still, and **every building that shoots is 100** - which is why a town centre's volley never scatters and
needs no special case.

The model:

- **Accuracy is a percentage chance that the shot is aimed true.** Rolled once per shot from
  `splitmix( tick * prime + shooter id )`, so there is no generator state to carry and a replay makes the same
  misses in the same places. A shot that *is* aimed true goes exactly where the archer meant, whether that is
  the target's feet (D35) or its intercept under Ballistics (D42).
- **A failed roll moves the aim point** by `[combat] accuracy_scatter` (2 tiles, authored - the data says how
  accurate a unit is, never how far a miss goes) in one of **eight directions**, since a scatter needs a
  direction and integer octants need no trigonometry. The landing rule from D42 then does the rest: an arrow
  that lands two tiles off is a miss, and nothing has to know it was "inaccurate".

Interaction with D42 is the point: accuracy scatters the aim, Ballistics moves the aim, and **one landing rule
decides the hit** - so the three rules compose without any of them knowing about the others.

**Cost, and where it deliberately is not paid:** the roll is one hash per shot fired, and the whole path is
skipped when `accuracy_scatter` is zero, which is the default for any pack that says nothing about it. Two test
fixtures pin the scatter off (`combat.rs`, and the aim-geometry tests in `ballistics.rs`) because their archers
shoot at immobile targets and a scatter roll would make a hit depend on which tick the test fired on. That is a
fixture choice, stated here so it does not read as "accuracy is untested": `accuracy_decides_whether_the_arrow_
goes_where_it_was_aimed` is the test, and it uses the real `combat.toml`.

Mutation-proved twice: never consulting accuracy fails the test, and making every roll pass fails it too.

Still not modelled: accuracy against a target the shooter has to lead (the two are multiplied rather than
composed), and no "arrow hit the wrong unit" - a missed arrow lands on empty ground even if something stands
there.

## D44 - Minimum range is a dead zone the unit brings with it, and the engine backs out of it

The dat stores `min_range` per unit beside `max_range`, and it is not a siege-only field: 46 of the 1,166 units
have one. A mangonel's is 3 tiles, a bombard cannon's 5, a trebuchet's 4, a dock's 2, a heavy scorpion's 1 -
and an archer's is 0. So the rule is **data**, not a list of siege ids: a unit cannot strike a target inside its
own dead zone, whatever kind of unit it is.

The engine's half is the part the data cannot state: **the attacker backs away**. A mangonel ordered onto a
neighbouring unit would otherwise stand there forever, unable to shoot and unable to leave, so inside the dead
zone the attack task steps *away* from the target - reflected through its own position, so the retreat is along
the line it already lies on - until the shot is legal, and only then strikes. It is a direct step, not a path,
so no route has to be invalidated while retreating.

Attribute 20 moves the dead zone and is consumed here as well: it arrives as a kind-0 `set` (Andean Sling sets a
slinger's minimum range to 0), which is the same shape population and accuracy use.

The number is resolved **at order time** with `range`, `damage` and `reload`, and it is part of the task and of
the state hash. A tech that lands mid-fight therefore does not change where an attack that has already begun may
be fired from - the same rule the other three numbers follow.

Mutation-proved: ignoring `too_close` fails the mangonel test, because the engine fires from inside its dead zone
on the tick the order lands.

Not modelled: blast damage (a mangonel's shot hurts a group, not a point), and the retreat does not reconsider
terrain - a mangonel backed against a wall will keep trying to step into it, which leaves it unable to fire until
it can move. Both are recorded rather than assumed.

## D44a - The dead zone is not siege-only, `set` modifiers were being dropped, and it is measured body to body

A review of D44 against the user's knowledge of the game ("skirmisher has minimum range too, and Andean Sling
removed it") found **three** things wrong. The data was right; the engine was not.

**1. A skirmisher has a dead zone as well.** Unit 7 - the skirmisher, whose name field says `XBOWM` one id ahead
as usual - carries `min_range` 1 beside its range 4. So do the slinger (185) and the genitour line (583, 596,
1010, 1012). D44's "mangonel 3, bombard cannon 5" was true but understated: 46 units have a dead zone and they
are not all siege.

**2. `set` modifiers never reached the value they name - Andean Sling did nothing.** `scale_fixed` applied `Add`
and `Multiply` and silently ignored `Set`. Population and accuracy handle `Set` themselves (which is why they
worked), but everything routed through `scale_fixed` - range, speed, gather rate, carry, **minimum range** - lost
it. So D44's claim that "attribute 20 is consumed" was false: Andean Sling's seven `set 0` commands and the
Koreans' three `set 1` commands (mangonel, onager, siege onager - their team bonus reduces the minimum range from
3 to 1) were all ignored. Now the **last** `set` replaces the base before additions and multiplications, in
research order, which is the rule every other modifier follows.

**3. A one-tile dead zone could never bite on a tile grid.** The comparison was centre to centre, and the closest
two units can stand is a whole tile - so a skirmisher's 1 was unsatisfiable, while a mangonel's 3 was fine. The
dead zone is now measured **to the target's edge**: `centre distance < min_range + radius(shooter) + radius
(target)`. Radii are data the engine already imports (0.2 tiles for a skirmisher and a villager), and reach stays
centre-to-centre on purpose - a dead zone is about two bodies that cannot share space, a reach is about the range
of a weapon.

Fixed test-first, with the test rewritten once: the first version asserted that a skirmisher "cannot shoot a
neighbour", which is wrong - it *can*, after stepping back. The test now says what actually distinguishes the two
cases: **where the first shot comes from** (1.6 tiles without Andean Sling, 1.0 with it) and **when** (tick 12
against tick 3).

Mutation-proved twice: dropping `Set` again makes the Andean Sling case fail, and measuring centre-to-centre
makes the skirmisher step back with the tech as well.

**What this says about the earlier review:** D44's own mutation check only removed the dead-zone comparison, so it
could not notice that the *modifier* path was dead. A feature is not covered by testing the branch it enables if
the input to that branch is never exercised.

## D45 - `overrides.toml` is real: the per-unit configuration surface, and it refuses rather than ignores

D6 promised a hand-written override layer, `docs/config.md` documented its path syntax, and **no code ever read
the file**. An edit there was a silent no-op — the worst shape a configuration surface can have, because the user
does everything right and the game ignores them. It is implemented now, test-first.

**Shape.** `"units.<id>.<field>" = value`, applied after every table has loaded. Fields covered: `hp`, `los`,
`speed`, `carry`, `work_rate`, `range`, `reload`, `min_range`, `accuracy`, `garrison`, `train_time`, `attacks`,
`armours` — the numbers a match depends on, each written to a field the engine actually reads. Arrays are
replaced wholesale; a partial merge of two attack arrays would be invisible in the running game.

**Three properties, each a test:**

1. **Refusal, not silence.** `techs.202.research_time` fails with a message saying units are what the layer
   covers; a unit id that does not exist fails naming the id; a field outside the list fails naming the field and
   the ones it does cover; a string where a number belongs fails saying what it *found*.
2. **Every replacement is reported** with its old and new form (`override units.79.min_range: 1.0 -> 0.0`),
   through `ContentPack::override_log` and through `ruage-headless --overrides`.
3. **The file is content.** It joins the handshake hash when it exists, so two peers with different overrides
   refuse each other's match — the whole point of D16's identity. A pack without the file hashes exactly as
   before, which is why it is not simply appended to `PACK_FILES`.

**Proven end to end, not just parsed:** `content_can_take_a_towers_dead_zone_away_without_a_technology` overrides
`units.79.min_range` to zero and watches a watch tower shoot the unit standing against it — with no Murder Holes
researched. That is the user's ask ("each unit would have its own config, and min range would be one of them")
reaching the simulation.

**Also confirmed while looking:** castles and towers really do carry `min_range` 1, and **Murder Holes is an
`add -1`**, not a `set 0` — the two shapes a removal arrives in. It needed no code: `scale_fixed` applies
additions, and the dead zone is data. The town centre has no dead zone, and Murder Holes does not list it.

**Not covered, and said plainly:** `techs` and `effects` overrides. The layer refuses them by name rather than
pretending, and widening it is a change in `crates/data/src/overrides.rs`.

## D45a - The review of the override layer: one pass wrote half a file, and two refusals were missing

D45 shipped without the adversarial review the project rule calls for. Doing it found three things; two were
defects and one was an untested promise.

**1. A refused override left the ones above it applied.** `apply_units` validated and wrote in the same loop, so a
file whose second entry was bad left the first entry applied and *then* returned an error. A half-loaded table is
exactly the failure a strict loader exists to prevent, and it would have been invisible: the caller sees an error,
the table is not what it was, and the difference is one number in one unit. Fixed by planning first: every path,
id, field and value is resolved into a plan without touching the table, and only a complete, valid plan is written.

**2. Nothing stopped `hp = 0`.** Content could set a unit's health to zero or negative, which is a unit that
cannot exist - dead on arrival, and every maximum-health computation would floor it back up. Health is now refused
below 1, and accuracy is refused outside 0..100 while the same check is being written.

**3. The hash promise was documented and untested.** "The file joins the pack's identity when it exists" had no
test behind it. It was true - the new test passed first time - but a claim in a doc is not a claim in a test, and
the value of finding that out is that the *next* change to the hash cannot quietly break it.

Mutated all three: removing the health check, dropping `overrides.toml` from the hash, and not applying the
validated plan each fail exactly the test written for them.

**What the review did not find**, said so that the absence is a claim and not an omission: no per-tick cost (the
layer runs once at load), no determinism risk (`BTreeMap` iteration is sorted, so the log and the application order
are stable), and no leak of the override into the simulation's state hash - an override changes the *tables*, and
the world already hashes what it derives from them.

## D46 - A match ends when a seat is gone

The prototype list's first item is victory and defeat, and the smallest honest rule is the one the list states:
destroy the enemy town centre, or lose yours. So **the seat rule** is the whole conquest rule - a side that has no
living building of the family content names in `[start] town_center` has lost - and it is enough to decide a
two-side match, a draw (both seats gone on one tick), and to leave a three-way match running while two seats still
stand.

Two implementation choices are part of the decision rather than of the code:

- **The side is a family, not an id.** An age-up replaces the town centre 109 -> 71 -> 141, so the check goes
  through `Units::root`. A rule that compared ids would stop seeing a Castle-Age player's seat.
- **The roster is hashed, the outcome is not.** "The enemy has lost everything" and "nobody was ever here" are
  different facts, and only the first ends a match; a world built by hand for a movement test has no players and
  must hash as it did before. The outcome is derived from `entities` plus `roster` at the end of the tick, so
  hashing it would be a second source of truth for the same fact - and it is recomputed only on a tick where
  something died, because a seat cannot disappear any other way. The idle tick pays one boolean.

**Known simplifications.** A town centre *under construction* counts as a seat - you have not lost while you are
rebuilding. A mode with a different victory condition (relics, wonders, regicide) is content the engine has no
vocabulary for yet, and the rule is deliberately one rule rather than a switch.

## D46a - Demolishing your own building lets its occupants out; losing it does not

Deleting is a death and takes the death path: health to zero, and the end-of-tick sweep does the rest. The one
deliberate difference is the garrison. `Command::Delete` of a building you own **ejects** whoever is inside,
unharmed, where they stand; a building **razed by an enemy** kills them, which is the behaviour the sweep already
had and which the D37/D38 work was built around.

The reason is not symmetry, it is intent: demolishing your own town centre is how a player pulls villagers out of
it, and a command that killed them would be a trap. The two paths now share one `ungarrison` helper, so they cannot
drift apart silently; the eject rule is pinned by `a_deleted_building_releases_its_garrison`, and the older
`a_garrison_goes_down_with_its_shelter` still pins the enemy case.

## D47 - Attack-move and patrol are one walk with two arrival rules

`Command::Move`, `AttackMove` and `Patrol` all plan a route and walk it. What separates them is what
happens while walking and what happens on arrival, and both are cheaper to express as state beside the
task than as three tasks:

- **Hunting** is `Task::Hunt { to }`: each tick (on a schedule, see below) it takes the nearest enemy
  inside `[combat] aggro_radius` and starts the ordinary `Task::Attack`. The walk it interrupted is
  stored in `World.resuming` and restored when the fight ends - that is the "survives contact"
  property an attack-move is bought for.
- **Patrolling** keeps the far end in `World.patrols`, keyed by unit. On arrival the two ends swap, so
  patrol is a hunt with a memory of where it came from and nothing else.

**Why side tables rather than fields in `Task`.** A patrol needs two positions and a radius, which is
40 bytes; `Task` is 32 and `Entity` is walked every tick (pass seven: widening it cost 4-6% for no
behaviour). A map lookup happens on arrival and on order, not per entity per tick.

**Why the scan is decimated.** It is a pass over every entity for every hunting unit - the only
order-path cost that grows with the world. `[combat] aggro_scan_ticks = 5`, staggered by entity id,
means four looks a second at 20 Hz, which no player can see, for a fifth of the cost. `0` means every
tick, and `aggro_radius = 0` means a hunt never engages - the behaviour of a pack that says nothing.

**A defect this slice found rather than added.** A unit that finished an order kept its `velocity`,
which is the speed `Command::Spawn` gives a fixture mover, so it slid east forever after arriving.
`stand_down` now clears task *and* motion wherever an order ends, which is what `Entity.velocity`'s
documentation always claimed and no test had pinned.

**Named limits.** Cover, terrain and stance are not consulted (that is D48's work): a hunt engages
whatever is inside the radius, walls included. A hunt that cannot reach its interrupted walk logs
nothing - it stands down, and `docs/next-step.md` keeps "no feedback for a failed order" as the one
open gap.

## D48 - A stance is a posture plus the ground it was given on

`Stance` is per unit and lives in a side table with its **post**: the position the unit stood on when the
stance was set. Aggressive is the absence of an entry, so a world that never sets a stance carries nothing
extra and behaves exactly as it did before stances existed.

- **Aggressive** chases anywhere (the old behaviour).
- **Defensive** chases only while the target is inside `aggro_radius` *of the post*, and walks home when it
  is not. That is what makes it useful for holding a chokepoint rather than for following an army.
- **Stand ground** never moves to fight and still strikes what is already within reach. It also does not
  engage at all while attack-moving: a unit that stopped to stare at an enemy beside its road would never
  arrive, and standing ground is about not *leaving*, not about refusing to go where it was sent.
- **No attack** refuses an attack order at the validator (`StanceForbidsAttack`) and never auto-engages.

**The validator reads the queue.** With `command_apply = "next_tick"`, a `Stance` and an `Attack` in the same
tick are validated before the stance lands, so `effective_stance` takes the last pending `Stance` for that
unit. This is the third appearance of the same trap (training queue, research guard, now stances) and it is
worth stating as a rule: *any check that reads state a queued command will change must read the queue too.*

**Not modelled:** stances do not affect a unit that is already fighting beyond the chase rule (no
"aggressive units return fire from further away"), and a defensive post is a position, not a leash length
that content can set.

## D49 - An entity's bookkeeping dies with it, and the sweep is the one place that knows

Six tables are keyed by entity id: `paths`, `rallies`, `queued`, `patrols`, `resuming`, `stances`. The state
hash walks every one of them, so an entry for a dead id is a leak that no test sees in play: it costs a little
state, hides a divergence inside a hash comparison, and never goes away. `paths` had this bug once; adding
three more tables in one session would have added it three more times.

The rule: **the end-of-tick sweep is the only place an entity leaves the world, so it is the only place that
prunes.** Two consequences worth writing down:

1. A unit killed *inside* a building is removed by the garrison retain, not by the hp sweep, so its id is
   collected into `buried` before the pruning loop. Without that, a razed town centre leaves its villagers'
   stances behind - the rarest case and the easiest to forget.
2. `bookkeeping_entries()` counts all six tables. It exists because "a dead entity left something behind" is
   otherwise unfalsifiable: the stability probe samples it, and the tests assert it returns to its baseline.

**Named cost:** the sweep runs per death, not per tick, and the tables are `BTreeMap`s of a handful of
entries, so the pruning is a handful of removals on a tick that already walks the world to remove entities.

## D50 - Vision is three states, and only one of them is state

Per tile and per player: **unseen** (never looked at), **explored** (looked at once, no longer watched),
**visible** (watched now). The radius is the entity's own `los` from the dat - 844 units carry one - rounded up
to whole tiles, so nothing lists which unit sees how far.

`explored` is one bit per tile per player and **is hashed**: it is a record of where a player has been, no other
table knows it, and a replay that lost it would show a different map. `visible` is derived on demand from the
positions of the player's entities, so it is not stored; `visible_mask` exists for a renderer, which needs one
pass per frame rather than one pass per tile.

Two consequences that are rules rather than details:

- **A garrisoned unit does not see out of its building.** It is not on the map; the building's own sight covers
  what it can watch.
- **An entity's sight is stamped when it arrives and when it moves**, never in a pass over the world. Fog only
  grows, so an entity that stood still has nothing to add.

## D50a - The fog pass that cost 19x the tick, and the fix that is a rule

The first version stamped every entity's disc every tick. It measured **2.2 us/tick at 126 entities and 466 us at
2,126** against a 36 us tick - a 19x regression that the idle probe caught in one run, before anything else was
built on top of it. Two fixes were tried in order, and the first one is worth recording because it *looked*
right: a cache of "the tile I last stamped from", checked per entity per tick, still paid a `BTreeMap` lookup and
a table resolve for every entity on the map (2,126 x ~220 ns = 466 us -> 130 us; better, still 3x the tick).

What worked is not a cache but a deletion: the tick already knows which entities moved (the delta list it is
about to apply), so the stamp moved **into that loop**, and into `insert` for arrivals. An idle world pays
nothing; a moving unit pays ~50 ns for the tick it moves, which is the only tick on which its disc can contain
anything new.

**The hole the tests had:** a building never moves, so it can only be stamped on arrival. Without that, a castle
built in the dark stays blind for the rest of the match - `a_building_sees_around_itself_when_it_appears` is the
test, and deleting the arrival stamp fails exactly it while every moving-unit test still passes.

**Known simplification:** sight is a disc and does not check line of sight through terrain - a unit sees over a
forest. That is the difference between "vision" and "fog with occlusion", and the terrain in this pack is grass
and forest only. Recorded rather than implied; occlusion belongs with terrain heights.

## D51 - A blast is where the stone lands, and the radius is the gate

The dat carries blast on the unit: `blast_width` (radius in tiles) and `blast_attack_level`. **27 of 1,166 units**
have a non-zero radius - mangonel 1.0, onager 1.25, bombard cannon 0.5, petard 0.5, scorpion 0 - so this is
imported data, not a list of siege ids, and the importer emits both fields (its own commit).

Three decisions the data does not make for us:

1. **The radius rides on the projectile, not on the shooter.** `Task::Fly.blast` means the blast belongs to *where
   the stone landed*, which is what a missed shot does in the game: the stone breaks on the ground somebody did not
   want it to break on. This also gives the instant-shot pack (`projectile_speed = 0`) the same behaviour through
   the same helper - a test, because otherwise "instant" and "flying" would silently differ.
2. **Everything inside takes the shot's damage, including the shooter's own units.** That is why a mangonel needs
   protecting, and it is the one place in the engine where friendly fire is not filtered.
3. **`blast_level` is carried and not consumed.** Every unit that has a level has a radius with it, so the radius is
   sufficient and the level's semantics stay an open question. A guess here would be a rule the data cannot
   contradict - exactly the kind of thing D44's `set` bug came from.

**Not modelled:** damage does not fall off with distance (the game reduces it for secondary targets); buildings take
the aimed hit only; and the blast is a disc with no line-of-sight check, like vision (D50).

## D52 - Soft avoidance shapes the last step, and only for a place you go to

The owner chose soft avoidance over pushing: units slide past each other, and overlapping remains possible.
What the engine implements is narrower than "collision", deliberately:

- **An exclusive goal** - `Move`, and the walk inside `AttackMove`/`Patrol` - does not take its **last step** onto a
  tile another unit already occupies. It stands one step short. That is the stacking a player actually sees: forty
  units ordered to one tile.
- **A goal that is a thing to be worked is not exclusive**: gather, build, return, attack. Two villagers must work
  one tree, two villagers must hand in at one town centre on the same tick, two attackers must reach one target.
- **Nothing is ever pushed.** A unit that already shares a tile stays there. Pushing is a different mechanic whose
  cascades are state to hash and tests to write.

## D52a - Three versions failed the suite, and that is what settled the rule

The first version blocked **any** occupied tile a mover's step would enter. Nine gather tests failed at once, and so
did "a plain move walks past an enemy": a villager could not leave the town centre's crowded doorstep, and a unit
walking past a standing enemy oscillated beside it forever. The second version cached the tile each entity last
stamped (`BTreeMap` per entity per tick) and cost 130 us at 2,126 entities - the same mistake the fog pass had just
made, in a different table. The third version sidestepped by quarter turns when blocked, which is arithmetically
exact (a 90-degree rotation preserves a step's length in fixed point, where 45 degrees needs a square root) and
behaviourally wrong: the mover stepped aside, the path pulled it back, and it never arrived.

What survived is the smallest rule that fixes the visible defect and breaks nothing else, and the *shape* of the
failure is the lesson: **a collision rule that can say "no" to a step must be able to say "yes" to the reason the
unit was walking in the first place.** Work goals are the ones with a reason beyond standing there.

**Cost:** a bitset of unit tiles, rebuilt on ticks where something moves (an idle world pays one `any` over the
delta list, and the idle probe confirms nothing is built there: 2.70 us/tick at 126 entities). Not hashed: it is
derived from positions the hash already covers.

## D53 - An order to one tile is an order to a place

A crowd ordered at one tile spreads over the tiles around it: each unit takes its own, counting the units already
heading there and searching in ring order. No state is added - the count is answered by asking the units' own
goals - so there is nothing to release when an order ends, which is what a claims table would have needed.

The planner was never the problem. Five units aiming at one tile was, and with D52's last-step rule all but one
stopped short of it and never arrived.

## D54 - Walls: content names what blocks, the grid grows a writable layer

`[build] blocking_roles = ["wall"]` with `wall = 72`: a wall is a **content** word and a content id, so the engine
learns neither a dat class nor a list of wall ids. A blocking building closes its tile on **insertion** (a half-built
wall is not walk-through) and frees it in the death sweep, where the tile is remembered *before* the sweep because
after `retain` there is no position left to look at.

Nothing else blocks. A ring of houses is walk-through, which is a deliberate simplification: making every building
an obstacle changes every path in every existing test, and the visible complaint the step was written for was walls.

**Not modelled:** gates (owner-passable pathing needs a per-player grid), and walls are one tile because every
building in this slice is.

## D55 - The presentation boundary: a snapshot, a camera, and no window

`crates/view` exists so the simulation never learns what a screen is:

- **`Snapshot`** is a plain description of what one player can see. The visibility filter is the simulation's answer,
  not a renderer's guess.
- **`Camera`** is tiles to pixels, flat ground only.
- **`render`** is a software rasteriser: rectangles for tiles, squares for buildings, dots for units. Colours are the
  *view's* own constants, and an asset pack later replaces `render`'s entity branch with a sprite lookup - the only
  function that has to change (D17's asset layer).
- **No window, no dependency.** A frame is a value; PPM is written by hand and PNG is one conversion away. A window
  would have needed a display this container does not have and a crate the offline build cannot fetch, and neither
  would have made the frame testable.

**Floats live here and nowhere else** (D9): pixels are not state.

## D56 - Frames and ticks: an accumulator, a cap, and a dropped backlog

`Clock::ticks_for(frame_ms)` runs a tick for every whole tick period accumulated. Sixty frames a second at 20 Hz is
exactly 200 ticks in ten seconds (a test asserts it), and `alpha()` gives a renderer the fraction between ticks.

Catch-up is capped at five ticks. A machine that stalls for a second must not then run twenty ticks inside one frame
- and the backlog is *dropped* rather than repaid, or every later frame would try to catch up for ever.

## D57 - The UI issues orders; it does not execute them

`Selection::order` returns `Vec<Command>`, and the caller submits them. Nothing in `crates/view` calls `step`, and the
test that matters is the one that hands the produced orders to `World::submit_checked` and watches the villager start
gathering - the UI cannot invent an order the engine would refuse.

The gestures are the game's: a click takes a unit before a building on the same tile, a band takes units and leaves
buildings, a double click takes every unit of that kind on screen, and a right click is gather on a node, attack on an
enemy, move on ground, rally on a building.

**One slice was not written test-first** - these three (22-24) were written test-and-code together - and the mutation
evidence stands in for the recorded RED: deleting the click's tile filter, letting a band take buildings, and issuing
no orders each fail exactly their own test. Recorded because a claim of TDD that was not TDD is worse than a named
exception.

## D58 - Foundations: one per tile, and a blind one is not a claim

The owner's rule, in his words: one player's foundations cannot be placed over one another; a foundation a
player cannot see does not block that player; and if someone places over a foundation and **starts
constructing**, the other player's building is cancelled.

**Visibility is read when the order is placed, not when the builder arrives.** That is the whole rule. A player
choosing a tile in fog is choosing blind - by the time the villager walks over, its own sight has made the tile
visible, so a check on arrival would refuse every overbuild and the rule would be unobservable. `check_build`
asks `vision(player, tile)`; if it is not `Visible`, another player's *foundation* on that tile is not in the way.

**Two judgement calls, both beyond the three sentences:**

1. **Only a foundation may be overbuilt.** A *finished* building blocks whoever is looking, fog or no fog. The
   owner's words are about foundations twice over, and the alternative lets a player cancel a town centre by
   laying a house in the fog - which is not the game either version plays. A finished building is also what
   makes the race decidable: if the other player finished before I arrived, my site is the one cancelled, since
   a newcomer does not raze a completed house by walking up to it.
2. **The newcomer wins a foundation-versus-foundation race.** The earlier builder does not defend its tile; the
   builder whose *first* work tick happens after the other foundation was placed takes the ground. That is the
   owner's sentence read literally ("places his building over mine and starts constructing it, my building gets
   cancelled"), and it is why cancellation is gated on `progress_ticks == 0`: cancelling on every tick makes the
   earlier worker defend instead, which is the opposite rule (mutation D, which failed the two cancel tests).

**The cancelled site is killed, not deleted**, so it leaves through the ordinary death sweep: its tile is freed
by the blocking sweep (`D54`) and its bookkeeping by the removal sweep. `bookkeeping_touches(id)` was added to
make that provable - the earlier `bookkeeping_entries()` count could not tell one entity's leak from another's.

**Known untested edge, recorded rather than hidden:** the "the other player finished first" branch is
implemented but not covered by a test. Staging the race needs a builder slower than the other side's build
time, and the only lever is `Command::Spawn`'s speed, where a tenth of a tile a second produced a villager that
walked to the map edge instead of the site - so the test would have asserted a pathing oddity rather than the
rule. The branch is four lines and its control (a finished building refuses placement) *is* tested.

**Resource nodes always block**, sighted or not: a node is nobody's foundation to lose.

## D58a - A cancelled foundation is refunded; the winner is not

When an overbuild cancels a foundation, the owner gets its cost back in full. The price is **recomputed from
content** at the moment of cancellation (`refund_unit_cost`) instead of being carried on the entity, because
costs are not per-player yet: what was paid and what is returned are then the same number by construction. The
day a civ discount or an upgrade changes a price, this one function has to learn about it - a `paid` field on
every entity would spread that knowledge everywhere and change every hash for nothing.

The player who **took** the tile is not refunded. The refund follows the building that went, not the tile.

**Amended by D60:** `Command::Delete` on a foundation *does* refund, proportionally - see below. The
cancellation path and the demolition path now agree, which is what the owner asked for.

## D59 - Defeat is the absence of anything that counts (supersedes D46)

The original rule, from the manual, twice:

> "To defeat a player, you must destroy all units and buildings but the following: towers, walls, gates,
> relics, Fishing Ships, Fish Traps, Transport Ships, Trade Cogs, Trade Carts, Farms, and sheep."

So a side is out when **nothing qualifying of theirs stands** - no unit, no building, no foundation - and the
**town centre is not special**. D46 said the opposite: losing the town-centre family ended the match, which made
a player with twenty villagers and a barracks across the map "dead" and unable to rebuild. The original treats a
razed base as a setback, and that is now what happens.

**Content names what does not count.** `[conquest] ignored_roles` in `economy.toml`, resolved once at seeding to
unit *families* (`units.root`, so an upgraded tower or a gated wall is still the same thing). The manual's list is
by class; this pack can name only the roles it has today - `wall` and `farm` - and the rest (tower, gate, trade
cart, sheep, the ships, relic) join the list when the pack gains them. A role content does not have is silently
absent, which is why the list is short and not an error.

**What is deliberately kept from D46:** the `roster` (who was ever in the match, so a world that never had two
sides is a sandbox and not a one-player win), the draw when every side goes on the same tick, and the refusal of
orders once the match is over.

**Still derived, still free.** `outcome` is a function of the entity list; nothing new is hashed, and the check
still runs only on a tick where something died (`something_died`), which was D46's one good performance idea.

**Not implemented:** a grace window. The DE client is described as giving a short window before elimination in
which a captured town centre or an ungarrisoned unit can save you; that is a client behaviour, not part of the
manual's rule the owner asked for, so it waits for a decision if a rebuild race ever turns out to be the
interesting part of a match.

**The three D46 tests were rewritten, not deleted** (`delete.rs`): razing the town centre is now asserted to
*not* end the match, and the match-over tests wipe a side completely instead. Eight new tests in `defeat.rs` cover
the rule itself: a lone villager survives, the last villager ends it, walls and farms do not count, a house does,
a foundation does, a double wipe on one tick is a draw, and eliminating every other side wins.

## D59a - A building counts only if it produces or takes in (owner's amendment)

D59 began with the manual's exclusion list, which is a list of *buildings* that do not count. The owner
narrowed it to a **property**: "farms and any other non-production and non-gathering building don't count for
survival". That is the better shape - one sentence instead of an enumeration that would have to be kept in step
with the unit table - and it is what the engine now does:

- a **unit** always counts (it can walk somewhere and make something happen);
- a **building** counts only if it can **make a unit** or **take a resource in**;
- everything else - house, wall, gate, tower, farm - keeps nobody in the match.

**Production is derived, not listed.** `[train] by_building` already says which buildings create a unit, so the
engine asks it rather than being told twice. **Gathering is content**: which buildings a villager may deposit at
is not otherwise written down, so `[conquest] gathering_roles` names them (today: the town centre).

**Stricter than the manual, on purpose.** The manual's exclusion list does not mention houses, so a literal
reading keeps a player alive with one house in a corner. The owner's rule does not, and the owner's rule wins:
a house is not a reason to still be in a match.

**The gathering half is carried but not exercised** - exactly the shape `blast_level` had (D51). The pack's only
deposit site is the town centre, which is also production, so deleting the gathering loop changes no test. It is
recorded in `docs/next-step.md` with what would fix it: roles for a mill, a lumber camp and a mining camp, and a
test that a mill alone keeps a player alive. Written down rather than claimed as covered.

## D59b - Defeat uses the dat's building classes (supersedes D59a's property)

D59a asked a property - "produces or gathers" - and needed a hand-kept list of gathering roles, whose second
half no test could observe. The owner chose the other way once the evidence was laid out, and the evidence is
what made the choice easy:

The manual's exclusion list is not a list of buildings, it is a list of **classes**. The Genie Editor's table
says it outright - "3 - Building: all buildings except towers, walls, gates and Monument" - and our own parse of
`empires2_x2_p1.dat` agrees, class by class:

| class | what | our units |
|---|---|---|
| 3 | ordinary buildings - **counts** | 129: `HOUS`, `BRKS`, `ARRG`, `BLAC`, `MRKT`, `RTWC/2/3`, `CSTL` |
| 27 | walls | 13: `WALL`, `TWAL`, `CWAL`, `SWAL`, `Aqueduct` |
| 39 | gates | 48: `GTAA*`, `GTAB*` … |
| 49 | farms and fish traps | 6: `FARM`, `FTRAP`, `RFARM` |
| 52 | towers | 10: `WCTW`, `HTAT`, `HTOF` |
| 1 | artifacts and relics | 4: `ARTCT`, `KOH-FLAG` |

So the rule is `surviving_classes = [3]` in `[conquest]`, and everything else keeps nobody in the match. The
consequences, in order of importance:

- **No list to forget.** A mill, a lumber camp or a market counts the moment it exists, because it is class 3.
  Adding content cannot silently break the rule the way a role list could.
- **Houses count.** A house is class 3, the same class as a barracks. The original counts it, so we do. This is
  the place where the owner's earlier property (D59a) was stricter than the game, and it is now the game's rule.
- **The content list is observed.** Emptying `surviving_classes` fails five tests; the deleted gathering loop
  of D59a failed none. The gap is closed by construction rather than by a promise.

Units still count unconditionally: the pack has no sheep, livestock or ships, so the manual's unit exclusions
have nothing to bite on yet. When they arrive, the same idea applies - a class list, not a role list - and the
test to write is "a sheep does not keep a player alive".

**Not changed:** the roster, the draw, the refusal of orders after the end, `outcome` staying derived, and the
check still running only on a tick where something died.

## D60 - Demolishing work in progress returns the part that is not built

The owner's rule, and the game's: deleting a foundation returns 100% of its cost; deleting a building while it
is going up returns the share that is **left to build**; deleting a finished building returns nothing, because
the resources are in the building.

Verified before implementing, from several independent accounts of the game (the Age of Empires forums thread on
deleting foundations, and two r/aoe2 threads), which agree on the shape and on two details:

- the refund is tied to **construction progress only**, not to damage - a half-built house that an enemy has
  beaten down still returns the unbuilt half;
- a building **destroyed by an enemy** returns nothing, however little was built. `cancel_site` (the overbuild
  path, D58) and `Delete` (this path) are the only two refunds, and both follow the same arithmetic.

The arithmetic is `cost * (total_ticks - progress_ticks) / total_ticks`, integer, flooring - so a nearly
finished building returns almost nothing and a refund can never exceed what was paid. One helper does it for
both paths (`refund_unit_cost_part`), because two refunds with two formulas is how they drift apart.

The test does not assume the price: it measures the curve on two fresh worlds - untouched, then half built - and
asserts that the half one returns about half of the untouched one. The first version of that test indexed
"wood" as slot 1 and failed, which was a useful accident: the resource order is content's, so the assertion now
sums the stockpile instead of trusting a slot.

## D61 - Blind placement, sight, and who wins a tile

Three of the owner's rules, replacing D58's "the newcomer wins" with the game's:

1. **You may lay a foundation where you cannot see.** Fog hides a building, so the click is accepted; the
   player chose blind and is not refused at the click.
2. **The placement is cancelled when your own units see the tile.** Not at the click, not when the builder
   arrives - when any unit of yours can see it. The resources come back (D58a), because the player was never
   given a chance to know.
3. **Whoever starts building first wins, and the other building is cancelled.** A site the other side has
   already put a builder to work on is a settled fact: an untouched foundation loses to the first builder, and
   a finished building is not something a latecomer can undo.

**Read "your units see it", not "the tile is visible".** A foundation projects sight as soon as it is laid, so
asking the vision layer had the site see the enemy building on its own tile and cancel itself on the tick it
was placed - blind placement became impossible. `unit_sees` asks the units only, which is both what the rule
says and what makes it reachable. That bug cost three failing tests before it was spotted, and it is the reason
the helper exists rather than an inline `vision()` call.

**Overbill found by the mutation pass:** with the sight rule in place, `cancel_overbuilt`'s "already started"
branch is effectively unreachable - a builder cannot reach the tile without its own sight reaching it first, and
at that point the placement is already cancelled by rule 2. The branch stays (it is the rule as stated, and it
covers two placements on the same tick), but the tests pin the *behaviour*, not that branch: flipping it to
"finished only" changes no test. Recorded here rather than dressed up as coverage. The refund's `hp > 0` guard is
in the same position: unreachable in the tested paths, kept because a double refund is the kind of bug that only
appears when it is expensive.

**Resources, all three cases, as the owner states them:** overbuilt or seen - refunded in full (D58a); deleted
by its owner while being built - refunded for the unbuilt share (D60); destroyed by an enemy - nothing, however
little was built. The last one is the ordinary death path, which pays nobody by construction.

Two smaller things fixed on the way: `cancel_site` is now idempotent (a site cancelled earlier in a tick is
still in the list until the sweep), and the sight check runs only when something is under construction - one
boolean scan on a map with nothing being built.

## D62 - The Castle Age and the mangonel: content, verified by price

The first step towards a match that can be *won* rather than fought. Three content additions, each identified by the
dat's own numbers rather than assumed:

| Role | Unit/tech | Evidence |
|---|---|---|
| `siege_workshop` | 49 `SIWS` | class 3, 1 800 hp, 200 wood |
| `mangonel` | 280 `MANGO` | class 13, 50 hp, range 7 / minimum 3, 160 wood + 135 gold, 6 s reload, 35 vs buildings, 40 vs units |
| `castle_age` | tech 102 | 800 food + 200 gold, 160 s, at the town centre (location 109) |

`[[train.entry]] siege_workshop → mangonel` is the authored edge, like every other train edge (D17). The Castle Age
opens *availability*: a test asserts the refusal changes from `UnitUnavailable` to a price shortfall once the tech is
granted, which is the honest claim - content says what may be built, the stockpile says whether it can be paid for.

**A mechanic found while writing the test, worth knowing before tuning any siege.** A mangonel four tiles from a house
never fires: the dead zone is `min_range + the two bodies' radii` (D44), and a house's radius is large enough that
4 tiles is *inside* it. The shooter backs away instead - and with no speed, it backs away for ever. Six tiles works.
Any future "why does the siege do nothing" question starts here.

**Still outstanding for this step:** the acceptance test - a scripted match that reaches Castle and breaks a town
centre inside fifteen minutes. That needs a booming opening (villagers queued from tick 0, farms, ten-plus on food);
the content is in place, the economy script is not. `docs/next-step.md` carries it as the top item.

## D63 - Building prerequisites are history, not the map

The owner noticed the siege workshop's prerequisite and asked for a check. **Checked, and the memory was of the
first game**: in Age of Empires II a Siege Workshop requires a **Blacksmith** (Liquipedia's building page, the
Fandom building table, and a r/AoE2 thread all say so); Barracks + Archery Range is the *Age of Empires I* rule.
Two AoE2 sources agree, so that is what the pack implements.

**The tense is the whole design.** The requirement is that the building has been built **once**, not that it still
stands - destroying the blacksmith does not take the siege workshop away (state from the same sources). So the
engine remembers a per-player set of families that have ever been finished (`built`), rather than scanning the map
in `check_build`. That set is *state*: it changes what the world will allow, so it is hashed, and a replay that
lost it would refuse different orders.

Recorded in three places, because "finished" has three shapes: a building raised by a villager (completion in the
tick loop), one that arrives finished (the opening's town centre, a scenario's building - `insert`), and the
rejection itself (`SimError::MissingBuilding { need }`, whose message names the role rather than an id).

Content shape is a role pair, not an id pair: `[build.requires] siege_workshop = "blacksmith"` and
`blacksmith = 103` (`BLAC`, class 3, 1 800 hp, 150 wood). The rest of the game's pairs are written in a comment for
whoever adds the roles: `archery_range`/`stable` need `barracks`, `market`/`farm` need `mill`.

**Two tests**: a workshop is refused with no blacksmith and allowed once one exists; and a blacksmith that is
*razed* does not take the right away. Two mutations bite (no check; spawned buildings not counted as built).

**Not modelled:** the Castle Age requirement for the workshop is separate and already handled by availability -
the workshop's *unit* gate - which is why the test grants the Castle Age first and lets the prerequisite be the
only variable.

## D64 - A builder comes to rest, and that includes its velocity

The drift, caught at last. A villager that finished a house **on its own doorstep** kept walking into it at its
own speed for the rest of the match - 0.1 tiles a second, which is exactly `speed / tick_rate`, so it looked like a
pathing problem and was not one.

What it was: the completion path set the builder's task to `Task::Idle` and cleared its route, but left
**`velocity`** alone. Velocity is the fixture-grade field used to carry a body's motion (spawn fixtures and the
order-start path set it), and the integrator adds `velocity / tick_rate` every tick for any entity that still has
one. With a task of idle and no route, nothing cleared it, so the body kept sliding east for ever. `stand_down`
already did the right thing - task and velocity - so the fix is one call, and the route clear stays because the
route is a separate fact.

**Why it took three attempts to find.** The first report was mistaken for "work at a distance" (it is not: building
work is adjacency-gated), then for a pathing oddity with a slow villager (it was not: a slow *move* order arrives
correctly, tested). What finally pinned it was writing the state down in a test - *"a unit that finishes its order
does not move afterwards"* - and noticing that the failing case was the **build** order, with the same unit speed
producing 100 raw units of movement per tick. The number matching `speed / tick_rate` exactly is what said
"velocity", not "path".

**Three tests** in `movement.rs`: an arriving unit stops dead, a builder stops when the building is finished (the
one that failed), and a very slow unit still reaches the tile it was sent to.

## D65 - An age wants two buildings of the age you are in

The owner remembered that advancing needs buildings of the current age; checked, and the rule is exactly that,
with three parts, from the official learn-to-play page and Liquipedia (which agree):

> "Advancement requires you to construct at least two buildings from the current Age."

- **Two buildings of the current age.** Dark Age buildings that count: Lumber Camp, Mining Camp, Mill, Dock,
  Barracks. Houses, farms, walls and outposts never do.
- **Feudal Age buildings**, for the Castle Age: Blacksmith, Market, Stable, Archery Range.
- **The Castle Age's exception**: the Imperial Age needs two Castle Age buildings *or* **a single Castle**.

Content, not engine: `[age.requires]` in `economy.toml` maps a tech role to `count`, `of` (roles that count) and
`single` (roles that satisfy it alone). Today it names the buildings this pack has: `mill`, `mining_camp`,
`barracks` for the Feudal Age; `blacksmith`, `market`, `stable`, `archery_range` for the Castle Age; `castle` and
`siege_workshop` for the Imperial, with `castle` in `single`.

**Only finished, standing buildings count.** A foundation is not an age - a villager has said where a mill will be,
not that there is a mill - and rubble is not either. That is the opposite tense from D63's prerequisite, and
deliberately so: a *prerequisite building* is a fact about the player's history, while an *age requirement* is a
fact about the base as it stands now. Two rules that read similarly and do not share an implementation.

**Checked before the price**, so the player is told what is actually missing ("this age needs 2 buildings of the
current age and this player has 1") rather than "not enough food" when both are true.

**Three tests** (`ages.rs`): the Feudal Age refused with 0 and with 1 Dark Age building and allowed with 2; a mill
*foundation* counting for nothing; and a single Castle satisfying the Imperial Age. Four mutations bite (no check,
foundations counting, the castle exception ignored, one building being enough).

Three roles arrived with this: `mill` (68), `mining_camp` (584), `market` (84), `stable` (101) and `archery_range`
(87) - all class 3, verified by cost and hp in the dat. Without them a pack could not reach the Feudal Age at all,
which is also why the acceptance scenario from D62 needs them before it can boom.

## D66 - A second build order queues behind the first

*Asked by the owner: "is build order queuing possible? That means that builder never stops building until
done and then moves to the next".*

**Yes, and this is how.** `Command::Build` given to a villager who is already building - or who already has
something waiting - **appends** to that villager's queue instead of replacing its task (`World.build_queue`,
a `BTreeMap<EntityId, VecDeque<(unit, tile)>>`).

The alternatives, and why not:

- *Replace the task, as before.* That is what the world did, and it is the bug that cost a probe run: a
  builder handed a second site abandons the half-built first one, which no player and no script ever means.
- *A general unit order queue* (shift-queue of move/attack/gather/build). The right eventual answer, and too
  wide for this slice: it touches every task, every interruption and every stance. Builds are the case that
  is both wanted now and self-contained.
- *Queue the order but charge for it immediately.* Then a player could order five houses with the money for
  one and go into debt, or worse, be silently over-charged when a later site is cancelled.

**Two clocks, as usual.** The order is **checked** when given (`check_build`: availability, prerequisites,
tile), so a site that could never be built is refused at the moment the player asks; it is **paid for** when
it starts, because that is when the foundation is placed. The next site starts on the very tick the previous
one finishes, so a villager with a queue never stands idle between buildings.

**State, so it is hashed** (D25): the queue is part of the world and is in `state_hash`. A replay that lost it
would build a different number of houses. It dies with its builder, and `bookkeeping_touches` sees it, which
is how the removal sweep is tested.

Two tests in `build_queue.rs`: a queued site does not appear until the one in front is finished, and it is
paid for on the tick it starts rather than when it was ordered. Mutation: removing the queueing branch makes
both fail - the second order replaces the first, one foundation appears, and no second one ever does.

## D67 - The simulation keeps no floating point in its tick, and a gate says so

*Borrowed from `openaok`, whose `--verify` gate fails on `float`, `double` or `ToFloat()` anywhere in its
simulation sources.* D9 has said "fixed-point only" since the beginning; a rule with no gate is a rule
nobody keeps. `crates/sim/tests/no_float.rs` walks `crates/sim/src`, and every line mentioning `f64` or
`f32` must sit in a function that is allowed one, or the suite fails and names the line.

**Two exemptions, both named in the test rather than silently tolerated:**

- `script.rs` and `headless.rs` - **authoring**: a scenario is written in seconds ("mill at 2:10") and
  converted to ticks once. Nothing from that conversion survives into a match.
- **stat derivation at load** - `scale_fixed`, `multiply_class`, `scaled_health`: the dat's percentages are
  applied once when the world is built and stored as `Fixed`. It is content arithmetic, not tick arithmetic,
  and it cannot differ between machines because it does not depend on a tick.

**The gate found two real violations, both in the tick path:**

- `pierce_dps` divided damage per second as `f64` to decide how many arrows a garrison adds. It is now
  integer, stated in **millionths** of a damage per second (`DPS_ONE`). The precision is not vanity: at the
  first, coarser attempt (raw fixed-point units) the town centre with three archers in it fired **four**
  arrows instead of three, because truncating each side of a ratio bends it. A finer common scale cancels
  exactly where a coarse one does not.
- the **carry cap** went through `apply_scalar`; it now uses `scale_fixed` like speed and gather rate. That
  one taught the second lesson: a plain integer division truncated 10 x 1.25 to 12 where the game's
  Wheelbarrow gives 13, so the conversion rounds (`+ scale/2`) rather than truncates. Both mistakes were
  caught by tests that already existed - which is the argument for the gate being cheap: it cost two
  one-line fixes, and the suite said which ones.

**Not adopted from the same digest**, recorded in `docs/next-step.md`: box-to-box reach (wants a building
footprint, and a building is one tile today), crowd claims at work goals (D52 says work goals are not
exclusive, and the claims table is exactly what D53 declined), routing round still bodies (needs route
invalidation), and `genie`'s deferred-write schedule (an order to keep, not an architecture to adopt).

**The judgement above held, and the project that had the architecture made it for us (2026-10-30).** One
round of work later `librematch/genie` has deleted its deferred-write design: `world/context.rs` is gone,
`docs/EFFECTS.md` counts **0 world outputs, 14 unit AI effects and 4 hand-offs** (15, 15 and 46 yesterday,
down from 51 the day before), and what replaced it is "a call on another object takes effect at once, and
the acting object is out of its slot for the length of its update, so a call on itself waits". The order it
was all built around survived the deletion untouched - still 20 steps, 10 world and 10 per player. So the
split we drew is the right one: the *order* is a fact about the game worth keeping, the *channels* were a
Rust borrow-checker workaround that a year of work removed. Nothing here changes D67; the no-float gate
stands, and the same rules file now says why the other choice is expensive - floats reproduced field by
field, with MSVC's unstable sort and the game's swap-remove list order.

## D68 - The pack is the union of the playable civ and Gaia

*Found while starting the sheep and boar slice: there was no boar in the pack to make angry.*

The dat keeps a playable civ's units on `civilizations[1]` and the world's own objects on
`civilizations[0]`, which is Gaia. The importer built from civ 1 alone - correctly, for techs and civs -
and so **silently dropped all 223 units that exist only on Gaia**: the boar (`BOARX`, 48, class 10, 75 hp),
the deer, the wolf, the hawk, the fish, the mines-as-units, and the carcasses a hunt leaves. Sheep was in the
pack only because it is duplicated onto the playable civ as `SHEEPG`, which is exactly why the gap went
unnoticed: every test and every scenario used sheep.

**The rule:** the pack carries the **union**, and Gaia's own records are marked `gaia = true`. The importer's
job is facts, not curation (D17) - it should not be deciding which of the game's objects are worth having, and
the engine only builds what content names, so the Age of Empires I leftovers that come with Gaia sit inert.
1,166 units became **1,389**, of which 223 are Gaia's.

**What it changed that nobody predicted:** a `ruage-data` test asserted "13 kind-101 ids are tech-only" - the
finding that a unit/tech id range overlaps but does not swallow. With Gaia's units in the pack those 13 ids
became unit ids too, so every tech id an effect names is now also in the unit table: **ambiguity grew, not
shrank**, and the test's own conclusion ("an id alone cannot say which table a command means") is stronger
than before. That is the sort of thing a union exposes, and it argues for the union rather than against it.

The size went from 402 KB to 461 KB of generated TOML, which is a price worth paying for a pack that can name
the animals the researched openings are built on.

## D69 - An animal's meat, its body, and its rot

*The sheep-and-boar slice. The meat is not where the obvious field says it is.*

Three facts per unit now come out of the dat, and only one of them was found where it looked like it would be:

- **Meat** is `unit.resources` filtered by **type 0** - a sheep **100**, a boar **340**, a deer **140**, all
  matching the game. It is **not** `resource_carry_capacity`, which is 0 on a live sheep and 300 on its
  corpse; that 300 is resource **type 12**, the corpse's own marker. Reading it as meat would have made a
  sheep worth three times what it is, and the number would have looked plausible enough to ship.
- **The body** is `dead_unit_id`: sheep 594 -> 595, boar 48 -> 356, deer 65 -> 43. It is **not** an animal's
  property - a knight's body is 111. It is the corpse *any* unit leaves; what makes a body edible is the
  meat, not the body.
- **Rot** is `resource_decay_rate`: 0.25 for a live sheep, 1.0 for its corpse. Carried, unread by the engine
  until corpses decay.

**The trap this leaves for the next increment, written down before it bites:** `food > 0` does **not** mean
"animal". Five non-animal units carry food - the forage bush 59 and its gaia twin (125 each), a food pile
262 (1), a deer's meat pile 333 (100) and one oddity 544 (100). Herding and retaliation must key on the
animal classes (**9 and 10 huntable, 58 herdable**) or on a content role, never on "has meat".

**And a dat quirk worth knowing rather than fixing:** 640 unit records name a body that exists in no civ's
array - the yurts all point at 768. Nothing eats a yurt, and the engine will look a body up and find nothing,
so it is a lookup that must tolerate absence rather than an import error.

The herding rule is the owner's, in his words: *"villager is herding sheep towards closest mill or town
centre. So sheep do not close distance to him but move in the direction of mill or town centre."* The real
game has the sheep flee *from* the villager, which reads as the same thing because you approach from the far
side; the rule as stated is what gets implemented, and this note exists so nobody "corrects" it back into a
follow.

## D70 - Herding, meat, and a herdable that changes hands

*The sheep-and-boar slice, as the owner specified it over four corrections.*

- **Herding is gathering from a live animal**, not an order of its own. The animal walks toward the **nearest
  drop site of the villager's player** - not toward the villager - and the **villager follows it**, so the pair
  travel together and the work continues on arrival.
- **Both kinds of animal carry meat**: huntable (classes 9 and 10) and herdable (58). The content key is
  therefore a list of *roles*, not one `herdable_roles` list that would leave the boar and deer out.
- **Only a villager makes meat**: an animal killed by anyone else leaves a body with nothing in it (D69's
  addendum; the owner's rule, with our doubt about HD recorded alongside it).
- **A herdable changes hands by line of sight, immediately**: an unowned animal joins the first player whose
  unit sees it, and an owned one is stolen by another player whose unit sees it *and* whose owner has no unit
  seeing it too; ties go to whoever has more units near it. Celts ignore the second condition (out of scope).
  This is the first mechanic that changes an entity's **owner** rather than killing or spawning it, so it needs
  its own decision on a gatherer mid-load: what happens to the meat already in hand.

**D70, addendum: the first increment.** Herding and edibility are separate content lists - `animal_roles` (all
animals worth meat, both kinds) and `herdable_roles` (the ones that walk) - because the owner corrected the
first of them: a boar is as edible as a sheep. A live animal is a gather source with `[hunt] resource` as what
it yields, and a placed animal carries the pack's `food` as its amount, so a sheep is worth 100 and not zero.

**D70, increment two: herding works.** While a villager is gathering a herdable animal, a per-tick pass sends
the **animal** toward the nearest drop site of that villager's player and drops the **villager's** cached route,
so it steps at where the animal is rather than at where it was when the order was given. The animal's route is
planned only when it is not already heading there, so a herded sheep costs one path per leg, not one per tick.
The pass is gated on content: `[hunt] herdable_roles`, so a boar is food but never walks (D70's split).

**D70, the slaughter (owner, 2026-10-29): on arrival at the drop site.** *"If villager takes meat, it means
sheep is dead, so it cannot be stolen out from under it"* - and then, asked when the killing blow lands, **"a -
on arrival at the drop site"**.

So the sequence is **herd -> slaughter -> gather**, and the animal is alive for the whole of the herding:
the villager walks it to the nearest drop site of its player, kills it there, and only then takes meat from
the body. Two consequences:

- The gather order points at a **living** animal and stays pointed at the body afterwards, so the sheep
  becomes its `dead_unit` **in place** - same entity, same id, different unit id and no health - rather than
  being deleted and re-created. That keeps the gatherer's task valid without a re-target step.
- A load can therefore never be "stolen out from under" a villager: the meat exists only after the animal is
  dead, and by then the only thing that could take it is the villager holding it.
- The window for stealing a herdable is **while it walks**, which is exactly the window conversion lives in.

**D70, increment three: the slaughter and the body.** When a herded animal reaches its drop site the villager
kills it there, and the animal **becomes its `dead_unit` in place** - same entity, same id, new unit id, no
health - so the gather task that names it stays valid and the villager simply carries on with a carcass. Content
names the bodies (`[hunt] carcass_roles` = sheep 595, boar 356, deer 43) rather than the engine inferring "no
health means edible", and the removal sweep is taught that a body is not a corpse to clear away: a unit with zero
health survives if its unit id is in that set.

Two things this bought, and one it exposed:

- the villager's load is never "stolen out from under" it, because the meat only exists after death (the owner's
  rule, D70's addendum);
- no re-target machinery: the node is the same entity before and after the blow;
- **exposed:** an animal killed by anything *other* than this slaughter is still swept away, leaving nothing. The
  last-blow rule and the carcass that a soldier's kill leaves are therefore still missing, and they need the
  killer to be known at the moment of death - which melee knows and arrows do not.

**D70, correction to increment three (owner): the shepherd kills the animal, it does not become a corpse by
itself.** *"No, shepherd has to kill the animal. It doesn't change to corpse itself."*

So arriving at the drop site is not the kill. The villager **strikes the animal** - the shepherd with the knife -
the animal **dies of that damage**, and the body is what death leaves. What increment three built was the *shape*
of the body (a `dead_unit` in place, meat intact, exempt from the sweep) with the wrong *trigger*: it promoted the
animal the moment it arrived, which is a corpse without a blow and therefore not the game.

What survives the correction, and what it costs:

- **Survives:** death is what makes a body; the body is content-named; it holds the meat; it is not swept; and if the
  body keeps the entity's id, the gatherer's task stays valid across the blow.
- **Costs:** a real attack. The villager must deal damage, which means the *melee* path has to be the one that
  lands - and the hunter/shepherd split (owner: *"Hunter villagers use the bow, shepherd the knife"*) says exactly
  where the two differ. A villager that never lands a blow, which is what the instrumentation showed, cannot kill
  anything at all.
- **Follows:** the death path must know *who* killed it, because the last blow decides the meat - villager's kill
  leaves 340 on a boar, a soldier's or a tower's leaves none. Melee knows its attacker; arrows do not.

## D71 - The damage path, the refusal, and the animal's leash

*Found by item 1 of the "everything that needs no verification from the owner" list. The previous session's
diagnosis was wrong, and the way it was wrong is the lesson.*

**The melee path had never run, so `start_attack`'s "silent refusal" was a misdiagnosis.** `combat.toml` sets
`projectile_speed = 10.0`, and the strike loop turned **every** blow into a projectile whenever
`flight > 0 && source.0 != aim`. For an attacker without Ballistics `aim` is the victim's own position, which
differs from the attacker's for every melee blow - so a villager's knife, a spear and a sword all flew as arrows.
Melee damage therefore landed in the `landings` loop, which carried `(target, damage, aim, blast)` and **no
shooter**: `retaliate` was never called (it lives on the instant path), the last blow could not be named, and the
env-gated `eprintln!` the previous session added could never print because the code it lived in never ran.

**The fix, in two halves.** (1) A blow is a projectile only when the attacker's *resolved* range is above zero;
a melee blow lands on the tick it is swung, in the loop that knows `(attacker, target)`. (2) `Task::Fly` records
its `shooter`, the landing carries it, and ranged damage applies damage **and** the reaction. Ranged damage is
now attributable exactly as melee damage is, which is the single fact three rules were waiting on: retaliation,
the last-blow carcass, and kill credit.

**A swallowed refusal is now readable.** A reaction is not a command: there is no caller to return the error to,
so `retaliate` keeps the last refusal (`World::last_refusal()`: entity, error, tick) instead of dropping it. It is
diagnostic only and is **not hashed** - it changes nothing about the world. A `NoAttack` stance is the one case a
test can stage deterministically, and it is the control that proves the record works.

**The leash is three states in one entry**, as the owner described: grazing at home (no entry), chasing (an entry),
returning (the same entry with `chasing` false and an ordinary `Move` task home). `Entity` grows nothing: the
entry lives in `World.leash`, keyed by id, only for animals that have been struck.

- **The origin is the place of the first blow**, recorded then and kept across a return, so a boar shot on its way
  home resumes the chase instead of inventing a new home.
- **Every blow renews the clock** - which is what the owner's second arrow is *for*.
- **Interest ends** on distance from the origin (`leash_tiles`) or on time since the last blow (`leash_seconds`);
  both are **authored**, because the dat carries no leash field (`BOARX` has `los = 4.0`, `speed = 0.8` and nothing
  that means "gives up"). They are whole seconds and whole tiles: a fractional value would only invite back the
  float the fixed-point rule keeps out of state. The reference pack says 10 s and 12 tiles, which a lure from a
  boar camp to a town centre needs to fit inside.
- **Content that states neither** gets the old behaviour - an animal that chases for ever - and pays nothing for
  it, because no entry is ever made.
- **Losing the target is not losing interest**: a boar whose quarry garrisoned stands where it is until the clock
  runs out, and only then walks home. That standstill *is* the garrison lure's window.
- **Bookkeeping:** the leash joins `bookkeeping_entries`/`bookkeeping_touches` and is dropped with the entity, so a
  dead animal leaves nothing behind; and it is hashed when present, because where an animal will go is state.

**D70, increment four: the last blow leaves the body (item 2, 2026-10-29).** Death by damage now promotes an
animal **in place** to its `dead_unit` - same id, no health, no speed, no variant, idle - so the gatherer's task
stays valid across the blow, which is the shape the slaughter transform already had and the reason to keep it. What
is new is *what decides the meat*: the killer of the **last** blow, and nothing about the wound history.

- **The killer is per tick, not per entity.** The blow that kills is always a blow of the tick the animal dies on
  (a victim at or below zero health is promoted before that tick ends), so `step` collects `(victim, attacker)` for
  every blow this tick and the promotion reads the last entry for its victim. Nothing is carried between ticks,
  nothing new is hashed, and the vector is empty in any world where nobody is fighting.
- **"A villager" is content, not an id**: `[hunt] butcher_roles = ["villager"]`, resolved to families at seed and
  held as ids *and* families so the tick needs no table walk. An empty list means every kill leaves its meat.
- **A body the pack cannot name** (640 records point at one no civ has, D69) means no promotion at all: the animal
  is swept away exactly as before, which is a lookup that finds nothing rather than an error.
- **A body is not a fight.** With the body surviving the sweep, "the target is gone" stopped being how a fight with
  an animal ends: an army would have swung at a corpse for the rest of the match. The attack phase now requires a
  target with health, and a test pins it (`an_attacker_stops_swinging_once_its_quarry_is_a_body`, which failed
  before the filter existed).
- **Cost, stated:** promotion runs only on a tick where blows landed and walks only those victims; it does not scan
  the entity list. A first version did exactly that (a full pass per tick), and the review caught it before commit.

**D70, increment five: the shepherd kills, and a live animal yields nothing (item 3, 2026-10-29).** The owner's
correction - *"shepherd has to kill the animal. It doesn't change to corpse itself"* - had two halves, and only one
of them was about the transform.

1. **Arriving is not killing.** On arrival at the drop site the villager is now **ordered to attack** the animal:
   the ordinary combat path, so the blow carries its attacker (D71), the animal dies of that damage, and death
   promotes it to its body in place (increment four), where the villager's next blow is not needed and the meat is
   already decided. A sheep is 7 hp against a villager's 3 damage at a 2 s reload: three blows, six seconds.
2. **Meat implies death, and the gather pass was breaking that rule.** `take_amount` read the node's `amount`, and a
   **live** animal carries its meat - so a villager following a sheep home was eating it on the walk. A live animal
   now yields **nothing**: the work during the walk is following. That is what makes the slaughter a mechanic rather
   than a formality, and it is pinned by `a_live_animal_yields_no_meat`.

**The gather order survives the fight.** Combat replaces `Task::Gather`, so the pair is remembered in
`slaughter: villager -> animal`; each tick, a villager in that map whose task has ended is sent back to the same id
once it is a body. Because the body **keeps the entity's id**, that resume is an ordinary gather order on a node
that is now a carcass - no re-target machinery, and no window in which a load can be stolen.

**An animal that cannot fight does not try.** `armed_animals` (ids and families, resolved at seed from the pack's
own attack lists) gates retaliation: a struck sheep has no answer, and attempting one would record an `Unarmed`
refusal for every slaughtered animal - noise that would bury the refusals that matter. Mutation-proved: removing the
guard fails the shepherd test on its refusal assertion.

**A fixture defect the new refusal record exposed:** the animals fixture never called `load_combat`, so every
slaughter refused with `NoTables` and the villager stood beside a living sheep for 2,000 ticks. The refusal was
readable the moment it existed - the previous session's silent `let _ =` is exactly why this class of bug used to
take an evening.

**D70, increment six: conversion by line of sight (item 5, 2026-10-29).** The manual's rule, implemented as written:
a **unit's** line of sight (a building's does not count, and neither does proximity), immediate, herdables only. An
unowned animal joins the first player whose unit sees it; an owned one is stolen only when another player sees it and
**no unit of its owner's** does; a tie goes to the side with **more units near it**, and an exact tie to the lower
player number so a replay agrees. The Celts' exemption is a civ bonus and stays out of scope while civs decide
nothing mechanical.

- **Content answers "which animals".** `[hunt] herdable_roles` - the list that already decides which animals *walk* -
  also decides which ones change hands, because in this pack they are the same animals and a second list could only
  disagree with the first.
- **A witness is a unit that is not an animal.** The first version counted animals as seers, and since a sheep has a
  line of sight of its own, every owned sheep kept itself. `seen = [1, 0, ...]` in a debug print was the tell.
- **A steal disturbs the herder; a join does not.** An unowned animal becoming yours *because* your villager is
  walking it is the ordinary case, so its route stays and the gather task survives. A steal drops the animal's route
  and stands its old herder down: it belongs to somebody else and walks to nobody's drop site. That is the window
  D70 kept pointing at - the walk, while the animal is alive - and the meat is never in question because it only
  exists after death.
- **Friendly fire had to learn about livestock.** Once a sheep joins your player, your own shepherd must be able to
  kill it, or the slaughter cannot happen for the ordinary case at all. `check_attack` therefore refuses friendly
  fire for everything *except* livestock, and keeps the allied check for different players: a teammate's sheep is
  still not yours to kill. Both halves are tested.
- **Cost:** one walk of the units times the herdables alive, returning at once when no herdable stands. `los` is
  squared and resolved at seed, so the tick does no table read and no square root. Not measured with the A/B probe.

## D72 - The blind-site guard, and a body's lifetime

*Two of the three "known-wrong answers" from item 6; the third (pack counts) is a test, not a decision.*

**`settle_blind_sites` was paying O(n) to learn "nothing to do".** The guard was the scan ("is anything under
construction?"), which every tick walked the whole entity list in every match. `World.foundations` is now the same
answer as a set lookup, maintained where a construction starts, finishes, and dies, with a `debug_assert!` that
compares it against the world whenever it is non-empty - because a maintained set's failure mode is silent drift, and
a behaviour test cannot see it. Measured interleaved against a `HEAD` worktree at 2,000 buildings: 68.5-70.1 µs/tick
against 71.5-72.0, hash unchanged, so the change is neutral in behaviour and worth 2-4% in a match that never builds.

**A body's lifetime is derived, not chosen.** An empty carcass has nothing to rot, so item 4's rot gave it no way off
the map. Every body now carries two numbers from the pack: how much meat it loses a tick (zero if it was never
butchered) and the tick it must be gone by - **its species' own meat divided by its own rot rate**, which is 850 s for
a boar and 400 s for a sheep. That keeps the rule auditable: change a species' meat or its rot in the dat and its
corpse's lifetime follows, with nothing to remember.

## D73 - The owner's four corrections: hunters, arrows, one blow, and moving on

*The owner read the last report and corrected four things. All four were real, and three of them were visible in the
pack already.*

**(1) Hunters shoot, shepherds cut.** The pack says so: **VMHUN 122 and VFHUN 216 carry `range = 3.0`**, every other
variant carries none. So the engine never needed a new rule - it needed to **choose the task by the node**: an animal
is *hunted* (`[gather.variant] hunt = 122`, a bow) and a herdable is *herded* (`herd = 592`, a knife). Two things had
to follow: the stats come from **`variant`, not `unit`** (a villager on a hunt task *is* VMHUN, and reading the base
villager gave every hunter a knife), and the hunt order is given **before** the walk (a hunter that first walked onto
the boar would be inside knife range at its first swing, and the bow would never matter).

**(2) A hunter's arrow that lands the last blow keeps the meat.** It already did - the arrow carries its shooter
(D71) and the shooter is a villager - but nothing tested it, and the rule is only true because of the damage-path fix.
It is now a test.

**(3) A shepherd kills a sheep in one blow.** A herdable is **livestock, not an opponent**: a butcher's blow takes it
outright, whatever the swing is worth, so a 7-health sheep is never *seen* wounded. A boar or a deer is still fought,
which is what makes a boar dangerous. Both damage paths obey it - the knife and the arrow - because the rule is about
the victim and the killer, not about which loop applied the damage.

**(4) A worker whose source runs out moves to the next one of its kind** - next tree, next bush, next pile of gold,
next sheep. This is the rule that unblocked a real economy and it exposed two bugs of mine:

- **A full load did not always go home.** The switch to `Return` required `take > 0` on the same tick, so a villager
  that arrived at a *new* node already carrying a full load stood there with no room to take anything, for ever. It
  was invisible while every order reset the load (`start_gather` used to zero `carry`, which is itself wrong - the load
  is the villager's, not the node's) and it is now caught by `Return` being triggered by a brimming load.
- **Reachability of a node is not `can_reach(node.position)`.** A tree's own tile is usually blocked, so asking that
  refused every tree and sent every lumberjack to Idle; the question is whether a worker can reach a tile it can
  *work* the node from, which is what `work_position` answers.

**And one consequence outside the engine:** the ten-minute scenario used to send its spearman at the nearest enemy
*villager*. With villagers now spreading through the wood cluster, that villager is sometimes walled in, and the step
killed the run at apply time. `nearest_enemy` now prefers an enemy it can walk to, and the scenario aims at the town
centre instead - a target a walk can always reach, and a scratch on the wall teaches more than a refused step.

## D74 - The client: a native window, and text on screen

*The owner's answers, 2026-10-29, to the two questions item 15 was waiting on.*

- **Windowing stack: native.** `winit` with software presentation, i.e. the swap the earlier decision reserved ("a
  headless rasteriser now, a window later, as a swap"). The reason it is the cheap side of the fork: the
  rasteriser already produces pixels into a buffer, so the window is a loop around it - while a browser client
  (wasm + canvas) would drag the whole simulation into a target with a different time source and a different
  determinism story.
- **HUD text: allowed.** The earlier decision - *"HUD is data, not pixels; no bitmap font"* - is **superseded**: a
  playable prototype has to show numbers, so a font comes in with the client. The rule that survives is the layering
  one: the sim still carries no text, and what the HUD draws stays a *reading* of state, not a second source of it.

Nothing is built against this yet: items 10-14 (animation data, asset pack, sprites, terrain, camera) come first,
because a window onto rectangles is not the milestone the owner asked for.

## D75 - Rules and shares: the two things the game's own AI does, now done here

*Read from the install, not guessed: `docs/ai-of-the-game.md`. The acceptance script had been a list of timestamps for
months of game time; HD's own AI never writes one.*

**(1) `[[rule]]`: a condition and one action, re-evaluated.** `Economy/Dark Age.per` trains a villager while food is
under 400 **or** villagers under 28, and advances the age at 28 villagers with `can-research`. Every guard is a count, an
amount, a `can-*` predicate or an age. Ours is now the same shape: `when` (villager count, buildings present or absent,
a researched or un-researched tech, stock thresholds, housing headroom, a unit present) and one `do` action, with a
cooldown so a satisfied condition is an order rather than a flood. `at_seconds` still exists and is the weaker half -
it holds a rule back, it does not schedule one.

**It is additive, and the schema stays 2.** A v2 file without rules means exactly what it did, and the schema number is
for changes that *break* a file. `step` became optional in the same spirit: a scenario that is only rules is legal.

**(2) `[policy]`: shares, not assignments.** `Low-pop Villager Distribution.per` is 76 `set-strategic-number` lines and
no orders: at ≤8 villagers food is 100%, and wood takes 12/20/28/34% at 9/10/11/12 villagers. `config/economy.toml` now
carries HD's whole 19-row table (every row sums to 100, which the loader **refuses** to accept otherwise), and the engine
moves idle villagers to meet the row that applies.

**What the engine will not do: fight the player.** It moves a villager only when that villager is idle - and "idle"
includes the state 0 A.D. calls `FINDINGNEWTARGET`: standing at a dead node with nothing in hand. That state is the one
this economy used to freeze in, because the workers *looked* employed. It never re-issues an order to somebody already
working, and it remembers a villager it has moved and a node it could not reach (`share_moved`, `share_failed` - 0 A.D.
keeps the same book as `lastFailedGather`), so a second pass cannot cost a second path search for the same nothing.

**The performance lesson, measured rather than argued.** The first version asked reachability of every candidate node,
once a second, per player: **400 game-seconds went from 319 ms to 11,922 ms** - a 37x tax on the whole run for a check
that `start_gather` performs anyway. Removing it and drawing a line at "one order per villager per retry window" brought
the pass back to noise. Two rules came out of it, and both are in the code's comments: reachability is asked **once**,
by the call that needs it; and a policy pass that can re-issue the same order every second is not a policy.

## D76 - What a building occupies: footprints from the dat, and the Town Centre's walkable wings

*The owner's answer, 2026-10-30, with a diagram of the Town Centre.*

A building's **footprint** comes from the dat: `radius_x`/`radius_y` are half-extents in tiles, so the Town Centre
(`2.0`) is **4x4**, the mill and lumber camp (`1.0`) are 2x2, the barracks (`1.5`) 3x3, the palisade (`0.5`) 1x1. Until
now the engine blocked **one tile** per building that blocked at all, which is neither the game's shape nor enough for
placement rules to mean anything.

**What stands on the footprint is content's business, not the engine's.** The owner's diagram of the Town Centre: the
building reads as a **diamond**, with the two-storey **back core** solid (impassable) and the **left and right wings plus
the front courtyard under-hang fully walkable** - three quarters of the covered area, which is what his first message
said. A farm is the extreme case: `radius 1.5` (3x3) and **entirely walkable**.

So content describes, per role, which tiles of the footprint are solid:

```toml
[footprint.town_center]
# Rows top-to-bottom, columns left-to-right, anchored at the building's own tile.
# '#' is impassable, '.' is ground a unit may cross. The diamond's core is the back half.
blocked = ["##..", "##..", "....", "...."]
```

Rules that follow, and are what will be built next:

1. A building with no `[footprint]` entry blocks **its own tile** - today's behaviour, so nothing regresses.
2. A `[footprint]` entry blocks exactly the `#` tiles on placement and frees them on destruction or on a cancelled
   foundation, through the same `Walk` layer walls already use (D54).
3. A farm names an all-`.` mask, which is the "farms are floors" rule as data rather than as a special case.
4. Placement and reachability then ask the footprint, not the anchor: a site may not overlap another footprint, and a
   villager may cross a Town Centre's wings the way the game lets it.

## D77-D79 - The owner's five answers (2026-10-30)

**D77 - Farm upgrades are proportional, not a refill.** Horse Collar (+75), Heavy Plow (+125) and Crop Rotation (+175) are
resource-36 modifiers in the dat, and the pacing the owner chose is: an upgrade adds its amount **in proportion to what the
plot has left**. A full plot gains +75; a plot half eaten gains **+37** (rounded down); a spent plot gains nothing, because
its food is already eaten. The engine adds `added * remaining / was_total` to every plot of the player's that still holds
food, in id order, so the result is a function of state.

**D78 - Victory is conquest.** The match ends when a side has nothing left that keeps it alive (D59's rule, unchanged). The
acceptance script's prose said "a mangonel at the enemy town centre" and that is not the same thing: a razed town centre
with three villagers in a corner is not a victory, in this engine or in the game.

**D79 - The rendering order, the map, and the footprints.** Items 10-15 go **window first**: terrain and coloured blocks
from the existing rasteriser, with sprites swapping in behind the `SpriteRef` boundary once the art extraction lands - the
boundary is the thing worth proving early, and the art does not need the window rewritten. The map stays **ours** (the
authored 64x64 mirrored one); reading `.aoe2scenario` files is a future item, not a prerequisite. Footprints (D76, the
Town Centre's walkable wings) come **after** the renderer's first milestone, because they cannot make a match win or lose
and will be far easier to judge once there is something on screen.

## D80-D82 - The look, the art, and the mods (2026-10-30)

**D80 - The camera is the game's projection.** 2:1 isometric diamonds, `project` to the centre of a tile's diamond and
`pick` as its exact inverse, camera position in tile space so zoom does not move the centre tile. Everything the renderer
draws and every click it accepts goes through that pair.

**D81 - Assets resolve through a mod chain; the original game is the last link, and its art is never redistributed.**
The owner's answer to the art-licensing question, and a bigger decision than the question was:

- A mod is a **folder**. The load order is a **priority list**, and the **original game's assets come last** - so a mod
  overrides the game, never the other way round.
- Resolution is **by id in the pack, first chain entry that has it wins**. The engine asks for "the graphic for unit 83",
  not for a path, so a mod can replace any part without the engine knowing a mod exists.
- The original game's art is **loaded at runtime from the player's install and never committed**. Placeholder shapes remain
  the in-repo default, so the repository is runnable by someone with no game install - which is also what keeps it legal.
- Consequence for the content pack: an asset reference is an id plus a chain position, and `SpriteRef` (item 11) must carry
  the resolved source, not a filesystem path, or the chain cannot work.

**D82 - A bitmap font for the HUD, PNG frames as the yardstick, borderless by default.** The HUD uses a bundled bitmap font
(no parser, no dependency, licence-clean by construction); every rendering step is shown to the owner as a **PNG rendered
headlessly**, so nothing waits on a display; fullscreen is **borderless** first, with exclusive as an option.

## D83 - A footprint is anchored at the tile nearest its middle, and picking follows the shape (2026-10-30)

The dat gives `radius_x`/`radius_y` as **half-extents**, so the tiles a building covers are `across = 2 * radius`
tiles. The engine has to say *which* tiles those are relative to the anchor tile, and the two candidates differ by
half a tile on screen:

1. **The anchor is the tile nearest the middle** (chosen): a 4x4 building covers `anchor-2 ..= anchor+1`; a 3x3 one
   covers `anchor-1 ..= anchor+1`; a 2x2 one `anchor-1 ..= anchor`. An even footprint has no middle tile, so the extra
   tile falls on the +x/+y side. `footprint_tiles(anchor, footprint)` in `crates/view` is the one implementation.
2. The anchor is the block's north corner, and the footprint extends `+x/+y`. Equivalent to (1) shifted by half a tile.

Why (1): it is the reading that matches "radius is a distance from the position", which is how every other radius in
the pack is used, and it keeps an odd footprint (a 3x3 barracks) centred on its own tile, which is where a scenario
places it.

**Picking follows the drawing.** A player aims at the shape on the screen, so `pick_at` accepts any tile of the
footprint rather than the anchor tile alone; a unit standing on a footprint tile still outranks the building, which is
the rule that makes clicking a villager on a farm select the villager.

**Open, and not a blocker:** which convention the original uses cannot be read out of the dat (it carries the extent,
not the anchoring). It is one constant in `footprint_tiles`, and importing a real `.aoe2scenario`, or measuring a
building's blocked ground in a recorded game, is what would settle it. Nothing else depends on the answer: the
simulation still blocks one tile per building (D76's masks are recorded and not built).

## D84 - Terrain is ground; what stands on it blocks (2026-10-31)

The owner's rule, in his words: **"a tree blocks passage until cut and fully harvested. Then dirt remains but it is
walkable."** So the model splits in two, where it used to be one thing:

1. **Terrain never blocks.** `Terrain::walkable()` is true for grass, dirt and forest *floor* (`Map::terrain` gained
   `Dirt`, letter `D`, for bare ground - a yard, a clearing, the ground a felled wood leaves behind). `Walk::from_map`
   now reads `!walkable()` instead of `!= Grass`, so the change is a property of the data rather than a negation.
2. **An entity blocks its own tile.** A **tree** - a resource node carrying hit points, which is what a tree has and a
   bush, a mine or a pile does not (`hp = 20` in `map.toml`) - closes the tile it stands on from the tick it is inserted,
   and `step_spent_nodes` frees the tile and removes the entity once the wood is gone and nobody is working it any more.

Two defects closed by the same change, both previously parked `#[ignore]`d with their reasons written down:
`a_standing_tree_blocks_its_tile` and `a_chopped_tree_leaves_no_obstacle`. The first needed one thing the fixture had
been faking: a spawned tree with `hp: 0` is not a tree, so the fixture now spawns one with 20 hit points - a node without
them is a thing you walk past, which is right for every node that is not a tree.

**Consequence for content, and the traps it sets:** a wall in a fixture is now a **row of trees**, not a column of `F`.
Four fixtures said otherwise and were corrected (the `WALLED_MAP`, `WALL_MAP`, `SEALED_MAP` and the forest-pocket rally
map), each by adding the wood group that actually closes the way - and a fixture that lists wood without `hp` no longer
blocks, which is the tell that it was leaning on terrain.

**Not carried over:** "a resource node does not block its own tile" was never a general rule - bushes and mines are still
walked over, as they were, and only trees block. If that turns out to be wrong for a mine, it is one predicate in the
insert path, keyed on content rather than on `hp > 0`.

**And it settles D83's anchor.** The owner's follow-up - *"top corner is the centre, bottom corner is walkable yard, left
and right corner is walkable sheltered yard"* - is the Town Centre's diamond read from its **north corner**, so the
footprint's anchor tile is that corner and the block extends `+x/+y`. D83's "tile nearest the middle" is superseded; the
change is one constant in `footprint_tiles`, and D76's mask rows already assume the corner order.

## D85 - A trained unit that cannot be placed is garrisoned, and a full building pauses its queue (2026-10-31)

The owner's rule, after the footprint masks exposed it (`838235d`: the Huns train 18 of 20 villagers and two are lost):

1. When a finished unit has **no free tile beside its building**, put it **inside that building** instead - the garrison
   path the engine already has.
2. When there is **no garrison room either**, **pause training at that building** and show the reason **beside the training
   queue** in the HUD, rather than dropping the unit or silently retrying.

This replaces "find a free tile, else lose the unit", which is what the spawn search does today. The observable rule to
test: 20 villagers trained at a surrounded town centre end up 20 alive (in the building or beside it), and a queue that
cannot place anything stops with a reason a HUD can draw - never a recruit that vanishes.

## D86 - REA runs on this container; the Windows box is a target, not an analyst (2026-10-31)

The owner asked whether a Windows Hermes running REA could act as a slave delivering findings here. Measured
first, then decided; the numbers, versions and install traps are in `docs/re-toolchain.md`.

1. **REA is an MCP tool server, not an agent**, and its Windows support is an experimental read-only boundary:
   27 Ghidra inventory/memory/function operations on native PE, x64-first, 32-bit PE admission only on its
   `main` branch, and a transport that is authenticated IPv4 loopback with a Job Object - deliberately not
   reachable from another host.
2. **The target is a 32-bit i386 PE** (`AoK HD.exe`, 7,521,280 bytes, PE timestamp 2018-08-22), which is the
   one cell that boundary admits worst, while Ghidra on Linux decompiles it with no restriction at all.
3. **So the analyser runs here**, beside the engine and the dat; the Windows machine keeps the job `oracle.md`
   gave it - run the game, produce save files. If a Windows agent is ever added for live-process work, A2A is
   the transport, and its output is artifacts plus provenance, never a mechanic asserted from a machine this
   one cannot re-derive it from.

Nothing in `crates/` changes for this: it is a toolchain decision, not an engine one.

## D87 - How ruage is compared to the original: a register ruled by reading, closed by recorded AI-vs-AI games (2026-11-01)

The owner's question was whether the disassembly can give the **complete** behavioural difference between ruage and
`AoK HD.exe`, "to rule one", with functional gaps deferred. The answer, written down rather than carried in a conversation
(`docs/behaviour-diff.md`), is that completeness is not computable - 15,766 unnamed procedures with no PDB names, no
completion criterion for "have we found every rule", and code that is not behaviour. What replaces it is a **register**:
one row per rule ruage implements, each ruled *same / different / unknown-static / needs-oracle* with the original's address
and our file:line. The count is honest and small: **1 of 12 ruled today**.

The owner then stated the other half of the method: **two AIs playing each other, the match recorded, with captures taken as
it runs** - "then we have a multitude of the data points". That is the oracle D2 already defined, pointed at a match nobody
has to play by hand.

Consequences, and they are what future sessions should act on:

1. **Static reading proposes, the game disposes.** Rows that are local arithmetic can be ruled from the binary alone; rows
   about tick order, pathing tie-breaks, decay and the age-up boundary cannot, and are marked `needs-oracle` rather than
   guessed.
2. **The mode is Random Map, not Death Match** - RM exercises the economy and age ramp that our timings differ on, and DM's
   AI is unrepresentative because it never earns anything. DM is kept for the combat rows later.
3. **Saves matter more than screenshots.** A screenshot closes no register row; two saves five minutes apart close the decay
   row outright. The capture set is therefore record + periodic saves + event screenshots.
4. **Nothing install-derived is committed** (D8/D22) - recordings and saves stay out of the repo; the finding and a SHA-256
   go in.
5. **Settled 2026-11-01, by the owner: both player slots can be set to AI.** The farm is a plain all-AI match, with no
   workaround and no passive human slot - which is what makes it a data source rather than a chore.

## D88 - Fights are ordered: the tick applies object by object, as the original does (2026-11-01)

The owner's instruction, verbatim: **"units cannot trade killing blows, follow original."** That settles U18 in favour of
the original's model and against ours.

What we do today: `Sim::step` decides every entity against the state the tick opened with, then applies. Two units can
therefore trade killing blows in one tick and both die, and the outcome is independent of iteration order. The original
(per `librematch/genie`'s reading of the DE port, `crates/engine/genie-core/src/world/step.rs` and `world/after_step.rs`)
walks objects in player-then-slot order and reads live, so a later object sees an earlier one's kill, and a killed unit's
actions are torn down in its own life step before its head action would run. In that model the later unit does not swing.

**The consequence that must be written down, not discovered later:** order-dependence becomes a *contract*. Determinism
stops resting on order-independence and starts resting on the iteration order being fixed - player, then slot - and on no
step of it being reached through a hash map. D90 is therefore part of this decision, not a separate nicety.

**What it costs, honestly:**

1. **Every fight test that assumed simultaneous resolution re-baselines.** The acceptance hash changes; that is expected
   and must be recorded as a new baseline rather than treated as a regression.
2. **Damage inside one tick becomes slot-ordered**, so a mirror fight can resolve differently from its slot-swapped twin.
   That is not a bug in the new model; it is the model, and a test should say so.
3. **The AI's own code must not depend on the old symmetry** - anything that compares two sides by summing simultaneous
   damage needs re-reading.

**First step is a test, not the loop:** pin what we do *now* (`a unit killed by an earlier blow still swings`), watch it
pass, then write its inverse - `the later unit does not swing` - and watch that one fail against the current loop before
touching `Sim::step`. Two tests, one behaviour, and the failing run is the permission to change the engine.

## D89 - The effect schema carries tech cost and tech time (2026-11-01)

U16 and U2 closed on 2026-10-27: **kind 101 is tech cost** (`a` = tech id, `b` = resource 0-3, `c` = 0 set / 1 add,
`d` = signed amount) and **kind 103 is tech time**. Our data holds **97 kind-101 and 25 kind-103 commands**, and today
they are parsed, classified and then dropped: there is no `tech_cost` or `tech_time` modifier anywhere in `crates/` or
`config/`, so the importer keeps the command and the engine cannot see the effect.

Owner decision: adopt. The effect schema grows `tech_cost` and `tech_time` modifiers beside the existing ones, the
importer maps 101 and 103 into them, and the sim applies them where a tech's price or research length is read. The
party digit applies here as it does elsewhere (tens: 1 allies, 2 enemies, 3 neutrals, 4 Gaia).

**Why this is not cosmetic:** a cost-changing effect is how the game's cheaper-tech bonuses are expressed at all. Effect
10 ("Lombards Technology Tree") is the data's own worked example - Feudal/Castle/Imperial at 425/680/850 food as *set*,
then an *add* of -220/-157/-250 - and it survives into our `commands` while having no representation in `modifiers`.

**First tests:** a kind-101 command survives import as a `tech_cost` modifier; applying effect 10 sets Feudal cost to 425
and then subtracts 220 from it; a kind-103 command shortens the research length it names.

## D90 - The workspace denies hash iteration and a bare `#[allow]` (2026-11-01)

Borrowed in shape from `librematch/genie`'s lint set, adopted on the owner's "adopt adoptable". Determinism rule 3 says
no collection the sim walks is a hash map, and **nothing enforces it**: this workspace has no `[workspace.lints]` section
at all, so the rule rests on review. `clippy::iter_over_hash_type` is available on this toolchain and free today because
`crates/` holds no `HashMap`.

Adopt: a `[workspace.lints.clippy]` section denying `iter_over_hash_type`, and the same section denying a new `#[allow]`
that carries no reason (`allow_attributes_without_reason`). D88 is why this is not a style question - the moment fights
are ordered, the iteration order *is* the rule.

**How to verify a lint config, since a lint cannot be unit-tested from inside the thing it lints:** introduce the
violation on purpose once (a `HashMap` walk in a scratch test), watch the build fail, remove it, and record the failing
output in the commit or the day's review. `crates/sim/tests/no_float.rs` is the precedent for pinning a rule that would
otherwise be a comment.

## D88 - Blows land in order: two units cannot trade killing blows (2026-11-01)

The owner's answer to U18, verbatim: **"units cannot trade killing blows, follow original."** That settles a question that
had stood open for three weeks with three consistent positions available (ours simultaneous; genie's ordered-by-object;
openaok's ordered-but-spread-over-0.6s for looks).

**What was implemented.** Melee blows are applied in the order the decide pass collected them - entity-id order - and a unit
whose health an earlier blow took below zero **does not swing**. The pass keeps a small `killed` list for that; a blow
aimed at something already dead is skipped too, so nothing is credited to a corpse and no corpse retaliates.

**What was deliberately not changed, and why it is written down:** an **arrow already in flight still lands** if its shooter
dies while it is in the air, because a shot is an entity with its own lifetime. Ordering arrows would be a second decision
the owner has not given, and inventing it here would be the guess this project refuses.

**What is still unknown.** *Which* order the original uses - by object id, by side, or something else. Entity-id order is a
deterministic order that satisfies the rule; register row 3 in `docs/behaviour-diff.md` stays `needs-oracle` until a
recording or the binary says which.

## D89 - The adoptables from the prior-art round: taken, deferred, or refused (2026-11-01)

The owner: **"adopt adoptable"**, and **"3 adopt"** for tech cost/time. Recorded so a later session does not re-litigate:

- **Taken: `clippy::iter_over_hash_type` denied workspace-wide.** Determinism rule 3 stopped being a review habit. Verified
  by a throwaway probe that iterated a `HashMap` and watching clippy refuse it.
- **Taken: D88** (above), the U18 decision.
- **Taken, next slice: tech cost and time modifiers (kinds 101 and 103).** 97 kind-101 and 25 kind-103 commands are parsed,
  classified, and then dropped; the data is already ours, so this is schema plus interpreter plus tests - not research.
- **Deferred with a reason: the `None`-vs-zeroed-memory lesson from genie.** It needs a save format to test, and we have no
  save writer yet; a test cannot be written against a format that does not exist. It joins the save-format slice.
- **Deferred: openaok's per-tick pathing budget and landmark A\* estimate.** Real, cheap and MIT-clean as *facts*, but they
  are the pathing slice's business, not this round's.

## D90 - A missile whose launcher died does no damage (2026-11-01)

The owner's second combat rule, verbatim: **"For projectiles also cause zero damage once launcher died in original game. If
trebuchet goes missile and dies, missile causes 0 damage."**

This closes the gap D88 deliberately left open - D88 ordered melee blows and left arrows in flight alone, with the note that
ordering arrows was "a second decision the owner has not given". He has given it, and the answer is not "order the arrows"
but **"the arrow lands and is worth nothing"**.

Implemented as: the landing checks its shooter, and if the launcher is gone (swept on death) or at/below zero (killed earlier
in the same tick) the landing resolves nothing - no damage, no blast, no retaliation, no carcass. The arrow still flies and
still disappears; only its blow is void.

Consequences worth naming: a razed tower's volley already in the air hits nothing (same rule, no special case); a dead
hunter's arrow leaves no carcass; and the rule is invisible in the acceptance match (its hash did not move), which means the
case does not arise there rather than that the rule does nothing.

## D91 - The counter relation is derived from the pack, never authored (2026-11-01)

Owner's ruling on `docs/ai-scripts.md` fork 1: **agreed as proposed.** "Which of my units beats which of theirs" is
arithmetic over the pack - `damage_between(attacks, armours, rules)` against each candidate's `attacks`/`armours` pairs -
so it is computed, never written down.

The two rejected alternatives and why, because both will be proposed again: a **hand-written table** is AoE2 knowledge
in a file that is meant to describe *any* pack (it stops being true the moment our own units exist, D17/D18); a
**generated file** is a second source of truth that drifts the first time the damage code changes.

Consequence worth stating plainly: with **D93** (conditions, not a symbolic `counter_to`), the table is needed at
**authoring time first** - when someone writes the `[[rule]]` that answers knights with pikemen - and only inside the
engine if a later decision asks the *engine* to pick the unit. So the first increment is a **report derived from the
pack**, and the engine surface stays at zero until a rule needs it. When the engine does need it, it goes in
`crates/data` beside the other load-time derivations and is hashed like the rest.

## D92 - The AI knows only what it has seen (2026-11-01)

Owner's ruling on fork 2: **agreed as proposed.** Enemy knowledge is a per-player **memory** of observed units and their
last-seen tick, never the world's ground truth. An unscouted army is unknown, so a counter rule cannot fire; a stale
memory produces a wrong choice, which is what a human's information advantage is worth.

Binding consequences:

- The memory is filled **only** from what that player's units and buildings can watch (`World::vision`), updated in
  entity-id order, and hashed with the rest of state - it changes what the world will allow, like `last_refusal`.
- **The anti-cheat is a gate, not a promise.** A test walks the condition variants and fails any that reaches into true
  state, in the same spirit as `no_float`'s scan. Without it, "does not cheat" is a claim nobody checks.
- Its cost is named: scouting must exist before counters are useful, and a `scout` action plus a chooser for unexplored
  ground is part of this line of work, not an optional extra.

## D93 - The rule language grows by conditions, not by a symbolic "counter to" (2026-11-01)

Owner's ruling on fork 3: **agreed as proposed.** A countering rule names the unit it trains and the observation that
triggers it:

```toml
when = { enemy_share = { role = "knight", at_least = 0.40 } }
do   = { train = { building = "barracks", role = "pikeman" } }
```

Explicit, visible in the refusal log, and it needs no new resolution inside the engine. A symbolic
`do = { train = { counter_to = "their most common" } }` is **refused for now**: it hides the choice from the script,
which is the one thing a scripted AI exists to make visible. The signal to revisit is rule files repeating a dozen
near-identical blocks; D91's table is the thing that would back the change if that day comes.

**What this buys immediately:** `enemy_share`, `enemy_at_least`, `army_at_least` and `health_share_below` are conditions
over D92's memory, so scouting, raid response, attack timing and retreat are the same increment, not four.

## D94 - AI behaviour lives in its own folder, apart from the mechanics config (2026-11-01)

The owner's instruction, verbatim: **"I want to keep separated AI from normal configs. Whatever sets things like height
bonus and other internals it a configuration. Whatever sets how the AI player behaviour like which units to train how
many villagers to sent here and there etc. that has to go to separate folder for AI."**

Settled by that sentence:

- **`config/` is the world**: sim pacing and scales, map, economy roles and gather rates, combat including the height
  bonus, movement, resources, `gamedata/`, overrides. None of it decides *what a player does*.
- **AI behaviour moves out**: the `[policy]` villager-share table (today in `economy.toml`) and every `[[rule]]` block
  (today inside the scenario files) are AI, not configuration, and they belong in an AI folder.

Two consequences that follow rather than being chosen: the loader has to read AI content from the new place (and the
pack's identity, D19, has to cover it, or two players could disagree about the AI and desync), and every test that
loads `[policy]` or a `[[rule]]` points at the new files.

**Shape settled by the owner (2026-11-01, second exchange):**

- **Location: `config/ai/`** - one pack, one hash, one handshake.
- **Layout: a base AI plus a per-civilisation delta** ("per civ delta sounds more sensible"). The base holds the rules
  every civ shares; `config/ai/civ/<name>.toml` carries only what that civ does differently. This matches what the
  original's own scripts do in spirit - civ differences are the exception, not a script set per civ
  (`docs/ai-scripts.md` measured 15 `(civ-selected ...)` uses across 39 files) - while keeping "what does Britons do"
  answerable in one file, which is the maintenance property the owner asked for.
- **Still open: derived defaults.** Whether a civ's *tuning* is derived from the bonuses already in `civs.toml` (with a
  hand-written entry overriding) or authored per civ. The distinction is about the *values*, not the files: a delta file
  exists either way.

**The rest of the shape, still open from the first exchange:**

1. **Location**: `config/ai/` (one content pack, one hash, one handshake) or a top-level `ai/` beside `config/`
   (clearly separate, but a second thing to version and hash). *Default if nothing is said: `config/ai/`.*
2. **Granularity**: one file per **personality** (`aggressive.toml`, `booming.toml`), reusable across matches and named
   per seat by the match, or one AI file per match. *Default: personalities.*
3. **Do the rules leave the scenario files entirely?** Then a scenario is what it says - timed scripted steps - and it
   names the personality its seats use. *Default: yes, rules leave; the acceptance's fourteen rules become a personality
   and the scenario points at it.*
4. **Is a scenario's timed step (`at_seconds = 300`, build a barracks) AI?** It is a scripted command, not a decision
   the engine makes, so it stays with the match. *Default: steps stay, rules move.*

### D94a - the shares moved, and the type stayed (2026-11-03, applied)

The other half of D94 is built: `[policy]` is no longer in `economy.toml`. It lives in the personality
(`config/ai/base.toml`), `Personality` carries `policy`, and `Scenario` carries the **resolved** table so the runners
have one thing to install (`apply_policy(&scenario.policy)`, once per match, at seed). Three consequences are rules now,
not preferences:

- **The type stays in `crates/data`; the parse goes with the table.** `Policy` is content vocabulary and both crates
  need it, but the sum-to-100 refusal is enforced by its `Deserialize`, so it fires wherever the table is written - the
  personality - and the message names that file rather than a block in a file it is no longer in.
- **A leftover `[policy]` in an economy is refused by name.** `deny_unknown_fields` would refuse it anyway as an
  anonymous unknown key; the refusal says the shares are behaviour, names `config/ai/<name>.toml`, and says to remove it.
  A block nothing reads is the dead-knob failure (`cost_diagonal`) from the other side.
- **A personality may be shares without rules.** "Where the villagers go" is a whole AI if that is all a match needs, so
  the emptiness refusal now requires *both* empty. And a scenario that names a personality may carry neither rules nor
  shares of its own - one source for one AI, the same rule `[[rule]]` already followed.

The behaviour change worth stating: a run with **no scenario** has no personality and therefore installs no shares. It
used to read the table out of the economy regardless. That is D94 read literally, and the only invocation it touches is
a bare `--ticks` probe.

## D95 - The AI names *lines*, not units (owner, 2026-11-01)

The owner chose **A1 (explicit naming)** with one addition that does the work a capability resolver would have done:
the AI refers to a **line** - "knight-line, knight and its upgrades, the best cavalry generalist", "spear-line,
spearman and upgrades, the cavalry counter" - and a civ override says what that civ fields in its place.

Why this is better than the capability vocabulary the owner rejected:

- A line is **content**, not judgement: an ordered list of roles (`knight -> cavalier -> paladin`), so "the best one I
  can field" is simply the highest tier the civ can actually train, resolved from the tech tree that already exists. The
  ambiguity the owner objected to ("best cavalry" - Paladin vs Cataphract vs Camel) is answered by the *civ override*,
  which is authored, not inferred by a scoring function.
- The base AI stays readable: it asks for a line and a purpose; the civ file says which units answer it.
- D93 survives: the choice of unit is visible, in one file, named.

**Shape (to confirm):** `[line]` in the content pack maps a line name to role names in ascending order; the base AI
names lines; `config/ai/civ/<name>.toml` overrides line membership where the civ differs (Byzantines: camels for
anti-cavalry, cataphracts as the generalist). The engine still knows no AoE2: "knight" is a line name in the pack, and
the resolver only picks the highest tier the world lets the player train.

**Lines and what each answers, confirmed by the owner (2026-11-01):** `knight` is the generalist cavalry line; `spear`
(spearman -> pikeman -> halberdier) counters **cavalry**; `skirmisher` counters **archers**. The owner's first phrasing
read as if the skirmisher line countered cavalry - asked, and answered: "I meant skirmisher-line -> archer counter,
spear-line -> cavalry counter." Recorded because the difference decides whether a line carries a `counters` field or
whether purpose comes only from the AI file.

**Still open:** whether a line is the unit family alone or family-plus-intent (the owner's Light Cavalry/Hussar point says
raiding is an intent, and no line may answer it), and whether a civ override may replace a line, reorder it, or only add
a tier.

## D96 - Spatial awareness is in scope, as a visibility-respecting local query (2026-11-01)

Not a separate choice after D95 and the group decisions: "retreat from an approaching force" needs to know *what is near
this group*, and a fight ratio computed from global counts would be the cheating the owner rejected. So:

- **The primitive is a query, not a memory.** `local_contacts(player, point, radius)` counts the enemy units that
  player's `visible_mask` already exposes, by unit id. It is computed at decision time from what the player can see, so
  D92 holds by construction: no sight, no contact.
- **The anchor is a group's members**, not its centroid: an enemy counts if it is within the radius of *any* member.
  A stretched-out raiding party that is flanked at one end must see the flank, and a centroid would average it away.
- **Cached per group per game second**, the same rule as the memory pass - a per-rule-per-tick scan over every entity
  would cost O(entities x rules) for a number that cannot change within a tick.
- **The same query anchors on a building** ("is my town centre under attack"), which is why it takes a point rather than
  a group.

Cost, measured against what exists: `visible_mask` is already computed per player, so the new code is a filter with a
squared-distance test; the work is the condition that combines the local count with the time-to-wipe ratio, and its
tests. One slice after groups exist - it cannot land first, because "near" has no subject until there is a group.

Still the owner's call, if he disagrees: whether the radius is one number for every group or a per-group field (a raider
cares about 8 tiles, a siege group about 12).

## D97 - Units get readable names, derived from the game's own strings (owner, 2026-11-02)

The owner's instruction: *"They also contain legacy and rather cryptic names. I would like those names to be human
readable like 'villager', 'elite_cataphract' etc."* - every populated unit, corpses included.

**Where the names come from, and why not the codes.** The dat's five-letter codes are neither names nor stable: 283 says
`PALDN` and is the cavalier, 569 says `HEROC` and is the paladin, 7 says `XBOWM` and is the skirmisher, 74 says `SPRMN`
and is the militia. Every record instead carries `name_str_id`, and the install's string table turns it into "Elite
Cataphract". So the table is **derived and reviewed, not typed**: `tools/names.py` joins the two and normalises the
result to snake_case.

**What is committed and what is not.** `config/names.toml` is ours - a list of identifiers - and is committed. The
extracted tables beside it stay out of git under D8, unchanged for now (the owner chose "C now": pin and verify, ship
later). The owner's plan is *no reimport*: once the configuration no longer needs the dat's numbering, the tables become
hand-maintained content and can be committed, because the ids were the only thing that had to be extracted.

**The rules the table follows.**

- A name is an **identifier**: lowercase snake_case, unique, and therefore a rename is a migration.
- A unit with `name_str_id = 5000` has no name in the game either (the editor's props: `TERRC`, `WNDR`,
  `INDESTRUCT`), so it falls back to its code, slugged - ugly on purpose, and honest about its origin.
- A corpse is named after what leaves it: 449 is `scout_cavalry_corpse`, and when two animals share one body the base
  animal names it.
- Duplicate display names are suffixed by ascending id (`barracks`, `barracks_2`): 413 of them, mostly age variants of
  one building.

**Direction (owner, same message):** ids should disappear from the configuration where feasible, and names become the
reference. `names.toml` is keyed by id today because every table still is; flipping the tables to name keys is the next
step, and it needs names for every reference the effect table makes - units *and* the ~30 armour classes.

## D98 - Two owners' rulings on movement (owner, 2026-11-02)

Asked what needed deciding before the evasion work, the owner ruled both:

1. **The dead pathing knobs are deleted, not wired.** `movement.toml` carried `cost_straight = 1.0` and
   `cost_diagonal = 1.41421356` while `path.rs` hardcoded 10 and 14 - the same ratio, so nothing changed either way, and
   the knobs advertised a control that did not exist. The ratio is a property of the grid (sqrt(2) on an 8-way grid), not
   of a content pack: a pack has no opinion about the geometry of a step. The comment that replaced them says so, because
   the knobs were the first thing a diagonal-movement complaint reached for.
2. **Soft evasion is "wait and let pass".** A unit whose next tile is occupied yields for a tick or two rather than
   sliding sideways: deterministic, no repathing, and a headless test can pin it exactly - two villagers crossing never
   share a tile and both arrive. Sliding is a later refinement, only if the waiting looks wrong on screen.

### D98a - superseded by the original's own mechanism (owner, 2026-11-03)

The owner asked whether pathfinding and evasion could be read out of the game binaries, was shown the map and the
collision block, and ruled: **"B adopt from the original game."** So point 2 above is superseded in its *mechanism*; its
**intent** - two units never pass through each other, and a crossing is deterministic and testable - stands and is what
the tests assert.

What the original does, read from `doIntelligentMovementUpdate` (move_obj.cpp 3,290+, collision block 4,600-4,760):

- collision is queried **at the new position** with the unit's radius, not per tile;
- a collision **with the order's target** means the move is done;
- a collision with an object the path *already* avoids, or one its own class filter discards, is **past** - not counted;
- otherwise the collisions are **non-past**, and the rule is "**repathing**", planning around the units it just met;
- terrain collisions are counted separately and force the same repath;
- when there is nothing actionable it says so ("Don't know what to do about my collision(s)") and moves on.

Three consequences the engine inherits: a repath needs a path search that accepts extra obstacles
(`find_path_avoiding`); the set of units a route is planned around is **state** (hashed, cleared with the route); and
"past" is, until the one unread predicate is decoded, defined as **already in this route's avoid set** - which is what
the binary's membership test reads as and is operationally the same statement.

The straight-line shortcut (no non-target collision on the line to the goal) is part of the same original block and is
recorded as the second slice, not the first: it changes nothing until collision exists at all.

## D99 - An army, and the ground a farm leaves behind (this slice)

Four rulings, each forced by a measurement rather than chosen:

1. **An army is a count, and an attack is a body.** A rule may wait on `army_at_least` / `army_at_most`, and an attack
   rule orders **every** unit of the family with a target chosen per soldier. The old shape ordered the newest unit only,
   which is why the owner saw one mangonel fighting and called it "attack per unit". The number the base personality uses
   (three to send, twelve to stop) is ours, not the game's - the AI's own `.per` files are not in this install - and is
   recorded as a threshold that a measurement can move.
2. **A spent farm is a floor, in all three places that ask.** Its `amount` is what it holds (never its capacity, which is
   why a spawned husk used to come back full), it is free ground for the next farm, and it is not a wall for the lane rule
   - a farm's `[footprint.farm]` is entirely walkable, so it cannot wall anybody in. The risk this accepts is a packed
   field; the first worker trapped by one makes this rule the suspect.
3. **A building a player cannot see does not block their ground, and the walk layer does not get a vote.** D58 said so and
   the code did not: the footprint was skipped and then the same tiles were refused because `insert` had blocked them.
   Terrain and resource nodes still block a blind site; an unseen building's tiles do not.
4. **A refusal the engine gives itself is state, whoever issues it** - a queued site that cannot start included - **and it
   names its actor.** `entity_of_last_command` returned `EntityId(0)` for every command, which made every refusal
   undiagnosable.

## D100 - What an order may change, and what a rule may see (this slice)

1. **An identical order is a no-op.** Re-sending the same attack at the same target (same resolved numbers) keeps the
   reload and the route; the AI's twelve-second army order must not become the unit's combat clock, and a crowd of
   soldiers must not shuffle between equally-near stand tiles because their orders are refreshed. A *different* fight
   resets, and validation still runs first, so a re-order at a dead or friendly target is refused as it always was.
2. **A condition may read the work, not only the bank.** `working` counts villagers on a resource; a drop site is built
   because somebody is walking a long way with every load, not because a stockpile is low. Two miners was tried and never
   fired (the personality sends one at a time), so the threshold is one - the owner's own wording.
3. **A site nobody can walk to is not a site.** The placement search asks reachability after the cheap footprint and lane
   tests. This is the prior art's point (`Zaydme/openaok` remembers the distance of a site it found no way to), reached
   here from a measured refusal rather than from the reference.

## D101 - A speed change is the window's, not the match's (owner's speed selector)

The speed selector (1x, 1.5x, 2x, 3x, 5x) is **pacing**, and pacing is the window's: the simulation runs at its own fixed
rate (D16), a recording is a tick stream, and how fast a person watches is not part of the match. So a change is allowed
**mid-match**, is not hashed, is not in a recording, and is logged by the window (`speed -> 1.5x`) rather than recorded in
state. The fractional cases are carried by a thousandths accumulator, so 1.5x runs 1, 2, 1, 2 ticks and never drifts by a
rounding rule applied per frame.

## D118 - A click is a point, not the middle of a tile

The owner, after D114 made a tile order end in the tile's *centre*:

> "I don't want units to walk to the centre of the square. That can walk wherever not blocked."

Right, and D114 only fixed half of it. A tile order used to end on a tile's **corner**; D114 moved that to the **centre**,
which is a better corner but still not what a player clicked. The click is projected to a continuous world position, and the
view threw it away: `Camera::pick_point` gave a position, `Camera::pick` rounded it to a tile, and the order carried the tile.

**Now:** the ground click sends `Command::WalkTo { to: (Fixed, Fixed) }` - the point under the cursor - which the engine has
carried all along (`start_walk` takes a point; the planner aims at points; `tile_of(point)` is the goal tile for the ground
test). `Command::Move` (a tile) stays for scripts and the AI, where a *tile* is what the scenario means, and it keeps
`tile_centre`: a script that says "tile 5,4" has to name a point somewhere, and the middle is the sane one.

**`Snapshot` gained `scale`** to make the conversion honest: the engine's units-per-tile is the pack's parameter, and a
hard-coded 20,000 in the view would be a second answer to a question the config already answers.

**What does not change:** the ground rule. "Wherever not blocked" is the tile rule (D117): the point's tile must be walkable,
and a click on a blocked tile still means "come as close as you can" - the planner's goal-tile exemption and the mover's
standoff, both already in place.

Review: `docs/reviews/2026-11-04-a-click-is-a-point.md`.

## D117 - Passage is a property of the ground, not of the body

The owner's rule, given while I was three attempts into making the ground obey a body's radius:

> "Either fully occupied tiles touch (even by corners) and there is no passage ever at all, or [they] don't, and then there
> is passage for anyone. No matter the size of the enveloping circle or number of units."

**So: ground passability is decided by tiles, and a body's circle is never consulted for it.**

- Occupied tiles that **touch, even at a corner**, leave no passage - ever, for anyone. That is the diagonal rule the engine
  already had ("two trees touching at a corner leave no gap").
- Tiles that do **not** touch leave passage for **everyone**: any circle size, any number of units. A formation that cannot
  fit squashes and goes through; it is not refused.
- The body's circle still governs **bodies**: `collide`'s standoff between two units, and how the unit is drawn. Different
  sizes have different circles, and that is what the owner sees when selecting them.

**What this changes in this repository: nothing, which is the finding.** `Walk::blocked` is per tile and a radius never
enters it; `two_trees_touching_at_a_corner_leave_no_gap` has pinned the corner rule since the passage slice. The slice is a
test and this record: `a_big_unit_walks_through_a_one_tile_gap` walks a `trebuchet_packed` (radius **0.5**, the biggest land
circle in the pack) through the one-tile gap between two houses, and it passes unchanged.

**What it cancels: M3 "the body's shape against the ground", i.e. M3c.** That slice judged the ground by the body's disc
(`ground_overlap`, "a step may not make the overlap worse"), and in a one-tile gap a 0.5-radius disc *touches both walls*:
the overlap is zero exactly on the centre line and any deviation increases it, so the body can only move perfectly straight
and jams on the first drift. That arithmetic is what left eight workers stuck in the prototype. It is not how AoE2 works, and
the owner has said so. The prototype stays at `/opt/data/cache/m3-findings/` for the record; **the model is cancelled, not
deferred**.

**Named for the future:** `plan_path_for` still expands obstacles by the body's radius. That does not refuse a one-tile gap
(the expansion is exact and touching is allowed), so it is not wrong today - but if a moving unit ever has a radius above
half a tile, the planner would refuse a gap the rule says is passable, and the expansion must become a follower margin
instead.

Review: `docs/reviews/2026-11-04-passage-is-the-grounds.md`. **663 passed / 0 failed / 19 ignored.**

## D116 - A sheep stands still: herding is dropped (supersedes D70's movement)

The owner, on reading that the gather approach loses its route when a node moves:

> "Is it about sheep being herded by shepherd? If yes, I'd like to drop this feature. Sheep are more likely to be found by
> scout and player and AI are more likely to direct sheep to the drop site themselves."

So a gathered herdable is **worked where it stands**. Moving livestock is an ordinary order its owner gives - the player or
the AI - and the animal does not steer itself anywhere.

**Removed:** the `herds` pass's drop-site lookup, `walk_towards(animal, drop_pos)`, and the `paths.remove(&villager)` that
dropped the shepherd's route each tick so it stepped at a moving target. **Kept:** everything about a herdable that is not
movement - ownership by sight (D70 item 5), the one-blow kill, the shepherd's-knife task name, and the
slaughter → body → resume-gather chain.

**The strike trigger moved from the drop site to the animal.** It used to fire when the animal *arrived* at the nearest drop
site; it now fires when the villager is within reach of it. A strict `distance^2 <= reach^2` never fires, because a body
walking at another comes to rest a step short of the standoff - measured **10,399** against **10,000**. The rule is reach
**plus one tick's step**, or the animal's own tile: the same "a radius, not an equality" rule the engine uses everywhere
else for arrival (C2).

**Tests:** `a_sheep_stops_at_the_drop_site` is retired with its rule (an animal arriving at a building no longer happens);
`a_gathered_sheep_stands_still` and `a_sheep_walks_where_its_owner_points` replace it; `the_villager_follows_the_sheep`
survives re-worded, its tolerance tightened 6 tiles -> 2 because there is no longer a moving target to trail. One fixture
was wrong in its own right: `a_live_animal_is_a_gather_source` put the sheep at `centre + 1`, the town centre's **solid
core**, and had only been passing because a body's centre could stand on the core's corner.

**Consequence for M3:** the automatic moving-target gather is gone, so the blind-walk hazard
(`next_waypoint(id).unwrap_or(target)`) is no longer in M3c's way. It remains real for a player who walks a sheep away
mid-gather, and is recorded in `docs/next-step.md` rather than fixed.

Acceptance: **unchanged**, hash `82efabd6f614c798` - that scenario gathers no herdable, which is the honest reading of a
change the benchmark cannot see. **662 passed / 0 failed / 19 ignored.** Review:
`docs/reviews/2026-11-04-a-sheep-stands-still.md`.

## D114 - A walk to a tile ends in the middle of it

`World::tile()` returns a tile's **corner** - `x * scale, y * scale` - because that is the anchor a building's footprint and
a spawn position are placed from. It was also the point every walk aimed at, so a tile order ended where four cells meet:
half a tile off the middle of the tile the player clicked, and exactly on a wall's or a tree's corner whenever one of those
four cells was solid. Measured before the change: **10,000 raw units** from the middle of the goal tile, and **0** raw
units from a house the order was placed beside.

`tile_centre()` is now the walk-goal convention (`x * scale + scale/2`), and `tile()` keeps its meaning for placement. The
two are separate functions with a comment saying why, because the bug was that one function was doing both jobs.

**Which goals are centred**, and which are not:

- centred: `Command::Move`, `AttackMove`, `Patrol`, a unit's order to a tile, a work stand tile, a rally point on plain
  ground;
- **not** centred: a garrison rally (the goal is "come to this building" and the entering rule decides when it is close
  enough - centring it walked the unit *into* the footprint and it never arrived), a rally onto a resource (the gather
  approach owns where the body stands), and `Command::Spawn`, which is a placement.

**Tests corrected rather than loosened.** Four assertions were measuring a proxy for the old convention, and each now
measures the rule: `points.rs`'s `a_tile_order_still_means_its_centre` had a *name* that said centre and an assertion that
demanded the origin; `straight_walk.rs`'s exact-landing test kept its intent and moved its point; `collision.rs` and
`evasion.rs` compared **tiles** where the rule is a **standoff**, so two bodies 0.7 of a tile apart in one cell failed a
test that claimed they were inside each other.

**The repath test's premise was the fixture, not a defect - found the same day, not a bug.** `a_repath_route_is_pulled_like_a_planned_one`
places a body on the walk's line to force a repath, and after centring the goals the walker arrived with `repaths = 0`. The
cause is the mover's own order of answers: from a *corner* to a *centre* the walk carries a small vertical component, so when
the body blocks the step the mover **slides** on the other axis and gets past without replanning. From **centre to centre**
the walk has one axis only, the slide is the very step that was refused, and the mover reaches its repath rule - so the
fixture now spawns the walker at the middle of its tile and the body at the line's exact midpoint, and the test measures the
pulled route again. **Sliding before replanning is the design** (D98a): a nudge is cheaper than a search and is what a player
sees as a unit stepping round one body. The two hypotheses I wrote down first - an idle body missing from `bodies`, or the
3x3 bucket index - were both wrong, and neither mechanism was ever at fault.

Acceptance: `players=1` at 36,000 ticks in 10.54-10.81 s, 148 entities, hash `82efabd6f614c798` (was `623c3ff8c769eadd`);
**661 passed / 0 failed / 20 ignored**. Review: `docs/reviews/2026-11-04-the-goal-is-the-middle-of-a-tile.md`.

## D113 - A route bends where the ground does, at the body's own clearance

M1 (`step_towards` walks the segment) made a walk straight **between waypoints**, and a waypoint was a tile's origin - the
point where four cells meet. So a villager rounding a house passed exactly through the corner of its footprint: distance
zero, body inside the wall, which the owner saw as "villagers went through the house". Measured, by mutating the clearance
away: **1 raw unit** from the wall with the body's 4,000.

`crates/sim/src/short.rs` is the answer, and it is the algorithm the original's maintainers documented
(`docs/pathfinding-from-the-talk.md`): the body is shrunk to a point and every obstruction expands by its radius; a ray at
the goal, and where it hits, a walk over the corners of the merged rectangles. The walk is a **visibility graph** rather than
the talk's edge-following - the same geodesic, and an invariant that is much easier to test (an edge exists only where a
straight line enters no expanded rectangle's interior).

The rules that make it fit this engine:

1. **All integers, exact.** Segment-versus-rectangle is Liang-Barsky in rationals with `i128` cross-multiplication - no
   sampled segment (which can step over a thin rect) and no float (the project's rule, and the talk's own lesson that
   precision is what breaks pathing).
2. **The window is fenced.** "Whoops, I edge-walked the whole map": an obstacle that runs out of the search window is not an
   obstacle that ends there, so four thin boxes at the window's tile boundaries make leaving it impossible. Without them a
   sealed wall was walked around through its cut end - the test that caught it is `a_wall_across_the_world_is_no_path`.
3. **A minimum radius of one raw unit.** With no radius at all, two blocked tiles touching at a corner leave a zero-width
   pinch a line may pass exactly through, and the tile rule forbids it ("no passage between two trees that touch at a
   corner"). One unit closes every pinch and is invisible in play.
4. **Two exemptions, the same two the engine already had**: the start's own tile (a worker stands on its node) and the
   goal's (an order on a wall means "come as close as you can").
5. **The start is not a waypoint.** A route that began with the unit's own position cost a tick of standing still, and it
   made a lead-shot prediction read zero motion for a runner.
6. **Bounded, and it declines rather than guessing.** A window over 40 tiles a side, more than 96 rectangles or 400 nodes,
   and the solver returns `None`; the route then falls back to the pulled tile chain with its corners at **tile centres**.
   A long strategic walk keeps the cheap answer; the corners a player watches are the ones in a base.
7. **Bodies are obstacles only where the caller says so** - the repath's avoid set, because a route planned around a body
   that the geometry then straightened through it is worse than no smoothing at all.

**Measured:** 40.2 us a solve in a base-sized window (eight buildings and a wood, seven corners on average, `short_probe`);
the acceptance match wins at 36,000 ticks in 10.5-10.7 s with **148** entities against M1's 8.7-8.9 s with 126 - different
matches, so that difference is not a like-for-like cost, and an ablation that forces the tile fallback played yet another
match (12.4-13.0 s, 156 entities) rather than measuring the geometry. What the isolated number says is that a plan is
microseconds, which is the question a per-plan solver has to answer.

## D112 - An enemy may be inspected, and an enemy's maximum health is the owner's

The owner, one line after D111: *"Enemy hit points on click"* - the half D111 explicitly left out.

1. **Another player's units and buildings are selectable**, after your own: the order is own unit, enemy unit, own building,
   enemy building, node - units before buildings because that is the order the picture is drawn in, and yours before theirs
   among equals. So an enemy standing on a farm or a node is what the click means.
2. **The fog is the filter, not the selection.** An enemy is in the snapshot only while the player can see them
   (`Snapshot::of`'s rule: a unit out of sight is gone), so "select an enemy" cannot be a peek at a place nobody is
   watching. Pinned by `an_enemy_across_the_map_is_not_in_the_snapshot`.
3. **An enemy's maximum health is the owner's.** Health upgrades (Loom, Masonry) belong to whoever researched them;
   dividing an enemy's hit points by the *viewer's* maximum made a full-health enemy read as 25 of 40 - `hp_fraction`
   0.625 - once an enemy could be selected at all. That was my own bug, present since the snapshot was written, and it also
   put the same wrong bar over the enemy's head on the map.
4. **No card for somebody else's building.** `World::card` already refused a building that is not the player's; a test now
   pins it, because with enemy buildings selectable the guard is load-bearing rather than unreachable. Orders were never at
   risk: they are built from the entity under the pointer and only from the player's own units in the selection.

**Named rather than changed:** a *remembered* enemy building (seen before, not watched now) carries **live** hit points in
the snapshot, and its health bar is drawn live on the map. Freezing a memory means capturing the hp at the moment of last
sight, which is its own slice with its own question - what a player should be told about a building they are not watching.

## D111 - A resource node may be inspected, and a click shows rather than orders

The owner's ask (2026-11-04): *"I would like trees to be clickable showing hit points and volume of wood they have. Same for
farm, can I see amount of food it has left? Same for berries, stone and gold ... in the central part of the bottom bar"* -
and his one clarification: *"left click on resource (berry bush, farm, mine...) just shows its statistics. Nothing else."*

1. **A click on a node selects it and nothing more.** It orders no gather; that stays where it was. A click that both
   selected and commanded would make inspecting a forest an accident.
2. **Nodes are inspectable whoever owns them.** Every node a map seeds belongs to Gaia (player 0), so "only your own" would
   exclude all of them. Another player's units and buildings stay out of the selection: enemy hit points are a separate
   item with their own question about what the fog lets a player know.
3. **A node's hit points are a number, not a bar.** The simulation stores no maximum for a node - its `hp` *is* its health -
   so a fraction would be a fraction of itself. A unit or a building has a maximum from the pack, and keeps its bar.
4. **The panel says whose number it is.** `World::held_slot` answers which resource column an entity holds: a node's own
   kind, or, for a **farm**, content's `[farm] resource` - the same table the gather rules ask, so there is one answer to
   "what does a farm yield". The pack names the column; the view never learns what wood is (D17).
5. **A space is a width in the font, not a missing glyph.** The font has no blank shape and "a character with no shape takes
   no room", so every word in the interface ran together (`WOOD100`, `HP88`, `FEUDALAGE`) until a rendered frame was read.
   `font::SPACE` is half a glyph plus the gap and paints nothing.

Not decided here: enemy hit points on click, a band or double-click that sweeps nodes, and a node's maximum hit points
(which would be a new field in the simulation, not a drawing change).

## D110 - The step walks the segment, and bodies still slide

D109 said continuous steering is a pass over path, arrival and collision. This is the first slice of that pass, and the
boundary of what it changes:

1. **The step is along the line, not per axis.** `step_towards` clamped each axis to its own remaining gap, which turns
   every heading that is not a multiple of 45 degrees into a 45-degree leg plus a cardinal leg - the owner's "45 degree
   walk followed by horizontal walk", reported three times, the third with a screenshot. It now moves `delta` along the
   segment and lands *exactly* on the point when the remainder is within one step, so `position == waypoint` stays the
   spend test and every `position == target` arrival rule needs no new case. All integer (`integer_sqrt`, an `i128`
   product, `f64` nowhere): floats in the tick are the desync the project forbids.
2. **Two details are load-bearing, and both were found by a test.** A component that rounds to zero freezes the unit with
   a purely axis-aligned step, which has no slide to fall back on against a wall, so every axis with a gap keeps at least
   one raw unit of movement per tick; and a *zero* step must move nothing at all, or a unit the pack gives no speed creeps
   a raw unit a tick for the rest of the match.
3. **A repath's route is pulled, like a planned one.** It was not: the search's answer is a node per tile, so a unit that
   repathed after a collision walked tile to tile - and in a base full of villagers most long walks are repaths. The pull
   obeys the same avoid set as the search, because a jump back through the body the search went round would be worse than
   the route it replaced.
4. **Bodies still slide; nothing is adopted.** Slide 9 of the talk states the original's rule - "units bump into others,
   stop and repath around. No pushing allowed" - and this engine does the opposite (`push_units_off`, plus `collide`'s
   slide and one nudge per route). That is a *gameplay* rule, not a correctness one, it would move the acceptance hash for
   a reason the owner never gave, and it is his call. Until he makes it, sliding stays and this row records that the
   question is open rather than silently decided.

**Because a step now covers the true length of a segment, a diagonal walk is no longer sqrt(2) faster than a straight
one.** That is a correctness fix - the dat's speed is tiles per second - and it retimes every build order written under
the old speed, including `config/scenario.toml` (one step moved 380 s -> 390 s; see the review).

Still to come, and next: the route's **corners** are tile centres, not the corners of the obstacles expanded by the body's
radius (`docs/pathfinding-from-the-talk.md`), so a walker still turns at the middle of a tile and can graze a wall by half
a tile. Everything else about collision - the goal exemption, the both-corners rule, the one nudge per route - is
unchanged. A change of this size is measured against `HEAD` interleaved, and the acceptance hash moves: what a unit walks
to is what it does next.

## D109 - Continuous steering is a pass, not a step function

`step_towards` still walks eight-way. Normalising it (so a unit walks the line it was given) was implemented and measured, and
it broke arrival: path, arrival and collision each decide what "the next point" means, and changing one leaves the other two
disagreeing. Continuous steering has to change all three at once - aim at a point rather than a tile centre, arrive within a
radius rather than on a coordinate, and slide a shape along a shape - and land with a test for each. Recorded so the next
attempt starts from the reason this one failed rather than from the idea.

## D108 - A unit is painted above a building, and the bar is laid out from its own numbers

A flat-shape renderer has no sprite height, so depth alone hid a villager standing in the back half of a house or a farm. The
owner asked for units to be visible over buildings, so `layer_of` has three passes: ground cover, buildings, units - depth
orders within each. The old rule ("a unit behind a building is covered by it") was a guess about how the art works; this is a
statement about what the player needs to see.

The top bar is laid out from `font::text_width` rather than from fixed slots. Two numbers that are computed in two places
drift, and this project has already paid for that once (the panel height, 76 against 142): `stock_columns` and `stock_rows` are
the only places a stock position is decided, and the tests ask them too.

## D107 - The minimap is the world's projection, and its box is the camera

The minimap uses the same isometric projection as the world (`u = tx - ty`, `v = tx + ty`), so a player reads one picture in
two sizes. The view box is the camera's **four screen corners picked back to tiles** and projected - a slanted parallelogram,
not a bounding box, because a box would say the view is axis-aligned in tile space when it is not. `minimap_tile` returns
`None` outside the diamond, so the dead corners of a rectangular frame cannot move the camera somewhere that is not on the map;
a drag that leaves the box is clamped rather than cancelled. The old square `ui::minimap` is deleted: two printers of one
picture is how the panel height drifted to two numbers earlier in this project.

## D106 - The farm ring is content's, and reachability is asked last

`[build] farm` names the plot and `[build] farm_spacing` is how many farm widths sit between the town centre and the mill. The
ring's reach is `spacing x farm width + 1`, the width taken from the dat's own footprint. A farm fills the ring; a drop site
may stand in its outer half (the mill *is* the ring's outer edge); everything else stands outside it. A pack with no `[build]
farm` has no ring and every building is free, which is the behaviour that came before.

**The rule that came out of the performance bug: a reachability test is asked last, and of the fewest candidates.** Three
places were doing a path search per candidate - the camp rule (840 ms a firing), the farm search (256 candidates) and
`nearest_like` (**1,336 path searches in one tick**) - and each looked innocent on its own. Geometry (`is_free_ground`,
`site_fits`, a distance sort) is cheap and can be asked of everything; the walk is a search and belongs on the handful of
sites that survived. Measured: 104 s -> 11.9 s, with a byte-identical acceptance hash.

## D105 - A birth is never inside a building, and a rally into one walks to the door

Two rules that the opening scout exposed rather than caused:

1. **A trained unit's spawn tile is never inside any building's footprint.** The town centre's yard is walkable and free, so
   `free_spot_near` used to hand a finished villager a tile inside the town centre it had just left - a unit standing where
   the game does not put one, and (2) made it unable to walk back in.
2. **A rally to "inside" is a walk, then an entry.** `check_garrison` refuses a unit that is not at the door; the code used
   to drop that refusal and idle the unit for ever. Now the unit walks to the building and enters on arrival
   (`World::entering`). This is what the rally means, and the silence was the bug.

Population follows from the first: an opening scout spends one of the town centre's five slots, so a match opens with three
villagers, one scout and room for one more villager.

## D104 - A card offers what the age allows

An item whose refusal is **the age** is hidden; an item whose refusal is a price, a queue or a building the player could put
up now stays on the card, greyed, with the engine's own sentence. The classification lives on `World::is_age_gate`, not on
`SimError`: it needs the pack's `[age] order` to know that the missing prerequisite is an age (the dat's numbering says
nothing), and putting it on the error type forced `AgeNeedsBuildings` into the gate set, which hid the button that tells the
player how to age up.

Money never moves a slot; an age that opens an item appends it.

## D103 - A camp is not built where a drop site already serves the resource

Content names the roles that get the rule (`[build] spaced_roles`) and the spacing (`[build] drop_site_spacing = 5`); the
predicate asks the *worker's own* variant which buildings accept its load, so the dat's `drop_sites` edge is the only source
of "drop site". The number and the range are the reference implementation's (OpenAoK, MIT): `kCovered = 5` and
`sn-camp-max-distance = 25`.

**One deliberate difference**: OpenAoK's camp rule passes `town = false`, so its camps ignore the town centre's coverage. The
owner asked for the town centre to count, and a camp beside a town centre that already takes wood buys nothing, so our camp
rule passes `true` and a mill passes `false` (a mill must be able to stand at the berries beside the town centre).

Not yet enforced: the 25-tile range (a far uncovered forest still wins over a near covered one), which needs a decision
about which of those two is better rather than a number.

## D102 - A place is approached by its ground, and a unit is evicted before it turns solid

Three rules, learned together because they were one bug wearing three coats, all measured in the acceptance match:

1. **A building's place is its footprint, not its anchor tile.** The anchor is a blocked tile in the middle of the
   diamond, so "how far from the anchor" is a question with no physical meaning: a villager handing in from a town
   centre's yard is two tiles from it and stands *on* the building. `World::touches_footprint` answers the real question
   (on or beside the ground, one tile of slack), and the deposit rule now accepts it.
2. **The route to a drop site is the anchor's, with the footprint as the fallback.** `approach_tile` finds the nearest
   walkable tile of the footprint (the yard), else the nearest tile touching it, and is used **only when the route to the
   anchor does not exist**. Making it the route for every trip was tried and measured: 176 entities and a won match became
   127 and a draw. A fix for a boxed-in town centre is not a new route for every trip.
3. **A building that appears evicts the units on its ground, before the ground turns solid.** A unit inside a blocked
   footprint can never plan a route again (A* refuses to start on a blocked tile), and the old eviction matched exact
   positions - while a unit stopped by collision stands inside a tile, rarely on its origin. The escape tile is chosen on
   the grid as it stood before the insert, by a ring search (a square scan cost 60 s of suite time).

**Cost of the change, reported:** the acceptance match still ends in a win (160 entities at 36,000 ticks, hash
`ab314fb1969fde95`, 29 s) but no longer matches HEAD's flow (176 entities, `c12bdc86aab31513`): moving units off ground
they should not have been standing on changes what happens next. The alternative - keeping villagers inside buildings -
is not a behaviour worth preserving.


## D119 - The unit vocabulary is names, and the upgrade edge belongs to the tech

*2026-11-04. The owner: "I want to get rid of unit IDs. Instead it's like each unit having clear parameters - starting
availability and which unit it upgrades", and on modding: "What if I chose that scout cavalry will be upgrade of hussar, not
the other way around?"*

1. **The plan is written before the schema, and measured first.** `docs/unit-vocabulary.md` is the plan;
   `tools/vocabulary.py` is the inventory every number in it is measured against and it exits non-zero while the pack would
   block a name-keyed schema (today: **56** unnamed units, **28** duplicated names).
2. **The edge is the unit of content, not a per-unit field.** An upgrade is `(from, to, tech)`; the tech is the actor. A
   per-unit `upgrades_from` would be a lossy projection and could not say which of two effects performed an upgrade.
3. **The line is derived from edges *plus* prerequisites.** An edge whose `from` unit cannot exist when its tech is researched
   is a safety net, not a rung - the halberdier's `93 -> 359` command is exactly that, because tech 429 requires tech 197,
   which has already turned every spearman into a pikeman.
4. **The loader must detect a cycle, not assume acyclic.** The dat has one: `RMSHP` and `SDGAL`, the two fishing-ship effects,
   upgrade each into the other.
5. **Two of my own readings were wrong before this settled**, and both the same way: I read the **effect table** as if it were
   the game's rules. The owner corrected the halberdier twice ("reached from something else than pikeman? How?" then
   "halberdier replaces only pikeman and pikeman replaces only spearman"). The lesson is the decision: when a data table and a
   player's rule disagree, the **tech tree** decides, and the table is wiring.

## D120 - A card in the bottom bar, and the wide health bar is gone

*2026-11-04. The owner: "let's add a functionality where unit stats are shown in the bottom central part. the big long health
bar can be removed. For Villager: renamed to match the task he is doing and show stats and carried resources ... For units see
Selection-Bar.png for ui example." Answers: cards for several units (3A), the wide bar out and the small one under the portrait
kept (4A), attack + both armours + range (5A), job names from content (2A).*

1. **One card per selected thing, in a row, as many as fit.** The zone's whole width was one bar; it is now cards.
2. **The numbers are the owner's, not the viewer's.** `World::stats_of(player, unit)` returns `UnitStats` after that player's
   upgrades - D112's rule for maximum health, applied to every number. A pack's unit table is not the view's to read.
3. **Attack is the larger of class 3 and class 4**, which is what the game's own card shows: an archer reads 4 and a villager
   reads 3, not the 6 its class-13 anti-building bonus carries.
4. **Hundredths across the boundary.** Speed is 96, not 0.96: integer only, so no float can reach a tick.
5. **The portrait is an empty frame** until the renderer has art. A placeholder that is visibly empty is honest; an invented
   silhouette would read as content the pack does not have.
6. **The job names are not in this step.** They need the pack to say what a *target* implies (a berry bush is a Forager, a farm
   is a Farmer), and that is a content question with its own answer - the follow-up slice.

## D121 - A working villager is named by his job, and the words are content

*2026-11-04. The owner: "For Villager: renamed to match the task he is doing and show stats and carried resources - see
latest.png". Asked how the pack should key the words, he chose **the target's role name** (option A).*

1. **The engine says which target; content says the word.** `World::work_target(id)` returns the target's **role name**
   (`"farm"`, `"sheep"`) and the **resource column** it holds. `config/jobs.toml` maps those to `"Farmer"`, `"Shepherd"`,
   `"Forager"`. Nothing in `crates/` knows that a woodcutter is called a Lumberjack.
2. **A column key exists because a role key could not cover the ground.** The reference pack's 40 roles cover buildings,
   animals and soldiers but **no tree, bush or mine** - so `target = "tree"` would never match and a woodcutter would have no
   name. The loader therefore accepts `target` (a role) or `column` (a resource), refuses an entry with neither, and a role
   wins when both match, so a pack that adds roles for its trees overrides the column entry without moving anything.
3. **No match means no job, and the unit keeps the pack's own name** - a villager doing nothing is a Villager.
4. **One rule, one place.** `ruage_view::label_for(job, role, fallback)` is asserted once and used by the window and the
   diagnostic picture, so the game and the picture cannot disagree about what a thing is called.

## D122 - A farm takes one farmer, and an empty one leaves a husk that goes

*2026-11-04. The owner: "Only 1 farmer per farm (can be constructed by multiple, but only 1 can farm once construction is
finished). Once farm expires, it decays and can/needs to be replanted. It will cost 60 wood", then: "1 A but vanishing takes
some time, husk first ... Husk is effect only and only serves the player who can left click villager, right click on the husk
and villager starts planting new farm. It obstructs nothing as if it was not there", and on the second man: "2 C if there is
no empty farm nearby, villager becomes idle."*

1. **One farmer a farm, answered rather than refused.** A gather order on a farm that already has a worker on it sends the
   villager to the **nearest free farm** of his own side - free meaning his, finished, holding food and without a worker - and
   leaves him **idle** when there is none. Nothing is refused: the order is taken and answered, which is what the owner asked
   for.
2. **An empty farm becomes a husk, and the husk has a life.** `is_expired_farm` was already the reading (finished, no food);
   what is new is that a husk is now **tracked and removed** after `[farm.depletion] husk_seconds` (30, content's number). The
   clock lives in `World::husks` and is in the state hash: when a husk leaves decides what the match looks like, so two runs
   that disagree are two matches.
3. **A husk obstructs nothing and holds nothing.** It was already outside the overlap rule and walkable; the test walks a
   villager over it to prove it, rather than trusting the code path.
4. **A husk is a plot.** A villager right-clicked onto a spent farm of his own side gets a **Build order for the farm the husk
   was** (`EntityView::is_husk` tells the view, and the build order pays the pack's 60 wood). An enemy's husk is not yours to
   replant.
5. **The farm's price was already right.** The pack's `FARM` (50) costs `[[1, 60]]` - 60 wood - and the earlier suspicion that
   it cost 50 food came from reading unit 259, which is the *farmer villager variant*, not the building. Checked, cleared, and
   recorded here so the next session does not re-suspect it.

## D123 - The vocabulary is `names.toml`, and the husk is the dat's own unit

*2026-11-05. The owner, asked what to call the units that had no usable name: "1 A. I may rename them later on. IIRC language DLL
files should contain proper names. That way it was possible to translate the game in hex editor. Age2HD could do it differently
though. If none is found there or in other files, follow A" - and, of the two numbers I chose: "2 can you find in original
files? It must be somewhere there."*

1. **The names were already found, and already in use.** `resources/en/strings/key-value/key-value-strings-utf8.txt` carries the
   header *"Strings from Game/en/language.dll"* - the file the owner remembered - and `tools/names.py` (D97) has been joining
   each unit's `name_str_id` to it all along: `5121 "Villager"`, `5149 "Farm"`, `5409 "Halberdier"`. `config/names.toml` holds
   **1,390 unique keys for 1,389 units, no duplicates and no gaps**. Option A is therefore unnecessary, and nothing is left to
   decide.
2. **My D119 inventory measured the wrong column, and the plan inherited its numbers.** It read the `name` field of
   `units.toml` - the dat's five-letter codes, kept as provenance - and reported "56 unnamed, 28 duplicated". Both were true of
   the codes and false of the vocabulary. `tools/vocabulary.py` now reads `names.toml`, so the gate cannot measure the wrong
   column again; the plan's phase 2 is marked done.
3. **The husk is a unit in the dat, not an invention of ours.** The farm (50, `FARM`) has **`dead_unit_id = 357`** - unit 357,
   `FARM_`, string **5407**, hp 0, `resource_decay_rate = 1.0` - which is exactly the "effect only" husk the owner described.
   The faithful model is to **replace the farm with unit 357** when it empties and let its decay remove it, rather than to keep
   the farm's id and a flag. That is the next slice; today's engine keeps the farm and a husk timer, and the evidence is
   recorded so the switch is mechanical.
4. **The husk's duration is still ours.** 357's `resource_decay_rate = 1.0` is the dat's input, but *what* it decays is not in
   the dat and the exe is where it would be. `husk_seconds = 30` stays, named as our number.
5. **"Nearby" has no constant in the original files.** The AI constants carry `sn-lumber-camp-max-distance` and
   `sn-mining-camp-max-distance` (both commented out) and `sn-camp-max-distance`; **nothing for farms**. `free_farm_radius = 12`
   is ours, and the review records why.

## D125 - A selected unit wears a ring the size of its own body

*2026-11-05. The owner: "Now I would like to proceed with circle enveloping the units ... Units have around legs a circle.
Smaller units have smaller circle (infantry, villagers...) and bigger units have bigger circle (cavalry, elephants, siege
units, ships...). Sizes of the circles should be parameters of units (in config only, not shown in unit details). They should
be centered on the center of the base of the unit." Answers: **selected units only**, **the pack's radius times a factor**
(`k = 1.5`), **an ellipse on the ground plane**.*

1. **The size is the pack's `radius`, which already exists** - 0.2 for a villager or any infantryman, 0.25 for cavalry, 0.5 for
   siege and ships, 1.0-2.0 for buildings. Before this, every selected thing wore a ring the size of its **footprint**, and a
   unit's footprint is one tile: a villager, a knight and a trebuchet all wore the same 0.9-tile diamond.
2. **The ring is `radius * k`, with `k` in content** (`[ring] scale = 1.5` in `economy.toml`). Exactly the collision radius
   would draw the ring through the sprite's feet; `k = 1.0` still means "the ring *is* the collision circle", which is why the
   knob exists rather than a hard-coded fudge.
3. **`k` crosses the boundary as hundredths** (`ring_scale_percent`, 150) because a float in the simulation is a desync
   (D9). The `no_float` guard caught my first version, which returned `f32` from a `World` method - the guard was right and the
   accessor now answers an `i32`.
4. **An ellipse, not a screen circle**: the ring is a diamond outline under the isometric projection, which is what a ground
   circle looks like in this view. Measured on a rendered frame at 960x720: a villager's ring is **29 px wide and 15 px tall**,
   the 2:1 squash of the projection.
5. **Buildings keep their footprint ring.** A farm's radius is 1.5, which would draw a ring *inside* its own walls; a building's
   body is its footprint, so the old rule stays for buildings and the new one applies to units.
6. **Selected only**, so the game does not spend draw calls on units nobody is looking at.

## D127 - A route whose ground goes stale is re-planned, at most twice

*2026-11-05. The owner: "I still see scout bumping into trees and getting stuck. Can we learn something from
https://github.com/glouw/openempire?"*

1. **The gap**: `collide` re-planned when a **body** blocked a step, and did nothing when **ground** did - it slid on one axis if
   it could, and otherwise returned zero, for ever.
2. **The fix**: a ground refusal asks for a re-plan of the route, **capped at two** per route (`MAX_GROUND_REPLANS`), through the
   same per-tick budget the body repaths use. The count lives on the route, is part of the state hash, and only a ground re-plan
   spends one.
3. **From `openempire`**: the principle that a route must be re-examined rather than walked into. Its implementation - re-plan
   every 25 cycles for every unit, in ten slices, plus boid wall-push - is recorded in `docs/sources.md` and **not** copied: the
   timer is a search per unit per 25 ticks (unmeasured here, and more work than a stall check), and the wall push is float
   steering, which D9 forbids in the tick and which would let a body pass a gap D117 says is closed.
4. **When the goal is truly unreachable** the unit stops after its two attempts and the engine records `NoPath`: searching every
   tick for an impossible target is the cost D98a's "past" rule was written to avoid.

## D128 - The external review, checked: two severities wrong, five fixes landed

*2026-11-05. The owner handed over an external static review and said "please scrutinize".*

1. **Fifteen findings, checked one by one against the tree.** Nine confirmed, **two wrong in severity** (an `i64` overflow that
   needs a position no map can hold, and a `raw * 100` whose raw is a unit stat, not a position), four confirmed but low-value.
   The reviewer's line numbers were accurate; its method could not compile anything, and it says so.
2. **The latent one is now impossible**: `seed_from` refuses a pack whose `2 * ((tiles - 1) * scale)^2` would not fit `i64`
   (`SimError::GeometryOverflow`, `max_scale_for`). The shipped pack is **seven orders of magnitude** inside the limit; the guard
   is for the mod that is not.
3. **Panics became errors** in the three production binaries: a resource pack that does not fit is reported and exits non-zero,
   and a diagnostic no longer dies on a vanished selection.
4. **One lookup, not four**, in the share pass - per villager per tick; measured **-8.3%** on the acceptance match with the hash
   unchanged, which is what makes the comparison honest.
5. **`i128` where a multiply can overflow** before a divide, even where the bound is unreachable today: the cost is nothing and
   the next caller may not be as lucky.
6. **Deferred, with reasons**: the `EntityId -> index` map (the real fix for #6/#8, and a slice of its own), `nearest_like` and
   `nearest_resource` (the review's best performance finding - short list first, reachability only on the head - and it needs a
   measurement), the footprint-mask assert, and the O(n²) RNG duplicate check.

## D129 - The chop is a rate per second, and its remainder is carried

*2026-11-05. Found by the second external review; the review filed it as a performance complaint and did not notice the arithmetic.*

1. **`fell_hp_per_second` is divided by the tick rate, never multiplied by it.** The pack says 1.5 hit points a second; the
   tick turned that into **30 a tick at 20 Hz**, so every tree fell to its first blow - 0.05 s against an intended 13.3 s.
   The felling mechanic had never run in a match.
2. **The rate is converted once, at load**, into `Gather::fell_milli_per_second` (thousandths). The tick adds it to a
   per-node carry and takes a whole point every `1_000 * hz` of it, so "1.5 a second" is exact at any tick rate and no float
   enters the simulation (D9).
3. **The carry is state and is hashed.** It decides which tick the next point comes off; two runs that disagree about it
   fall the same tree on different ticks.
4. **The rate is content.** `fell_hp_per_second = 0` still turns felling off; the loader refuses a negative or absurd rate
   rather than propagating it.
5. **A rate needs a test that states it.** "The tree is down by 15 s" was true before the fix, for the wrong reason - the
   pair of tests (standing at 10 s, down by 15 s) is what makes the rate observable.


## D130 - A fixture that cannot read its pack fails, and names the file

*2026-11-06. The second external review's test-hygiene batch, checked: 36 of the 82 sim test files turned every load error
into `None`, so their tests reported **passed** while asserting nothing.*

1. **The generated pack's absence skips a fixture.** `config/gamedata/` is produced from the game's install and is not
   committed (D8), so a fresh checkout has no pack and those tests must not fail because of it.
2. **A pack that is present and will not load fails the fixture**, with the file's path and the loader's error in the
   message. The distinction is whether `config/gamedata/units.toml` - the pack's own marker - exists, never whether an error
   was returned.
3. **One loader.** `crates/sim/tests/common/mod.rs` holds the only copy: five entry points for the five shapes the eight
   hand-copied `fresh()` bodies had, every other table through `present()`, which panics. A test file keeps a one-line
   delegate so that no call site has to move.
4. **The rule is testable, so it is tested.** `crates/sim/tests/fixture_rules.rs` drives the loader over a *broken copy* of
   the pack (must panic, naming the file) and over a directory with no pack (must return `None`). Watched failing first: the
   broken case returned `None`.
5. **Why it is more than tidiness.** The suite is the only thing that says the engine still behaves. A harness that turns
   "I could not read the content" into "the test passed" reports a green suite over a broken tree - the same failure as
   piping `cargo test` through `tail`, a green that is not evidence.


## D131 - The make-room step has never run, and it fails: two head-on bodies overlap

*2026-11-06. Found by D130 within the hour: a fixture that had been skipping since the loader gained its forest-tile check.*

1. `crates/sim/tests/footprint.rs`'s inline corridor map declared its wood nodes on **grass** - `terrain` was `G` on every
   row - and `load_map_str` refuses a wood node that is not on a forest tile. `.ok()?` turned that refusal into `None`, so
   `a_head_on_pair_in_a_one_tile_corridor_passes` returned at its first line and reported **passed**. The map now has
   forest rows where its tree rows are, and the test runs.
2. **It fails.** The pair comes **7,372 raw units** apart on tick 66, against the **8,000** their footprints need -
   `reach_between`, computed from the pack's radii, not a number written into the test. The mechanism is sequential
   integration: the second mover of the tick is checked against the first's *new* position, finds itself "already inside"
   (D52's rule 3) and is therefore allowed to close the gap further.
3. **The test is ignored with that reason on it**, not deleted and not weakened. A red that is no longer watched is how
   this was lost the first time.
4. **The repair is the next step**: a step whose candidate lies inside the reach may not *reduce* the gap, for either
   mover, whichever order the pass visits them in - and if that cannot hold for both, the pair waits a tick instead.

5. **The mechanism, confirmed against the arithmetic.** Rule 9 (`wait_or_give_way`) returns the **full** step for the unit
   that gives way, so when the other has already refused, the giver closes from exactly `reach` (8,372) to `reach - step`
   (**7,372**) - one step, 1,000 raw, 0.05 tiles. Rule 8 is not the cause: at tick 66 both bodies are mid-route (7 tiles at
   1 tile/s is 140 ticks, so they meet at 66), so neither is "at rest".
6. **The invariant is asserted elsewhere and holds.** `collision.rs` checks every pair of a three-unit converging crowd
   after 600 ticks; what breaks the pair rule is a genuine **head-on pass in ground too narrow to pass in** - two bodies of
   `radius = 0.2` need **0.4** of a tile, and the lateral nudge of rule 6 offers a quarter of a tile, once per route.
7. **DECIDED 2026-11-06: option A, scoped to two players' units.** The owner: *"I think A should be it. I do remember in
   original game that narrow passes with 2 units passing in opposing directions were a problem."* Implementation:
   `World::bound_to_reach` in `crates/sim/src/world/movement.rs` - rules 4 (same tile, both moving) and 9 (give way) no
   longer hand a body its full step into another body's space; the largest step that keeps the pair at or outside `reach` is
   tried by halving, five integer probes, no sqrt. **The bar applies to units of two different non-neutral players**, because:
   - the deck's own page 9 says *"formations allow for some friendly overlap"*, so friends brushing past is the game, not a
     bug;
   - measured, bounding **every** pair broke the acceptance match: **164 entities against 141**, both players still alive at
     1,800 s where the baseline had eliminated one, refusals **574 -> 1,021**. With the hostile-only scope the match is
     **byte-identical**: hash `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 574 refusals, 11.7 s.
   - **A body at rest is not bounded either**: a resource node carries no radius, so `reach` falls back to a whole tile, and
     the bound then forbade a villager from standing beside a tree it had been sent to chop - seven gather and rally tests
     failed, one villager stopping 5 tiles out on the node's boundary circle. Rules 4 and 9 are about two bodies that both
     want the ground.
   - The cost, as priced in A: a head-on pair in a one-tile corridor now **stops** instead of passing. The corridor test was
     renamed from `..._passes` to `..._never_overlaps` and asserts the invariant over 2,000 ticks plus that the pair actually
     met - the old name asserted the behaviour the owner has now ruled out.

8. **The options, kept for the record - the owner chose A:
   - **A: bound the give-way step** so it never ends inside the reach - zero overlap, but a head-on pair in a one-tile
     corridor **deadlocks**, and in the open it jams after the first quarter-tile nudge, because rule 6 allows one nudge per
     route.
   - **B: let them slip past** - give way laterally, each body to its own side, until clear. Nearest to how the game looks,
     but it needs rule 6's cap relaxed, and that cap exists because uncapped nudging turned a base into a mosh pit
     (measured then: 89 wood / 20 food / 50 gold at 1,600 s against 729 / 792 / 370).
   - **C: keep it and bound it** - a giving-way step may close by at most one step, and the test asserts `reach - one step`.
     No engine change; turns an absolute invariant into a bounded one and writes down that a pair may overlap by 0.05 tiles
     for the ticks of a pass.

9. **What the first-party deck says about the policy, added 2026-11-06 while checking the research thread** (Klingers,
   Meeting C++ 2025, page 9 - `docs/pathfinding-from-the-talk.md`): *"In Age of Empires 2: Units bump into others, stop and
   repath around. **No pushing allowed**. Formations allow for some friendly overlap."* Consequences, which narrow the
   choice:
   - **B is not the game.** Two bodies do not slip past each other; overlap in AoE2 belongs to **formations**, not to a
     corridor pass. B stays buildable as our own rule, but it can no longer be argued as fidelity.
   - **A stall in a one-tile corridor is correct behaviour, not a defect** - the game's own answer to two head-on units with
     no room is to stand there and re-plan. That makes A/C honest and makes "they must always get through" a requirement
     nobody has asked for.
   - **The deck's model is the fix's shape**: a unit is a point, every obstruction is expanded by that unit's radius, so the
     other body *is* an expanded obstacle. Separation then comes from the geometry of the plan, not from a pairwise reach
     test applied after the step - which is exactly the asymmetry this decision is about (rule 9 steps first and asks later).
   - **What the deck does not cover**: it ends at "brute-force smooth" - path *generation*. Following, standoff, arrival and
     unstick are absent, and this overlap lives in that gap, so a further reading will not settle it.


## D132 - Passage is the surface: grass and dirt, never wood

*2026-11-06. The owner, asked D117's question: "If dirt is a surface after the tree was cut and fully gathered from to
have 0 wood, then grass and dirt passable, wood impassable."*

1. **A town-centre site and a seeded body stand on grass or dirt, never on forest.** `Map::is_walkable` is
   `Grass | Dirt`; it was `Grass` alone.
2. **The route rule is unchanged and is a different question.** `Terrain::walkable()` keeps grass, dirt **and** forest,
   because terrain is ground and the **tree** is the obstacle (the owner's 2026-10-31 rule, D84). A cleared wood therefore
   becomes walkable ground the moment its node goes - no terrain mutation, nothing new in the hash.
3. **Two questions, two predicates, and now two doc comments.** `Map::is_walkable` answers "may something be placed
   here?", `Terrain::walkable` answers "is this surface ground?". They had been one predicate in the code's mind.
4. **Both directions are tested**: a map whose town centre stands on dirt loads, and one whose town centre stands in the
   woods is refused **by tile** (`(2, 0)` in the message), not by a general "not on open ground".
5. **The shipped pack does not move**: the acceptance hash is unchanged (`394c4892aac776d2`, 141 entities, 184 repaths,
   187 made_room) because the seeder's candidate tiles were grass already. The rule is for content that has dirt.

## D133 - The acceptance match is thirty minutes

*The owner: "30 minutes is enough."*

1. `config/acceptance.toml` sets `duration_seconds = 1800.0`; it was `2400.0` - forty - while the file's own prose said
   "a thirty-minute game" and the canonical command played `--ticks 36000` = 1,800 s at 20 Hz. One number, three places.
2. **A test ties the file to the command**: 1,800 s is 36,000 ticks, which is what the documented run passes.
3. **Nothing is truncated**: the match is decided at **1,527.6 s** (seed 7), so a 1,800 s limit ends a finished match
   rather than a live one - and the headless output before and after the change is identical line for line in its
   refusals.


## D134 - Every click is accepted: a walk order's blocked target becomes the nearest legal standing point

*2026-11-06. The owner: "clicking into the middle of the forest ends up with error message. It has to be allowed. Scout
cavalry will simply try to close to the point as close as it can without running into trees."*

1. **A walk order is never refused for the ground.** A target on blocked ground - the middle of a wood, a wall, a building's
   footprint - is a request to come as close to it as the ground allows. `Move`, `AttackMove`, `Patrol` and `WalkTo` are
   accepted and walked, and so is **any target no route reaches** (a courtyard behind a wall): same request, same answer.
2. **The order's own goal becomes the nearest legal standing point** (the owner: *"nearest legal standing point"*), decided
   **once, when the order is given** - `World::walk_goal`, used by `check_order` and `start_walk` - so no later re-plan,
   crowd slot, stall check or repath aims at a tile nobody can occupy. That is what removed the second half of the bug: the
   first version only fixed the plan, and the mover then re-planned every tick at the clicked tree.
3. **Walk around, not straight in** (the owner: *"walk around"*): the route is planned to the standing point by the normal
   planners, so the unit follows the ground instead of pressing at the wood's face.
4. **`nearest_reachable`** (`path.rs`) is the search: best-first over free tiles ordered by distance to the click alone, so
   the first tile off the queue is the reachable tile closest to it; ties break by `(y, x)`, never by iteration order. It is
   bounded by the map and runs once per order.
5. **Gather, build and attack orders keep their refusals**, and the suite says so: `a_node_with_no_path_is_refused_at_order_time`
   and `an_unreachable_site_is_refused_before_the_wood_is_spent` both fail if the snap is put in the shared planner - which is
   how the first attempt was built, and the suite caught it in one run. A tree behind a wall is not "walk to the fence", and
   a build the builder cannot reach must not spend the wood.
6. **The bug the test caught first was mine**: the first version of the search returned the tile the unit already stood on -
   it is free by definition - so every order resolved to "stay where you are". The test failed with *"it never left its spawn
   tile"*, and the search now excludes its own start unless nothing else is reachable.

8. **It arrived with a 17x cost, and D135 is what removed it.** The match this rule produces ran **3.5 ms a tick against
   0.3** from tick ~10,000. The cause was not the click: a **failed** ground re-plan cost one search every tick (D135), and
   the new trajectory walked into that state. With D135 landed the same match is **11.8 s** again, at a hash D134 fixed.

## D135 - A re-plan that fails spends one of the route's two attempts

*2026-11-06. Found by chasing the 17x tick that D134's acceptance match ran into: 3.5 ms a tick instead of 0.3 with the same
31 units, tasks, path lengths and plan count.*

1. **A failed ground re-plan is counted.** D127 caps ground re-plans at two per route - the count is `Route::stalls`, and it
   was incremented **only on success**. Ground that answers "no" therefore got asked again on the next tick, and the next:
   the cap never engaged on the case it existed for.
2. **A unit with no route entry is the case that looped.** `stalls` lives on the route, so a unit with a goal it cannot reach
   and **no** route (`get_mut` → `None`) counted nothing at all. A failed re-plan now creates the entry when it is missing.
3. **Measured**, acceptance match, 36,000 ticks: re-plan attempts **11,260 → 192**, failures **11,070 → 4**, time spent
   re-planning **35.4 s → 13.3 ms**, whole match **47.5 s → 11.8 s**.
4. **The world state does not move**: the hash is `f328ebc8d4d34b78` before and after the fix, 141 entities, 187 made_room.
   The change removes searches that changed nothing - which is why it is a performance fix and not a behaviour one.
5. **The test states the budget, not the timing**: `crates/sim/tests/repath_budget.rs` builds the case (a wall across the
   world and a walled goal) and asserts **at most two failed re-plans in 600 ticks**. Watched failing first: **177**.
6. **A cost that is not a function of the entity count is a defect.** The count of bodies could not have predicted this;
   what predicted it was per-tick time split from the AI's time, then the repath counter. That pair of instruments is worth
   keeping in mind for the next "why is this match slow".


## D137 - A name content writes must exist in the pack's own tables

*2026-11-06. The second review's findings against `economy.rs:1010/1065/1083`, checked (the triage of 2026-11-05 verified
14 of 205; the loader block of 41 came next).*

1. **The loader already refused four of these**: a `build.farm`, a `build.spaced_roles` entry, `build.farm_spacing` and an
   `age.order` name that are not in the pack. My first draft of the review said it checked **none** of them; that was wrong
   and is corrected in the review and the SPEC row.
2. **Six places accepted the same mistake**, where a typo is a silent no-op at runtime: `build.requires` (on **both** sides -
   the building and the building it needs), `build.blocking_roles`, `footprint` keys, the four `hunt` lists,
   `age.requires`' keys and its `of`/`single`.
3. **The refusal names the key and the name**: *"`\"sheeep\"` is not a name in this pack's [role] table"* under
   `hunt.herdable_roles`. "Invalid" is what the typo already was.
4. **`[metrics] military_roles` stays lenient, deliberately.** Its own comment says a role the pack cannot name is skipped
   because a metric is a measurement, not a rule of the game - so the review's request to check it is **refused**, with the
   comment as the evidence.
5. **Seven tests, watched failing first** (30 passed / 7 failed before the code): one per place, each swapping one real name
   in `config/economy.toml` for one the pack does not have, and each asserting the refusal names the key and the name.
6. **The shipped pack is clean**: a cross-check of every reference in `config/economy.toml` against its own `[role]` and
   `[tech_role]` tables found **0 mismatches** before the check was written, so this change refuses nothing today and the
   acceptance hash does not move.


## D138 - One picker, and it asks nothing: the script's gather action follows `nodes_of_slot`

*2026-11-06. The owner, on the reachability question: **"B"** - no reachability test in the picker; the order's own plan
is the check, and a failure is remembered.*

1. **There were two pickers.** `World::nodes_of_slot` (the economy pass's) already had the rule that was measured in
   2026-10-29: distance order, **no reachability test** - `can_reach` is a full path search, and asking it of six nodes once
   a second per player cost **37x the rest of the run** (400 game-seconds: 319 ms -> 11.9 s) - with failures remembered in
   `share_failed` and retried after `share_retry_ticks` (600 ticks, 30 s). The **script's** `nearest_resource` kept its own
   copy that did the expensive thing: a `can_reach` per node of the resource, on **every** call.
2. **And it asked the wrong question.** It tested `can_reach(node.position)`, and a tree's own tile is blocked ground, so
   the answer was structurally pessimistic - `world/gather.rs` carries the note from the first version, which sent every
   lumberjack to Idle that way.
3. **So the script now asks the engine**: `nearest_resource` is `world.nodes_of_slot(here, slot, nth + 1)` and takes the
   nth. One picker, one rule, and the expensive-but-wrong test is gone rather than duplicated. The acceptance map has
   **90** wood nodes, so a gather rule used to cost up to 90 searches per firing and now spends none.
4. **`start_gather` and `validate` are the authorities**, and they are the only places a search is spent. A refusal is
   **remembered**: `start_gather` records `share_failed` when its plan fails, and `submit_checked` records it when
   `validate` refuses a `Command::Gather` with `NoPath` - the second is the one that matters for scripts, because a
   walled-off node is refused at **order** time (`check_order` is `&self` and cannot book anything), and without that entry
   a rule took the same walled tree every cooldown: **measured, 29 refusals in 30 s**.
5. **The test** (`crates/sim/tests/gather_pick.rs`) is a walled-off near wood, a reachable far one, and a gather rule, and
   it asserts three things at once: the picker **takes the walled node** (so it is not pre-filtering), the far block is
   **worked** (so a refusal is not a dead end), and the refusals stay **bounded by the walled nodes, not by the firings**
   (so the memory works). RED, watched: *"it refused nothing, so it pre-filtered"*, then - with the picker changed and no
   bookkeeping - *"the rule was refused 29 times in 30 s for nine walled nodes"*.
6. **The acceptance match does not move**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 574 refusal lines -
   identical, because `config/acceptance.toml` carries no gather rule at all, so this change is for the packs that do.


## D139 - The second review's sim block: seven fixes, and what the block actually was

*2026-11-07. The 63 findings filed against `crates/sim/src/`. Read against the current tree; seven fixed here.*

1. **The block is not the engine core.** 19 findings against `script.rs`, 14 `bin/batch.rs`, 12 `bin/headless.rs`, 9
   `metrics.rs`, 9 `path.rs` - the script layer, the two tools, the metrics watcher and the path module. `lib.rs`, where
   `step`, the mover and the economy live, was not in it.
2. **A class-level correction, third time tonight:** `f64 as i64`/`as u32` **saturate** (Rust 1.45+). Of the five findings
   built on casts, three are wrong about the mechanism ("wrap", "panic in debug", "`Infinity as i64` is `i64::MIN`"), and one
   is right for the opposite reason - the range check never fired because the cast *succeeded*, clamped.
3. **Fixed (7), each with a test watched failing first except the binary rename:**
   - `path.rs:127` - `line_clear` refused a line whose **start** tile is occupied, while `find_path` allows a blocked start
     on purpose; every unit on a node paid a full A* for a line it could see.
   - `path.rs:127` - the same closure tested containment *and* `blocked()`, which already answers `true` off-map: a second
     bounds check per tile.
   - `path.rs:33` - `Walk::from_map` now asserts `terrain.len() == width * height` in debug builds; `Map` is public and
     fixtures build it directly.
   - `script.rs:1162` - `has_unit` matched one unit id, so `has_unit = "militia"` went false at the man-at-arms upgrade.
     `is_family` now.
   - `script.rs:583` - an attack step naming `militia` refused with *"no unit militia"* after the upgrade. `is_family` now.
   - `script.rs:271/287/307` - `Runner::new` accepted `tick_rate_hz = 0`, and accepted `at_seconds = 1e300` as 9.2e18 ticks
     through the saturating cast, making its own "does not fit in a tick" message unreachable. Both refuse now.
   - `bin/batch.rs` - the binary was documented as **`ruage-batch`** in its own USAGE, in `SPEC.md`, in `next-step.md` and
     in `config/acceptance.toml`, and cargo built it as **`batch`**: every documented invocation named a file that did not
     exist. The file is renamed `src/bin/ruage-batch.rs`, and a live `--seeds 7` run prints `#f328ebc8d4d34b78`.
4. **Refuted (6)**: saturating occupant counters (deliberate, documented); `octile`'s `as u32` (max ~3,610 for u16 tiles);
   `index`'s `None` (it *is* the bounds guard); `metrics`' multiply (seven orders inside `i64`); `nth_villager` exact id (one
   villager role, no upgrade in 5.7); and the two cast-mechanism claims above.
5. **Deferred (26 with reasons, 24 cheap and queued).** The queue's head, and the reason it is the head:
   - `script.rs:1533/1541` - **the enemy selectors can aim at a Gaia town centre or a foundation**; `enemy_town_center` does
     not filter `player != 0` and neither filters `construction.is_none()`.
   - `script.rs:1103` - a rule condition naming a role or resource the pack lacks makes the rule silently **never fire**,
     with nothing in `rule_refusals` - the D137 defect, one layer up.
   - `batch.rs:192`, `headless.rs:181` - a `duration_seconds` of 1e300 is a 9.2e18-iteration loop; `tick_at` applies.
   - `headless.rs:300` - `players=` prints the highest player id, not the count.
6. **Acceptance unmoved**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 574 refusals, **11.47 / 11.51 s**
   over two idle runs. The 16.97 s figure I reported earlier was a run whose compile overlapped the suite; withdrawn.


## D140 - The binary names its own modules, so the map is read from the file we own

*2026-11-07. Step 1 of four adopted from the prior-art watch thread (`docs/reviews/2026-11-07-the-binary-names-itself.md`).*

1. **What was adopted is a method, not a fact.** The thread reported that `AoK HD.exe` carries 78 source paths and 938
   RTTI class names. Verified here with `tools/exe_map.py`: **84** source paths, **541** classes (from 1,541 descriptors -
   a PE repeats them), 4 structs, and **31** `Comm*` classes. The digest's counts used a different string minimum.
2. **The build root is provenance in one line**: `c:\users\jakes_workstation\documents\projects\age_of_empires_2_de\
   phoenix\age2hd_archive_5.7_with_hotfix` - HD 5.7 was cut from the DE tree, which `SPEC.md` asserted from one string and
   now has 84 paths behind it.
3. **The module map confirms U7's shape from a second source**: `path.cpp` **and** `pathsys.cpp` separately, `motion.cpp`,
   `move_obj.cpp`, `act_gath.cpp`, four `tact_*.cpp` - the same six-pather picture that `docs/movement-in-the-binary.md`
   built from the decompiled corpus, now rebuildable without the corpus.
4. **The new part is the class taxonomy.** 31 `Comm*` classes name a turn-synchronised command stream:
   `CommPacketSyncTurn`, `CommPacketSyncCmdCount`, `CommPacketPlayerCommand`, `CommPacketChecksum`, plus speed change,
   resign, ping/pong and the pre-game state packets. U12's contract has a shape now: **per turn, a command batch, a
   count, a checksum**.
5. **The rate is still open, and the arithmetic says why.** 31,104 `SYNC` ops over 3,066.04 s is **10.15/s**, one every
   ~98.6 ms - every *second* turn at 20 Hz, and equally consistent with a 10 Hz sim, which the dat's 50 ms authoring
   granularity rules out. Recorded as a hypothesis in `docs/oracle.md`; not a settled rate.
6. **Nothing is copied.** A class name and a build path are facts about a file this project owns. genie is AGPL-3.0 and
   the UGC guide GPL-3.0; neither was read for this step, and the thread's own "NEVER COPY" rule is the reason.
7. **The tool has one rule per lesson already paid for**: a missing binary is an `exit 2`, never an empty map (D130); and a
   family is a word, not a prefix, because `of_family("Comm")` counted `CommandLineHandler` as a netplay packet until it
   required a capital after the prefix.


## D141 - The gather radius bound, adopted and then refused by measurement

*2026-11-07. Step 2 of four from the prior-art watch: genie's gather search is a BFS bounded by `LOS + 5`, with at most 30
objects reach-tested. Adopted as proposed, implemented, measured, and **reverted** - the numbers are the reason.*

1. **What the original does**: the gather search explores **standable tiles** out to the worker's line of sight plus five,
   and reach-tests at most 30 objects. The digest called the radius "a fidelity rule and cheap".
2. **The cap is moot for us** and always was: D138 removed the reachability test from the picker, so we spend **one**
   search per attempt inside `start_gather`, not thirty. The radius is the whole of the fidelity question.
3. **Implemented**: `World::gather_reach(worker)` = `(los + 5)` tiles in raw units (a villager's `los` is **4**, so
   **9** tiles), and `nodes_of_slot` refused candidates beyond it. Test first, watched failing: *"the only wood is ~18
   tiles away and the villager sees nine: the rule must say there is no node"* - it said nothing and sent him across the
   map, which was the defect.
4. **Measured, and it breaks the match**: 36,000 ticks, seed 7.

   | | before | bound on |
   |---|---|---|
   | entities at 1,800 s | **141** | **129** |
   | players alive | **1** | **2** |
   | repaths / made-room | 188 / 187 | 83 / 164 |
   | refusal lines | 574 | **767** |
   | hash | `f328ebc8d4d34b78` | `66032535f7d24483` |
   | wall | 11.5 s | 7.4 s (fewer units, less to do) |

   The match **stops resolving**: neither side eliminates the other by 30 minutes, because the economy idles once the wood
   within nine tiles is gone.
5. **Why, and what the original actually pairs with it.** The bound is not wrong; it is the wrong half. Theirs is a BFS
   **from where the worker stands**, in a game where the AI keeps re-tasking idle villagers and where a player's click is
   itself the long-range decision. Our `nodes_of_slot` is used for **both** jobs: it is the AI's "what should this villager
   do" *and* the unit's "which node, exactly". Bounding it applies a short-range rule to a long-range decision.
6. **Reverted** (`git checkout`), so the tree is back at `f328ebc8d4d34b78`. The corrected shape - a **two-stage** search,
   the AI choosing an area and the unit choosing a node within sight of it - is queued, not guessed at.
7. **What this costs the "cheap" claim**: the digest's own sentence says "the cap is a fidelity rule *and* the performance
   bound; adopting the bound and the cap is cheap". The cap is free; the bound is not, and the only way to know was to
   measure it.


## D142 - A shot is measured to the wall, not the middle: reach is box to box

*2026-11-07. Step 3 of four from the prior-art watch (openaok's divergence note, 2026-10-29), adopted.*

1. **What the original does**: attack reach is compared **box to box** - a unit is its dat size box, a building its
   footprint - and the primary report's caveat ("the missing datum is a building footprint, which our schema has not got")
   **expired at D76**, since every building has a `[footprint.*]` mask and a dat-derived extent.
2. **What we did**: `squared <= range * range` on **centre to centre**. For a unit target that is the same thing to within
   0.4 tiles; for a building it put the wall **two tiles** inside the reach, so a skirmisher could hit a town centre from
   two tiles outside its own range.
3. **The change**: `World::gap_squared_to(from, radius, target)` - a unit's box is a square of its `radius` about its
   centre, a building's is the rectangle `footprint_area` covers (anchored at a **corner**, so not centred on its
   position), and the attacker's radius shrinks the target box, which is the same arithmetic as growing the attacker to a
   point. **Both** sites that ask the question use it: the tick's blow and the route-ending test (E1b) - a rule and the
   engine disagreeing about reach is this project's most repeated defect class.
4. **`World::in_strike_range(attacker, target)`** is public so the rule has a test without a played-out match, and
   `unit_range` was extracted beside it. The box test shipped first answering the **old** way, so the RED was a boolean, not
   a compile error: *"the range is 4.0 tiles; the skirmisher stands 6.3 from the middle and 1.8 from the wall"*.
5. **A second test pins what must not move**: two units 3.2 tiles apart are inside a 4.0 range, two at 5.0 are not.
6. **One existing test was re-stated, and said so**: `fletching_adds_a_tile_of_reach` asserted the archer's exact tile (11
   then 10), a proxy for "Fletching is worth a tile of closing distance"; with the 0.4-tile box margin the archer stops a
   fraction earlier and both runs floor to 10. It now asserts the **distance moved** (>= half a tile without, < half with).
7. **The acceptance match is byte-identical** - `f328ebc8d4d34b78`, 141 entities, 188 repaths, 574 refusals, 10.82/10.83 s -
   and that is stated as what it is: this scenario does not probe the difference (its ranged units shoot at units, whose box
   is their own radius), not as evidence that the change is inert.


## D143 - The path search keeps two flat arrays, not two trees

*2026-11-07. Step 4 of four from the prior-art watch. Of its four pathing techniques, one applied here and three would have
been decisions smuggled in as optimisations.*

1. **Taken: flat arrays.** `find_path_avoiding` allocated a `BTreeMap` for the best cost and another for the parent of every
   visited tile, plus a comparison per lookup. It now allocates **two `Vec<u32>`** indexed `y * width + x`, the second
   holding the parent's *index* rather than its coordinates. `open` - and therefore the tie-break order - is untouched.
2. **Refused in this step, with reasons**: a **landmark (ALT) heuristic** is a per-map precomputation whose payoff is on long
   searches these maps barely have; **budgeting by route length** and a **per-tick search budget** both truncate searches,
   which means a route that existed sometimes does not - a behaviour change and a decision, exactly the shape of D141, where
   the last "cheap" bound cost twelve entities. **Re-sorting last tick's order** does not apply: this engine does not sort a
   per-tick order.
3. **Measured with a new instrument**: `crates/sim/examples/path_cost.rs` times 2,000 searches at three lengths and prints an
   **order-sensitive checksum of every route**, so "faster" and "different" cannot be confused.
   - corner to corner: **50.5 -> 8.9 us** (5.7x)
   - mid (~10 tiles): **5.2 -> 2.0 us** (2.6x)
   - short (~3 tiles): **1.2 -> 0.9 us** (1.3x)
   - all three checksums **identical** before and after.
4. **Priced honestly at match scale**: the acceptance match is **unchanged** (`f328ebc8d4d34b78`, 141 entities, 188
   repaths, 11.50/11.48 s) because 188 searches at 50 us is ~0.04 s of an 11.5 s run. The win is for the thousands-of-units
   target, not for this scenario, and the review says so rather than quoting 5.7x as a match speed-up.
5. **Memory also improves**: 32 KB of arrays per search on a 64x64 map against ~4,000 tree nodes (~300 KB) on a
   corner-to-corner search - bounded by the map instead of proportional to the work.


## D144 - Four more subjects out of `lib.rs`: seeding, training, construction, research

*2026-11-07. The owner: "Please split lib.rs into smaller files. It is still around 10000 lines." Stage 1 of D136 continues.*

1. **Moved, verbatim, by brace matching**: `world/seed.rs` 475 (`seed_from`, `insert`), `world/train.rs` 368 (`check_train`,
   `start_train`, `spawn_from_building`, `free_spot_near`), `world/build.rs` 284 (`settle_blind_sites`, `cancel_overbuilt`,
   `cancel_site`, `refund_unit_cost(_part)`, `work_one_tick`, `check_build`), `world/research.rs` 411 (`check_research`,
   `start_research`, `modifiers_for`, `adjusted`, `max_hp`, `effective_speed`, `finish_research`, `grow_plots`,
   `apply_tech`, `upgrade_units`). `lib.rs` **10,216 -> 8,731**.
2. **The proof is the acceptance hash**, checked after every move rather than once at the end: `f328ebc8d4d34b78`, 141
   entities, 188 repaths, 187 made-room, identical all four times. A pure move cannot change behaviour, so a hash that
   moved would mean the slice moved something.
3. **The compiler wrote the visibility list**, as in D136: fourteen methods and four free functions (`collect_modifiers`,
   `add_to_class`, `multiply_class`, `scaled_health`) became `pub(crate)`, and `TrainPlan` with them. `pub(crate)` does not
   change the crate's public API.
4. **The seam is checked by hand at every cut**, because no compiler catches an orphaned doc comment: each of these four
   blocks began at a doc comment, so the preceding method kept its own.
5. **One commit for the four slices, and why**: the cuts were made in one working tree, so the `lib.rs` diff holds all four
   removals and cannot be staged separately. Each move's hash was verified as it landed, and the gate (728 passed, clippy and
   fmt clean) covers the final tree.
6. **What is left is a different job**: `step` is **1,562 lines** in one function, and no file split touches it - that is
   D136's stage 2, named phases. The remaining stage-1 candidates, in order: `orders` (`apply` 223 + `validate` 101),
   `state` (`state_hash` 343 + `stamp_*` + `bookkeeping_*`), `combat` (the strike/volley/aim helpers), `sites`
   (`site_*`, `footprint_area`, `solid_tiles`, `nearest_drop_site`).
7. **The cost, stated**: about 1,500 lines of `git blame` history now point at a move commit. The mitigation is the one that
   keeps the code searchable - every doc comment moved verbatim, so the same sentences are one `grep` away.


## D145 - `lib.rs` becomes what the crate *is*; sixteen files say what it *does*

*2026-11-07. The owner: "Go ahead if it is still more than 5000 lines file." 8,731 -> **4,106**.*

1. **Twelve more moves**, all mechanical: `world/step.rs` 1,581 (`step` as a whole function), `world/battle.rs` 665,
   `error.rs` 475 (`SimError` + `Display`), `world/sites.rs` 461, `world/state.rs` 421, `world/orders.rs` 373,
   `world/shares.rs` 220, `world/report.rs` 221, `world/livestock.rs` 182, `world/garrison.rs` 108 - with D136's three and
   D144's four making sixteen subject files.
2. **`lib.rs` keeps the crate's nouns**: entity and command types, `World`, constants, free functions (FNV, PCG32,
   `step_towards`, `reaches`, `max_scale_for`), re-exports. The verbs moved out.
3. **`step` moved whole, on purpose.** It is 1,566 lines in one function and moving the file does not make it shorter. The
   point of the move is that `lib.rs` is now navigable and the tick is in one place to be cut into phases - that cut is
   stage 2 and is not mechanical.
4. **A move-tool defect found and fixed**: the extraction walks back over comments but not over `#[...]` attributes, so
   `#[must_use]` was left behind while its method left the file - four methods lost it (`in_strike_range`, `card`,
   `site_fits`, `site_has_lane`). The audit is now mechanical: all **42** must-use methods from `HEAD` checked against the
   new layout, 46 attributes present across `src/`. The skill's recipe carries the rule.
5. **Public API is unchanged**, and the near-miss is recorded: a text promotion appended `pub(crate) ` to signatures that
   were already `pub` (`max_scale_for`, `MAX_PLAYERS`), producing `pub pub(crate)`. Rule: promote only a signature that
   starts with four spaces and `fn `.
6. **The proof is the hash per move**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, identical after each of
   the twelve. 728 passed / 0 failed / 19 ignored; clippy and fmt clean.


## D146 - Stage 2: `step` becomes a list of named phases

*2026-11-07. The owner: "Proceed", on the note that stage 2 was the next job. `step` was 1,562 lines; it is 30.*

1. **`decide()` is the only pass that touches the entity list.** It changes nothing on `self`, fills a `TickWork` of
   nineteen vectors, and returns it. Everything after it is a phase that takes what it needs.
2. **`TickWork` is per-tick state, deliberately not hashed and not carried.** It is created and dropped inside `step`, so it
   cannot become a second source of truth; if a future phase needs to remember something across ticks, it goes on `World`
   and into `state_hash`, not into `TickWork`.
3. **Nothing was re-ordered.** The call sequence is the old statement sequence. A restructure that re-ordered phases would
   still hash identically in an idle world and differ in a busy one, which would make the hash proof worthless.
4. **The measured effect is a speed-up, and the cause is not proven**: 11.49-11.58 s before, 10.69-11.03 s after (five runs,
   36,000 ticks), non-overlapping ranges. Likely the short vector lifetimes and small stack frames; stated as inference.
5. **Extraction rules that held**: a block is only extractable if its only inputs are `self` and top-level `Vec`s; each such
   vector is passed **by value** into its phase, `&mut` only where the phase appends to one it does not own (`blows`,
   `shots`, `ejected`); a phase that uses `?` returns `Result`.
6. **What is left is `decide` at 432 lines** - one `for` with a `match` on `Task`, long because the game has that many verbs.
   Cutting it per task would make each arm return a delta and push into `work`: a change of shape for a modest gain, and not
   attempted here.


## D147 - A scenario's length is a length, and half a research table is an error

*2026-11-07. Second review [74] [173] [184] [112], the two deferred items with a named remedy. Both were engine defects, so
both were fixed rather than asked about.*

1. **`duration_seconds` was cast, and `f64 as i64` saturates**: `nan` became **0 ticks** (a match that plays nothing, exits
   0, reports success) and `1e300` became `i64::MAX` (a match that never ends). The `map_err` guard beside the cast was dead
   code - the same shape D139 fixed for `at_seconds`.
2. **One definition of "how long is this match"**: `Scenario::ticks(tick_rate_hz)` (which is `tick_at`) is used by both
   runners and by `Runner::new`, which stores it and exposes `Runner::duration_ticks()`. The loader refuses a
   `duration_seconds` that is not finite or is negative; the product is bounded where the rate is known.
3. **Half a research table is an error.** `if let (Some(techs), Some(effects))` skipped research entirely when one flag was
   given, so a typo changed the match silently. Now both-or-neither, and the refusal names the flag that is missing.
4. **Not changed, with reasons recorded**: [70] the `--dump` CSV has no escaping, but no value the `task` column can produce
   today contains a comma, so no test can be watched failing and a defensive quoting would be untested code; [71] `players=`
   reports the highest id rather than the count, which is unobservable while seeding gives players `1..=n` - it needs a
   sparse-player or mass-death fixture; [177] is stale (fixed in D139).
5. **A process loss, recorded**: the review's 205-finding JSON lived in a pruned cache and is gone. The 41 loader and 63 sim
   findings were written into `docs/reviews/` before it expired and survive; the 49 tests / 26 config / 16 other findings
   were never written out and **cannot be recovered verbatim**. Findings get committed to `docs/reviews/` as they are
   triaged.


## D148 - A worker only goes where it has looked

*2026-11-07. The owner: "villagers go across the map to gather berries even when they were not found before… Villagers are
supposed to look for resources which are nearby when their current supply is depleted. If nothing is found, they should be
idle."*

1. **The picker asked the world, not the player.** `nodes_of_slot` (the script's gather action and the share pass) and
   `nearest_like` (the "my node ran dry" path) both ranked **every node in the world** by distance. A berry patch in
   unexplored fog was therefore a legitimate target, which is what the owner watched.
2. **D141 was the wrong axis.** A radius (`los + 5`) was tried and reverted by measurement (141 -> 129 entities, one player
   alive). The engine already keeps a per-player fog and a **resource node never moves**, so "found" is exactly "its tile
   has been explored" - a fact the picker ignored.
3. **`World::knows(player, entity)`**: one predicate on the fog (`explored_bit`), observer (0) sees everything. Used at every
   automatic pick; `nodes_of_slot` takes the player as a parameter because the question is *whose* knowledge.
4. **Scripted steps are not exempt.** A scenario `gather` step goes through the same picker, so it too must name a node the
   player has scouted; `a_scenario_researches_a_tech_named_by_a_role` was rewritten to walk a unit over the gold first
   (player 1's gold is 9.4 tiles out, just past the starting sight) rather than given a bypass.
5. **The acceptance hash changed on purpose, from `f328ebc8d4d34b78` to `dfc835be5d880a47`**, and what moved is known
   exactly: counts identical (141 entities, 1 player, 188 repaths, 187 made-room), and a `--dump` diff shows **one line** -
   node 117 keeping 90 raw units (0.09 of a resource) that the old picker spent on an unseen node. Every earlier slice was
   proved by an unmoved hash; this one is proved by explaining the diff.
6. **"Nearby" is still unbounded.** Among *known* nodes the nearest is taken however far. The radius is the second half of
   the owner's rule and a gameplay number, so it is his: (A) none beyond knowledge, today's behaviour; (B) the worker's
   line of sight, 4 tiles; (C) `los + 5`, 9 tiles. Each is measurable with the same acceptance run, and B is the one D141
   is a warning about.


## D149 - "Nearby" is bounded by the worker's sight, and the share table measures from the base

*2026-11-07. The owner chose **C - line of sight + 5** when asked how far a depleted worker should look. Measured, that
number breaks the acceptance match; the shipped constant is sight + 8.*

1. **`World::gather_reach(worker)`** = the worker's own line of sight (whole tiles, the dat's `los`) plus a look-around,
   scaled once. `sight_of` answers in **whole tiles** - adding a raw offset to it gives a five-tile reach and looks like a
   rule that refuses everything, which is how the first version failed its own control test.
2. **The owner's +5 is the failure point, measured**: reach 9 leaves **129 entities, both players alive at 30 minutes, 83
   repaths** (from 141 / 1 / 188). A 12-tile reach keeps the match resolving. This is D141's collapse reproduced **with**
   the fog filter, which is what makes it a statement about the radius.
3. **Shipped: sight + 8 = 12 tiles.** The smallest bound measured that keeps the acceptance match resolving, and still
   local. It is not the owner's number, and the difference is stated rather than glossed.
4. **The share table is a question about the base, not about the spot a villager stands on.** With the reach applied from
   the worker, `the_share_row_follows_the_villager_count` failed - five villagers spawned at (9.4, 32.4) had no wood within
   reach and "eight on wood" never fired. The pass now anchors on the player's town centre (falling back to the worker).
   Consequence, measured: the acceptance run goes 141 -> **152 entities**, repaths 186, made-room 159, hash
   `82a5063dcca63108`.
5. **The reach applies where a worker chooses for itself**: the "my node ran dry" path (`nearest_like`) and the script's
   gather action. A *scripted* step and a *share* row are orders, and are bounded only by the fog (D148) - the reach is
   about what a worker notices, not about what a player may ask for.
6. **New acceptance baseline `82a5063dcca63108`** (141 entities -> 152, both counts measured, match resolves, one player
   alive).
