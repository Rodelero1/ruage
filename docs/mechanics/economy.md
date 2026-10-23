# Economy and gathering

Source: civ-1 units and `unit_data.tasks[]` in `empires2_x2_p1.dat`.

## Villager, measured

| field | value | source |
|---|---|---|
| carry capacity | **10** | `unit.resource_carry_capacity` |
| drop sites | **109** (TC), **68** (mill) | `unit.task_info.drop_site_unit_ids` |
| work rate | 1.0 | `unit.task_info.work_rate` |
| search radius | 4.0 | `unit.task_info.search_radius` |
| speed | 0.80 | `animation_info.speed` (see parse-notes wart) |
| tasks | **18** | `unit_data.tasks[83]` |

All **known**. Carry 10 and speed 0.8 match the published villager.

Note the drop-site list is *data*, not code: the villager is told which buildings accept
resources. Lumber camps, mining camps and docks are **not** in this list, so either they
register dynamically or the list is a partial default — **unknown**, and it matters,
because it decides whether a villager can drop wood at a dock.

## Tasks are the gather table

Each of the villager's 18 tasks is an action record:

| field | example | reading |
|---|---|---|
| `action_type` | 110, 3, 109, 122, 7 | which activity (build, move, gather, …) — **unknown** mapping |
| `gather_type` | 0, 1 | category |
| `resource_in` | 15 | resource consumed |
| `resource_out` | 0 | resource produced |
| `work_value1` | 1.0 | the rate — units **unknown** (U1) |
| `work_range` | 0.1, 1.0 | reach |

Sample rows for the villager (task id order as stored):

```
action=110 gather=0 in=15 out=0  wv1=1.0  range=0.10      ← build?
action=3   gather=0 in=-1 out=-1 wv1=0.0  range=1.00      ← move?
action=122 gather=0 in=-1 out=-1 wv1=1.0  range=0.0
```

**This is where gather rates live**, and they are neither unknown nor hand-written: the
rate is `unit.task_info.work_rate` on the villager *variant* for that task. Measured from
the dat (civ 1), cross-checked against the published table:

| task variant | unit id | `work_rate` | published | carry | drop sites |
|---|---|---|---|---|---|
| Forager (`VMFOR`/`VFFOR`) | 120 / 354 | **0.310** | 0.310 | 10 | TC 109, mill 68 |
| Shepherd (`VMSHE`/`VFSHE`) | 592 / 590 | **0.330** | 0.330 | 10 | TC 109, mill 68 |
| Stone miner (`VMMIN`/`VFMIN`) | 124 / 220 | **0.360** | 0.359 | 10 | TC 109, camp 584 |
| Gold miner (`VMGLD`/`VFGLD`) | 579 / 581 | **0.380** | 0.379 | 10 | TC 109, camp 584 |
| Lumberjack (`VMLUM`/`VFLUM`) | 123 / 218 | **0.390** | 0.388 | 10 | TC 109, camp 562 |
| Hunter (`VMHUN`/`VFHUN`) | 122 / 216 | **0.410** | 0.408 | **35** | TC 109, mill 68 |
| Fisher (`VMFIS`/`VFFIS`) | 56 / 57 | **0.430** | 0.426 | 10 | TC 109, mill 68 |
| Farmer (`VMFAR`/`VFFAR`) | 259 / 214 | **0.530** | 0.319–0.356 | 10 | TC 109, mill 68 |
| Fishing ship (`FSHSP`) | 13 | **0.280** | 0.280 | 15 | dock 45 |
| Trade cart (`TCART`) | 128 | **0.230** | — | 100 | market 84 |

Eight of nine land resources agree with the published table to within rounding. **Farming is
the outlier**, and it is now resolved by decision rather than by measurement (D5,
`docs/decisions.md`): the dat gives the farmer variant 0.530 and the farm *building* (`FARM`,
id 50) **0.400**. Checked and ruled out: `work_value1` is 1.0 on every gather task, so nothing
scales 0.530 down to a plausible number.

**Taken: effective farm rate = 0.400**, the farm building's own `work_rate`. 250 food lasts
625 s at 0.400, 472 s at 0.530, ~784 s at the published 0.319.

Two hypotheses tested and rejected, so they don't get re-litigated:

- **Construction speed** — no. `HOUS` has `work_rate = 0.0` and still builds in 25 s, build
  time is a separate field (`creation_info`, farm 15 s / house 25 s / barracks 50 s), and a
  0.4 multiplier would make the 15 s farm slower than the 25 s house.
- **A task-record scaling factor** — no. `work_value1 = 1.0` on every gather task in the dat.

What the farm actually is: a resource *container*. Its two tasks are `action 21` / `action 121`
with `resource_in = 16` (food), `resource_out = -1` — it holds food rather than producing it,
so its `work_rate` reads as a throughput limit on extraction. Not proven: `FTRAP` carries
`work_rate = 1.0`, so containers are not uniform. See D5 for the test that would settle it.

This also corrects an earlier note: the base villager (`VMBAS`, 83) lists drop sites
`[109, 68]` only, but each **task variant** carries its own — the lumberjack lists the
lumber camp (562) and the miner the mining camp (584). Drop-site lookup must use the
variant, not the base unit.

## Tech multipliers — decoded, and the DBA question is closed

Type-5 commands are multipliers over unit attributes, and the attribute id is the third
field. Verified against published bonus values:

| tech | effect | command | reading |
|---|---|---|---|
| Wheelbarrow | 200 | `(5, -1, 4, 5, 1.1)` | speed ×1.1 |
| Wheelbarrow | 200 | `(5, -1, 4, 14, 1.25)` | carry capacity ×1.25 |
| **Double-Bit Axe** (tech 202) | 195 | `(5, 218, -1, 13, 1.2)`<br>`(5, 123, -1, 13, 1.2)` | **wood gather ×1.2** on `VFLUM`/`VMLUM` |
| Bow Saw (203) | 196 | same pair, `1.2` | wood gather ×1.2 |
| Gold Mining (55) | 55 | `(5, 579/581, -1, 13, 1.15)` | gold gather ×1.15 |

So: **`c = 13` is the gather-rate attribute**, `c = 5` is speed, `c = 14` is carry capacity,
`c = 0` is hit points. 115 type-5 commands touch `c = 13` — the economy upgrades are a
regular, decodable family.

**Correction to an earlier note in this project:** the +20% was previously reported as
missing from the dat. It is present. The error was a wrong tech id — tech 200 is **Bodkin
Arrow** (200 food / 100 gold, 35 s), not Double-Bit Axe, which is tech **202**. Matching on
cost instead of assuming ids is what exposed it.

Consequence: the economy is `base work_rate × Π(type-5 multipliers) × civ bonuses`, all of it
in the dat.

## Resource ids

0 = Food, 1 = Wood, 2 = Stone, 3 = Gold — **inferred** from costs matching published
prices across dozens of units. Resource 4 appears with quantity 1 on nearly every unit and
is not a cost.

## Farm and decay

`unit.resource_decay_rate` and the farm's 480 HP are in the data; how farm food depletes
(250 per farm, and whether decay interacts) is **not** stated — **unknown**, oracle test.

## Open

- ~~**U1** gather rate application and the missing DBA multiplier~~ — **resolved**: rates are
  `work_rate` per task variant, multipliers are `c=13` type-5 commands.
- ~~**U11** farming rate~~ — **resolved by decision (D5)**: effective rate 0.400, the farm
  building's own `work_rate`, capping the villager's 0.530. Rationale and the cheap
  re-measurement test are in `docs/decisions.md`.
- **U8** the 190-entry per-civ `resources` array: likely holds civ-wide economic constants.
- Farm depletion and resource decay semantics (`resource_decay_rate` is in the dat, unused
  so far).
- Whether the villager *swaps unit id* on task change (variants suggest yes) — the sim needs
  to model that or fold the variants into one entity with a task state.
