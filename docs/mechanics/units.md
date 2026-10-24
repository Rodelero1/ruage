# Units

Source: `build/tools/units-table.py` against `empires2_x2_p1.dat`, civ 1 (British).
Unit ids and internal names are the game's own; display names need the language tables
(unknown U9).

## Core units — measured

| id | internal name | HP | atk (melee/pierce) | armour (m/p) | range | reload | speed | cost | train |
|---|---|---|---|---|---|---|---|---|---|
| 83 | `VMBAS` | 25 | 3/— | 0/0 | — | 2.0 | 0.80 | 50F | 25 |
| 74 | `SPRMN` | 40 | 4/— | 0/1 | — | 2.0 | 0.90 | 60F 20G | 21 |
| 75 | `SWDMN` | 45 | 6/— | 0/1 | — | 2.0 | 0.90 | 60F 20G | 21 |
| 77 | `THSWD` | 60 | 9/— | 0/1 | — | 2.0 | 0.90 | 60F 20G | 21 |
| 93 | `PKEMN` | 45 | 3/— | 0/0 | — | 3.0 | 1.00 | 35F 25W | 22 |
| 358 | `ISPKM` | 55 | 4/— | 0/0 | — | 3.0 | 1.00 | 35F 25W | 22 |
| 4 | `ARCHR` | 30 | —/4 | 0/0 | 4.0 | 2.0 | 0.96 | 25W 45G | 35 |
| 7 | `XBOWM` | 30 | —/2 | 0/3 | 4.0 | 3.0 | 0.96 | 25F 35W | 22 |
| 24 | `CARCH` | 35 | —/5 | 0/0 | 5.0 | 2.0 | 0.96 | 25W 45G | 27 |
| 38 | `KNGHT` | 100 | 10/— | 2/2 | — | 1.8 | 1.35 | 60F 75G | 30 |
| 40 | `CATAP` | 110 | 9/— | 2/1 | — | 1.8 | 1.35 | 70F 75G | 20 |
| 39 | `CVRCH` | 50 | —/6 | 0/0 | 4.0 | 2.0 | 1.40 | 40W 60G | 34 |
| 448 | `SCOUT` | 45 | 3/— | 0/2 | — | 2.0 | 1.20 | 80F | 30 |
| 279 | `SCBAL` | 40 | 0/12 | 0/7 | 7.0 | 3.6 | 0.65 | 75W 75G | 30 |
| 12 | `BRKS` | 1200 | — | 0/7 | — | — | — | 175W | 50 |
| 68 | `MILL` | 600 | — | 0/7 | — | — | — | 100W | 35 |
| 109 | `RTWC` | 2400 | —/5 | 3/5 | 6.0 | 2.0 | — | 275W 100S | 100 |
| 70 | `HOUS` | 550 | — | 0/7 | — | — | — | 25W | 25 |
| 45 | `DOCK` | 1800 | — | 0/7 | — | — | — | 150W | 35 |
| 50 | `FARM` | 480 | — | 0/0 | — | — | — | 60W | 15 |
| 562 | `SMIL` | 600 | — | 0/7 | — | — | — | 100W | 35 |
| 84 | `MRKT` | 1800 | — | 1/8 | — | 2.5 | — | 175W | 60 |

Every value above matches the community-known AoC/HD figures (knight 100 HP 2/2 at 60F 75G,
archer 30 HP 4 pierce at 25W 45G, TC 2400 HP at 275W 100S, house 550 at 25W, farm 480 at
60W). That agreement is the evidence that the field layout is right — **known**.

Notes:
- `7`'s internal name is `XBOWM` although its stats are the skirmisher's (25F 35W, 3 pierce
  armour). Internal names are historical, not descriptive. Trust stats, not names.
- Buildings have `speed = 0` and no reload, except towers/TCs which do shoot (`RTWC` has
  range 6, reload 2.0, pierce attack 5).
- Costs are `[resource_id, quantity]`; resource ids 0=Food, 1=Wood, 2=Stone, 3=Gold are
  **inferred** from the values lining up with known costs. A fourth slot with `quantity=1`
  appears on every unit and is not a cost (candidate: population or a flag) — see U2.

## Counts

- 1,402 unit **slots** per civilisation, of which **1,166 are populated** (ids 0..1401 with 236 holes); 32 civilisations, civ 0 = Gaia — **known**. Iterate the populated set, never `0..N`.
- 12 distinct `class` values carry most of the table; class 6 (83 entries) and class 12
  (72) are the largest bins — **known**, semantics **unknown** (U3).
- Units are duplicated per civ: most rows are identical across civs, with per-civ stat and
  availability differences layered on top — **known**.

## Train time

Train time is not a unit field. It lives in the **training building's** list of what it can
produce: `creation_info.train_locations[] = [unit_id, train_time, button, hotkey]`, i.e. the
barracks lists "militia, 21 s". The number in the table above is the first entry found for
that unit — **known**, cross-checked (villager 25 s, archer 35 s, knight 30 s).

Implication: to answer "how long does this unit take in *this* building", the sim must
consult the building, not the unit. If the same unit is trainable in two buildings with
different times, the dat allows both.

## Per-civ differentiation is not in the unit table (measured 2026-10-23)

The units are stored per civilisation (1,402 slots × 32), which invites the assumption that each
civ needs a delta layer. Measured: it does not. Comparing every populated unit of all 31 non-Gaia
civs against civ 1 (British) gives **five differing rows in total**:

| civ | unit | civ 1 | that civ |
|---|---|---|---|
| Khmer (28) | 70 (house) and 463–465 (drop sites) | `carry` 100, `work_rate` 0.0 | `carry` 0, `work_rate` 1.0 |
| Celts (13) | 444 | absent | present (1950 hp, speed 0.825) |

Everything else — hp, speed, costs, attacks, armours, range, reload — is byte-for-byte the same
table repeated. Civ identity therefore lives elsewhere:

- `civ.tech_tree_effect_id` — 31 distinct effects; each is a list of **type-102** commands whose
  `d` is the id of a *disabled* tech. British disables 31, Goths 38. This is the tech tree.
- `civ.team_bonus_effect_id`, `civ.unique_unit_effect_ids` (the latter unreadable through
  GenieDatParser at `VER 5.7.2`), and the 211-value `resources` array (U8).

Consequence for D3: making a second civ playable costs a disable-list join, not a data-model
change. The expensive part of civ support is U8 and unique units, not the unit table.

## Open

- **U3** — class id → semantic name for all classes.
- **U9** — display names from string ids.
- Whether any per-civ unit differs in HP/attack (not just availability): requires a diff
  across all 32 civs. Cheap to run, not yet done.
