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
