# The dependency tree: what is built, what it rests on, and the order the rest goes in

*2026-11-14. Written because a slice kept being started and then found to depend on something unbuilt. The rule this
file follows is the project's own: a status is a **claim with evidence**, so every row carries the file, the line, the
test or the measurement that decides it — and a row whose evidence is a grep says so, so the next session can re-run the
grep rather than trust the table.*

Read this **after** `SPEC.md` (§7 procedure, §13 changelog) and **before** choosing a slice. It answers one question —
*what must exist before this can be done, and is it there?* — and one only. It is not a changelog: `SPEC.md` §13 already
is. It is not the register: the U-rows stay in `docs/mechanics/README.md` and the binary rows in `docs/behaviour-diff.md`
and are **cited here by id**, never copied.

Two things are deliberately *not* in this file: an estimate in hours (this project prices work by measurement, not by
arithmetic), and a claim that the order below is the only order. It is the order the *dependencies* allow, with the
cheap live defects first.

---

## 1. How to read it

**Status.**

| Tag | Means |
|---|---|
| **DONE** | Tested, green, and the test bites (a mutation was tried in the slice's own review). |
| **DONE-ish** | The mechanism runs, and something it *feeds* is still a placeholder. The hole is named in column 5 and repeated in §3. **This is the tag that causes the rework.** |
| **PART** | Runs for some inputs; the rest is refused or absent, and the refusal is named. |
| **OPEN** | A `U<n>` row or a register row — a question the code cannot answer today. |
| **MISSING** | No code at all. Named so nobody spends a session discovering it. |

**Edges.** `needs →` names what must exist first. A row with no dependency can be started today; a row whose dependency is
`MISSING` is a trap until that is paid.

**Ids.** `F` foundation, `S` simulation, `T` truth (a data or semantics debt), `M` a missing mechanic, `P` product.
`U<n>` and `D<n>` keep their existing ids so this file never becomes a second numbering.

---

## 2. Foundations — all shipped, and nothing may be regressed

| id | Item | Status | Evidence |
|---|---|---|---|
| F1 | Exact fixed point, no floats in sim state | **DONE** | `crates/fixed`; `crates/sim/tests/no_float.rs`; hard invariant "fixed-point only in simulation state" |
| F2 | Strict config loading, `-1.0` = unresolved, refuse rather than default | **DONE** | `crates/data`; `docs/config.md`; every loader has a "refuses" test |
| F3 | Content pack as one identified thing, hashed for the handshake | **DONE** | `ContentPack::load` `crates/data/src/pack.rs:98`; `identity()` `:89` (D19) |
| F4 | `overrides.toml` layer, plan-then-apply, logged | **DONE** | `crates/data/src/overrides.rs`; SPEC v0.88–v0.89 |
| F5 | dat → TOML importer, in its own repo | **DONE (external)** | `ruage-tools` (D12); `config/gamedata/` is generated and not committed (D8) |
| F6 | Tick loop, seeded RNG streams, state hash, deterministic order | **DONE** | `crates/sim/src/world/step.rs`; `tests/seeding.rs`; §6.1 "exact, zero tolerance" |
| F7 | Authored map + seeding (town centres, villagers, nodes at tile centres) | **DONE** | `config/map.toml`; `crates/sim/src/world/seed.rs`; D172 |
| F8 | Instruments: `--dump`, `--report`, batch runner, perf probes, `trace_steps` | **DONE** | `crates/harness/src/bin/`; `docs/performance.md` |
| F9 | Roles, marks and the content vocabulary (no dat id in `crates/`) | **DONE** | `config/economy.toml` `[role]`/`[mark]`/`[tech_role]`; D17 audit |

## 3. The match today — what actually runs

| id | Item | Status | Needs | Evidence / the hole |
|---|---|---|---|---|
| S1 | Movement, collision, one A*, short-range pather, clearance, slide / give-way / repath | **DONE** | F1, F6 | `world/movement.rs`, `src/path.rs`, `src/short.rs`; `tests/collision.rs`, `body_clearance.rs`, `repath_*`; D206 |
| S2 | Orders: `Move`, `WalkTo`, `AttackMove`, `Patrol`, `Stop`, `Stance` | **DONE** | S1 | `Command` in `crates/sim/src/lib.rs:544`; `tests/orders.rs`, `stances.rs` |
| S3 | Gather: place/standoff/touching, per-node crowd, fill, share pass, retarget | **DONE-ish** | S1, F9 | `world/gather.rs`, `world/shares.rs`; 12 `tests/gather_*.rs` files. **Hole: T7 (unbounded search)** |
| S4 | Return and drop sites, whole units + fraction in hand | **DONE-ish** | S3 | `world/sites.rs:427 nearest_drop_site`; `tests/dropoff.rs`, `whole_units.rs`. **Hole: T7** |
| S5 | Build: placement, foundation as a drawing, overbuild/cancel/refund, join, wall line, farm husk | **DONE-ish** | S3, S4 | `world/build.rs`, `world/lines.rs`; `tests/foundations.rs`, `build_*.rs`, `walls.rs`. **Hole: T3 (construction-rate modifier never read)** |
| S6 | Production queue: units and techs in one queue, rally, garrison capacity | **DONE** | S5 | `world/train.rs`, `world/orders.rs`; `tests/production_queue.rs`, `rally.rs`; D157 |
| S7 | Tech/effect interpreter: kinds 1, 2, 3, 4, 5, 6 | **DONE-ish** | F5 | `world/research.rs`; `tests/research.rs`, `modifiers.rs`. **Holes: T1, T2, T6, T9** |
| S8 | Ages: order, requirements, advancing | **DONE-ish** | S7 | `tests/ages.rs`, `economy.toml [age]`. **Hole: T6 (kind 3 replace semantics, U10)** |
| S9 | Population, houses, headroom | **DONE** | S6 | `tests/population.rs` |
| S10 | Combat: damage formula, min range, reload, projectiles, ballistics, blast, blow order | **DONE-ish** | S7 | `world/battle.rs`; `tests/combat.rs`, `blast.rs`, `blow_order.rs`, `ballistics.rs`, `min_range.rs`. **Holes: T1 (range/LOS), T8 (register rows 2–12), T10 (elevation)** |
| S11 | Vision: unseen / explored / visible, hashed, stamped on arrival and movement | **PART** | F6 | `tests/vision.rs`; D50a. **Hole: M1 (no occlusion — sight is a disc)** |
| S12 | Garrison: enter, exit, capacity, eject at a damage fraction, dies with its shelter | **DONE** | S6, S10 | `world/garrison.rs`; `lib.rs:2186 garrison_eject_fraction`; `tests/garrison.rs` |
| S13 | Animals: herdables convert by line of sight, hunting, corpse, carcass decay | **DONE** | S3 | `world/livestock.rs`; `tests/animals.rs`, `conversion.rs`, `carcass.rs`; D69/D70 |
| S14 | Deletion, death sweep, bookkeeping pruning, conquest victory | **DONE** | S10 | `tests/delete.rs`, `removal.rs`, `defeat.rs` (D46a, D59b) |
| S15 | Scripted rules: scenario steps, selectors, personality, memory (anti-cheat), scouting | **DONE** | S2–S6, S11 | `crates/script/src/lib.rs`; `tests/scenario.rs`, `memory.rs`, `scout.rs`, `policy.rs`; `config/ai/base.toml`; D91–D94 |
| S16 | Civ identity: civ per player, team-bonus effect applied, units barred by the tech tree | **PART** | F5, S7 | `lib.rs:4075 load_civs`, `:4101 bonus_effects`, `:4139 civ_barred_unit`, `research.rs:221`; `data/src/civs.rs`. **Hole: T5 (techs barred, unique units, one civ in the pack)** |
| S17 | View: frame, projection, terrain, fog, minimap, HUD, panel, cards, clicking, placement ghost | **DONE** (by its own tests) | F6 | `crates/view`; `ui.rs`, `placement.rs`, 36 view test files; D151–D192. **Not seen as a *game*: colours, not art (P4)** |
| S18 | Gates: open for anyone, closed for everyone; walls by drag | **DONE** | S1, S5 | `world/gates.rs`; `tests/gates.rs`, `wall_lines.rs`; D153/D154/D155 |

## 4. The debts — where a "DONE" row is hollow, which is what this file exists for

Every row here is a place where something runs, is tested, and **still does not do the thing another row needs**. They
are ordered by what they unblock, not by size.

| id | Debt | Why it is a dependency, not a nit | Evidence (grep it) | Unblocks |
|---|---|---|---|---|
| **T1** | **Attribute 1 is line of sight, 12 is range — and the names come from the importer** | Town Watch/Town Patrol move a tower's **attack range** 8 → 12 where the game moves its *sight*, and Greek Fire (effect 518, attribute 12) is inert because the pack calls 12 `other:12`. Two repos: the name table is `ruage-tools` `ATTRIBUTE_NAMES` (`1: "range"`, no 12), the classifier is `crates/data/src/techs.rs:96`, and no sight modifier is consumed anywhere. | `ruage-tools/genconfig/records.py:125`, `crates/data/src/techs.rs:96`, raw `los` at `lib.rs:3155`, `seed.rs:146`, `memory.rs:250`, `report.rs:128` (U21) | S10, S11, S16, M2, and every tower/archer upgrade |
| **T2** | **Four effect kinds are still unread: 0, 101 (tech cost), 102 (disable tech), 103 (tech time)** | 97 tech-cost and 25 tech-time commands apply nothing today; a cheaper-tech bonus or a civ's barred tech silently does nothing. `DisableTech` is *parsed* and never consulted. | `crates/data/src/techs.rs:131 DisableTech`, `:501 kind 101`; `grep -rn DisableTech crates/sim` → **0** (U2) | S7, S8, S16, M2 |
| **T3** | **`build` counts builder-ticks and never reads construction rate (resource 195)** | Spanish barracks finish in the same ticks as a British one; the dat's 1.3 is loaded and unused. Also the civ's kind-1 *sets* are not applied at match start. | `grep -rn '\b195\b' crates --include='*.rs'` → no hit (U19) | S5, S16, M2 |
| **T4** | **Farm food base is 250 in `economy.toml`; the dat's base is 175** | A Horse Collar farm holds 325 where the game holds 250; a Chinese one 295 against 220. The number is authored, not read from the pack's per-civ resource 36. | `config/economy.toml` `[farm.depletion] food_per_farm = 250`; asserted at `crates/data/tests/economy.rs:78` (U20) | Economy fidelity, S16, M2 |
| **T5** | **Civ breadth is one civ, and a civ's *tech* bars are unenforced** | Unit bars (kind 2 = 0) work; the type-102 disable lists — which is what a civ's identity *is* (U13) — are not consulted, so a British player may research the Paladin upgrade. Unique units are not derived from the tech tree. | `lib.rs:4085 civ_by_player`, `:4113 civ_barred_unit` (kind 2 only); U13 | S16, P5, any "add a civ" request |
| **T6** | **Age-up replace semantics (kind 3) unanswered, and units in production at the moment of the upgrade** | Wrong here breaks every game; the effect fires today as a table edit with no test for live units or the queue. | U10; `crates/data/src/techs.rs` `Meaning::Upgrade` | S8, S16 |
| **T7** | **The resource search is unbounded: no sight bound, no explored test, no 30-reach-test cap** | Past LOS+5 the game finds nothing where we find a target, and a camp asks what it *serves* before what is nearest. Also the measured 37× cost of a per-candidate `can_reach`. | U22; `world/sites.rs:427`, `crates/script/src/lib.rs:712 nearest_resource` | S3, S4, S15, and the cost of every new gather rule |
| **T8** | **The binary register: 11 of 12 rows are `unknown-static` / `needs-oracle`** | Only row 1 (the dat version gate) is ruled `same`. Row 2 (damage) is *located* — `FUN_00744d60` — but not ruled: per-entry drop vs total floor is an open shape difference. | `docs/behaviour-diff.md`, the register table | S10, S13, S14, and every "is this right?" question |
| **T9** | **Negative packed attack values (U17)** | Six kind-4/5 commands carry −136 / −226; today they cannot be applied as read, so two effects (Non-Research, Shinkichon) land a wrong or absent number. | U17; named in `crates/data/tests/techs.rs` | S7, Korean/Mongol content |
| **T10** | **Elevation is loaded and unused; no chance-to-hit roll** | The published formula needs elevation, and the pack has no heights, so the term is pinned to 0. Named rather than implied. | `config/combat.toml:56` "PROVENANCE: unknown"; `tests/combat.rs` (U4 residual) | S10 |
| **T11** | **No tick rate proof, and the acceptance bands are stated in ticks** | 20 Hz is corroborated twice and unproven; if it is 60 Hz every "1 tick" band silently tightens 3×. | U12; `config/sim.toml:10`; §6.1's ms column | every timing assertion, T12 |
| **T12** | **Intra-tick order: we decide every entity against the tick's opening state, so two units can trade killing blows** | The original is a sequence (Gaia, players 1–8, slot order); blow order landed (D88) but the *update* order did not. | U18; `world/step.rs` | S10, S13, replay equivalence |
| **T13** | **Movement is a deliberate divergence and its two numbers are sentinels** | `world_units_per_tile = -1.0` and the pathing weights are `-1.0` (U5/U7). Rendering and any arrival-time comparison need the first; any path-cost comparison needs the second. | `config/movement.toml:6-17` | S17, P4, T7 |
| **T14** | **Farm reseed queue absent** | `farm_queue` is in the engine's own subsystem list and there is no code: a farm that runs dry is gone, and the AI's economy stalls on it late. | `grep -rn reseed crates` → **0**; §7.1 | S3, S15, long games |
| **T15** | **The random protocol: eleven streams, all reseeded from a frame seed every tick** | "A roll is its own stream" (R3) is weaker than the original: `genie:math/rng.rs` and `genie:docs/ENGINE_SYSTEMS.md` describe **11 WELL512a streams that all take the frame seed at the start of every tick**, so no draw reaches the next tick. We have **two PCG32 streams and never reseed them** — a divergence no test can see until a roll exists. | `crates/fixed`/`sim` streams (`docs/decisions.md` D9), `genie:docs/ENGINE_SYSTEMS.md` "Determinism"; HD's sync records, 10/s | R3, R1's crowd, M15 (conversion rolls), M19 (market swing), M21+ |
| **T16** | **The turn: only the current player's unit AIs run fully, and the turn passes each tick** | A second axis beside T12's intra-tick order. `Turn` is schedule step 16 in `genie:docs/ENGINE_SYSTEMS.md`; every player's objects still update, but only one player's unit AI does. Our step is owner-blind within a tick and has no turn. | `crates/sim/src/world/step.rs` (entity-id order, no turn); `genie:docs/ENGINE_SYSTEMS.md` step 16 and "One tick" | T12, S15, M23, and any AI that must not see the future |
| **T17** | **Extra projectiles: does any 5.7 unit fire more than one?** | Attribute 102 (`extra_projectiles`) appears on **12 commands in the HD dat's own effect table** (effect 0 `Yasama`, 540, 606) and the engine reads no such attribute; the per-unit field is not imported at all, so the pack cannot answer whether the Chu Ko Nu's second bolt exists here. | `config/gamedata/effects.toml` (`attribute = "extra_projectiles"`), `crates/sim` (no read), `install:resources/_common/dat/empires2_x2_p1.dat` carries the name | S10, and it decides whether a volley is an item at all |

**One more, smaller:** the standoff **was** switched. `standoff_point` is D192's rule — the worker's circle touching the
node's tile, places generated by `tiles::touching_places` (`lib.rs:3435`), spaced 0.66 comfortable and 0.40 packed — while
`GATHER_PLACES` and `ring_point` survive for the crowd layout and the buried-ring test. SPEC v1.153's row still says
*"Not yet switched: the standoffs still use the ring"*, which is now wrong; the correct sentence is the one above.

## 4b. Stale claims found while writing this — the other way a dependency gets missed

A doc that says *"not built"* about something built makes a session rebuild it; a doc that says *"built"* about something
not built makes a session build on sand. Four found, each fixed here rather than in the doc it lives in (those are their
owners' rows):

| Where | Says | Is | What a reader would wrongly do |
|---|---|---|---|
| `docs/decisions.md` D152, item 3 | the functionality gaps are "**no gates** (D54), **no unit pushing** (D52), fog without occlusion" | gates shipped (D154/D155), pushing shipped (D204); **fog occlusion is still missing** | look for gate/pushing work that is done, and skip the one gap that is real |
| `crates/data/src/civs.rs:10` | "`team_bonus_effect` — carried for completeness; **nothing applies it yet**" | collected at `lib.rs:4101`, applied at `world/research.rs:221` | re-implement the team bonus, or decide civs have none |
| `crates/view/src/bin/window.rs:12` | "what it does not have yet: **input** (no selection, no orders), **a HUD**, sprites" | selection/clicking (D187), the HUD and panel (`docs/ui.md`) all exist | treat the window as a static frame and rebuild the input layer |
| SPEC v1.153 / `docs/next-step.md` | D191 "**not yet switched**: the standoffs still use the ring" | D192 switched `standoff_point` to the tile geometry | add a second standoff rule beside the working one |

**The rule that follows** (and it is the project's own, in `AGENTS.md`): *a claim in `SPEC.md` or `docs/` is a hypothesis,
not a fact.* A status table like this one is worth exactly its probe — §11 is that probe, and each row was re-run when this
file was written.

## 5. Missing mechanics — named, so they are choices rather than surprises

| id | Mechanic | Needs before it is worth starting | Evidence it is absent |
|---|---|---|---|
| M1 | Fog occlusion (sight through terrain) | S11, plus a terrain-height/opacity field the pack does not have | `grep occlusion crates` → 0; D50 |
| M2 | Civ breadth (31 → playable), unique units/techs, team bonuses verified | **T1, T2, T3, T4, T5** — a civ that cannot be enforced is a skin | `economy.toml [start] civs` names one; U13 |
| M3 | Monks: heal, convert, faith/relic arithmetic | S10 + T12 (order), plus heal/faith fields the pack does not carry yet | `grep -i monk crates` → **0** |
| M4 | Trade: market, trade cart/cog, moving prices | S3/S6 + a price model the pack does not carry | `market` appears only in comments; D-guide |
| M5 | Naval: water terrain, docks, ships, transport | F7 (map has `G`/`F` only, water deliberately absent) | `tests/defeat.rs` quotes the rule; no dock/ship code |
| M6 | Relics and artifacts | S12 (garrison holds them), S14 (victory conditions) | `relic` only in comments |
| M7 | Repair | **the `repair_hp_per_second = -1.0` sentinel** (`config/economy.toml:134`) — a dat candidate exists (VMREP `work_rate` 12.5) | loader refuses; no `Task::Repair` |
| M8 | Attack-ground | S10 (a projectile with a point) | `Command` has no variant |
| M9 | Formations | D206: **speed is constant; formations are the only speed adjustment, and they are later** | `Command` has no variant; `formation` is a comment in `movement.rs:297` |
| M10 | Replay record/playback, lockstep, desync detection | T11, T12 first — a replay that cannot agree on a tick is a format with no oracle | `crates/script` is the scenario format; no `.aoe2record` writer |
| M11 | Order queue (shift-click waypoints): several orders per unit | S2; `waypoint` appears **83 times** in the exe and `genie:movement/waypoint.cpp` exists | `Command` has no queue — the only queues are a building's production and a wall drag |
| M12 | Follow and guard orders | S2; `follow` 12 and `guard` 9 in the exe; `genie:orders/` lists follows beside moves and patrols | no `Command` variant |
| M13 | The town bell (ring a building, villagers garrison) | S12 (garrison) and S6; `TRIBE_COMMAND_TOWN_BELL` and `TownBell(uId=%d, activate=%d)` are in the exe; openaok lists it under War | no `Command` variant |
| M14 | Facing, and the animation clock the simulation owns | S1; `genie:objects/` holds "its sprite and facing", and `genie:motion.cpp` steps along the forward vector; arrows and formations need it | `grep -ri facing crates` finds one comment — there is no facing at all |
| M15 | Player diplomacy: stance, teams, allied victory, shared vision, treason (allies see enemy kings), resign | S11 (vision) and S16; `diplomacy` 13, `alliance` 1, `cartography` 1, `resign` 18 in the exe; AI `players-stance` (16 uses); `genie:players/diplomacy.rs` | `diplomacy` appears only as a word in the HUD; `MatchOutcome` has no allied victory; `Command` has no resign |
| M16 | Tribute: resources to an ally | M15; `tribute` 47 in the exe; AI `tribute-to-player` (4); `genie:players/tribute.rs` | no `Command` variant; the stockpile is per-player with no transfer path |
| M17 | Treaty: a timed no-attack window, and the no-build zones about town centres | M15; `mTreatyLength`, `Treaty Length`, `TREATY_COUNTDOWN_MSG`, `TREATY_OVER_MSG` in the exe; `genie:players/treaty/` | nothing |
| M18 | The market: commodity prices, buy and sell, escrow | a price table in content (R5's seam) and S4; the AI's own vocabulary settles it — `buy-commodity`/`sell-commodity` (31 each), `can-*-commodity`, `commodity-buying-price`, `set-escrow-percentage`, and `release-escrow` (**1,531 uses**); `genie:players/market.rs` and `genie:players/market_values.rs` | `grep -ri market` in `crates/` is only prose in comments; no price state, no command |
| M19 | Trade carts and cogs: a route between two markets, priced by its length | M18 and S3/S4; the dat carries `TCART`/`TCARTF` and `COGXX`; openaok lists "trade carts and cogs" under Economy | nothing |
| M20 | Victory modes beyond conquest: wonder, relics, regicide, score or time limit, a scenario's own conditions | **R4** (the seam) first — this is what the seam is for; `wonder` 17, `relic` 26, `regicide` 2, `VICTORY-SCORE`/`current-score` in the exe; the dat carries `KINGX` and `RELIC`; `genie:players/game_status/`, `genie:players/history.rs` | `MatchOutcome` is the seat roster and nothing else |
| M21 | Scenario triggers and campaigns | R4 and a trigger format; `trigger.cpp` survives in the exe's own strings ("scenario" 153 times); openaok runs "campaigns and scenarios, with their triggers" | no trigger system; `crates/script` is our own scenario format, not `.scx` |
| M22 | Random map generation from `.rms` | F7 (the map as content) — and it is the answer to U23's map sizes; `rmm_*.cpp` and `randommapscript*.cpp` in the exe, **122 `.rms`** in the install; openaok generates "random maps from the original map scripts" | §7.1 calls it future work and the tree had no item for it |
| M23 | The `.per` AI script interpreter | S15 (our own rules) and a script parser; **104 `.per`/`.per2` files** in the install, whose vocabulary is the game's own action list; openaok "runs the original AI scripts in an interpreter"; D151's aim | nothing — our AI is content rules, not the game's scripts |
| M24 | The network layer: transport, lobby, ready-checks and votes, desync detection | M10 (lockstep and replays) first; `commhandler.cpp`, `commhandlervotecontroller.cpp`, `commplayer.cpp`, `steam_com_provider.cpp`, `synclogger.cpp`, `networkspeeddata.cpp` in the exe; openaok plays "lockstep over TCP, with desync detection" and a lobby | nothing |
| M25 | Chat, taunts, flares and cheat codes | M24 for chat over a network; `Audio Taunts`, `LOC_TAUNT`, `flare`, `OBJECT_FLARE3 created at %.1f, %.1f`, `cheats-enabled` in the exe; openaok lists all four | nothing |

## 5b. Named and **not** added: what HD 5.7 does not have (2026-11-14)

The survey that produced M11-M25 also produced a list of things a neighbouring engine has and this game does not, and a
list like that is worth as much as the other one: it stops the next session adding a mechanic HD never had, or porting a
class name from a later build. Each of these is settled by a zero in the install's own strings or data.

| Candidate | Where it comes from | Why it is not an item |
|---|---|---|
| Pastures, the mule cart | `genie:gathering/` (DE's herding and the Georgian mule cart) | `pasture` and `mule cart` are **0 hits** in `install:AoK HD.exe`; both are Definitive Edition content |
| Empire Wars (idle villagers start work on tick 1) | `genie:docs/ENGINE_SYSTEMS.md` step 4, `StartWork` | `empire wars` is **0 hits** in the exe; the mode is DE's |
| `Wait_Solidify` (a foundation that must set before building) | `genie:construction/` | `solidify` is **0 hits** in the exe — it is DE's action name. HD's own foundation behaviour is what we already built (D183/D185) |
| Passive ability task kinds: generation (151), movement damage (152), terrain look (159), removal reward (161) | `genie:combat/abilities/README.md` | genie's own README says build 158041's dat holds "only generation tasks", and the Feitoria and Keshik that carry them are expansion civs this install does not have. Recorded as a question, not an item: whether any 5.7 `.dat` carries a task the engine must run passively is **open** |
| `fmath` (the game's f32 emulation) | `genie:math/fmath/` | deliberately not adopted (D9): fixed point replaces it under G1 |
| The `.drs` archive reader | openaok, genie's format crates | this install ships **0 `.drs`** files — they are already extracted (§3.5) |

## 6. Product — the route to "a stranger plays it"

| id | Item | Status | Needs |
|---|---|---|---|
| P1 | `World::from_state` + a tool-side converter (`--load`), the "B shape" of the save oracle | **MISSING** | F3, and the licence answer A-vs-B (owner) |
| P2 | The 18 install saves as a differential oracle (run 1,200 ticks from `brit7`, compare `brit8`) | **MISSING** | P1 |
| P3 | The importer inside `tools/` and a bootstrap step, so a clean clone with only `Age2HD/` produces `config/gamedata/` | **MISSING** | D12 split (done), licence plumbing |
| P4 | Asset pack: entity + state → sprite ref, generatable from the dat's sprite table; then a renderer that draws it | **MISSING** | P1 (not strictly), T13 (`world_units_per_tile`), D18 |
| P5 | AI as content (D151): every control out of the binary, enumerated in `docs/ai-extraction.md` | **PART** | S15 exists as rules; the *removal* inventory is a plan, not code |
| P6 | Publish cleanup: README/quickstart, a content check in CI, engine comments no longer citing internal docs | **MISSING** | §6.2 checklist; owner's A-vs-B |
| P7 | Content (D201): names by three lists, prose rewritten, campaigns ours only | **MISSING** | M2, P4 |
| P8 | Sound and music | **MISSING** | last by D152; nothing depends on it |

---

## 7. The tree, as edges

```
F1 fixed point ─┬─ F6 tick/hash ─┬─ S1 movement ─┬─ S2 orders ─┬─ S15 scripted rules
F2 config ──────┤                │               ├─ S3 gather ─┴─ S4 return
F5 importer ────┘                │               ├─ S5 build ─── S6 production ─┬─ S9 population
                                 │               └─ S10 combat ─┬─ S12 garrison ┤
                                 │                             ├─ S13 animals  │
F4 overrides ────────────────────┘                             └─ S14 delete ──┴─ S8 ages ─ S7 effects
                                                                                     │
   T1 attribute 1/12 ─────────────► S10 (range), S7 (modifiers)  ◄─── T9 negative pack
   T2 kinds 0/101/102/103 ────────► S7 ─► S8 ─► T5 ─► M2 civs
   T3 resource 195 ───────────────► S5 ─► M2
   T4 farm 175 vs 250 ────────────► S3 ─► M2
   T5 tech bars/unique units ─────► S16 ─► M2 ─► P7 content
   T6 age replace (kind 3) ───────► S8 ─► M2
   T7 search contract ────────────► S3, S4, S15 (and the cost of every later gather rule)
   T8 register rows ──────────────► S10, S13, S14 (belief, not code)
   T11 tick rate ─► T12 intra-tick order ─► S10/S13 ─► M10 replays
   T13 world scale / weights ─────► S17 view scale, P4 renderer, any arrival-time claim
   T14 farm reseed ───────────────► S3 ─► long-game economy ─► P5 AI late game

   M1 fog occlusion ──► S11
   M3 monks ─┐
   M4 trade ─┼─► need S10/S3/S6 plus content the pack does not carry; each also needs M2 to matter
   M5 naval ─┤
   M6 relics ┘
   M7 repair ─► a resolved sentinel (owner or a save)
   M9 formations ─► D206 (later by rule)

   P1 from_state ─► P2 save oracle ─► (answers T4, T7, T11, T12 without a copy of the game running)
   P3 importer in tools ─► P6 publish ─► P4 assets ─► P5 AI ─► P8 sound
```

## 7b. The seams that keep the deferred mechanics cheap — the owner's "architecture must be ready"

**Ruled 2026-11-14: monks, trade, naval and relics are *not* in v1, and the architecture must be ready for them.** "Ready"
is a testable claim, so it is written as five **seams** — each proved by a fixture *pack*, never by the mechanic, and each
small enough to land in one pass. None of them puts a game's word in `crates/` (D17).

| id | Seam | Today | The proof (a test that fails until the seam exists) |
|---|---|---|---|
| **R1** | **A body has a domain; ground has a surface** — one walkability grid becomes one predicate | `tile_walkable`, `is_free_ground`, `can_reach` and the placement tests each read `Terrain` plus footprints directly | a pack declares `domain = "water"` on a unit and a map tile `W`; a fixture unit stands on the water and **refuses the grass**, with no enum in `crates/` naming either |
| **R2** | **An order that is an ability** | `Command` grows a variant per mechanic (a monk, then attack-ground) and `Task` grows the matching arm | `Command::Cast { unit, ability, target }` where the ability resolves through a content table (range, cooldown, roll, effect); a pack declares an ability that only starts a cooldown, and the engine runs it |
| **R3** | **The random protocol: a roll draws from a stream, and the streams are reseeded every tick** | two PCG32 streams, `sim` and `map`, chosen at load and **never reseeded** (`docs/decisions.md` D9). The original has **11 WELL512a streams that all take the frame seed at the start of every tick** (`genie:math/rng.rs`, `genie:docs/ENGINE_SYSTEMS.md` "Determinism") — so a draw in one tick cannot reach the next, and a replay cannot depend on how many rolls happened earlier. **T15** | the same seed gives the same sequence; a match that never rolls leaves the state hash identical; and a roll added on tick *n* does not move a draw made on tick *n+1* — the property the original's protocol exists for |
| **R4** | **Victory is a table the engine evaluates, not a function** | `MatchOutcome` is conquest, derived from the seat roster | `[[victory]]` in content, with kinds the engine can evaluate from state (seats, relics, a building standing); the conquest rule re-expressed in it, and **the acceptance hash does not move** |
| **R5** | **A price is state, not a field** | the stockpile is a fixed-width row per player; nothing moves | prices as a hashed vector derived from content and updated by a rule; a world with no market hashes exactly as today |

**Where they go:** after **Phase 3** (they touch the state hash and the RNG, which Phase 3 settles) and **before Phase 5**
(breadth multiplies the call sites each seam has to change). **R1 is the expensive one** and the only one on a hot path —
it must carry a before/after tick measurement, and the acceptance hash must be byte-identical.

**What "ready" does not mean:** it does not mean building the mechanics early, and it does not mean guessing their numbers.
A seam is a shape; a monk's faith cost, a trade route's value and a relic's count stay open until they are read from the
data or asked.

## 8. The sequence

Seven phases, in the order the dependencies allow and with the cheap live defects first. **A phase's gate is the previous
phase's last item green and pushed.** Every item here is named by its id, so `tools/plan_check.py` refuses an item that a
table defines and no phase names — the check this list exists for.

### Phase 0 — the live wrong numbers (no owner input needed)
1. **T1a** — the attribute names, **in the importer**: `ruage-tools/genconfig/records.py`'s `ATTRIBUTE_NAMES` has
   `1: "range"` and no `12`, so the pack names 177 range modifiers from attribute 1 and attribute 12 becomes `other:12`.
   Two dict entries (`1: "line_of_sight"`, `12: "range"`), its own pytest, a regenerated `config/gamedata/`, and the two
   facts it must keep (Fletching is +1 range; Town Watch is +4 line of sight for classes 3 and 52). **A second
   repository** (D12) — which is why T1 looked like a one-line change and is not.
2. **T1b** — the engine half: `Attribute::from_id` (`crates/data/src/techs.rs:96`) gains `LineOfSight` and `Range` in the
   right slots, and **vision consumes a line-of-sight modifier** — `los` is read raw at four sites (`lib.rs:3155`,
   `seed.rs:146`, `memory.rs:250`, `report.rs:128`), so a +4 sight tech is inert unless this lands with T1a. Range
   modifiers *are* consumed (`battle.rs:96`), which is exactly why attribute-1 modifiers reach combat's range today.
3. **T4** — farm base from the pack's per-civ resource 36 (175), techs added on top; tests for 175 / 250 / 220.
4. **T9** — U17's negative packing, or an explicit refusal that names the effect (a wrong number is worse than a stop).
5. **T10** — either an elevation field or a stated zero with a test that says so.

*Why first:* each is a **wrong** number rather than a missing feature, and each is an input to a later phase (T1 to M2,
T4 to M2, T9 to S7). *Why T1a and T1b are one phase and two commits:* they are one behaviour split by the licence
boundary, and landing either alone leaves the pack and the engine disagreeing about what attribute 1 means.

### Phase 1 — the effect interpreter closes
6. **T2** — kinds 0, 101, 102, 103 (tech cost, tech time, disable tech, set attribute), then **T5**'s tech bars, which is
   the same code path: a civ's identity is a disable list.
7. **T8** — the register's row 2 (damage): the only row already *located* (`FUN_00744d60`, five call sites), so it
   leaves the register with an answer rather than joining a queue. Rows 3-12 wait for the oracle (owner's answer 5).
   **T17** closes here too: whether a 5.7 unit fires more than one projectile, decided from the dat before a volley
   becomes an item.
8. **T6** — U10's kind-3 replace semantics: live units of the upgraded id, units in production, and the queue.

*Gate:* a British player cannot research a British-barred tech; a tech-cost bonus changes a price, with a test.

### Phase 2 — the economy's own rules (one slice each)
9. **T3** — construction rate 195 and the civ's kind-1 sets applied at match start.
10. **T7** — U22's contract in three tests: the LOS + 5 bound, the footprint-seeded drop-site search at site LOS + 10, and
    30 reach tests then the fallback. **Measure before and after**: `repaths` and `made_room` already exist.
11. **T14** — farm reseed queue, once the farm's food base is right (T4).

*Gate:* the acceptance run's food/wood columns move only where the change says they should.

### Phase 3 — determinism and order
12. **T11** — tick rate: one discriminating probe (a reload-bound fight captured at high fps, or a save's own clock), then
    restate §6.1's millisecond column if it moves.
13. **T15** — the eleven-stream protocol: the streams take a frame seed each tick, and a draw cannot reach the next
    tick. **T16** — the turn: one player's unit AIs run fully per tick while the others only take orders.
14. **T12** — U18: read the original's intra-tick order from `BaseWorld::update` (the one address the register has not
    read), test first ("a unit killed by an earlier blow still swings"), then the apply loop.

*Gate:* the acceptance hash is unchanged at the same tick rate — an order change that moves it is a behaviour change and
must be recorded as one.

### Phase 3b — the seams for the mechanics that are deferred but must not be blocked (§7b)
15. **R3** (**T15**'s eleven-stream protocol, then a roll) and **R4** (victory as a content table) first, then **R5** (a price is state), then
    **R2** (an order that is an ability), then **R1** (a body's domain).

*Why this order inside the phase:* R3 and R4 move the state hash, so they go first while the acceptance hash is still the
instrument; R1 is a hot-path refactor and goes last, with a before/after tick measurement and a byte-identical hash as its
gate. *Why here:* after Phase 3, which settles the RNG and the update order these seams touch, and before Phase 5, where
breadth multiplies every call site R1 has to change.

*Gate:* a fixture pack expresses a water-only unit, an ability, a relic victory and a market price **without** `crates/`
learning any of their names — and the acceptance hash is byte-identical.

### Phase 4 — the oracle (where the questions stop being arguments)
16. **P1** — `World::from_state` + `--load` in the **B shape** the owner ruled on 2026-11-14: a tool-side converter behind
    the seam, so the engine needs no reader of its own.
17. **P2** — the 18 install saves as a differential oracle: 1,200 ticks from `brit7` compared with `brit8`.
18. **P3** — the importer inside `tools/` and a bootstrap step, so a clean clone with only `Age2HD/` produces
    `config/gamedata/`.

*Why after 0-3b:* the saves can settle T4, T7, T11 and T12, but they settle them **better** once the code they test is
already right — and P1/P2 turns every later "is this right?" into a comparison.

### Phase 5 — breadth, on a correct base
19. **M2** civ breadth — **not in v1** (owner, 2026-11-14). Listed so the tree is complete; it waits for T1, T2, T3, T4,
    T5 and T6.
20. **M1** fog occlusion.
21. Then, in the owner's stated order (D152): UI polish, then graphics — **T13** first, because a renderer needs
    `world_units_per_tile` resolved — then **P4** (asset pack + renderer), then **P5** AI, then **P8** sound.
22. **Not in v1, named so none goes missing** (owner, 2026-11-14), and each is listed rather than ranged because a range
    is exactly where one disappears. From the first list: **M3** monks, **M4** trade, **M5** naval, **M6** relics, **M7**
    repair, **M8** attack-ground. From the 2026-11-14 survey of the original and its neighbours: **M11** waypoint queues,
    **M12** follow and guard, **M13** the town bell, **M14** facing and the animation clock, **M15** player diplomacy,
    **M16** tribute, **M17** treaty, **M18** the market's prices, **M19** trade carts and cogs, **M20** victory modes,
    **M21** scenario triggers, **M22** random map generation, **M23** the `.per` interpreter, **M24** the network layer,
    **M25** chat, taunts, flares and cheats. The seams (§7b) are what keeps this a schedule question rather than a
    rewrite; the survey and its exclusions are §5b and `docs/reviews/2026-11-14-missing-items-survey.md`.

### Phase 6 — the product
23. **P6** publish cleanup, **P7** content (D201), **M10** replays and lockstep.
24. **M9** formations last, per D206.

## 9. What not to start yet, and the honest reason

| Not yet | Because |
|---|---|
| A second civ / 31 civs | Its identity is a **type-102** list nothing enforces (T5) and three of its bonuses land in the wrong field (T1, T3) |
| Formations, group speed | D206: speed is constant until formations exist, and formations are explicitly later |
| Renderer / sprites | P4's binding does not exist and `world_units_per_tile` is still `-1.0` |
| Monks, trade, naval | Each needs content fields the pack does not carry, and each would rest on the same unbounded search (T7) and the same simultaneous tick (T12) |
| Replays / lockstep | A replay whose tick order is not the original's is a format with no oracle (T11, T12) |
| Widening an acceptance band | §6.1: that is a bug with a repro, not a tolerance |

## 10. The question that catches the problem you described

Before a slice starts, five questions, in this order — and the first one that answers *no* is the dependency you found late:

1. **Does the stage this feeds exist as code, or only as a row in `economy.toml`?** (a `-1.0`, a comment, or a
   `PROVENANCE: published` with no field is the tell)
2. **Is the number it reads in the pack, and in the *right* field?** (T1 and T4 were both "the number is there, in the
   wrong place")
3. **Is the field it writes actually consumed anywhere?** (`grep` the name; `bonus_effects`, `DisableTech` and `195` were
   each parsed and dead)
4. **Does the test that pins it fail if the mechanism is removed?** (the D204 carrier half and the `D191` standoff were
   both "tested" by an observable that did not witness the rule)
5. **Is this ordering in `docs/behaviour-diff.md` still `unknown-static`?** If yes, the slice is a hypothesis, and the
   review must say so in those words.

## 11. Probes — re-verify a status in one command

```sh
grep -n '1 =>' crates/data/src/techs.rs            # T1 closed when 1 is line_of_sight
grep -rn '\b195\b' crates --include='*.rs'         # T3 closed when a hit exists
grep -rn 'DisableTech' crates/sim                  # T2/T5 closed when a hit exists
grep -rn reseed crates                             # T14 closed when a hit exists
grep -rn 'from_state' crates                       # P1 closed when a hit exists
grep -n food_per_farm config/economy.toml          # T4 closed when it is not 250
grep -n 'world_units_per_tile' config/movement.toml # T13 closed when it is not -1.0
grep -c '^| [0-9]' docs/behaviour-diff.md          # rows still open outweigh rows ruled
grep -rin 'extra_projectiles' crates                # T17 closed when a hit exists
grep -rn 'reseed\|frame seed' crates                # T15 closed when the streams take a seed per tick
grep -rn 'is_mate\|turn' crates/sim/src/world/step.rs | head   # T16 closed when a turn exists
grep -rn 'TRIBE_COMMAND' Age2HD/AoK\ HD.exe | head  # the original's own command names, per feature
```

## 12. The owner's answers, 2026-11-14 — and what each changes

| # | Question | Answer | Consequence |
|---|---|---|---|
| 1 | The milestone | **(a) a correct engine** | the phase order stands as written; P4-P8 stay last |
| 2 | Civ breadth in v1 | **(a) no** — one civ until the debts close | M2 stays in Phase 5, after T1, T2, T3, T4, T5, T6 |
| 3 | Monks / trade / naval / relics in v1 | **none — but the architecture must be ready** | §7b's five seams, R1-R5, are scheduled work in Phase 3b; leaving them out would be the miss |
| 4 | A vs B publishing | **B** | the engine is published, the spec and mechanics stay internal; `--load` starts as the tool-side converter behind `World::from_state`, and P2/P3 follow that shape |
| 5 | Register rows vs the oracle | **row 2, then pause for the oracle** | T8 narrows to row 2 (Phase 1); rows 3-12 resume with the oracle from Phase 4 |
| 6 | "The enemy is eliminated inside 40 minutes" | **not a requirement** | the pacing question is closed rather than deferred; nothing tests it, and the AI's aggression is tuned later with the AI (D152) |

**Surveyed 2026-11-14, after the answers:** the original's own strings and data, plus `librematch/genie` (DE build
158041) and `Zaydme/openaok` (the 1999 game), added **16 items** — M11-M25 and T15-T17 — and killed six candidates as
later-build features (§5b). Every item carries the string count or the dat entry that put it there, because the survey's
whole value is that a reader can check one in a minute.

**Where these live in the corpus:** `docs/decisions.md` **D208**, filed under *Project process* in
`docs/decision-areas.toml` — the six answers, the fourteen debts, the eight checks and the five seams, as one decision
rather than six loose remarks. This table stays the short form; D208 is the one a reader is sent to.
