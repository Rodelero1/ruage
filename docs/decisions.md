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

**Evidence, 2026-11-08 — the dat carries this, and the mask's end is unchecked.** `empires2_x2_p1.dat` gives every unit an `obstruction_type`: 0 for 654 populated records, 2 for 257, 3 for 28, 5 for 383, 10 for 11; among the 235 records with `radius_x >= 1.0` (buildings) it is 0 for 63 — the farm, the fish trap, all four town-centre variants, the wonder, gates and their pieces — and 2 for 161. A building of type 0 does not fill its footprint, and up to four `building_annex` pieces name what does: the Town Centre's are 618 (2×2, type 2) at displacement (1.0, −1.0), 619 (3×3, 0) at (−0.5, 0.5), 620 (2×2, 0) at (−1.0, 1.0) and an empty 890. Reading the displacement as from the centre and `radius` as half-extent, the solid piece covers the **right-hand** columns of a 4×4 whose rows this engine anchors at the top-left tile, where the mask above blocks the left-hand ones; which pair of rows depends on the dat's y sign. One test should settle which end is solid before the importer learns to generate the mask — generation is the direction, because it is right for all 63.

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


## D150 - +5 was never the problem: the share table's anchor was

*2026-11-07. The owner's reply to D149's measurement: "+5 doesn't work while +8 does - either (1) an issue when the villager
started to look for another resource, or (2) a map issue (forest near town centre depleted). If 1, then the villager has to
return to where it gathered last time and only then look for another tree. If 2, the villager was right to become idle."*

1. **Instrumented, not argued.** Every failed pick printed its worker, position, column and the distance to the nearest
   **known** node of that column. At +5 the acceptance run had **586 failures, all of them gold (slot 2)**, with the nearest
   known gold at 9.2-18.5 tiles. The map puts player 1's gold at (24,11)-(25,12) against a town centre at (16,16): **9.4
   tiles**, so a nine-tile reach misses the base's own gold by 0.2 tiles.
2. **Hypothesis (2) is false**: wood and berries sit 4-6 tiles from the town centre and **not one wood or food pick failed**.
   Nothing near the base was exhausted. What is true in it is the conclusion - the villager *was* right to go idle.
3. **Hypothesis (1) is false where it points, and true one step sideways.** The depletion path already chooses while the
   worker stands at the node: the owner's rule was implemented explicitly (choose at the node, carry it through the deposit in
   `pending_gather`) and the test passed **with and without** the patch, so the patch was **reverted** - unprovable code does
   not ship, and the finding is recorded instead. The faulty anchor was the **share table** (D149.4): measured from the
   villager, a worker standing away from its base found nothing, and gold was never assigned.
4. **With the anchor fixed, +5 works**: the acceptance match resolves at reach 9 - **135 entities, one player alive, 125
   repaths, 116 made-room, hash `45596da62c9e3668`**, three consecutive runs identical. The shipped constant is the owner's
   own **5**, not 8.
5. **`nodes_of_slot(worker, slot, limit, reach: Option<i64>)`**: the reach is `Some` when the **engine** chooses (a rule, a
   share row, a retask) and `None` when a **script points at a resource**. An order is not a guess about what a villager can
   notice - which is D149.5's own distinction, and applying the reach to scripted steps broke two existing scenarios
   (`the_ten_minute_scenario_builds_trains_and_fights`, `the_skirmish_only_refuses_what_the_fight_has_taken`, both "player 1
   has no food node number 0 to gather").
6. **New acceptance baseline `45596da62c9e3668`** (135 entities, 125 repaths, 116 made-room).


## D151 - The AI leaves the binary: nine choices enumerated, placement first

*2026-11-07. The owner: "By extract AI I mean removing all the AI controls from the binary and moving it exclusively into ai
files. Do this first." And: "every unit control should be removed from game code and moved to AI." Answers given to the ten
questions: translation of the game's `.per` files is allowed; the default/base AI is the target; the share table follows the
original; disabled rules are extracted too; **pass condition is a working economy and a filling military queue, not victory**.*

1. **The line is choice vs mechanism.** Simulation stays in Rust (movement, collision, pathfinding, damage, harvest rates,
   production timers, vision, death, rot, validation, geometry). A **choice made for a player** — which node, which tile,
   which unit, whom to fight, when to redistribute villagers — moves to `config/ai/*.toml`.
2. **The inventory is `docs/ai-extraction.md`**: nine choices currently in Rust, each with its file and line, plus the rows
   that must **not** move (`site_fits`/`site_refusal` validation, `approach_tile` geometry, `settle_blind_sites` bookkeeping,
   `step_leashes`, `step_conversion`, order fulfilment).
3. **`nearest_like` stays in the engine**: the original's engine also continues a villager on a nearby resource; D148 (only
   looked-at nodes) and D150 (the look-around) already made it a rule rather than a cheat.
4. **`step_shares` is half**: the *table* is AI (and already lives in the personality file), the *assignment* is the engine -
   exactly the split the original uses, where the scripts set `sn-*-gatherer-percentage` and the engine moves the villagers.
5. **Order: placement first** (`nearest_free_farm` and the `where = "<resource>"` resolution), because it is the one the owner
   named and the gold loop needs it; then the gold loop itself (scout → sighting → camp tile → villagers); then the combat
   choices one at a time (D88's blow order makes them entangled); then `resume_idle` and `grow_plots`; `nearest_drop_site`
   last, because every economy test depends on it.
6. **Engine gaps to fill, test-first**: a **build-at-a-tile** order (today `where` resolves a resource name to a tile inside
   the engine), and a **sighting** query the AI can read ("where have I seen gold?"), on the per-player memory that already
   exists.
7. **Fidelity is three simple checks**, since their game cannot be run beside ours: every rule cites the file and values it
   came from; our match's counts are compared against their script's own thresholds (28 villagers before Feudal, 31 -> 40
   through Feudal, market + blacksmith for Castle with two builders); and the same seed reproduces the same match.
8. **Pass condition**: a working economy and a military training queue that fills and empties. Victory is not a condition.


## D152 - Priority order: functionality, UI, graphics, AI, music - and what "functionality" is missing

*2026-11-07. The owner: "Q9, I don't care about comparison and AI now. That would be addressed later. Pathfinding and
general functionality matter for now. I think best order of priority is: 1 general functionality, 2 ui, 3 graphics, 4 AI,
5 music."*

1. **The order is recorded and governs**: general functionality first, then UI, then graphics, then AI (D151 is a plan that
   waits), then music. No AI work until 1-3 are where he wants them.
2. **Pathfinding is measured, not assumed.** `find_path_avoiding` costs **8.9 us** corner to corner on a 64x64 map (D143,
   two flat `Vec<u32>`), applies the **both-corners** rule for diagonals, refuses a line whose start tile is occupied
   (D139), and the world **smooths** the route by jumping to the furthest node `line_clear_avoiding` reaches (`lib.rs`
   `plan_path`). A short-range pathfinder module (`src/short.rs`) already exists for the local case the original game uses.
   The review's "a sealed map expands the whole grid" is true and costs **~9 us**, not a stall - so it is not the next
   thing to fix, and saying otherwise would be inventing work.
3. **The functionality gaps are the documented ones**, each with its decision: **no gates** (D54 - a wall blocks its owner
   too, because pathing is one grid), **no unit pushing** (D52 - overlap is possible and a unit already sharing a tile is
   never moved aside), **fog without occlusion** (D50 - sight is a disc, not a line through terrain), and **victory rules**
   beyond the seat (which the owner has now explicitly deprioritised: a working economy and a filling military queue are the
   pass condition, not victory).
4. **Order within 1**: **gates first** (a wall the owner can pass is the difference between a wall and an obstacle, and it is
   a pathing question, which is what he named), then **unit pushing** (D52), then **fog occlusion** (D50).
5. **UI and graphics already have a foothold**, so they are not greenfield: `crates/view` has the bar, panel, minimap,
   widgets, layout and a frame buffer (U1-U3, SPEC v1.68-1.70), and there is **no renderer** by choice (D18) - graphics
   begins at sprites (SLP) and the entity+state -> sprite binding that the dat's sprite table can generate.


## D153 - Gates are a global state, not per-player passability (research only)

*2026-11-08. The owner corrected D54's premise: "Gates are more difficult than 'passable for owner and allies, solid for
others'. Enemy unit can pass together with your own. Please do mere research." Research is `docs/gates-research.md`.*

1. **D54's sentence is corrected**: *"Not modelled: gates (owner-passable pathing needs a per-player grid)"* is **wrong**. A
   gate is a **time-varying global state** - open means walkable for everyone, closed means walkable for nobody - so it is
   `Walk::block`/`unblock` on the one writable grid, never a second grid per player.
2. **The game's rule, from the wiki**: a gate opens when a **friendly** unit approaches and *"only closes again if no unit
   stands right under the Gate"*, so *"enemy units can also pass or keep the Gate open on purpose"*; it can be **locked**,
   even while a foundation, and then does not open even for its owner. A player report says the same from the other side: one
   enemy soldier under an open gate lets a whole army through.
3. **0 A.D. implements exactly that shape**: `opened`/`locked` state, a range query restricted to **allies** to trigger
   opening, close only when *no allies in range and no units inside the gate's obstruction*, and **opening flips a global
   `SetDisableBlockMovementPathfinding`** rather than any per-player walkability. Read as a model, not copied (GPL-2+).
4. **The gate is a building in the dat**: class **39**, id 63 `GTAA3`, 4000 hp, LOS 6, 30 stone, 70 s, `dead_unit` 144, **4x1
   tiles**, built inside an existing wall or wall foundation; repair costs **1.5x build** (45 stone); our pack already
   excludes class 39 from the conquest condition.
5. **The one number the data does not give is the trigger radius** - 1, 2, or the gate's LOS of 6. That is a gameplay rule and
   the owner's to set.
6. **The hard part is route invalidation**, and both the game and 0 A.D. dodge it the same way: a gate never closes while
   anything stands in it, so no unit is ever trapped inside a closing gate. That is cheaper than re-planning and is the
   behaviour to copy.


## D154 - Gates: open for anyone, closed for everyone

*2026-11-08. The owner's two numbers: **trigger radius 1**, and **an enemy standing in an open gate keeps it open**. The model
is D153's research.*

1. **A gate is content, not a class**: `config/economy.toml` gains `gate = 64` (the dat's palisade gate `GTAA2`, 1375 hp, 2x1)
   and `[build] gate_roles = ["gate"]`, validated like `blocking_roles` - a name that is not a `[role]` is a load error, so the
   engine still learns no dat class (D17).
2. **`World::step_gates()` runs before the movement pass**, so a gate that opens this tick is walkable for this tick's steps.
   It **opens** when a unit of the owner or of a player on its team stands within **one tile** of any gate tile, and **closes**
   only when **no unit of any side** stands on those tiles - which is what makes an enemy inside keep it open, the owner's
   rule and the game's.
3. **The grid is written only on a transition** (`Walk::unblock`/`block`), and `open_gates` remembers the state, because
   `Walk` counts occupants: an unbalanced pair leaves a tile blocked for ever or free for ever. That invariant is the reason
   the code is shaped around a `GateChange` list rather than writing inline.
4. **Inert where no gate exists**: the acceptance match has none, and its hash is byte-identical (`45596da62c9e3668`, 135
   entities, 125 repaths, 116 made-room). The per-tick cost is one early return.
5. **Not done, deliberately**: the **lock** (a gate that will not open even for its owner), the placement rule *"a gate must
   be built inside a wall"*, diagonal gates and the other three wall types, and clearing `open_gates` when a gate dies (a
   small `BTreeSet` leak, recorded rather than hidden).


## D155 - Walls by drag: one bend, and a queue that finally works

*2026-11-08. The owner: walls go **vertically, horizontally and at 45 degrees**; *first left click starts placement, second click
ends it*; *one corner can be done*. His answers: **C** (the view hands the sim a **list** of tiles), **B** (a blocked segment is
skipped, the rest stands, and the preview shows it red), **A** (the engine picks the bend - 45 degrees first, then straight),
**5A** (sim and tests first).*

1. **`World::wall_line(from, to)`** in `world/lines.rs` is the *only* answer to "where does the wall go": a plain function, no
   world, no state, public so the view draws its preview from the same code the command runs. One bend at most: 45 degrees
   first, then the straight leg; both ends included; a click with no drag is one tile.
2. **`Command::BuildLine { villager, unit, tiles }`** takes the list. A tile that cannot be built is **skipped** and the line
   continues **past** it; the whole command fails only when no tile could be built, with the first refusal.
3. **`Command` is no longer `Copy`** (still `Clone`): a line of a hundred segments is handed over once.
4. **A pre-existing engine defect was found and fixed**: the build queue's advance path popped a site and called `start_build`,
   which re-queues when a queue exists - so with **three or more** sites queued the villager finished one, put the next back at
   the back, and never started another. Split into `start_build_site` (start now, no queue) for the advance path and
   `start_build` (the player's rule) for orders. The regression test is the four-segment drag.
5. **Two of the agent's own errors, recorded**: `submit_checked` queues and only `step` applies (a spawn followed immediately by
   an order is refused `MissingEntity`), and a house is **2x2**, so a six-tile drag past one loses **two** segments, not one.
6. **Not done**: the UI drag itself (the view has no placement path yet), the other three wall types, and editing a line after
   the second click.


## D156 - The drag on screen: two clicks, and a ghost that is the order

*2026-11-08. The owner asked for the UI side of D155.*

1. **`ruage_view::placement::Placement` holds the whole rule with no window in it** - a `winit` loop cannot be driven from a test,
   so the first click *starts* (returns `None`), the second *places* (returns the command), `hover` lengthens, and `segments`
   answers per tile with `valid`. Before the first click **nothing is drawn**, because there is no run.
2. **The ghost is the order**: `tiles()` calls `World::wall_line` - there is no second implementation of "where does the wall go"
   - and the test asserts the command's `tiles` equal the drawn ones exactly.
3. **Red is per segment**: `World::is_free_ground` per tile, drawn with the new `Frame::blend_diamond` (integer `>> 8` blend, so
   two machines draw the same picture) in green `[120,220,140]` or red `[230,80,70]`.
4. **An all-red second click places nothing and keeps the placement up** - a botched drag costs a click, not the placement.
5. **Window wiring**: `B` begins a placement for the first selected thing (provisional until the villager's build card exists, and
   the notice says so); the map's left click belongs to the placement while one is up and never selects; Escape **cancels before
   it opens the menu**.
6. **The drawing test was proved by mutation**, because it passed on arrival: ignoring `valid` and always drawing green makes it
   fail. A drawing test that cannot fail is decoration.
7. **Not done**: the villager build card (placement is key-only), a single-building ghost, and any cost readout while placing.


## D157 - One queue per building: units and techs, one after another

*2026-11-08. The owner: *"When units are trained in a building, there should be a training queue with visible progress. Same for
tech research. Units and tech can be queued in the building, but they are performed one after another not in parallel."*

1. **Training and research were already timed** (`train_time`/`research_time` x tick rate, proven by two existing tests), so
   nothing was made slower. What was wrong was the **queue**: units had one, techs did not (a second research was refused
   `AlreadyResearching`), and a building could **train and research at the same time** - the parallel behaviour this decision
   removes.
2. **`queued` is now `Vec<Job>`** where `Job::Train(Training) | Job::Research { tech }`, so the order is the player's order.
   `start_next_jobs` runs at the end of `step_production` and starts the head of any idle building's queue.
3. **Validation at order time, payment at start time** - the rule D66 set for units, now for techs: a queue cannot spend money a
   player has not got yet.
4. **The same tech twice is still refused** (`TechInProgress`): in the running slot, in this queue, or in another of the player's
   buildings. A busy building is no longer a refusal.
5. **`World::production(building) -> Production`** is the one call the interface makes: the running job with `(done, total)` ticks
   and the waiting items in order. `ui::production_bar` draws it as a 4-pixel bar under the health bar, in `palette::PROGRESS`,
   with `+N` waiting.
6. **The queue's kind is in the state hash**, so unit 83 and tech 83 cannot collide.
7. **The acceptance match changed materially and is recorded, not celebrated**: hash `45596da62c9e3668` -> **`a120ed0721c8768c`**,
   entities 135 -> **150**, repaths 125 -> **88**, made-room 116 -> **100**, and **both players alive at 30 minutes** instead of
   one - because a tech ordered behind units now waits its turn, so research lands later.
8. **An agent bug, found by the test**: removing the old queue advance left the training slot never cleared, so a town centre
   spawned a villager **every tick** (23 villagers where 4 were expected, ids 9904-9923). Fixed by clearing the slot explicitly.
9. **Not done**: cancelling a queued job, a building card for villagers, and tech *names* on the bar (the card says `+1`).


## D158 - Passing through a house: five hypotheses tested, none held

*2026-11-08. The owner's report with screenshots: *"villager can pass through the built house or stone mine."*

1. **No production code changed.** Five tests were added and four of them hunt the report; each passed against the unfixed engine,
   which is the finding - not a fix.
2. **Refuted, with a test each**: a route planned before a house appears goes **round** it; `WalkTo` (the player's right-click
   point) does not cut through; the **builder is not left inside** the house it finishes; a **miner never stands on the node's own
   tile**; and the **drawn footprint equals the blocked footprint** for a house (2x2), a barracks (3x3) and a wall (1x1), with the
   ring around each staying walkable.
3. **Found, recorded, not fixed**: a walker interrupted by a building **stops one tile short** of its goal - (15,29) for a goal of
   (16,30) in the first test. That is the D134 "goal snapped once" path, and it belongs with D134's own rule and test.
4. **The likely truth about the screenshots**: the grey diamonds are buildings (role glyphs: a chevron is a house), the brown ones
   are resource **piles**, and a mine is one tile in the dat (`STONM` radius 0.5). A unit on the tile *in front of* a building is
   drawn over the diamond's edge by depth ordering - an art problem, not a collision one.
5. **To settle it**: was the villager moving or standing, and what order had it been given (gather / build / right-click move /
   right-click on the building).
6. **Five permanent guards remain**: any regression on these five paths fails a test rather than reaching a screenshot.


## D159 - A building is drawn tile by tile, because one diamond is a different shape

*2026-11-08. The owner, after D158: *"Then there must be disconnect between headless and gui run."* The effect was real; the cause was
a **shape**, not a second simulation.*

1. **The engine blocks every tile of a footprint**; the renderer drew **one diamond** of `across x rx` centred on the block. The
   union of the footprint's tile diamonds has four corners a single diamond does not reach, and its own vertices stick out into
   tiles it does not own - so the picture painted ground the collision refused and refused ground it allowed. That is what "a
   villager passes through the house" looked like: it was walking past a corner tile it could not stand on.
2. **`render` now draws one diamond per footprint tile**, at each tile's centre. The silhouette is literally the set the walk grid
   blocks; the role mark stays at the block's centre.
3. **The test is a pixel test with a named opposite**: every footprint tile's centre is the building's colour and every tile around
   it is not, proved by **mutation** (restoring the big diamond fails with "the house's own tile (12, 11) is painted as the
   house").
4. **Two old tests encoded the old shape and were rewritten, not deleted**: the vertex-based silhouette test became a per-tile one,
   and the fog dimming test now samples a tile centre instead of a four-diamond meeting point.
5. **Cost**: footprint-area diamond fills per building (16 for a town centre, 4 for a house, 1 for a wall) instead of one, each
   clipping to the frame. The simulation is untouched; the acceptance hash is unchanged.


## D160 - A tile is the diamond centred on its integer point

*2026-11-08, after D159. The owner: *"Gray fields representing buildings also need to be moved."*

1. **The renderer has two projections and only two**: `camera.project((x, y))` for anything that occupies a tile - a tile is the
   diamond **centred on its integer point**, which is why neighbours meet edge to edge - and `camera.project(position)` for a body
   between tiles. The per-tile building pass added in D159 used `(x + 0.5, y + 0.5)`, half a tile down-right of its own ground.
2. **Fixed to the terrain's own point**, so a building tile and its ground tile are the same diamond.
3. **The D159 pixel test could not see a half-tile shift**: it sampled 55% of the way down from the tile point, where a shifted diamond
   still paints. It was moved to **45% of the way to the upper-left vertex** - inside the tile, outside a shifted one - and **proved by
   mutation**: restoring `+ 0.5` fails with *"the house's own tile (11, 10) is painted as the house"* and a grass pixel.
4. **`footprint.rs` needed the same sampling change**, having inherited the tile-centre convention.
5. **A pixel test that passes for a visibly wrong picture is worse than none**, and this one did for exactly one commit. That is the
   lesson recorded, not the line of arithmetic.


## D161 - The yard is not wall, and the outline goes outside

*2026-11-08. The owner, with a screenshot of a villager standing on a town centre: *"villager still goes through the town centre … maybe
the white diamond laying around the building is too small … I do remember that being so in the game. How is building collision handled by
the game?"* Research: `docs/building-collision.md`.*

1. **The game separates three things**: collision = the dat's `radius[0]`/`radius[1]` as **half-extents in tiles** (town centre 4x4,
   house 2x2, wall 1x1); the footprint is **not all solid** - the town centre keeps a walkable **yard** (D115, and aoe2mc's *"let units
   walk under the town center"* names the same cells); and the **sprite is bigger than the footprint**, so art overhangs and units are
   drawn behind it.
2. **So a unit on a town centre's footprint is the game working**: it is in the yard. Our bug was that the picture painted the yard the
   same grey as the walls - *that* is what read as "inside the building".
3. **`ruage_view::solids_of(world, &snapshot)`** asks the **walk grid** for every footprint tile, and `render_with_solids` paints solid
   tiles `BUILDING` and walkable ones `BUILDING_YARD`. The mask travels beside the snapshot, so `Snapshot` did not grow a field and no
   caller changed.
4. **The selection outline moves outside the footprint** (`+ 2 px`); at `0.9` it sat inside the grey, drawing a boundary that is not the
   boundary - the owner's "white diamond is too small", and he is right.
5. **Both new tests were proved by mutation**: forcing `solid = true` fails *"the yard was painted as wall"*; restoring `0.9` fails
   *"the selection outline is nowhere near the front edge"*.
6. **Not settled**: how far the original's sprites overhang (art-side, and we have no sprites yet), and whether every big building's yard
   is a ring or a rect (ours is a rect, from the engine's rule).

---

## D168 - The decision index is derived from the corpus, and it quotes its evidence

*2026-11-09. The owner: "I want to have a page I can browse with a web browser where I could see all the decisions that
have been made or are yet to be made and how they were made (research, decided by me, reverse engineered, assumed...)
and that would help me to spot gaps and bad decisions." Answers to the three shapes offered: **generated rather than
authored**, **the open register, the evidence ladder and broken references**, committed at **`docs/site/`**.*

1. **Derived, never authored.** `tools/decisions.py` reads `docs/decisions.md` and the U-register in
   `docs/mechanics/README.md` and writes one self-contained HTML file. Nothing on the page is typed by hand, so the page
   cannot drift from the corpus - and when a label is wrong, the classifier is wrong in a way a reader can see.
2. **Every label carries the phrase it was read from.** A basis is recorded only when the decision's own text says where
   it came from (`owner`, `dat`, `binary`, `measured`, `published`, `prior-art`, `decision`, `review`), and the sentence
   is shown beside the tag. This is the answer to "it needs to represent decisions correctly": correctness is not a
   promise the tool makes, it is a quotation the reader can check.
3. **An absence is a category, not a failure.** 33 of the 165 decisions name no source at all. That category needs no
   pattern to be right, which is why it is reported rather than papered over - and it is not a verdict: it says the
   reasoning is argument rather than a citation, and the reader decides what that is worth.
4. **Two registers, because they need different actions.** *Not decided yet* is the U-register (12 open questions);
   *decided, but not finished* is the 24 paragraphs where a decision names its own limits. Mixing them would turn
   "decide this" and "implement this" into one list.
5. **It does not sort by strength of evidence, and that is a change from what was approved.** A single ladder position
   per decision was the plan, but most decisions here cite three sources at once, and a sort key over that would be
   precision the prose does not contain - the same mistake as inventing a number. The filters are the honest version:
   *owner*, *dat*, *binary*, *measured*, *published*, *prior-art*, *decision*, *review*, *no basis*, *unresolved*.
6. **`unresolved` is a reading list, not a verdict.** Language that admits doubt ("stays a hypothesis", "not
   established") is quoted, unless the same sentence says the doubt was settled ("tested and is falsified") - because
   22 decisions flagging themselves is useful and 120 would be noise.
7. **Where it lives.** `docs/site/index.html`, committed, because `docs/decisions.md` is committed. Rebuild with
   `make decisions`; `make decisions-check` parses and reports without writing and fails on a heading it cannot place.
   It is deliberately **not** in `make check`: a docs tool cannot regress the engine, and adding it would make every
   commit pay for the corpus.
8. **What its first run found, which is the point of the exercise.** Ten ids are referenced in the docs or the code and
   never written down: **D78, D79, D81, D82, D115, D122b, D124, D125b, D126, D136**. Three decisions live at `###`
   under their parent (D24a, D94a, D98a) and were invisible to a reader looking only for `##`. 4 decisions are
   corrected by a later one, 5 are superseded, 5 ids carry an amendment section.

9. **A review queue, because 166 cards in one scroll is not a review.** Batches of 10, 20 or 50 with a resume point,
   a verdict on each decision (agree / question / wrong) and a one-line note. The verdicts live in this browser
   (`localStorage`), **export** writes them to `decisions-review.json` and **import** reads them back - a static page
   cannot write the repository, so the file is the bridge. The queue is narrowed by the same chips as the register,
   which is what makes "one part at a time" literal: the three shortest passes are *no stated basis* (35), *unresolved*
   (22) and *corrected* (4).
10. **Areas were tried and dropped.** The first version filed decisions under a topic by counting the words and paths
   they name. It put the **file-oracle** decision (D2) under *movement*, because "step 2b" contains "step", and it
   needed a date the corpus does not carry to do any better by time (88 of 166 decisions have no date inside their own
   section). A structure that is wrong one time in five is worse than the batch queue, so the queue has no topics in
   it - only batches and the evidence chips, which are derived from phrases and can be checked.

11. **The register is cut into collapsible bands of 25, and the band header says what is inside.** Bands are by
   **position, not by number range**: the numbering has holes (no D78-D82) and letter suffixes (D44a), so a numeric
   range would name ids that do not exist. Each header carries its own facts - "25 decisions - 1 corrected - 2
   superseded - 5 with no stated basis" - so a collapsed band still answers "is this worth opening". `expand all` and
   `collapse all` sit with the filters, a filter reveals the bands that hold matches, and the queue opens whatever band
   its batch lands in.

**Limits, named.** The basis patterns are a heuristic over English prose: a decision can be tagged from an incidental
mention, which is why the sentence is on the page. The U-register is read from one table, so a question written
anywhere else is not in the open register. Nothing links a decision to the *test* that pins it - the code cites ids
(155 of 178 `.rs` files do) but there is no check that the cited test still exists.


## D162 - The virtual fence was there; it was painted as yard

*2026-11-08. The owner, with three villagers standing on a town centre: *"I still see huge overlap … a virtual border around each building
and no unit['s] center can cross it … except Farm and Town Center (the fence around the impassable structure in the top quarter) …
Buildings cannot overlap another building or functioning farm … the drop would happen when the unit center touches this outlying diamond."*

1. **Checked against the engine, rule by rule**: a unit's position **is** its centre and a solid tile is blocked, so no centre crosses -
   `no_unit_centre_crosses_the_core` now pins it for 1,200 ticks; a **farm's floor is walkable** while it lives; the town centre's mask is
   exactly **4 solid tiles, a 2x2 at the north corner**, 12 yard (`[TTFF,TTFF,FFFF,FFFF)`) - the owner's "impassable structure in the top
   quarter"; overlapping a building or a **functioning farm** is refused (`a_building_cannot_be_raised_on_a_live_farm`); and the deposit runs
   on a tile **beside** the footprint, so the centre touches the border and never enters it.
2. **So the report was about the picture**: `BUILDING` `[200,200,200]` against `BUILDING_YARD` `[170,178,170]` is not a fence, it is two
   greys - and a villager in the yard read as a villager in the masonry.
3. **Fixed by tone and pinned by a test**: the impassable part is **dark slate** `[96,100,108]`, the yard **light** `[188,196,184]`, and the
   two must stay **60+ luminance apart** or the test fails.
4. **What this does not fix**: two units can still overlap **each other** - that is unit-unit separation (D131), a different mechanism, and
   the three villagers in the report were in the yard, not on the wall.


## D163 - The yard is painted as ground, and a switch that turns the next report into data

*2026-11-08. Third report of the same thing. Two changes and one admission.*

1. **A building's walkable part is ground.** The yard was painted in the *entity* pass, which runs back to front by depth; a building's depth
   is its **front** corner, so it was painted after every unit north of it and covered units standing in its own yard. The yard is now painted
   with the terrain, **before any entity**; only **solid** tiles stay in the entity pass, so a unit behind a wall is still hidden by it.
2. **`RUAGE_DEBUG_TILES=1`** prints, once a second, every unit's tile and `walkable`, plus every building's solid mask - so the next report
   arrives as numbers rather than as a picture.
3. **Admission**: `yard_drawing_order` passes, and its **mutation did not fail**, so it is a guard, not a proof. Nine hypotheses have now been
   tested across D158-D163; two were real (the half-tile offset, the yard painted as wall), and the third picture is still not reproduced.
4. **The three answers the debug output distinguishes**: `walkable=false` on the unit's tile = collision bug, and the mask names the building;
   `walkable=true` inside a footprint = a wrong mask; `walkable=true` outside = the fault is drawing, i.e. sprite size, not collision.


## D164 - `cargo run` starts the window

*2026-11-08. The owner typed `cargo run --release` to get the debug output and got *"could not determine which binary to run"*.*

1. `crates/view/Cargo.toml` gains **`default-run = "window"`**, so `cargo run --release -p ruage-view` starts the game; the `frame`
   diagnostic binary still needs `--bin frame`.
2. **Why it was not left alone**: a second binary in a package silently re-opens this, and the first command anyone types to start the game
   should not be a puzzle. No code changed.


## D165 - Depth decides between units and walls

*2026-11-08. The owner: *"Units are able to walk over the buildings at the top side but cannot get close to the buildings from the bottom
side. Therefore it seems that there is a shift between graphics representation and path of units."*

1. **The shift is not in the simulation, measured twice**: a **constructed** house's drawn tiles are exactly its blocked tiles
   (`built_house_alignment`), and two villagers approaching a house from north and south both arrive **exactly** on their goals
   (`approach_symmetry`: (30,29) and (30,33)).
2. **It is the paint**: `layer_of` put **every unit above every building** - the owner's own 2026-11-04 rule, *"villagers and units should be
   always visible over the buildings/farms"*, made when farms were drawn as floors and a villager behind one vanished. That rule paints a unit
   **north** of a house over its wall (reads as walking through) and paints a unit **south** of it correctly, one tile out (reads as a shift).
3. **The rule's reason expired**: a farm's floor is painted as **ground** since D163 (it is walkable), so no floor can hide anybody.
4. **`layer_of` now gives buildings, units and projectiles one layer**, so depth decides; resource nodes stay below everything.
5. **Two tests were reversed rather than deleted**, and their comments say why: `a_unit_behind_a_wall_is_covered_by_it` (was
   `..._is_still_painted`) and the depth-order test, whose expected order is now computed from the depths instead of typed as a literal.
6. **Traded explicitly**: a unit behind a tall building is hidden where it overlaps it - which is what "no unit passes through the building"
   means in a flat-shape renderer.

---

## D167 - A decision's area is written down, and the build refuses an unfiled one

*2026-11-09. The owner: "I would like the grouping to be done in more natural way like 'UI, Pathfinding, Economy,
Combat...'". The page had been grouped by position - bands of twenty-five ids - because a derived topic had failed. This
is the authored version of the same request.*

1. **The table is authored, and checked.** `docs/decision-areas.toml` carries one line per decision - 170 of them across
   fifteen areas - plus the order the areas are shown in. `make decisions` **refuses to build** when a decision has no
   area, when the table names an id the corpus does not define, or when an area is not in `[order]`. Same rule as D6 and
   D45: a surface that silently ignores is worse than one that refuses.
2. **The derived version stays refused.** Counting the words and paths a decision names filed the file-oracle decision
   under *Pathfinding & movement* ("step 2b" contains "step") and put 45 decisions in one area whose first four were
   licensing, fixed-point and scope; chapters by week need a date 88 of 169 decisions do not carry. An area is a claim
   about what a decision is *about*, so somebody who read it writes it down.
3. **The cost is one line per decision, and that is the point.** Adding a decision now costs an entry in the table or the
   build fails naming it - the reminder that would have been useful the four times today a decision was added without
   anyone asking where it belongs.
4. **Fifteen areas, in the owner's order**: Combat, Pathfinding & movement, Economy, Buildings & construction, Production &
   queues, Techs & civs, Vision & fog, Victory & defeat, UI, Graphics, AI, Simulation core, Data & tooling, Licensing &
   publication, Project process. The largest are Pathfinding & movement and Project process; the smallest are Vision & fog,
   Victory & defeat and Licensing & publication.
5. **The page renders them as collapsible groups**, each header carrying its own counts ("Combat - 15 decisions - 1
   corrected - 5 with no stated basis"), `expand all` / `collapse all` with the filters, and a filter opening the areas
   that hold matches rather than leaving fifteen collapsed headers.

6. **The two "not done" kinds moved into the groups, and each got a filter.** *Not decided yet* (the twelve open
   questions) and *Not finished* (the twenty-four paragraphs where a decision names its limit) used to be sections of
   their own at the top of the page; they are now the first two blocks inside the area they belong to, above that area's
   decisions, with `decisions` / `undecided` / `unfinished` chips that show and hide each kind. The questions needed an
   area too, so `[unknown]` in the same file files them - twelve lines - and the same build check refuses an unfiled
   open question. They come **before** the decisions because an area with fifteen cards put its two gaps fifteen cards
   down, which is the same as not having them.

**Limits, named.** One area per decision: a decision that is half pathing and half rendering gets the one it is mostly
about, and D113 (a route bending at the body's clearance) is filed under movement although its measurement is geometric.
The page cannot edit the table - a correction is an edit to `docs/decision-areas.toml` - and the review queue's export
does not carry areas, so a reviewer's disagreement with a filing has to be said in a note.


## D166 - A unit's marker is clipped to its own tile

*2026-11-08. Fifth round on the same report. The marker is now drawn pixel by pixel and clipped to the diamond of the tile its centre is in, for
all three silhouettes - so no marker pixel can land on a neighbouring tile, and a unit standing beside a building can no longer paint inside it.*

1. **Why**: neighbouring tiles **share edges**, so a `fill_rect` of `0.35 * rx` half-width could reach past its own diamond into the next tile.
   Shrinking it only moves the threshold; clipping ends the class.
2. **What is NOT established**: the overlap is still not reproduced in a test. `marker_fits_tile` passes, and **neither mutation made it
   fail** (back to `0.35`, and removing the clip) - so my geometric model does not match what the renderer actually puts on screen, and that is
   an open question. Two earlier versions of that test were blind by construction (sampling only a tile's lower half, then only a rectangle).
3. **The remaining overlap is inherent to continuous positions**: a unit between tiles straddles four of them; the original hides it with art
   (feet and a shadow), which is an asset-pack question (D18).
4. **Two options if the owner wants zero overlap**, both real behaviour changes and both needing his word: **snap the marker to its tile's
   centre** while the body stays continuous, or shrink the marker to `0.25` of a tile.


## D167 - Units above buildings again, because both orders were reported

*2026-11-08, sixth round. Measured once more: a villager ordered to the tile **touching** a house's north wall and another to the tile touching its
south wall both arrive **exactly on their goal tile's centre** - `(40.50, 39.50)` and `(40.50, 42.50)`. There is no reachability asymmetry; the
engine stops a unit at the tile boundary on every side.*

1. **Both orderings have now been tried and both were reported**: units painted last gave *"a villager can pass through the built house"*; depth-decides
   (D165) gave *"units are below the dark gray"*. This decision restores **units above buildings** - the owner's 2026-11-04 rule - because the clip
   from D166 means drawing units last can no longer paint inside a wall.
2. **All three states of that decision are recorded in the tests' comments**, not just the last one: a test that encodes an owner ruling has to say
   which ruling, or the next person re-reverses it blind.
3. **What actually ends this is art, not a rule**: the original draws objects with height (feet, a shadow), and this renderer draws flat shapes with
   none. That is the asset pack's job (D18).
4. **Two one-line options if the owner wants them now**: a **dark outline** around a unit's marker so it reads as in front over grey, or **snapping the
   marker to its tile's centre** so it can never straddle a neighbour. Both change how the game looks, so both are his call.


## D168 - Two frames, one conversion: `drawn_at` subtracts half a tile

*2026-11-08. The defect under six rounds of "the unit is inside the building". `Camera::project` puts a tile's diamond **centre on the tile's
point**, so tile `(x, y)` covers `x - 0.5 .. x + 0.5` on screen; the simulation's tile `(x, y)` covers `x .. x + 1` and a settled unit stands at
`x + 0.5`. Drawn with the raw position, every settled unit landed on the **corner of four diamonds** - half a tile down-right of its own tile -
and its marker painted a quarter of itself into each of the four neighbours.*

1. **The conversion is one public function**: `ruage_view::drawn_at(position) = (x - 0.5, y - 0.5)`, used by the marker, the selection ring, the
   rubber band's hit test and the double-click's on-screen test. Four sites, one subtraction, or they drift apart again.
2. **Proven failing first**: `unit_centring` measured the marker's painted centroid at the centre of tile `(42,42)` as `(450.0, 398.0)` against a
   diamond centre of `(450.0, 374.0)` - **24 px, exactly half a tile**. Reverting `drawn_at` to the identity reproduces it.
3. **Buildings were already right**: their ring is the tile midpoint of the footprint, which is in the view's frame. Only bodies were wrong.
4. **The lesson belongs in the tests**: three earlier tests used *integer* positions - a tile corner in the simulation's frame and a diamond centre
   in the view's - so they agreed with the bug. **Fixtures must place units where units stand: at tile centres.** Six test files were corrected.
5. **Marker back to `0.30 * rx`**: its corner then sits at `0.90` of the diamond, so the D166 clip no longer cuts a settled marker's corners.


## D169 - A gatherer stands where the two bodies touch: 0.7 of a tile, not 1.0

*2026-11-08. The owner, with two screenshots of the original: *"villagers are standing around 1/2 tile away from what they are doing. In the game they
are much closer. Can you research from the game?"*

1. **The numbers are in the pack.** `radius[0]` is a half-extent: **0.2** for a villager (`VMBAS`), **0.5** for a forest tree (`FOAK`/`FPIN`/`FPAL`),
   a forage bush (`FORAG`) and a stone mine (`STONM`). Two discs touch at **0.7** of a tile, and that is the standoff the original draws. The engine
   already used the sum for "is this worker at the node" (`reach_raw`); it used a **whole tile** for where the walk ends, and the difference is the
   half tile the owner sees.
2. **The walk still ends at a tile centre.** Two attempts to make the route end at 0.7 failed the same way - a route the movement rules never call
   *arrived* is a worker that never works, and both deadlocked the economy (a tree untouched for ten seconds; a lumberjack standing at a spent tree).
   **A settle, not a step**: on the tick a finished walk leaves the worker at the node, `settle_workers` places it on the standoff point, once, on
   free ground.
3. **The node's radius is content**: `resources.toml` gives each resource `radius = 0.5`, and `load_resources` resolves it into engine units once, so
   no tick touches a float - `load_resources` is named in `no_float`'s `LOAD_TIME` list with that reason.
4. **Buildings and farms are untouched**: their `reach` is at or above a tile, so 0.7 would move nothing, and the settle is gated on a resource node.
5. **Recorded trade-off, not fixed here**: workers now crowd at the side they approached from - the acceptance run drops from 150 to **137 entities**
   and repaths rise from 88 to **144**. Spreading workers round the node (approach axis, then the opposite, then across, skipping a taken point) is
   the next step, with its own test.
6. **Acceptance has moved, and it had to**: `a120ed0721c8768c` -> **`5703f537f92c0bf2`**, 137 entities, 2 players, 144 repaths, 132 made-room.


## D170 - A mine is a *pile per tile*, not one object: so the standoff is measured per tile

*2026-11-08, after the owner's answer to "should a mine be one object or four tiles?".*

**The owner**: *"Each tile has one pile of stone and gold."*

1. **Our map model is the game's**: a stone or gold field is a set of **one-tile piles**, each with its own amount, each mined on its own. The dat's
   `radius = 1.0` on `STONM`/`MINE2/3/4` describes the *drawing* of the rock, not a single minable object, so it is **not** evidence for one 2x2 node.
   Our four independent tiles stay (option B of the question, answered A/B/C by the owner rather than guessed).
2. **Therefore D169's 0.7 standoff is right for a mining tile too**: the worker works the pile on one tile, and stands where its body (0.2) touches
   that pile's (0.5). Only a *building* is worked from beside its footprint, as before.
3. **What is left open, and is not a rules question**: several workers on the same node settle on the same side; spreading them round the node is
   the recorded next step in D169's review.


## D171 - Workers fan out round a node instead of sharing its approach side

*2026-11-08. The owner chose **A** ("spread workers round the node"). `standoff_point` tries the **four axis sides** at the touching distance,
ordered by how well each faces the worker's approach, and takes the first that is free ground, **not held by another body**, and **reachable from
where the worker stands**. The approach side still wins when free, so a lone worker is unchanged (D169's 0.70).*

1. **Proven failing first**: two villagers sent at one tree from the same side settled on the **identical raw coordinate** - 0.00 tiles apart.
2. **Reachability is part of the check**, not just free ground: a settle point inside a forest pocket is free, walkable and a trap. `can_reach` from
   the worker's own position is the proof.
3. **Four sides is the ceiling** for a one-tile node with a blocked tile; a fifth worker keeps the tile centre it walked to.

## D172 - A map node stands at its tile's **centre**, not on its corner

*2026-11-08, found by D171. `seed` placed every map node at `self.tile((x, y))` - the tile's **corner**, the anchor a building's footprint is placed
from. A body of half a tile's radius centred on a corner spills a quarter into each of four tiles, so a node's four working sides were **not**
symmetric: 0.7 north of a corner is in a free tile, 0.7 south of it is in the node's own blocked tile. Woodcutters could approach a tree from the
north and west and not from the south and east, and the settle placed one inside a blocked tile.*

1. **The fix is `tile_centre`**, the convention D114 gave walk goals and D168 gave every body - the same half-tile mistake in a third place.
2. **Two engine sites compared a raw position to a tile corner** and had to learn the rule: the rally resolution in `train.rs` (a rally onto a
   resource silently became a plain walk) and the view's resource pass (a node is a body, so it is drawn with `drawn_at`).
3. **Seven test fixtures located a node by its corner**; they now locate it by its **tile**, or place it at `x + 0.5`, each with the reason in its
   comment. A fixture at a corner is a node no map can produce.
4. **Measured**: entities 137 -> **153** against a 150 baseline, repaths 144 -> **24** (baseline 88), made-room 132 -> **16** (baseline 100). The
   crowding was never a gather rule - it was bodies standing on the wrong half of a tile. Acceptance **`6d23d672eb18ef40`**, players=2.


## D173 - OPEN: the gatherer's last third of a tile is a **placement**, and that is the jump the owner sees

*2026-11-08. Owner, with three screenshots: jump towards the bush, pile up, forage from a distance, appear and disappear - "regression of previous step".
**No engine change was shipped**; the reproduceable part is the jump, and it is `settle_workers` (D169) placing an arrived worker on its standoff.*

1. **Reproduced**: a gatherer moved **0.6 of a tile in one tick** where its speed allows 0.04 (`gather_motion`, test **ignored** and labelled). A
   placement is not a walk, and the eye sees it.
2. **Two walks were tried and rejected, with numbers**: (a) ending the *route* at the standoff strands a worker 1.57 tiles out when its chosen side is
   taken by the time it arrives, and at that distance "beside the node" no longer holds so it never returns; (b) stepping toward the standoff every
   tick costs **225 repaths against 24** in the acceptance run, because the standoff and a reachability search are recomputed per worker per tick.
3. **A third shape is the promising one**: a **one-waypoint approach route** earned on arrival and re-checked each tick. It needs "beside the node" to
   cover a worker pushed a tile and a half out by its neighbours - that widening is the missing piece.
4. **Not reproduced here**, and therefore still open with the owner: piling up on one bush, foraging from a distance in a **berry patch** (most sides of
   a bush are blocked by other bushes - the fixture is a lone bush on open ground), and the flicker (a lone worker is drawn on all 120 ticks).
5. **Own error, recorded**: I read design (b)'s 155 entities as a success before looking at its repaths. Both numbers are in the review.


## D175 - The search radius is the dat's own number, per unit

*2026-11-08. The owner: *"If the worker cannot reach the place it will search for another nearest source. ... Can you check in the file?"* - and,
offered three shapes, chose **B**: the dat's `search_radius` per unit.*

1. **The number is in the file**: `task_info.search_radius` - villager **4.0** tiles, scout 4.0, cog 6.0, fishing ship 12.0, trebuchet 17-18, widest
   21.0; 750 slots carry it, median 6.0. It bounds the *search for another source*; it is not a placement rule.
2. **The standoff is not in this file**: `work_range`, `auto_search_targets` and `search_wait_time` are **DE** (v7.x) fields. The AoK HD file is
   v5.7.2, so the touching distance there *is* the unit radii - 0.2 + 0.5 = **0.70** - which D169 already implements. The owner's model is the file's
   model.
3. **What changed**: `Unit.search_radius`, and `gather_reach` uses it when the pack carries it. Packs generated before the field existed keep sight
   plus five (villager: **9**), which is the number the game does not use. **Mutation-proved**: disabling the branch makes the test report *"looked 9
   instead"*.
4. **The fallback rule already existed**: a refused node is remembered (`share_failed`, D138) and the picker - distance order, reachability asked of
   the nearest eight - moves on; `a_walled_off_wood_is_tried_once_and_then_left_alone` proves it. The radius was the wrong part, and it is fixed.
5. **The pack changes the day `genconfig` emits the field**, and that generator lives in another repository (D12), so this checkout cannot
   regenerate `config/gamedata/`. Until then the acceptance hash is unchanged (`6d23d672eb18ef40`) and the behaviour is exactly as it was.
6. **Recorded, not changed**: `share_retry_ticks` is **600** (30 s), where DE's `search_wait_time` is about a second.


## D176 - The selection ellipse is the collision circle, and the body's clearance is a query

*2026-11-08. Owner: *"units still collide with the impassable building diamonds (the ellipse cannot overlap with it at all, not just center of it)."*

1. **Measured first, and it changed the answer**: a new test runs the village map under the skirmish script for **2,000 ticks** and asserts every
   unit's `clearance >= 0` - distance from its centre to the nearest blocked tile, minus its radius. It **passes on the shipped engine**, so no body
   was ever inside a wall; our ground test asks about the centre's tile, and the paths it produces keep the centre half a tile clear.
2. **What overlapped was the drawing**: `[ring] scale = 1.5` put a **0.3** ellipse on a **0.2** body. Set to **1.0** - the file's own comment
   already named that alternative - so the ring *is* the collision circle. The 1.5 existed to keep the ring out of a sprite's feet, and there are
   no sprites yet (D18); when there are, that becomes an art decision again.
3. **The rule is now expressible and testable**: `World::clearance_at(position, radius)` (distance to the nearest blocked tile minus the radius;
   zero = touching, negative = inside) and `World::body_clearance(id)`. That is the Forgotten Empires talk's *"shrink the unit to a point, expand
   every other obstruction"* as one integer expression.
4. **Mutation-proved**: restoring `scale = 1.5` fails `ring_is_the_body` with *"the pack draws the ring at 1.5 times the body"*.
5. **Not changed**: collision itself. The gather standoff (D173) is untouched - the jump is about *where a walk may end*, not about clearance.
6. Acceptance byte-identical: **`6d23d672eb18ef40`**, 153 entities, 24 repaths.


## D177 - The ground test is the body's, not the centre's

*2026-11-08, the second half of the owner's collision report: after D176 shrank the ring to the body he sent a screenshot of a villager on the town
centre's yard with its ellipse across the dark core's edge - **"I see clear overlap."***

1. **Reproduced on every tick**, not at the end of a walk: a villager ordered to the tile touching the core had its body **0.18 of a tile inside**
   the blocked tile at some point (`body_clearance` = distance to the nearest blocked tile minus the 0.2 radius).
2. **The rule is a distance**: refuse a step whose landing has `clearance < 0`, unless the body is **already inside** and the step does not dig
   deeper. That second clause is what keeps a pushed or spawned-overlapping body mobile; without it, a unit that arrives overlapping can never move
   again.
3. **One named exception**: a unit in `entering` (walking into its own building) uses the old rule - a unit on its way through a door is not a body
   looking for room. Without it, `a_unit_rallied_to_its_own_building_stays_inside` goes red.
4. **The cost, measured**: acceptance repaths **24 -> 529** (one every 68 ticks), entities 153 -> 138, hash `661e970e58bc0c57`. The rise is because
   the *planner* still smooths corners without knowing the radius; the fix is to give `short.rs`'s line test the same clearance, which is a planner
   step of its own and is recorded as next.
5. **What it fixes for the owner**: a villager can no longer stand with its ellipse inside a building, which is what he asked for twice.


## D178 - A body stands on its point; its base is the position

*2026-11-08. Owner, after D177: "the circle moves a bit elsewhere than it should... the unit above it stops a bit further" - with a screenshot of a
villager short of the yellow destination cross.*

1. **The walk was right and is now measured**: `move_arrival` sends a villager across empty grass and finds it within **0.1 tile** of the tile it named.
2. **The drawing was wrong**: the silhouette was painted `-reach ..= +reach` around the position, so a body's mass hung **half a shape below** where it
   stood (measured: lowest row 388, unit at 374 - 14 px) and the selection ellipse, centred on the point, crossed its waist.
3. **The anchor is the base**: rows `-2 * reach ..= 0`. The ellipse now rings the feet, which is what the owner asked for ("align the center of the
   ellipse with the bottom side of the square/triangle").
4. **half = 0.20 of a tile** (was 0.30), because the body rises `2 * half` into a diamond that narrows: it fits whole exactly when `5 * half <= rx`. A
   quarter clipped the head into the tile's corner and the shape test caught it.
5. **The health bar moves above the head**, not above the ring.
6. **Three tests learned the new anchor** (scan from `drawn_at`, lowest row instead of centroid, triangle rows narrow towards the apex) - each rewritten
   with its reason rather than deleted.
7. Acceptance **byte-identical** `661e970e58bc0c57`; the view now paints *fewer* pixels than before.


## D179 - The selection ring sits round the feet, not under them

*2026-11-08. Owner, after D178: "the circle is supposed to be at the bottom, not waist. The unit is above the circle."*

1. **D178 put the body's feet on the point and left the ring centred on the same point**, so half the ellipse lay on the **grass below** the unit and
   read as a circle the unit stood behind. Measured first: *"the ring's lowest row is 379 and the unit's feet are at 374: the ellipse is 5 px below the
   feet."*
2. **The ring's centre is the point raised by its own half-height**: its lowest row is the base, the ellipse embraces the lowest 19 px of the body, and
   body and ring agree about where "the bottom" is.
3. **The ring's size is unchanged**, so it still cannot reach a neighbouring tile (D176) - this moves the ellipse, it does not enlarge it.
4. **`ring.rs`'s curve-versus-diamond test moved with it**, raising its sampling centre by the same half-height, reason recorded in the test.
5. Acceptance **byte-identical** `661e970e58bc0c57`.


## D180 - The settle walks: a step per tick, capped by speed, and nothing else

*2026-11-08. Owner: "how about villagers jumping and stacking up at the same place or foraging from greater than allowed distance?" Measured again:
stacking and distance pass (D171, D169); the **jump** was still open - the test that had been `#[ignore]`d since D173 failed with the same number,
15,600 raw units in one tick.*

1. **Cause, confirmed by instrumenting**: `settle_workers` placed the worker on its standoff. The first fix (step toward it, capped by speed) still
   jumped 15,600, because `entity.speed` is **per second** (16,000) while a step is per tick (÷ 20 = 800): reading it raw allowed 1.56 tiles.
2. **The fix is one `step_towards`**: the movement pass's own arithmetic, so no step is a fraction of a scale unit and none overshoots. A worker
   arrives at its tree one or two ticks later, at its own speed.
3. **`velocity` is state, not a report**: writing it from the settle made a villager fifteen tiles away walk to a patch the D149 reach filter had
   refused. The settle now changes the position and nothing else.
4. **No new order layer**: D173's four rejected designs all changed where the *route* ends, and cost 6,118-19,096 acceptance repaths against 24. This
   adds no route - the existing one ends on the neighbouring tile, and the last third of a tile is walked by the movement rule that already exists.
5. **Acceptance**: entities 138 -> **154**, repaths 529 -> **297** (-44%, because a teleport invalidated routes planned around where the body was),
   made-room 110, hash **`834efe7c5efd3c62`**.
6. **Un-ignored**: `a_gatherer_never_moves_further_in_a_tick_than_its_speed` now runs with the suite.


## D181 - A standoff is taken when it is too close, not only when somebody stands on it

*2026-11-08. Owner, after D180: "Jumping and flicker is gone. However there is visible overlap of villagers. And occasionally they appear to be foraging
while the circle is not touching the square with bush."*

1. **Overlap, cause**: `standoff_point` treated a side as held only when another body was **exactly** on it, so two workers walking in from different
   sides both claimed the same spot and arrived at it (measured: two villagers at **0.00** tiles apart).
2. **Overlap, fix**: "held" is now *closer than two bodies are wide*, and the sides chosen earlier in the same decision pass count as taken - the list
   is `work.settles`, which the pass already builds, so no new state.
3. **Distance, measured**: a worker took food at **0.92** tiles where the bodies touch at **0.70**, because the take is gated on `at_node`, which also
   means "beside the node's tile" (up to 1.41 tiles).
4. **Distance, cause**: `reach_raw` answers the unit-to-unit question and returns the worker's radius alone (**0.2**) for a map node that has no unit,
   while `standoff_point` adds the node's content radius (**0.7**). Two answers to one question in one arm of one match.
5. **Distance, OPEN**: gating on `touching` with the right sum passes that test and turns **six** red, including `the_villager_walks_to_the_node_and_then_takes_from_it`
   (tree stays at 20 hp, never yields) - unexplained, so not shipped. The test is ignored with its measurement and the next step named: one
   `contact_raw(worker, node)` for both questions, then the gate.
6. Acceptance: entities 154 -> **160**, repaths 297 -> **254**, hash **`009e5cfe81b182ae`**.


## D182 - One gatherer a square, standing a body's radius off its edge

*2026-11-08. Owner: "Can you make the villagers to stand at the distance of half of diameter of the circle from the square they are gathering any resource
from? If there is no space at half diameter from another villager and the the resource try another nearest resource of the same kind."*

1. **The distance is `0.5 + radius = 0.70` from the tile's centre**, i.e. half the ring's diameter from the square's **edge** - D169's number, now
   measured as a gap to the edge and tested (`a_gatherer_stands_a_body_radius_from_the_square`).
2. **One place a worker, the side it approaches from**: `standoff_point` offers the approach side only. D171's four sides fanned five villagers onto
   one bush (measured: all five worked it), which is the crowding the owner photographed.
3. **A taken square is answered elsewhere**: `start_gather` sends the second worker to the nearest source of the same kind that nobody is working
   (`nearest_like_free`), or idles him when there is none - the same shape as the existing "one farmer a farm" rule.
4. **The rule is applied to player orders only.** Applied to every internal re-task it planned a route per candidate per tick: the acceptance run went
   from **16.3 s to over 180 s**. Split into `start_gather` (spreads) and `start_gather_internal` (does not).
5. **Acceptance, with its costs**: entities 160 -> **137**, repaths 254 -> **618**, hash **`dc9cf9905f65b61c`**, wall time **10.9 s**. Fewer units and
   more walking is the price of one worker per square; it is the owner's trade, recorded rather than argued.
6. **Two tests rewritten with their reasons** (D171's fan-out assertions): `two_woodcutters_on_one_tree_do_not_share_its_side`,
   `three_gatherers_on_one_bush_do_not_pile_up`.


## D183 - A foundation is walkable, invisible and free of cost until a villager touches it

*2026-11-08. The owner's model, given in two messages after his scout was pushed into the forest by a lumber-camp placement: **"When you place a
building on the ground, it is still walkable over ... and you and your allies see it. If any of your or allied units stand over it, they will leave
the area by the shortest route ... Enemy units don't see the foundation until villager started working on it ... As soon as the villager starts
building, it will turn dark grey (except farm) and start blocking the tiles"**. Supersedes D152 (a foundation blocked from insertion).*

1. **Place**: the cost is paid **at placement** and a **cancelled foundation is refunded in full**; the ground it covers stays **walkable** and is drawn
   light grey to its owner and allies, under normal fog. Own and allied foundations may not overlap each other; an **enemy** foundation is invisible, so
   overlapping one is possible and the first to start construction wins, with the loser told once.
2. **Invisible to enemies** while unstarted - not merely unrendered: their pathing and their eyes both treat the ground as free. It becomes **visible to
   everyone in line of sight** the moment construction starts.
3. **Builders approach like gatherers**: a contact point **outside** the footprint (the same standoff geometry D169 uses for a node), never inside it,
   so a builder can never block his own site.
4. **Construction cannot start while any unit is on the site** - every player's, allies and enemies alike, and animals, which are given a "leave the
   site" order like allies and **do** hold the start up until they are off. Allies and animals are told to leave; an enemy has no reason to and can stall
   a site indefinitely, which is accepted as a tactic. **The message is reported once per state change**, not per tick.
5. **The first stroke** turns it dark grey and makes its tiles **block**. That is safe **by construction**, because the site was empty the tick before -
   so "no unit is ever inside a solid footprint" is a guarantee, not a hope, and is to be pinned by a test.
6. **Cancel**: everything disappears as if it had never been placed, at any stage of construction, refunded in full.
7. **Farm is the exception**: constructed by walking **over** it, always walkable (D163), never blocks, multiple villagers may seed it and only one
   stays to farm it (the existing one-farmer-a-farm rule). Its site-clear gate therefore does not apply.

### D183 addendum - where a foundation may go, and what happens when the ground has changed

*Same conversation, one message later: **"building can be placed only on the scouted area (can be in fog but never in black). It may happen that in
meanwhile buildings are built there. Once villager sees that by his los, building construction is cancelled and refunded. You also cannot place any
building over tile with neutral/enemy in"**.*

- **Placement is allowed on explored ground, including fog, and refused on black.** The engine already keeps a per-player `explored` bitset
  (`World::explored`, `memory.rs`), so this is a gate on the order, not new state.
- **A blind placement that turns out to be occupied cancels itself when your sight reaches it, and refunds in full** - which is already the engine's
  behaviour: `settle_blind_sites` (D61) cancels a foundation placed in fog once the player can see that something is already standing there, with the
  resources back (D58a). Only something **started or finished** does that; two bare foundations are decided by whoever puts a builder to work first.
  So the owner's rule and the engine agree, and the reporting must be one message, not one per tick.
- **No foundation may be placed on a tile holding a neutral or enemy unit.** Own and allied units do not block placement - they are the ones evicted by
  route (D183 §1). An enemy who **walks onto** a site afterwards still stalls it, because he has no reason to leave: that stays as accepted.

## D184 - The reproducer does not reproduce here, and the tree model is why

*2026-11-08. The owner: **"I noticed that scout stopped near the forest, then villager placed there a lumber camp colliding with
his position, that pushed scout to the forrest. So reproducer: put scout as near to forest as possible, place lumber camp where
the scout is, he should immediately jump to the forrest."***

Written as a test **before any code moved**, in two shapes - the scout on the building's anchor tile, and on the far tile of a 2x2
footprint (a different eviction ring) - and it **passes on the shipped engine**: `work_position` already refuses ground that is not
free and already asks the pathfinder whether the tile is reachable, and our forests are **terrain** (`is_free_ground` answers no for
every tree tile), so the eviction physically cannot choose a tree.

In the original a tree is an **object standing on grass** - the ground under a trunk is walkable and the trunk is what blocks - and
we have the opposite model until scattered trees arrive. So the accident needs the model we have not built, not a patch in the
eviction. The test stays as a **guard** (`crates/sim/tests/eviction.rs`): a placed building never evicts a body onto a resource,
onto a building, or into its own footprint - true then, and it has to stay true now that D183 replaces the teleport with a walk.


## D185 - D183, built: the ghost is a drawing, the first stroke is the wall

*2026-11-11. D183's model, implemented in the order it was written down, each part with the test that pins it. The work is
`crates/sim/tests/foundation_model.rs` (11 tests), plus the D185.1 defect and the fixtures D183.1 broke.*

1. **`insert` no longer claims a building's ground** - it blocks a building's solid tiles only when there is nothing left to build
   (`construction.is_none_or(|c| c.progress_ticks > 0)`), so a building that arrives finished (the opening, a scenario, a save) still
   owns its tiles and a fresh foundation does not. `work_one_tick` takes the ground on the first stroke (`take_the_ground`), and the
   counter in `Walk` keeps the pair balanced: whatever blocks once is unblocked once by the death sweep.
2. **The empty-site gate** (`site_is_clear`): construction cannot start while any unit of any player, or an animal, stands on the
   footprint. Own and allied units and the animals are told to leave; an enemy is not, so he stalls the site until he moves or dies -
   accepted as a tactic in D183 §4. The refusal (`SimError::SiteNotClear`) goes out **once per state change**, kept by
   `stalled_sites`, a bookkeeping set like `foundations` and `open_gates` rather than hashed state.
3. **Eviction is a walk** (`evict_units_off` -> `step_aside` -> `exit_tile`): the nearest tile outside the footprint **that a route
   exists to**, never a placement. What the step-aside interrupted is kept in `resuming` and re-issued on arrival (`resume_idle` now
   resumes a `Gather` as well as a `Hunt`) - the owner's *"I think that villager resumes"*. A unit that cannot find a way out is
   remembered too, so the search is not repeated every tick; its site simply stalls.
4. **A builder works from outside the footprint** (`approach_footprint`, and the `Task::Build` arm's adjacency measured against the
   footprint rectangle rather than the anchor tile). A builder caught on his own site steps out by the shortest edge (`edge_out`) and
   keeps his order - the gate never takes the builder's task away from him.
5. **Placement gates**: `SiteUnscouted` refuses the black and allows the fog (the owner's *"can be in fog but never in black"*), and
   `SiteOccupied` refuses a **visible** enemy or neutral unit. A unit the player cannot see does not refuse the order - a refusal would
   leak it - so the site-clear gate is what answers that case.
6. **It is drawn**: `palette::FOUNDATION`, a light grey over every tile of the footprint (the owner asked for *"light grey
   colour like finished farm"*). Without an explicit branch the ghost would be **invisible**, including to its owner: the renderer
   paints a footprint tile only when `solids_of` says the engine blocks it, and an untouched site blocks nothing.
7. **An untouched foundation is invisible to other players** (`visible_to`), even in line of sight, and `site_refusal`/`site_has_lane`
   ask `visible_to` so an enemy ghost neither holds ground nor counts as a wall. The first stroke makes it visible to everybody in sight.

**D185.1 - a defect the review found: a cancelled site was resurrected by its builder.**

`work_one_tick` wrote `hp = max_hp * progress / total` on every builder-tick, so a site the owner had just cancelled (`hp = 0`)
came back to life on the next tick and its ground stayed claimed. Fixed with a guard: **a dead site is not worked**. Pinned by
`a_cancelled_site_gives_its_ground_back`, which also proves the block is given back exactly once.

**D185.2 - what is *not* built, said plainly.**

- **Resume covers a gather and a hunt.** A `Return`, `Build` or `Attack` interrupted by a step-aside stands down when the walk ends;
  the owner asked for resume generally and the gather is the case he named, so the rest is recorded here rather than guessed at.
- **A hunt that interrupted a unit blocks its step-aside**: `step_aside` treats "already in `resuming`" as "already leaving", so a
  unit fighting when a foundation lands on it waits for the fight to end. Rare, and the site stalls rather than misbehaving.
- **A unit that cannot leave stalls its site for ever**, on one message. That is D183 §4's accepted answer; a player sees why.
- **A farm is seeded from beside it, not from on it.** D183 §7 describes the owner's seeding animation - *"villagers walk a while,
  does the animation, walks again while farm is being seeded"* - and this engine has no seeding animation and no per-art pose, so
  the builder takes the same stand on the footprint's ring as every other builder. The behaviour the animation stands for is
  there: the plot is walkable throughout, several villagers may seed it, and only one stays to farm it. What is missing is the
  picture of a man walking over his own field, which belongs with the asset pack.
- **The invisible-ghost rule is engine-wide**, so the ordering system and the panel also cannot see an enemy's untouched foundation.
  That is the rule, not a side effect: D183 §2 says the ghost is invisible as a fact, not as a rendering.

## D186 - the gather ring: 1.156 tiles, eighteen men, measured from the owner's save

**The owner's rule, from his own game.** With a close-up of eighteen woodcutters round one tree he said: *"I'd focus instead on how
close men can stand to the bush, tree or mine. The ones behind them don't work. Only first row ever works."* Asked whether the men
crowd onto one node or stand idle, he corrected the reading of his screenshot himself - *"I am not so sure. Some are standing around
idle"* - so the crowd is a front row at work with idle men behind it, not eighteen men sharing one node.

**The number comes from the save, not from the dat.** `test.aoe2spgame` (VER 9.4, save 12.5) carries a per-unit order queue at the
end of the file - `[u32 type][u32 1][u32 0][u32 stamp][u32 n]` and `n` triples of `(x, 0, y)` in tile coordinates, 28 + 12n bytes a
record. A save taken **after the owner clicked the tree again while his men stood at it** (`owner-test2.aoe2spgame`) has their
standing positions in its *from* points, because an order records where the man was when it was given. Six men round one tree:

| man | tiles from the source square's centre |
| --- | --- |
| 1 | 1.105 |
| 2 | 1.142 |
| 3 | 1.160 |
| 4 | 1.170 |
| 5 | 1.170 |
| 6 | 1.189 |

**mean 1.156, spread 0.084.** That is a ring, not a scatter: six men within a twelfth of a tile of the same radius. The three
closest pairs are **0.404, 0.455 and 0.710** apart, and 0.404 is exactly two villager bodies (`radius[0]` = 0.2 each) touching.

**Nine men a square, not eighteen.** The first reading of this file took the ring's circumference (`2 * pi * 1.156 / 0.404 =
17.97`) for the game's capacity, and the owner corrected it: *"How about not 18 but 9 around single tile - corners and sides. 18
for one tile of resource of way too much."* His save agrees with him and had said so all along - the sixteen orders over his two
adjacent trees came out **nine and seven**, which is what a nine-a-square cap looks like and is not what an eighteen-a-square cap
would do (all sixteen would have gone to one tree). So the capacity is **nine**, at 40-degree steps, **0.79** of a tile apart.

**D186 replaces the standoff, not the picture.** The engine stood a worker where the two discs touched: `radius[0]` 0.2 + a forest
tree's 0.5 = **0.70** (D169), one worker a square (D182), so a single tree fed **one** man - measured, 1 of 19 ordered villagers.
Now:

1. **The ring is 1.156 tiles** (`GATHER_RING_MILLI`), round the **source square's centre** - the tile's, not the entity's anchor,
   because a map may place a node on a tile corner and a ring round a corner straddles four tiles (measured: a bush at (20.0, 13.0)
   left every slot blocked and its woodcutter at zero progress for 1,200 ticks).
2. **Nine slots at 40-degree steps** (`GATHER_RING_SLOTS`), each checked for walkable free ground, no body within two radii, not on
   a building site, and - when the order is given - a path. The probe starts at a slot no other worker has, `(worker + node) mod 9`,
   so a man's place does not move under him. Integer cosine and sine tables, because the tick is hashed and a platform's `sin` is not
   the same number twice.
3. **A man works when he is in the band 1.056..1.256 tiles** (`GATHER_BAND_MILLI`) from the square's centre. A man on the
   neighbouring tile's centre is at 1.000 - outside it - which is *"only the first row ever works"* falling out of the geometry.
4. **Nine is the cap**, and the tenth order is answered the way a second one used to be: the nearest free source of the same kind,
   or standing still when the map has none.
5. **A buried ring falls back to the neighbouring tiles.** A tree deep in a forest has all nine slots on blocked tiles; the men
   who cut it stand on the open edge beside it, and `ring_open` is what tells the two cases apart. That fallback is the owner's
   *"4 on corners and 4 on sides"*, so both of his descriptions - the eight neighbours and the eighteen-man ring - are true, in
   different ground.
6. **Walking out to the ring is a walk, not a placement** (D169 kept). The arrival that used to close the last third of a tile now
   closes seven tenths of one, one step at a time.

**A site is not a place to stand** (found by the skirmish). A foundation is walkable ground (D183), so a ring slot could land on
one - and a gatherer standing there and working is exactly the unit the empty-site gate waits for, so the site stalled for ever:
measured, a barracks frozen at **871 of its build ticks** with the man beside a bush. Ring slots on a building's footprint are
refused now. The same investigation found the second half of the story: with the ring, that barracks finishes at **~395 s** where it
used to finish before 380, and `config/skirmish.toml` pulled its only builder away at 380 s and left the site frozen. The step is
**460 s** now, and the numbers are in the file beside it.

**What is not claimed.** The radius is measured from six men and one source kind; the owner's mines and bushes are in the same file
but their front rows were not separated out, and a bush smaller than a tree might ring differently. The 0.084 spread is taken to be
men not yet settled rather than three different radii. And the *builder's* ring is not measured at all: no save with a site under
construction has been read, so D185's approach ring stands unchanged.

### D186 addendum - a place belongs to the nearest man, and the walk ends on the ring

**The defect.** Two men could end up on the same place. Measured twice: **0.001** tiles apart in the three-bush fixture and
**0.20** in the bush-and-tree one - the exact failure the owner had photographed in the original (two villagers at 0.00) and
the reason D181 exists at all.

**Three causes, found one at a time.**

1. **The place was chosen by distance, every tick, with no owner.** Two men walking at the same side of a bush kept swapping
   places and often chose the same one; nothing recorded who had claimed what.
2. **Owning it by rank made men walk the long way.** Giving each worker the slot at his rank among the men on the node
   (`(rank) mod 9`) did stop two men sharing, but the rank has nothing to do with where a man stands: measured, a woodcutter
   walking two tiles round the tree to the far side while a nearer place stood empty - and crossing the others on the way,
   which is its own collision.
3. **The walk stopped where the man entered the band, not on the ring.** `at_node` is true anywhere in 1.056..1.256, so the
   settle's "not there yet" test was satisfied at 1.256 - the band's *outer* edge. Two men on the same side stopped there,
   0.20 apart, both working, both technically on the ring.

**The fix, and what it is made of.**

- **Order by distance, own by nearest.** The ring is ordered by distance from the worker, and a place belongs to the *nearest*
  claimant: `(distance squared, worker id)` compared with every other gatherer of that node, so a man yields to anyone closer
  and two men exactly equidistant cannot both own it. No reservations, no new state, no per-tick growth.
- **Settle from either side of the radius.** "Not there yet" is "more than a fiftieth of a tile from the ring", inside or
  outside, and the arm gives that tick's step to the settle - so a man who entered the band at 1.256 keeps walking to 1.156.
- **A site is not a place to stand** (D186 proper, above): a ring slot on an unfinished footprint is refused, because a gatherer
  standing there is the unit the empty-site gate waits for.

Both previously `#[ignore]`d tests pass without a tolerance: `three_gatherers_on_one_bush_do_not_pile_up` and
`three_gatherers_all_work_the_same_square_on_its_ring`.

### D186 addendum 2 - the picker thought one man filled a square

`claimed_by_other` answers the picker's question *"is this source taken?"* - and it answered **yes if anybody at all was gathering
the node**, which is D182's one-worker-a-square living on in a second place after the ring replaced it in the first. Under a
nine-man ring a node with two men has seven places free, and the picker was treating it as full.

**Measured, twice.** A woodcutter whose patch ran dry, four tiles from a large patch holding **two men of nine**, ended with
`Task::Idle`. And the eleventh man ordered onto a square already holding **nine** - the case the cap exists for - found nothing at
all and stood still, where the right answer was the neighbouring square with one man of it.

**The rule now:** `on_node >= GATHER_RING_SLOTS`, counting both `Gather` and `Return` - a man carrying a load home still holds his
place, which is how `start_gather`'s own cap counts them.

**Pinned by `crates/sim/tests/gather_room.rs`**, and mutation-checked: putting `>= 1` back turns both tests red with the exact
measurements above (`Task::Idle`).

**D173's ignored test is un-ignored in the same change.** It asserted that a gatherer never moves further in a tick than its speed
allows, and was parked while the settle *placed* a worker on its standoff. The settle walks now, the test passes, and an
`#[ignore]` that no longer describes anything is a lie in the suite.

### D186 addendum 4 - the capacity is the ground, but our packing cannot yet hold the ground's number

**The nine is wrong, and the evidence is the owner's own game.** It came from his *"max 9 per each. 4 on corners and 4 on sides"*
and from the **9 + 7** split over two adjacent trees. `owner-bush2.aoe2spgame` carries **eleven** orders to one bush and he watched
*"11 villagers on it all foraging"*. Community sources repeat *"only 8 villagers can gather from the same sheep"* - a **carcass**,
where the men stand on the body itself - and nothing documents a cap for trees, bushes or mines; the AoE2 wiki's villager page
documents multiple *builders* (a diminishing formula) and no gathering limit at all.

**So the capacity is the geometry**: eighteen places on the 1.156 ring at two bodies (0.404) apart,
`2 * pi * 1.156 / 0.404 = 17.97`. That is also why it *feels* random to him - a crowd standing further apart fits fewer men on
the same ring, so the limit is the ground under them rather than a number in a table.

**Tried, and pulled back the same hour.** `GATHER_RING_SLOTS` went to eighteen with its slots at twenty-degree steps, and
`three_gatherers_all_work_the_same_square_on_its_ring` failed: **two settlers 0.243 tiles apart** on one bush, where two bodies are
0.40 wide. With slots one body apart, the settle and the collision pass fight - a man is nudged onto his place, the pass pushes him
off another man, he is nudged back - and the ring ends up packed more tightly than the bodies allow. At nine slots (0.79 apart)
there is room for that argument to resolve; at eighteen there is not.

**So nine stands, and not because it is right.** It is the largest cap this engine can currently place without visible overlap, and
that is a **defect in the packing, not a rule** - the game does eleven. The work to close it is in `docs/next-step.md`: a settle that
does not fight collision (a reserved place per man, not a re-decision every tick), after which the constant can go to the eighteen
the geometry gives.

**What is not claimed**: that the game's number is eighteen. Eleven is observed; eighteen is what the ring fits; the game may cap
somewhere between, and only a bigger crowd in his own save would say.

### D186 addendum 6 - a place on the ring belongs to the man (landed)

**The rule.** A gatherer takes a **place** on his node's ring when the order lands and keeps it until the task changes. The place
is `gather_place: BTreeMap<worker, (node, slot)>`; `standoff_point` will not give a reserved slot to anyone else, so a man who
walks up later cannot take a settled man's place however near he stands.

**Why it is a rule and not a heuristic.** The place used to be re-chosen **every tick** by nearest-claimant, and three separate
failures came out of that: two men **0.001**, **0.20** and **0.243** tiles apart, all of them supposed to be working. The owner
saw it in the renderer - *"several standing over the other"* - and had already described the behaviour as *"it feels rather
random"*. A place that belongs to an order cannot be stolen by a nearer arrival, and two men cannot swap it between ticks.

**A place that stops being a place is given up.** Two things invalidate one: a foundation laid on the ground it stands on, and
being moved off that ground. `place_stands_clear` re-validates walkable ground and the `foundations` footprints when the place is
read; `release_gather_place` drops it when `step_aside` moves the man. Without both, the ten-minute scenario's barracks stood at
`progress_ticks: 0` of 1000 for ever: the man kept walking back onto the footprint and the clear gate (D183 §4) could never
answer.

**The cost, measured where it can be measured.** In the acceptance run repaths go **226 -> 435** - but that run also ends
differently (player 2 is eliminated with the change, survives without), so its counters are not a measure of gathering. The
gathering-only fixture (`examples/gather_cost.rs`, eighteen men, six bushes, no war) prices it: **repaths 224 -> 229 (+2%)**,
**made_room 124 -> 63 (-49%)**, **men working 13 -> 16 of 18**. The rule costs almost nothing and puts three more men to work.

### D187 - a click selects the body that is drawn, not the tile under it

**The owner's rule, verbatim:** *"whenever I click on image representing a unit, it and only it gets selected. If units are
overlapping, the closes one to me gets selected. That also applies when unit is over the building (farm, passable part of town
center...) or in front of the building. If the unit is represented by sprite in the future, clicking on the transparent part
doesn't select unit."* And the report that found it: *"some villagers cannot be even chosen by clicking on the square
representing them."*

**What was wrong.** The click was decided **by tile**: `floor(position)` plus the footprint, while the body is drawn at
`position - 0.5` with its **base on the point** (D178). A villager at a tile corner is therefore drawn half over each of four
tiles and the picker recognised exactly one of them - and the half a player naturally clicks was a neighbour's, so the click
selected nothing. Measured: a click **8 px** left of the body's centre (over tile `(9,9)` while the villager stands at
`(10.0, 10.0)`) selected **nothing**; two villagers a fifth of a tile apart selected **nothing** where their bodies overlap.

**The rule now.** `body_contains(entity, camera, pixel)` is the one definition of where a unit is for input: a rectangle
`body_half` wide, rising `2 * body_half` above its point - exactly the rows and columns the renderer paints - and
`body_half(camera)` is now the renderer's own number, so a click and a pixel cannot disagree. `Selection::click` asks it first and
takes the **front-most** body by `depth_of`, the renderer's own "what is in front"; at equal depth the player's own wins, then the
id, so the answer never depends on snapshot order. A building is still its footprint (`covers`) - a town centre is drawn four tiles
wide - so a click on the farm where nobody stands still selects the farm, and a click on the villager on it selects the villager.

**Not covered.** `pick_at(world, tile)` (the window's right-click target) is still tile-based: an order is a place, not a body, and
whether *that* should follow the body is a separate question with its own evidence to gather.

### D188 - there is no cap: a place is a spot on the ring, and the geometry gives eighteen

**The owner:** *"I think that capping is pointless. Rule is rather simple: villager approaches designated resource to gather from,
if there is free place of a full diameter, villager started working. If no, villager looks for another one for the same resource. If
there is none, villager looks for another near source of the same kind."* Asked whether the cap was eighteen, eleven or nine after
his bush save showed **eleven** men on one bush against D186's nine, he answered that the count is not a rule at all.

**The rule.** `GATHER_RING_SLOTS` and its nine-place table are deleted. The ring is sampled every two degrees
(`GATHER_PLACES = 180`) and a place is the nearest sample that is walkable, not on a foundation, and **one body (0.40) clear** of
every other man and every reserved place. `ring_capacity()` derives the count from the circumference - `2 * pi * 1.156 / 0.40 =
18` - and is used only as a cheap "might there be room" bound; the settle decides sample by sample. Trig is a 46-entry quarter-sine
table, so no float enters a tick.

**Measured on landing:** repaths **435 -> 240** (-45%), `made_room` **151 -> 96** (-36%), entities 142 -> **139**, `World::step`
7.2 -> **6.7 us**, hash `ef7cdba9` -> `8999f1ba`. Ten men work one bush and eighteen work one tree at a body each.

**Recovered record.** This entry was lost with the rest of the log when `docs/decisions.md` was truncated from 385,025 bytes to
1,937 in commit `12b1e35`; the text above is written from `docs/reviews/2026-11-12-no-cap-and-pass-through.md`, which measured it at
the time. See `docs/reviews/2026-11-12-design-and-architecture.md`.

### D189 - your own units pass through each other on the way

**The owner chose it**, between the two ways out of the crowd problem: *"1"* - bodies may overlap while walking, collision at rest
only. His words for the need were *"I think that capping is pointless"* and the rule behind it, *"either villager fits in or
don't"*.

**The defect it fixes, measured.** With bodies solid, ten men sent at one bush cannot be delivered: the men behind cannot pass the
men in front, and the settle - which moves a body **directly**, not through the collision pass - either stepped inside a neighbour
(closest pair **0.144**, then **0.345**) or left a man stranded outside the band (**1.543**, **1.962** tiles from the bush, never
working). Whichever step direction was tried, one of the two failed.

**The rule.** A unit with **more than one waypoint left** ignores other **friendly** bodies: it takes the step it planned, and two
of the player's own villagers may overlap in transit. **The last step still respects a body**, because the final approach is where
order matters: without that bound a builder stacked on an idle villager and never reached its footprint (a house at **hp 1** after
2,000 ticks), and arrivals became a race rather than a queue. **Enemy bodies are unchanged**: `bound_to_reach` still holds two
players' units apart, which is the pass D131 was about.

At rest, the gather ring keeps men apart by reservation rather than by collision: a place is a point on the ring one body (0.40)
clear of every other place, and the settle walks a man exactly onto his place (`off_place` is any deviation, not a tolerance - a
0.125 tolerance let two men finish **0.376** apart, and 0.025 let them finish 0.345).

**Measured on landing:** ten men work one bush (nine was the ceiling), closest pair of places **0.408**; eighteen work one tree at a
body each, which is `ring_capacity()`; the acceptance hash and counters are in the review.

### D191 - tile geometry: the border is what a villager stands against

**The owner:** *"Do we know coordinates of the tiles and their corners? If not, it should be added. Because all work of villagers
depends on that (maybe except taking meat from animals) ... Villagers building or gathering resources (except meat from animal)
must be aligned with borders of the tiles, not center of it."* And on size: *"center stays at least half the diameter of the circle
(however big that circle is)."*

**Why the centre was wrong.** Every standoff so far measured from the tile's **centre** - 0.70 (D169), then 1.156 (D186/D188). His
saves show men at **0.70** from a resource tile's centre, which is `0.5 + 0.2`: a villager's circle **touching the tile's border**.
Two edges meeting put the same circle at `0.707 + 0.2 = 0.91`, and his saves carry men there too (0.76-0.99). The border is the
thing the circle touches; the centre was a proxy that happened to be right only for one particular row.

**The new module** (`crates/sim/src/tiles.rs`, public):

- `tile_corners(tile, scale)` - the four corners in raw map units;
- `distance_to_tile(point, tile, scale)` - zero inside, the distance to the nearest border point outside. "The circle may never
  overlap the tile" is then one comparison: `distance >= radius`;
- `touching_places(tile, radius, spacing, scale)` - the points where a circle of `radius` touches the tile without overlapping, at
  least `spacing` apart, in a **fixed order** (the simulation is hashed; a geometry that reorders itself is a desync waiting).

**The capacity falls out of it, for the first time without a number in a table.** Measured with the module: a villager (radius 0.2,
spacing 0.4) gets **eleven places** on one tile - the outline is `4 + 2 * pi * 0.2 = 5.26` tiles and a body takes 0.4 - a unit with
radius 0.4 gets **six**, and one with 0.05 gets forty-one. The owner's *"max 9 per each"* and the ten men his save shows on one bush
both sit inside eleven.

**Not yet switched.** The gather and build standoffs still measure from the ring and the centre; this commit adds the geometry and
its tests only, because moving the standoff changes every gather test's band and is its own slice (`docs/next-step.md`).

### D192 - the standoff is the tile's border, not a ring round its centre

**What it says.** A worker stands with **his circle touching the node's tile**: `distance_to_tile <= radius`, places `spacing` apart
along the outline, chosen nearest-first. His saves measure it - men at **0.70** from a resource tile's centre (`0.5 + 0.2`) and
**0.99** where two edges meet - against the 1.156 ring D186 read from the row *behind* them. The capacity follows from the geometry
(**eleven** places for a villager) and the outer band disappears: no place on the tile, then another resource within four tiles.

**It landed after a deadlock it exposed, and the deadlock was a movement rule** (addendum, 2026-11-13). A villager told to step off a
fresh foundation froze **1.011 tiles** from the tile he was sent to for 2,000 ticks while an idle neighbour stood on it, and the house
stayed at **hp 1**. Instrumenting `collide` named the rule: *"the blocker stands on the tile the order asked for"* fired on a
**friendly** body. That clause is now **hostile-only** - it still keeps two players' units apart (D131), and it no longer treats a
friend as a wall on the last step, which is what D189 already said in words.

**Measured, `main` at `b7d92f2` against the landing:** `made_room` **96 -> 0**, `World::step` 7.2 -> **6.7 us**, `Snapshot::of`
103.4 -> **89.3 us**, entities at 1,800 s 139 -> **142**, hash `8999f1ba` -> `91572e72`, and **repaths 240 -> 409 (+70%)** - the one
number that moves the wrong way, recorded here rather than smoothed over; the match still ends `players=1`. See
`docs/reviews/2026-11-13-the-standoff-lands.md`.

### D193 - a test asserts the rule and quotes the measurement

**What it says.** A test states the **relation** (`distance_to_tile(worker, tile) <= radius`) and may quote the measurement that
motivated it in its failure message. It does not assert the measurement. A measurement stays in a test only when the measurement *is*
the claim - a save-game fixture saying what the owner's file holds.

**Why, measured.** When the standoff moved from a 1.156-tile ring to the tile's border (D192), **eleven gather tests went red at once**,
every one of them asserting the number rather than the rule. The engine was *more* correct and the suite said it was broken, and rewrite
blame lands on whichever side is cheaper to change - which is never the rule. Recorded in SPEC §11 with both forms side by side.

### D194 - an error says which kind of failure it is

**What it says.** `SimError::class() -> ErrorClass`, three classes by **who is at fault**: `Refused` (38 variants - a legal order the
world will not take now, here, or with what the player has), `BadRequest` (18 - the command names something that is not there, is not
yours, or does not parse) and `Internal` (3 - fixed-point overflow, geometry overflow, a displacement with no legal spot). The match has
**no wildcard arm**, so a new variant does not compile until it is classified.

**Why.** Fifty-nine variants is right for a simulation - each names exactly what went wrong - but a caller could not tell "the player
cannot do that" from "the engine is wrong" without matching all of them, so every UI path either swallowed errors or printed internals.

### D195 - the next step is one step

**What it says.** `docs/next-step.md` is capped at **16,384 bytes**, must start with `# OPEN`, and `tools/next_step.py --split` moves
everything after the first block to `docs/history/next-step-<date>.md`. Enforced by `make check` and the pre-commit hook.

**Why, measured.** The owner's rule is "compact first: persist state to files so a fresh session resumes without the conversation". By
2026-11-12 that file was **350,038 bytes** - twenty times the largest source file - and this session read it at 101 KB and growing
before it could start. One run moved **348,807 bytes** out and left **1,230**.

### D196 - the movement decision is a value, and a hostile body at rest is still a body

**What it says.** `collide` returns `(dx, dy, StepDecision)`: thirteen named outcomes (`Idle`, `Take`, `TakeOverlapping`, `TakeAtRest`,
`Slide`, `MakeRoom`, `GiveWay`, `GiveWayOwnTile`, `WaitGround`, `WaitTarget`, `WaitGoal`, `WaitOwnTile`, `WaitBlocked`). The per-body
record lives on `Entity::last_step` (not in the state hash) and is written only while **tracing** is on (`World::trace_steps()`), with
flat counters in a `[u64; 13]`.

**Why.** "Why is this man standing still" had no value to print, test or compare - it took two hours of printing from inside the
function, twice in one week, and the second time the answer was a rule (`WaitGoal`) firing on a **friend**.

**The rule it found, measured.** Two tests written for this decision exposed a second defect: a walker passed an **idle enemy** at
**0.237 tiles** where two 0.2 radii need **0.4**, because rule 8's "a body at rest is not a wall" exemption - written for **resource
nodes**, which carry no radius - was being applied to units too. The exemption now applies to nodes, buildings and friends only; a
hostile unit at rest is bounded by the reach.

**Cost, measured.** Recording costs **0.4 us** of a 6.7 us tick (6.6 -> 7.0 at 128 bodies) and is **off by default**; a run that wants
the explanation pays it. A `BTreeMap` keyed by id was tried first and cost **1.8 us** on its own.

### D197 - repaths are counted by reason

**What it says.** `repaths_by_body()` and `repaths_by_ground()`, split at the single increment site using the `Repath.body` the struct
already carried; printed by the headless as `repaths_body=`/`repaths_ground=`.

**Why, measured.** D192 landed with repaths **240 -> 409** and no explanation. The split answered it: bodies stopped forcing re-plans
altogether (**31 -> 0**) and every added re-plan is **ground** (122 -> 166) - men standing on a tile's border instead of a ring graze the
node's blocked ground and re-plan, one every 217 ticks of 36,000.

### D198 - the scenario language and the runners are not the engine

**What it says.** `crates/script` (`ruage-script`) holds the scenario language, the scripted AI, the rule runner and the personality
loader; `crates/harness` (`ruage-harness`) holds the two binaries that drive them (`ruage-headless`, `ruage-batch`). `ruage-sim`
depends on `ruage-data` and `ruage-fixed` and on **nothing else**; `ruage-script` depends on `ruage-sim`; `ruage-harness` depends on
both. The dependency arrow never points back into the engine, which is what D17/D18 asked for in words (*"engine knows kinds, not
particulars"*) and what 1,656 lines inside `sim` contradicted.

**What the move found**, which is the reason to do it rather than describe it:

- Three `World` queries - `start_villager_unit`, `start_town_center_unit`, `is_free_ground` - lived in `script.rs` while **the engine
  itself** called all three (`metrics`, `movement`, `sites`, `shares`). They moved to `crates/sim/src/roles.rs`, where they belong.
- Two private methods (`nodes_of_slot`, `tile`) are needed by the scenario language and are now public, documented as *"public for the
  scenario language"* - a real API widening, which is what a layering break looks like when it is made visible.
- `crates/sim/tests/headless.rs` could not stay: `env!("CARGO_BIN_EXE_ruage-headless")` only exists in the package that defines the bin.
  It moved with the binary.

**Behaviour is unchanged, measured**: the acceptance run from the new crate prints `tick=36000 entities=142 players=1
hash=91572e72c33cee93 repaths=409 made_room=0` - the same hash as before the move - and the workspace gate is green at **183 test
binaries** with clippy and fmt clean.

### D199 - the standoff is the touch, measured on the owner's own saves; D186's ring is withdrawn

**What it says.** A gatherer stands with his body **touching the node's tile**: **0.70** from its centre at an edge
(`0.5 + 0.2`) and **0.94** at a corner (`0.5*sqrt(2) + 0.2`). That is D192's rule, and this is the measurement that
verifies it against the game rather than against a screenshot.

**The evidence.** The owner sent `tree`, `mine` and `bush` saves (eight men on each, game paused) and a screenshot on
2026-11-13. Each save's nodes are found without a guess - exactly three tile centres appear nine times each in the
coordinate stream: `(76.5, 61.5)`, `(82.5, 66.5)`, `(87.5, 71.5)`. Measured in the band **0.60-0.98**, every node carries
**seven** bodies at `0.641 / 0.701 x3 / 0.935 x3` (tree), `0.641 / 0.702 x3 / 0.935 x3` (mine),
`0.702 x3 / 0.760 / 0.936 x3` (bush): means **0.793, 0.793, 0.811**, closest pair **0.661-0.663**. One body per save sits
at 1.054, just outside, unexplained and recorded as such.

**What it withdraws.** D186 read a **1.156** ring from the owner's earlier save, and
`docs/reviews/2026-11-12-the-mine-and-the-ring.md` repeated it for three nodes (means 1.114-1.156). Both were measuring
**the tile grid**: the windowed scan reads every byte offset as a float pair, so round a one-tile node it finds the four
cardinal neighbouring centres at exactly **1.000** and the four diagonals at exactly **1.414**, and the old tool's default
band `1.05-1.20` sat on precisely that row. **D186's ring is withdrawn**, and with it the "eighteen slots" capacity that
was derived from the 1.156 circumference. The finding D186 got right is the one the owner stated: tree, mine and bush
behave the same.

**What it does not settle.** The **capacity**. The game shows seven to eight bodies at **0.66** spacing on the touching
envelope; D192's geometry gives eleven places. Eleven is on the permissive side of the observation, so no test fails, but
the two numbers are not the same claim and the game's spacing (0.66, not one body 0.40) says the limit is the ground, not
a constant. Recorded in `docs/next-step.md` rather than resolved.

**Also fixed, test-first, because the measurement exposed it.** `tools/savegame_positions.py`'s `ring_fit` raised
`ValueError` on a band holding exactly one body (`min()` of an empty sequence), which is the commonest case there is; and
its default band was the artefact band. Five tests in `tools/test_savegame_positions.py` went red
(`FAILED (errors=3)`) before the fix; the band is now **0.60-0.98** and the docstring says why 1.00 is a tile centre.
Review: `docs/reviews/2026-11-13-the-standoff-is-measured.md`.

**Commits us to.** Any ring measurement quotes its band and its window, and a band that matches `0.5`, `1.0` or `1.414`
exactly is a reason to suspect the tile stream rather than a body.

### D200 - collisions are owner-blind; a friend is a wall, and the winner is not the match outcome

**The ruling** (owner, 2026-11-13): *"Adopt original game behaviour. Don't make difference of friend or foe as in original
game."* And, on what to do when fidelity costs the acceptance run: *"For now, we're just testing functionality... functionality
implemented items is enough"* - match resolution is not the acceptance test at this stage.

**What the binary says, and it agrees.** `FUN_008a4360`'s collision block classifies each collision by **unit id against the
order's target** and nothing else; there is no player, team or alliance test anywhere in it, and the collision query is issued
with an owner sentinel rather than the mover's player (`docs/movement-in-the-binary.md`, read 2026-11-13). So a friend is a
non-past collision exactly like an enemy, and the original's answer is to **repath** around it, not to pass through it.

**This withdraws D189's point 1** ("your own units are not walls"). That rule was chosen to make a crowd deliverable, not
because the game does it, and it let a walker pass through a friend for 300 ticks (measured: **0.307 tiles at tick 79**
against the 0.4 two footprints need, test `collision::a_walker_never_ends_a_tick_inside_a_friend`).

**Built, measured and parked** on `d189-owner-blind` (`eaa2d98`): `collision` 4/4 and `evasion` 6/6 green, `made_room` 0 -> 24,
repaths 409 -> 45, and the acceptance match ends **`players=2`** where `main` ends `players=1`. Per the ruling above that is
**not** the blocker any more. The blocker is functional: our gather places are spaced **one body apart (0.40)**, which is now
exactly the reach that separates bodies, so a man cannot stand on the place next to a standing friend -
`three_gatherers_on_one_bush_do_not_pile_up` delivers 1 of 3. The game spaces its working men at **0.661-0.663** (owner's
saves, D199). Places must be spaced by more than the reach; that slice lands the rule.

**Full measurement and the landing plan:** `docs/reviews/2026-11-13-owner-blind-collisions.md`.

### D201 - content: our own campaigns only, names by three lists, all prose rewritten

**What it says.** The project ships **only content authored for it**. The original game's campaigns and scenarios are read
in place from the owner's install and played locally, and are **never** converted into a shipped artifact, committed, or
quoted in public docs - the owner's words: *"The only campaigns and scenarios to be shipped by the project are the one made
for it. Never the ones taken from original game. 100%."*

Names follow three lists (`docs/licensing.md`): **keep** historical ethnonyms and generic unit nouns (Persians, Huns,
Teutons, knight, archer - neither authored nor trademarked); **rename** what AoE2 coined (the "Woad Raider" is the clearest);
**rewrite all prose** - unit, tech and building descriptions, tooltips, campaign dialogue - because sentences are the thing
that is actually copyrightable. The medieval vocabulary is the register; 0 A.D.'s "Achaemenids" is antiquity and not ours.

**Why it matters now rather than at publish time.** The rule decides what the string tables and the content pack are allowed
to contain, and the string table is the first thing that would otherwise be copied without anyone thinking about it. It also
settles that a `.scx` reader is publishable while a converted `.scx` is not - the code is ours, the scenario is theirs.

**Commits us to.** No campaign data in `config/` or any tracked path; converted scenarios only in `build/`; descriptions
authored, never imported from the dat's string tables; a rename list maintained with the content pack; and the project's
presentation (title, logo, README, no "HD Edition") staying clear of the trademark.

### D202 - the target shape is "A-prime": self-contained sources, the install as the only input

**What it says.** The owner's goal, as he put it: *"the sources to contain everything so that I can install it on top of
Age2HD and nothing else would be needed"* - with only the game's sounds and graphics reused, and those from the install.
That is **A of the save-reader options, widened**: the engine, the importer and the content schema all live in this
repository, `--load` is the engine's own, and a user supplies one thing - their copy of the game.

Five pieces, and the order they become true:

| piece | state |
|---|---|
| engine builds and runs from a clean clone | **true today** (`cargo build` needs no game data) |
| `--load` in the engine | starts as **B** (tool-side converter behind `World::from_state`), becomes **A** once the reader is written from `docs/save-format.md` alone - gates in `docs/licensing.md` |
| the importer lives here, not in another repository | not yet (D12 moved it out); returns as a GPL-3.0 directory under `tools/`, or as our own Rust reader later |
| content: names by three lists, all prose rewritten, campaigns only ours | the rule is D201; the content work comes with the authored pack |
| assets read in place; ours later | true today (D21) |

**The one thing "A-prime" cannot include, and does not need.** *"The data generated once, reviewed, and made part of the
game"* is the only sentence of the goal that does not fit: the dat's numbers are a substantial extraction of their database
(Database Directive Art. 7) and committing them is the one act no packaging choice repairs. **It is also not required by the
user experience** - "install on top of Age2HD, nothing else needed" is satisfied by generating the tables at first run from
that very install. Committing them would only matter for running *without* the original game, which is the one thing the
goal explicitly keeps.

**Commits us to.** `World::from_state` as the seam between B and A; no dat-derived table in a tracked path (D8); the importer
back in this repository by the time a stranger is expected to build; and the target recorded here so a later session does not
re-open "which option did we pick".

### D203 - a rule asked for twice goes into the tree, and a question the owner must answer arrives after the evidence

**What it says.** Three standing instructions the owner gave on 2026-11-13, written down because a rule that lives in a
conversation does not reach the next thread:

1. **Every step is bracketed by a compact and an adversarial review.** The compact's first half is the feature's *prior
   implementation* - where it lives (`file:line`), which tests pin it, what it measures today - because a plan written
   over an unnamed implementation cannot say what it is changing. The review comes after the change, covers performance
   **and** stability, and carries a measured number. `AGENTS.md`'s four-step routine is that rule; its step 1 now says so.
2. **A decision the owner must make is the last step, not the first.** The order is: the original game first (run it,
   watch it, read its own data, read the binary), then the adjacent open-source projects and published sources
   (`docs/prior-art.md`, the wiki, the manual), and only then the question - which carries what each attempt found,
   including the attempt that found nothing.
3. **The question does not expire.** It goes in the reply with the turn ended, and it is recorded as a `U<n>` row in the
   U-register before that, so a reboot at 04:00, a new thread or a week's silence cannot lose it. A `clarify` form is for
   a choice that blocks the current step; `agent.clarify_timeout` is now `0`, so a form that is open cannot expire
   unanswered mid-conversation - but a form holds the turn, so it is not where a decision question goes.

**Why.** The owner asked for it in these words: *"I always want to compact prior implementation of feature and then
adversarial review on performance and stability after that. When my decision is needed, I always want original game to be
checked how it deals with the problem, if no solution is found then I want other OSS projects and other sources in the web
to be checked and then provided with the question which never timeouts with information gathered."* Each of the three has
already failed once here: a review was skipped twice before `tools/review_check.py` existed, a `clarify` form came back
unreadable and expired unanswered (`references/docs-and-decision-index.md`), and a question asked inside a thread was gone
with the thread.

**The enforcement, and what it does not cover.** Compact: `tools/next_step.py --check` refuses a `next-step.md` over
16 KB or not headed by one open step, and `.githooks/pre-commit` runs it. Review: `tools/review_check.py` refuses a
commit that changes `crates/`, `config/`, `tools/`, `Age2HD/` or `build/` without a staged note holding a measured
`## Performance` and a `## Stability`; the hook runs that too. Question: `tools/decisions.py --check` refuses an open
`U<n>` with no area in `docs/decision-areas.toml`, and this entry files itself (`D203 = "Project process"`). **Nothing
here stops `--no-verify`**, and no script can tell whether a review is honest - only a reader can. The gate's claim is
the narrow one: omission has to be deliberate.

**Commits us to.** A `clarify` form is never where a decision question goes; a question worth the owner's attention is
worth the evidence hunt before it, and a hunt that found nothing is part of the question rather than a reason to skip it.


### D200 - friends are solid: the owner's rule, and the crowd it has to carry

**What it says.** A body blocks whoever it belongs to. The collision block in the original has no player, team or alliance
test anywhere in it and its query passes an owner sentinel (`docs/movement-in-the-binary.md`), and the owner ruled on
2026-11-13: *"Adopt original game behaviour. Don't make difference of friend or foe as in original game."* So `hostile` is
gone from `collide`; a friend blocks, slides, gives way or re-paths exactly as an enemy does. **D189's "your own units are
not walls" is withdrawn** - it was chosen to make a crowd deliverable, not because the game does it.

**The rest of the rule, as built.** A body at rest is nudged only after **forty refused ticks**, only when it stands on the
walker's own tile or his next waypoint, and never when a gather order names it. A place a body stands on is **stale** - a
held place with a body inside the pair's reach is released and re-chosen. Placement is **comfortable 0.66 first, packed
0.40 only when the ring refuses**, with a node-level re-lay at most once per twenty ticks. A `Return` aims at the walkable
yard, not the building's blocked anchor. An order a standing friend blocks **finishes where the man stands**.

**The rule that made it land.** Rule 4 - a body on the mover's **own tile** - used to wait, and to give way, and nothing
else. At one body apart the give-way bound returns zero *and* rule 4 returns before rule 7's repath, so a man could neither
move nor ask for a route: measured, seven villagers on two tiles round one berry bush, five of them `WaitOwnTile` for the
whole match, and the acceptance bank at **0 food from 400 s to 2,400 s**. A plan cannot leave the tile it starts in; a
**slide** can, and a sideways step increases the separation the bound protects. Rule 4 now tries the two axis steps and then
the two perpendiculars, taking the first that keeps the pair outside the reach and lands on a clear tile.

**What it costs, measured.** `--seed 7 --ticks 48000`: food `4 / 14 / 484 / 594 / 1226 / 1396` against `main`'s
`56 / 397 / 667 / 388 / 650 / 690`, wood 1215 against 2019, `entities=143 players=2 hash=2d93236ebb16961b repaths=373
made_room=140` against `entities=128 players=1 hash=ace125e60ea99b60 repaths=579 made_room=0`. The enemy is **not
eliminated by 40:00** - town centre at 1451 of 2400 hp with four spearmen in `Attack` on it, where `main` had razed it with
eleven soldiers. `players` is the highest surviving player id, so `players=2` means "the enemy still stands", not "the match
is broken": with friends solid, an army arrives later and the match is slower, which is the owner's rule working as asked.

**Commits us to.** A crowd fixture at one node must always be able to bank - the guard is
`crates/sim/tests/acceptance_economy.rs`, which runs the acceptance world itself and was red on the first merge. And any
future "the match no longer resolves" report quotes the **town centre's hp and the attackers on it**, not the `players`
number alone.

### D204 - no pushing: a man who cannot get his square looks for another tile of the same resource

**What it says.** The owner, 2026-11-13, playing the D200 merge: *"I don't like villagers pushing other villagers. I want
instead to look for another free place at next tile with same resource instead of fighting over the place at the same
tile."* So displacement is **not** the answer to a contested work place. A worker who cannot stand his square at a node
does not shove the man who holds it: he takes the next node of the **same resource column** that has a free place, and only
when there is none does he stand down and wait.

**What it replaces.** `push_the_standing` (D200) - a body at rest displaced a twentieth of a tile after forty refused
ticks. It stays in the tree only as the *last* resort for a body that has no work at all (a man blocking a corridor); it may
no longer be the engine's answer to a crowd at a resource. This is also the owner's rule from D182, now applied on the
contested path as well as at order time: *"try another nearest resource of the same kind"*.

**Why it matters here.** The nudge was the only thing that let a crowd of seven on one bush move at all after D200, and the
acceptance match's food economy depended on it; with displacement gone, the crowd policy has to be **re-targeting**, which
is the shape the owner asked for and which the AI layer will want anyway.

**Commits us to.** A contest at a node resolves by finding another node, never by moving a man off his square; the fixture
is two men ordered onto one occupied square (the second arrives at a different node of the same column, or waits when there
is none), watched red first. Until it is built, `push_the_standing` is a **known contradiction** with this decision and
`docs/next-step.md` carries it as step 1.

#### D204 addendum - built: the refusal is counted, the next tile is taken, and nobody holding a square is moved

*2026-11-14. Branch `d204-no-pushing`, one commit. Two tests in `crates/sim/tests/no_pushing.rs`, watched red first:*
`a_man_who_cannot_get_his_square_takes_the_next_tree` *and* `a_crowd_at_a_tree_is_never_pushed`.

1. **The count.** `World::square_refused` brackets a worker as `(node, refused ticks in a row)`, folded in after `decide`
   (`count_refused_squares`) because `decide` reads the world and never writes it. After `STALLED_TICKS` - the wait the
   displacement rule used to spend - he asks `nearest_like_free` once, and is re-tasked through the existing
   `work.retasks` path. When the search finds nothing the count restarts at zero, so a second look is another
   `STALLED_TICKS` away rather than every tick, which is the cost D182 measured at **35 s -> 400 s**.
2. **The two gates in `push_the_standing`.** A man **refused a square at a node** does not shove anybody (he has the
   D204 answer instead), and a body **with a gather order** is never moved: he is working a resource or walking to one.
   Everything else keeps the nudge - a bystander in a doorway, a soldier in a corridor - which is D204's own scope.
3. **What the nudge was really doing**, measured on `main` over the acceptance run before any code moved: **310
   displacements, 232 on builders, 76 on loaded carriers walking home, 2 on idle bodies, 0 on gatherers**. So the
   "villagers pushing villagers" the owner saw in a match was at building sites and drop sites, and the resource case
   was not a shove at all but a man **standing in `NoRoom`**: **45,359 refusals** over the run, **18 distinct
   villagers**, one refused on **1,782 consecutive ticks**.
4. **The fixture is the owner's own picture**: every square of one tree's ring taken by a standing body, one more man
   sent at that tree, and a second tree of the same column five tiles east. On `main` he stands beside the crowded tree
   with **progress 0** for 2,000 ticks and one member of the crowd is shoved **0.15** tiles off his place; on this branch
   he works the far tree and nobody moves.
5. **Acceptance, measured**: `entities=143 players=2 hash=2d93236ebb16961b repaths=373 made_room=140` ->
   `entities=160 players=2 hash=c3b10f4b09e17a43 repaths=324 made_room=147`; food `4/14/484/594/1226/1396` ->
   `45/49/268/120/660/450`; wood `40/75/300/415/805/1215` -> `80/96/135/75/395/185`; units `20/29/33/37/37/37` ->
   `20/31/35/44/46/48`. **Eleven more units and a smaller bank**: the match spends what it has instead of banking it.
   Wall time **61.3 s -> 30.3 s**, and the like-for-like tick `7.4-8.4 us -> 6.5-6.8 us`. The full record, including the
   gate that was written first and killed by `gather_motion`, is `docs/reviews/2026-11-14-d204-no-pushing.md`.
6. **Withdrawn**: "a man with a gather order never nudges anybody" - the simpler reading of the owner's sentence. It took
   `a_gatherer_walks_round_a_friend_standing_in_his_path` red (*"never reached the bush in 1,500 ticks; closest 1.84"*),
   because D204 keeps the nudge for a body with no work at all. The rule is scoped to the shover being refused a square,
   and to the body moved having a gather order.

#### D204 second addendum - the carrier half is parked: the counter does not measure the rule

*2026-11-14, branch `d204-carriers` from `46cec66`. No production code changed; the fixture is
`crates/sim/tests/carrier_no_pushing.rs`, `#[ignore]`d, and the record is
`docs/reviews/2026-11-14-d204-carriers.md`.*

The slice was "extend the gate to `Task::Gather | Task::Return`", watched red through `World::made_room() == 0` in a
crowd that only gathers and carries home. **That observable does not witness this rule**: `made_room` counts the
*walker's own* make-room step (`movement.rs:400` -> `step.rs:1317`, C1b), while `push_the_standing` moves the standing
body and counts nothing. Measured: the seven-villager fixture reads `made_room = 7` on `main` and **7 again with the
gate extended**, and the acceptance run is identical both ways (`hash=c3b10f4b09e17a43 made_room=147`, 30.2 s). No test
can be watched green, so none was built.

**And the displacements it was written for are gone from the match.** Printing the task of every body
`push_the_standing` moved over the acceptance run: `46cec66` moves **161** - 147 `Build`, 12 `Attack`, 2 `Idle`,
**0 `Return`**, 0 `Gather` - while `7d00be8` (pre-D204) reproduces the recorded **310** = 232 `Build`, **76 `Return`**,
2 `Idle`. The 76 carriers were a property of the pre-D204 match; the one D204 left behind has none. Mechanically, all
**22,253** `Return` candidates the run considers are already skipped by `posted || (busy && !jammed)`: a carrier walking
home is busy, and a carrier who is jammed still holds his reserved ring place.

**What would reopen it**: a `Return` carrier who is jammed **and** holds no gather place. No fixture reached one. That
is the body a future test has to find, watched by position, not by `made_room`.

### D205 - right of way is the lower entity id, and the loser pauses 0.25-1.50 s on its own path

**What it says (owner, 2026-11-14: *"1"*).** Copy the original's collision tie-break rather than invent one. Read out of
`AoK HD.exe` byte for byte (`docs/movement-in-the-binary.md`, last two sections): when a step's collision list holds a unit
that is not the order's target, the mover compares **`[vtable+0x2b4]`** on both units - a virtual call whose move-object
implementation returns `*(int*)(this+8)`, the object handle - and the unit with the **higher** value is told to pause:

```
008a80ed call [edx+0x2b4]   ; self  -> esi
008a80f9 call [edx+0x2b4]   ; other -> eax
008a80ff cmp  eax,esi / jle ; pause only when other > self
008a8107 mov  esi,[ebx] / 008a8141 call [esi+0x2b0]   ; other->setPause(rand*1.25/32767+0.25)
```

`+0x2b0` = `0x8aa6d0`, whose whole body writes **the current path's pause timer** (`path+0x28`) and nothing else. So the loser
**keeps its path**, advances no waypoint, does not move, and resumes where it left off when the timer expires; the winner
repaths or stops. The duration is uniform **0.25-1.50 s** (`rand()*1.25/32767 + 0.25`).

**What it replaces.** The three inventions in `collide`: the give-way mutex (**rule 9** and its own-tile twin), the axis/
perpendicular **slide** (rule 5 and the own-tile slide), and the **nudge** (`push_the_standing`). A unit-level collision is now
answered one way: *the lower id proceeds, the higher id pauses*. D204's re-target at a contested **resource square** stays - it
is a placement rule for workers, not a collision answer - and so do the settle, the reserved places and the gather ring.

**What it costs, accepted with the decision.** Our `EntityId` is a monotonic spawn counter, so "lower handle" means **the older
unit wins every contested tile**; the owner chose the mechanism knowing that. The pause is drawn from **our seeded RNG**, never
a global (the original's own `rand()` wrapper logs *"not considered sync safe"*), which means the RNG stream is consumed at a
new place and every existing replay of a fixture with a collision changes its trajectory - determinism is preserved, the old
hashes are not.

**Why it is worth it.** A pause is bounded by a countdown, so the pair can never stand for ever: the 2026-11-13 freeze (seven
villagers on two tiles, `WaitOwnTile` for the whole match, the acceptance bank at 0 food) is impossible by construction rather
than by another exemption. And it removes state: no `refused` list, no per-route `gave_way` flag, no stall counter at all for
units.

**Owed, test-first.** (1) Two units meeting head-on: the higher id is paused for 0.25-1.50 s, keeps its waypoint list, and
resumes; the lower id gets through. (2) The owner's screenshot: a loaded carrier and a gatherer meeting on one tile both reach
their destinations. (3) The acceptance match still banks food (`crates/sim/tests/acceptance_economy.rs`). Until (1)-(3) are
green on a branch this decision is **built, not landed**, and `docs/next-step.md` carries it as step 1.

### D206 - speed is constant; formations are the only speed adjustment, and they are later

**What it says (owner, 2026-11-14).** *"units do not accelerate and decelerate under normal conditions. Maintenance of the
formation is the only speed adjustment."* So the engine keeps its current model - a unit moves at its dat speed divided by
the tick rate, from its first tick to its last, with no ramp - and the **one** place a speed ever varies is a formation
keeping station, which is not built and is deferred (*"Formations later"*, same message).

**What it withdraws.** The owner's own earlier instruction in the same sitting (*"Add acceleration"*) and the reading of
`move_obj.cpp:3069-3145` ("Maximum forward velocity", `getMaxVelFwd`, "vel < 10.f", "cap unit max vel") as proof that the
original ramps. It does have acceleration **plumbing** - `007ff920` integrates `acceleration`, `velocity` and position, and the
maximum-velocity function has a pause branch - but the owner, who has played it for years, says normal movement is at a constant
speed and only formation maintenance changes it. **The plumbing stays unexplained rather than refuted**: recorded as a register
row, not as a fact either way. What would settle it is a recording: trace one unit's per-tick displacement across a start and a
stop and look for the ramp.

**What it protects.** SPEC §6.1's exact row survives - *per-tick displacement is `speed / tick_rate`, enforced by
`Fixed::div_exact`* - so no tolerance needs restating, no fixture hash moves for this reason, and the arithmetic stays linear.
That is worth saying out loud because the acceleration slice would have cost all three.

**Commits us to.** When formations arrive they carry the only speed modifier: a unit behind its station moves faster to
regain it (the owner's own memory: *"units that somehow fell behind slightly speed up to catch up"*), a unit ahead does not
slow below its speed. That rule belongs with the formation, not with `Entity::speed`, and it may not be implemented as a
per-tick acceleration of individual units.
