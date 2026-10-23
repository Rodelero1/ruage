# Combat

Source: `unit.combat_info` in `empires2_x2_p1.dat` (civ 1), cross-checked against
community-known unit stats. All **known** unless tagged otherwise.

## The data model is an arbitrary pair of arrays

Each unit carries:

- `combat_info.attacks[]` — a list of `(class_id, amount)`
- `combat_info.armors[]` — a list of `(class_id, amount)`

There is no fixed "melee" and "pierce" slot. Both are open lists, and units use them
differently — attacker arrays are typically 4–9 entries.

## Base class ids — **inferred**

Two ids were pinned by finding units whose real-world armour values are publicly known:

| id | meaning | evidence |
|---|---|---|
| 4 | melee attack / melee armour | knight `atk (4,10)`, `arm (4,2),(3,2)` = 10 attack, 2/2 armour |
| 3 | pierce attack / pierce armour | archer `atk (3,4)`, skirmisher `arm (3,3)` |

Militia `arm (4,0),(3,1)` = 0 melee / 1 pierce, which is the published militia armour. Three
independent units agreeing on the same two ids is what makes this an inference rather than
a guess — but the remaining ids are **not** decoded (U3).

## Bonus damage lives in the same array

Class ids beyond 3/4 are damage *bonuses and resistances*. Two clean examples:

| unit | attacks | armours | reading |
|---|---|---|---|
| Cataphract (`CATAP`, 40) | `(4,9)`, `(1,9)`, `(32,9)`, … | `(4,2)`, `(8,12)`, `(3,1)`, `(19,0)` | 9 melee, +9 vs infantry (class 1), and **12 cavalry armour** (class 8) — the trait the cataphract is famous for |
| Pikeman/spearman family (`PKEMN`, 93) | `(4,3)`, `(5,15)`, `(8,15)`, `(16,9)`, `(30,12)`, `(35,4)`, `(34,9)` | `(4,0)`, `(3,0)` | 3 melee plus a spread of anti-cavalry/anti-building bonuses |

So the counter system is **entirely data-driven**: nothing about "pikes beat cavalry" is
hardcoded, it is a `(8, +15)` entry. A sim that reads these arrays gets the counter web for
free — provided the class ids are mapped (U3).

## Other combat fields

| field | type | example |
|---|---|---|
| `max_range` / `min_range` | f32 | archer 4.0, scorpion 7.0 |
| `displayed_reload_time` | f32 | archer 2.0, scorpion 3.6, knight 1.8 |
| `accuracy_percent` | int | archer 80, melee 100 |
| `blast_width` / `blast_attack_level` | | splash |
| `projectile_unit_id` | int16 | links to the arrow/bolt unit |

## The damage formula — **known** (published, two independent sources)

Not inferable from the dat, but thoroughly documented by the community. The per-class
structure the dat stores maps directly onto it:

```
damage = max(1, k_elev × k_cracked × Σ_i [ (1 − k_res) × max( min(0, k_mul_i × Atk_i),
                                                              k_mul_i × Atk_i − Arm_i ) ] )
```

| term | meaning |
|---|---|
| `Atk_i` | attacker's attack value for class *i* |
| `Arm_i` | target's armour for class *i*; if the target lacks the class, treated as a huge sentinel (1000/10000) so the term contributes 0 |
| `k_mul_i` | attack multiplier for a class (Siege Engineers ×1.2 on building attack, etc.), else 1 |
| `k_res` | bonus-damage resist multiplier (Sicilians/Bengalis in DE; **0 in AoC/HD**) |
| `k_elev` | 1.25 attacking from higher ground, 0.75 from lower, else 1 |
| `k_cracked` | 1.2 on cracked terrain vs buildings (DE feature; not in 5.7) |

Reading it in words, which is what the sim must implement:

1. **Per armour class**: `max(0, attack_i − armour_i)`. Bonus damage cannot go negative, and
   melee/pierce armour does not reduce bonus damage — each class is subtracted separately.
2. **Sum across classes** — this is why the dat's arrays are open lists rather than two slots.
3. **Multiply** by elevation (and cracked terrain).
4. **Floor at 1** — applied *after* the multipliers.

Sources: the Fandom wiki's *Armor class (Age of Empires II)* page (step-by-step derivation)
and *Pierce armor* (closed form), which agree with each other and with `aoe2database.com`.
Both describe DE; the AoC/HD case is the subset with `k_res = 0` and no cracked terrain,
which is what the tables above assume.

**Consequence for the sim:** the whole counter web is computable from the dat's
`attacks[]`/`armors[]` arrays plus this formula. The only remaining piece is the class id →
name mapping (U3), and even that is not needed to *compute* damage — only to document it.

## Rate of fire — **mostly known**

- Attack interval is driven by `displayed_reload_time` (dat, seconds). DPS = damage / interval.
- Techs modify the **interval**, not the rate: Celts' "siege fire 20% faster" is a ×0.8
  multiplier on reload (1/0.8 = 1.25, i.e. +25% rate); Japanese infantry ×0.75 → +33%.
- The animation matters: published analyses describe attack rate as
  `wind-up + damage frame + wind-down + reload`, and report that melee units can only land
  damage at 0% or 50% of the animation, so effective melee rate is slightly faster than the
  raw reload time.
- **Tick rate is unresolved** (U12): the AoK-era engine programmers described a fixed "game
  turn", and a modern measurement found combat timings quantised to 1/60 s. Whether 5.7
  simulates at 20 Hz, 60 Hz, or a mix is not established.

What this means for G1: reload-based DPS is safe to implement from the dat. Exact
animation-cancelling behaviour is not, and is only needed for G2.

## Buildings

Buildings carry armour like units (`BRKS` 0/7, `MRKT` 1/8) and some have attack
(`RTWC` pierce 5, range 6, reload 2.0). Siege bonus damage against buildings is expressed
as extra attack classes on the siege unit, not as a building flag.
