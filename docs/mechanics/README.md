# Mechanics knowledge base

Step 2's job is to turn the dat into **facts the sim can be written against**, and to turn
everything else into a *specific testable question*. Nothing here is prose for its own
sake: every claim carries a tag and a source.

## Tag legend

| Tag | Meaning |
|---|---|
| **known** | Measured from the game's own data by a script in `build/tools/`. Reproduce it; the script is the source. |
| **inferred** | Not stated anywhere; decoded by cross-checking data against independently known values. The evidence is named inline. Treat as strong hypothesis, not scripture. |
| **unknown** | Not in the data. Registered in the table below as a question with a test. |

Sources are either a dat field path (`unit.combat_info.attacks[].id`), a file
(`resources/_common/...`), or a script (`build/tools/extract.py`).

## Index

| File | Covers | State |
|---|---|---|
| [`units.md`](units.md) | Unit stats, costs, train times, class ids | first pass, data-backed |
| [`combat.md`](combat.md) | Attack/armour arrays, damage model | first pass, formula unknown |
| [`economy.md`](economy.md) | Villager tasks, carry, drop sites, gather rates | first pass, rates partly unknown |
| [`techs-effects.md`](techs-effects.md) | Techs, effect commands, ages | first pass, type semantics partly decoded |
| [`world-rms.md`](world-rms.md) | Terrain, maps, `.rms`, `.per` AI | first pass, inventory known |
| [`determinism.md`](determinism.md) | Sim/sync requirements | constraints written, tick rate unknown |

## Unknown registry

The exit criterion for step 2 is that every unknown is a question someone can answer, not
a hand-wave. These are the ones that block a working sim.

| # | Question | Why it blocks | How to answer |
|---|---|---|---|
| ~~U1~~ | ~~Where does Double-Bit Axe's +20% wood gather rate live?~~ **RESOLVED** — effect 195: `(5, 218, -1, 13, 1.2)`. `c=13` is the gather-rate attribute. The earlier "missing" was a wrong tech id: 200 = Bodkin Arrow, DBA = 202. | — | Done. Same method pins Gold Mining (×1.15) and Bow Saw (×1.2). |
| U2 | Effect command `type` semantics. **Partly decoded**: `4` (1,091 uses) = attribute modifier, `5` (576) = multiplier, `3` (227) = unit upgrade. Open: `0`, `2`, `6`, `101–103`, and the 1024-add convention on attack/armour. | Every tech application depends on it. | Same method — match a tech by cost+time against a published bonus, read the command. |
| U3 | What is the meaning of each attack/armour class id beyond 3 (pierce) and 4 (melee)? Units carry ids up to ~35. | Needed to *document* the counter web, not to compute it — the formula is class-agnostic. | Community class table; the wiki's armor-class page lists them. |
| ~~U4~~ | ~~Damage order and minimum damage~~ **RESOLVED** — published formula, two sources agreeing: per-class `max(0, atk−arm)`, summed, ×elevation, then floor at 1. | — | Implemented spec in `combat.md`. Only 5.7-specific elevation values remain unverified. |
| U5 | Are unit speeds in tiles/second, and what is the tile size in world units? Speed 1.35 (knight) is consistent with tiles/s. | Movement, collision, pathing all need a unit. | Oracle: time a unit crossing N tiles; compare against 1.35. |
| ~~U6~~ | ~~Reload semantics~~ **MOSTLY RESOLVED** — interval = `displayed_reload_time` (sec), DPS = damage/interval; techs scale the interval (×0.8, ×0.75). Animation adds wind-up/down. | — | Tick rate split out to U12. |
| U7 | What pathing algorithm does the game use (A*, flow field, something else), and what are its cost weights? | Any divergence changes unit arrival times and therefore every fight. | RE (question-scoped, xref-anchored) or behavioural matching. |
| U8 | What do the 190 entries of the per-civ `resources` array mean? British has `1.6`, `175`, `178`, `0.3`, `32000`, `700`, `15`, `25`, `25` at scattered indices. | Likely holds civ-wide constants (gather, decay, pop cap) that the sim needs. | Map known values (e.g. 32000 might be a max) then oracle-test each candidate. |
| U9 | How are display names resolved from string ids into the language files? | UI only — not needed for a headless sim. | Read `resources/en/` string tables. |
| U10 | When a `type 3` upgrade fires (e.g. Feudal Age: 109→71, 103→105, 68→129), does it replace live units, and what happens to units in production? | Age-up correctness; wrong here breaks every game. | Oracle: have units queued at age-up and observe. |
| ~~U11~~ | ~~Farming rate~~ **VALUE FIXED (D5, dat-authoritative): 0.400**, the farm building's `work_rate`; mechanism unresolved. Construction hypothesis **tested and falsified** (`HOUS` has `work_rate=0.0` and builds in 25 s; build time is a separate field). `work_value1 = 1.0` everywhere, so no task scaling hides a third number. | — | One-scenario test in `docs/decisions.md` D5. |
| **U13** | **Per-civ differentiation is not in the unit table.** Measured 2026-10-23: across all 31 non-Gaia civs, the unit tables differ from civ 1 (British) in **5 rows out of ~36,000** — Khmer zeroes `carry` (100→0) and sets `work_rate` 0.0→1.0 on houses/drop sites, and Celts carry one extra unit (444). Civ identity lives in `tech_tree_effect_id` (31 distinct effects, each a list of **type-102** commands whose `d` is a *disabled tech id*; British 31, Goths 38), `team_bonus_effect_id`, and the 211-value `resources` array (U8). **Corrected:** `unique_unit_effect_ids` and `name2` are **DE-5.9-only fields** (the parser gates them `min_ver 5.9, max_ver 5.9`), so they do not exist in the 5.7.2 file at all — the parser was right, and no fork will produce them. Unique units in 5.7 must be derived from the tech tree, not from a per-civ list. | Decides what "one civ" vs "31 civs" actually costs, and whether civ bonuses can be data at all. | Emit the type-102 disable lists; derive unique units from the tech-tree effect. |
| **U14** | **`resource_in` codes for gathering tasks.** Read from `dat.unit_data.tasks` (see below): 0 food, 1 wood, 2 stone, 3 gold are confirmed by the variant that carries them; **15, 16, 17 and 56 are pseudo-resources** whose meanings are inferred but not proven — VMHUN uses 15, VMFOR and VMFAR use 16, VMFIS uses 17, VMMIN also lists 56. | Needed to name the gather task → variant mapping, and to know what a gathered resource *is* for accounting. | Cross-check against drop-site shape (mill vs camp) and published tables; the mapping itself is already derivable without the names. |
| **U12** | **Simulation tick rate for 5.7.** AoK-era programmers describe a fixed "game turn"; a modern measurement found 1/60 s quantisation. 20 Hz vs 60 Hz changes every timer. | Fixed-point scale, timers, and animation all derive from it. | Measurement (recorded game timings) or RE of the main loop. |

## Method note

The decode loop that worked, and should be reused:

1. Pick something the outside world already knows exactly (knight 100 HP, 2/2 armour, 60F 75G).
2. Find it in the dat.
3. Whatever field layout reproduces those numbers is very likely the true layout.
4. Anything that *doesn't* reproduce known values is either an unknown or a parser bug —
   both are findings, neither is a guess.
5. **Identify records by cost and research time, never by assumed id.** Assuming tech 200
   was Double-Bit Axe produced a confident, wrong "the +20% is missing from the dat"
   conclusion; matching on (100 food, 50 wood) found it in tech 202 in one query. Ids shift
   with expansions; prices and times do not.
6. **When something is absent, check what you were looking at.** The "missing" multiplier was
   never missing — the effect belonged to a different tech (Bodkin Arrow). Absence is a
   finding only after the identity of the record is confirmed.

Two parser warts found this way, both recorded in `docs/parse-notes.md`: movement speed
lives in `animation_info.speed`, and internal names arrive as byte arrays.
