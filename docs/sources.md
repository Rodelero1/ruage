# Sources

Everything harvested from outside this machine, with what it was used for and how far to
trust it. Version caveat throughout: **most community documentation describes DE**, while
this project targets HD 5.7. Where they disagree, the dat wins and the disagreement is
logged as a question (e.g. U11 farming).

## Pathfinding: a first-party description of the short-range algorithm

- **Raymi Klingers (Engineering Director, Forgotten Empires), "25+ Years of Pathfinding Problems with C++"**, Meeting C++
  2025. Slides: `meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf` (104 pages). The original
  short-range pathfinder (pages 21-38) and their replacement for it (pages 82-96): shrink the unit to a point, expand the
  obstructions (axis-aligned rectangles), ray at the goal, edge-walk the union turning right at ends and left at
  intersections, re-ray at every vertex, brute-force smooth — all in `> < == + -`, because precision is what broke the
  original. Our mapping and plan: `docs/pathfinding-from-the-talk.md`.
- No AoE2 source has ever been released. What exists: pre-release **binaries** (Hidden Palace: seven 1998-99 AoK builds), and
  a matching **decompilation** of AoC 1.0c / AoK 2.0a / the PS2 ELF (`samuelcaldas/age-of-empires-2-decomp`, no licence —
  all rights reserved). Read the decompilation for behaviour and cite the module; never copy its expression (see
  `docs/licensing.md`), and note its addresses are AoC/PS2, not HD 5.7.

## Prior art: how openage handles the layer the talk does not

**SFTtech/openage** (GPL-3.0, C++ reimplementation, read at `master` on 2026-11-04). Asked the same question M3 ran into — who
*collides* a body with the ground and unsticks it — because the Klingers deck ends at "brute-force smooth".

- `libopenage/gamestate/system/move.cpp`, `Move::move_default`: a path becomes a **keyframe curve**, not a follower. Per leg
  it charges a turn-in-place delay (`angle / Turn.turn_speed`), then a straight move (`distance / Move.speed`), and writes
  `set_position(start + total_time, waypoint)` into the future. There is **no per-tick move, no ground test, no body-to-body
  collision, no re-plan mid-walk, no arrival radius and no unstuck**. If the pathfinder does not answer `FOUND`, `find_path`
  returns `{}` and the unit does nothing at all — the same silent stall as our M3b freeze.
- Their **waypoint convention is ours, derived independently**: start position, then `tile.to_phys3_center()` for every
  intermediate waypoint, then the **exact end point** last. That is exactly M3a ("the goal is the middle of a tile").
- Their pathfinder is **flow-field, not hull-based**: `pathfinding/{cost_field, integration_field, flow_field, sector, portal,
  integrator}` — sectors and portals, a wavefront integration field from the target with an optional LOS pass, a flow field
  built from it, cached per (portal, sector) and **evicted when the cost field goes dirty** (so a dropped building invalidates
  reachability without per-unit repaths). The Supreme-Commander-2 family: cheap for many units sharing a destination, and
  grid-only — no body radius anywhere, which is why openage never needed M3.
- Their tracker has **zero issues titled "collision" or "avoidance"**, and the one "stuck" fix (`#860`, 2017) is a gatherer
  whose dropsite was not found. The follower layer is simply not built there either.

**Verdict:** openage confirms the trap and supplies one convention we had already reasoned out; it is not a source for the
collision/arrival/stuck layer, and it is a reminder that a grid-only planner makes that layer unnecessary by making units
non-physical. Worth re-reading only if planning *cost* becomes our bottleneck, where its caching is the better design.

## Numbers used in `docs/mechanics/`

| Source | Used for | Trust |
|---|---|---|
| [Armor class (Age of Empires II)](https://ageofempires.fandom.com/wiki/Armor_class_(Age_of_Empires_II)) | Damage formula, step-by-step derivation, min-1 rule, bonus stacking | High — agrees with two others |
| [Pierce armor](https://ageofempires.fandom.com/wiki/Pierce_armor) | Closed-form damage formula, elevation multipliers (1.25 / 0.75) | High |
| [Attack](https://ageofempires.fandom.com/wiki/Attack) | Full formula with k_res / k_mul terms | High |
| [aoe2database — Damage Formula](https://www.aoe2database.com/damage_formula/en) | Third independent statement; sentinel armour for absent classes | Medium — self-described approximation |
| [Resource](https://ageofempires.fandom.com/wiki/Resource) | Base gather rates per source (0.388 wood, 0.379 gold, 0.359 stone, 0.310 berry, 0.280 fishing ship) | High — matches the dat |
| [Villager (Age of Empires II)](https://ageofempires.fandom.com/wiki/Villager_(Age_of_Empires_II)) | Gather rate table, carry capacities, tech bonus list | High |
| [Food](https://ageofempires.fandom.com/wiki/Food) | Hunter carry 35, practical farm rate ~0.356 | High |
| [aoe2db — Farming & Economy Upgrades](https://aoe2db.com/farming-rates) | Double-Bit Axe +20% wood, Gold/Stone Mining +15%, farm food per upgrade | High — used to *verify* the dat's `c=13` multipliers |
| [aoe2.ai methodology](https://aoe2.ai/en/methodology) | Practical vs theoretical rates (farms 20.3–24 food/min) | Medium — planning assumptions |
| [aoe2-de-tools](https://aoe2-de-tools.herokuapp.com/villagers-required/) | Independent practical rates (0.39 wood, 0.34 farm) | Medium |
| [Jineapple's Infosheet (Steam)](https://steamcommunity.com/sharedfiles/filedetails/?id=2169547705) | Attack delay, reload resampling to 1/60 s, Matt Pritchard's float/OOS quote | High for mechanics, DE-era |
| [Tech tree lies (Steam)](https://steamcommunity.com/sharedfiles/filedetails/?id=191310499) | Proof that speed bonuses multiply **reload** (Celts ×0.8, Japanese ×0.75) | High — fits the dat's type-5 encoding |

## Pathfinding: a first-party description of the short-range algorithm

- **Raymi Klingers (Engineering Director, Forgotten Empires), "25+ Years of Pathfinding Problems with C++"**, Meeting C++
  2025. Slides: `meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf` (104 pages). The original
  short-range pathfinder (pages 21-38) and their replacement for it (pages 82-96): shrink the unit to a point, expand the
  obstructions (axis-aligned rectangles), ray at the goal, edge-walk the union turning right at ends and left at
  intersections, re-ray at every vertex, brute-force smooth — all in `> < == + -`, because precision is what broke the
  original. Our mapping and plan: `docs/pathfinding-from-the-talk.md`.
- No AoE2 source has ever been released. What exists: pre-release **binaries** (Hidden Palace: seven 1998-99 AoK builds), and
  a matching **decompilation** of AoC 1.0c / AoK 2.0a / the PS2 ELF (`samuelcaldas/age-of-empires-2-decomp`, no licence —
  all rights reserved). Read the decompilation for behaviour and cite the module; never copy its expression (see
  `docs/licensing.md`), and note its addresses are AoC/PS2, not HD 5.7.

## Tooling and format references

| Source | Used for |
|---|---|
| [SiegeEngineers/aoc-dev-resources](https://github.com/SiegeEngineers/aoc-dev-resources) | Index of the ecosystem (this is how `geniedoc`/`aoeres` were found) |
| [aap/geniedoc](https://github.com/aap/geniedoc) | Documents the *other* `.dat` files (terrain, blendomatic, filters) — **not** `empires2*.dat` |
| [withmorten/aoeres](https://github.com/withmorten/aoeres) | Versioned resource dumps, AoC/AoK — not yet mined |
| [stefan-kolb/aoc-mgx-format](https://github.com/stefan-kolb/aoc-mgx-format) | Recorded-game format; its header section is a **stub**, so it did not help with saves |
| [Divy1211/GenieDatParser](https://github.com/Divy1211/GenieDatParser) | The parser that produced `build/converted/*.json` (GPL-3.0 — tool use only) |
| [SiegeEngineers/genie-rs](https://github.com/SiegeEngineers/genie-rs) | Cross-check oracle for block counts (GPL-3.0) |
| [Tapsa/genieutils](https://github.com/Tapsa/genieutils) | Read as the format *specification* (LGPL-3.0) |
| [AoE forums — sharing saves](https://forums.ageofempires.com/t/how-to-find-and-share-save-game-and-recorded-game-files/84162) | Confirms `.aoe2spgame` = full state, `.aoe2record` = replay |
| [morluto/rea](https://github.com/morluto/rea) | The MCP reverse-engineering server (6.8.0) that returns Ghidra/IDA pseudocode with evidence. Its [`docs/windows-ghidra-p0.md`](https://github.com/morluto/rea/blob/main/docs/windows-ghidra-p0.md) is the authority on the Windows boundary — see `docs/re-toolchain.md` (D86) |
| [Ghidra 12.1.4](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.1.4_build) | The decompiler REA drives on this box, pinned: REA 6.8.0 rejects 12.2 as `config_drift` |
| `Age2HD/AoK HD.exe` (the install) | The authority for behaviour the saves cannot show. Measured: i386 PE, 7,521,280 bytes, PE timestamp 2018-08-22T18:26:19Z |

## Pathfinding: a first-party description of the short-range algorithm

- **Raymi Klingers (Engineering Director, Forgotten Empires), "25+ Years of Pathfinding Problems with C++"**, Meeting C++
  2025. Slides: `meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf` (104 pages). The original
  short-range pathfinder (pages 21-38) and their replacement for it (pages 82-96): shrink the unit to a point, expand the
  obstructions (axis-aligned rectangles), ray at the goal, edge-walk the union turning right at ends and left at
  intersections, re-ray at every vertex, brute-force smooth — all in `> < == + -`, because precision is what broke the
  original. Our mapping and plan: `docs/pathfinding-from-the-talk.md`.
- No AoE2 source has ever been released. What exists: pre-release **binaries** (Hidden Palace: seven 1998-99 AoK builds), and
  a matching **decompilation** of AoC 1.0c / AoK 2.0a / the PS2 ELF (`samuelcaldas/age-of-empires-2-decomp`, no licence —
  all rights reserved). Read the decompilation for behaviour and cite the module; never copy its expression (see
  `docs/licensing.md`), and note its addresses are AoC/PS2, not HD 5.7.

## Rules for using these

1. **The dat outranks any wiki.** Community numbers describe DE; where the dat differs, the
   dat is what the sim must implement, and the difference is a question (U11).
2. **Two sources or it is `inferred`.** Every formula in `combat.md` has at least two
   agreeing statements; every multiplier decode was verified against a published bonus *and*
   found in the dat.
3. **Never assume an id.** The one wrong conclusion in this project so far — "the DBA
   multiplier is missing" — came from assuming tech 200 was Double-Bit Axe. Match by cost
   and research time. This is now a rule, not a note.

## Pathfinding: a first-party description of the short-range algorithm

- **Raymi Klingers (Engineering Director, Forgotten Empires), "25+ Years of Pathfinding Problems with C++"**, Meeting C++
  2025. Slides: `meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf` (104 pages). The original
  short-range pathfinder (pages 21-38) and their replacement for it (pages 82-96): shrink the unit to a point, expand the
  obstructions (axis-aligned rectangles), ray at the goal, edge-walk the union turning right at ends and left at
  intersections, re-ray at every vertex, brute-force smooth — all in `> < == + -`, because precision is what broke the
  original. Our mapping and plan: `docs/pathfinding-from-the-talk.md`.
- No AoE2 source has ever been released. What exists: pre-release **binaries** (Hidden Palace: seven 1998-99 AoK builds), and
  a matching **decompilation** of AoC 1.0c / AoK 2.0a / the PS2 ELF (`samuelcaldas/age-of-empires-2-decomp`, no licence —
  all rights reserved). Read the decompilation for behaviour and cite the module; never copy its expression (see
  `docs/licensing.md`), and note its addresses are AoC/PS2, not HD 5.7.

## Found via enhansome/enhansome-aoe2 (curated list, checked 2026-10-28)

The repository is an "awesome AoE2" index rather than a tool. Most of it is tournaments, streamers and civ
pickers; four entries are worth keeping on the watch list, in order of expected use:

| Source | Why it matters | Trust |
|---|---|---|
| [AoE2DE UGC Guide](https://ugc.aoe2.rocks/) ([source](https://github.com/Divy1211/AoE2DE_UGC_Guide)) | Divy1211 is the GenieDatParser author, and this documents user-generated content - scenario, AI and data modding. The place to check when a dat field's *behaviour* matters rather than its layout (build categories, class semantics). Python, updated this month. | High - same author and lineage as the parser we already trust |
| [AoE2 AI Scripting Encyclopedia](https://airef.github.io/) | The game's own AI API: object names, per-object facts, and the commands an AI may issue. Our scenario runner is a poorer cousin of this, so it doubles as a checklist of orders we have not modelled (and its object classes may settle questions like "is a farm a gathering building"). | Medium - documents DE's AI, not the sim |
| [Build order guide](https://buildorderguide.com/) / [AoE Companion](https://aoecompanion.com/) | Expert build orders with uptime and villager counts. This is what answers "can a player reach Castle in 15 minutes" with numbers instead of guesses - directly relevant to D59b's pending siege decision. | High for timings, community-maintained |
| [aoe-twools](https://aoetwools.firebaseapp.com/) ([source](https://github.com/Kheyzell/aoe-twools)) | Tech tree per civ, civs per unit and per tech. Useful if the pack ever grows past one civ, since it is the same "which civ may have what" question our `civ_barred_unit` answers for one. | Medium - DE data |

**Not useful yet:** unit-counter charts, civ pickers, replay packers, tournament boards. Recorded here so the
next session does not re-read the list looking for the same things.

## Prior art: `glouw/openempire` — the mover that never gets stuck

`openempire` is an open-source Age of Empires 1 in C. Its movement layer (`src/Units.c`, 1,749 lines) is the closest thing to
the layer this project's M1-M3 work has been building, and it answers the "unit bumps into a tree and stops" problem in a way
that is worth copying and worth contrasting.

**What it does**

1. **A straight line when the line is clear, A\* otherwise.** `FindPath` asks `Unit_HasDirectPath` first and only falls back to
   `Field_PathAStar`. One search per order, not per tick.
2. **Every unit with a path re-plans every 25 cycles** (`CONFIG_UNIT_PATHING_TIMEOUT_CYCLES`), spread over ten slices a tick
   (`CONFIG_UNITS_REPATH_SLICE_SIZE`) so no frame pays for the whole army. The comment in `Repath` is the whole idea:
   *"DOES NOT NEED TO BE THREADED - GIVES A MORE NATURAL FEEL TO HAVE THE PATHER REPATH IN TIME SLICES."*
3. **Steering, not just waypoints**: `CalculateBoidStressors` sums three pushes - alignment with neighbours, separation from
   neighbours, and **wall push** (`WallPushBoids`, which reads the grid's blocked tiles and repels the unit from them). A unit
   that walks into a wall is pushed sideways by the wall itself, so it slides off obstacles without a special case.
4. **A stressor deadzone** (`CONFIG_UNITS_STRESSOR_DEADZONE` 10): pushes below it are ignored, so a unit does not jitter on a
   crowd of one.
5. **Recovery is a first-class path**: `Units_FreeAllPathsForRecovery` drops every path and marks each unit
   `must_repath_with_recover`, because *"PATHS ARE NOT TRANSFERRED OVER THE WIRE"*.

**What it costs, and where this project differs**

- The re-plan is **unconditional** - it does not detect that a unit is stuck, it simply looks again on a timer. That is robust
  against reasons nobody predicted, at the price of a search every 25 cycles per unit (sliced).
- Its steering is **continuous and float-based**; this engine's mover is integer-only (D9), so a wall-push would have to be
  expressed in fixed point and would interact with the tile-passage rule (D117) - a push that shoves a body through a gap the
  tiles say is closed is exactly what D117 forbids.

**What this project took (D127)**: the *principle* that a route must be re-examined rather than walked into, done cheaply and
deterministically - a ground refusal asks for **at most two** re-plans per route, and the count is part of the state hash.
**What it did not take**: the 25-cycle unconditional re-plan (a search per unit per 25 ticks is affordable but there is no
measurement yet that it buys anything the stall check misses) and boid steering (floats, and a different passage model).
