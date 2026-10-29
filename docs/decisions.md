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
- the ceiling is `min(cap_default, houses x house_population + town_centres x town_centre_population)`,
  counted from **finished** buildings only: a house supports its five when it is finished, not when the
  site is marked out;
- a train order past the ceiling is refused with `PopulationCapped`, before any price is quoted.

`town_centre_population = 5` is new content, and it is published fact rather than invention: a house and a
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
living building of the family content names in `[start] town_centre` has lost - and it is enough to decide a
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
