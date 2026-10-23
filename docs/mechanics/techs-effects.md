# Techs and effects

Source: `techs[]` and `tech_effects[]` in `empires2_x2_p1.dat`.

## Shape of the system

- **801 techs**, **798 effects** — **known**.
- A tech is: costs (3 slots), research time, `effect_id`, `required_tech_ids[]` (6 slots),
  `location_unit_id` (where it is researched), string ids for name/description.
- An effect is a **name plus a list of commands**, each `(type, a, b, c, d)`:
  `type` is i8, `a`/`b`/`c` are i16, `d` is f32.

Tech → effect is a many-to-one-ish mapping by `effect_id`; several techs can share an
effect, and some effects are used directly as civ bonuses rather than by any tech.

## Command type histogram — **known**

| type | count | note |
|---|---|---|
| 4 | 1,091 | attribute modifier — the workhorse |
| 5 | 576 | multiplier (verified by Wheelbarrow) |
| 3 | 227 | unit upgrade / transform (verified by Feudal Age) |
| 2 | 117 | unknown |
| 0 | 106 | unknown |
| 101, 102, 103 | 97 / 942 / 25 | large blocks, likely disable/enable lists |
| 1 | 84 | resource grant (age-up) — inferred |
| -1 | 36 | sentinel/empty |
| 6 | 2 | rare — candidate for the missing gather bonus (U1) |
| 11, 15 | 2, 1 | rare |

## Verified decodes

The method: take a tech whose real effects are published, find its commands, read off the
fields.

**Loom** (tech 22 → effect 22, 50 gold, 25 s):

```
(4, -1, 4, 0, 15.0)      → +15 HP
```

Line up with the published Loom (+15 villager HP): `type 4` = attribute modifier,
`c = 0` = HP, `d = 15` = the amount, `a = -1` = all units, `b = 4` = class filter.
**Inferred**, high confidence.

**Wheelbarrow** (tech 213 → effect 200, 175F 50W, 75 s):

```
(5, -1, 4, 5,  1.1)      → speed ×1.1
(5, -1, 4, 14, 1.25)     → carry capacity ×1.25
```

Both match the published Wheelbarrow exactly. So `type 5` = multiplier, and `c` is an
attribute id: **5 = speed, 14 = carry**. **Inferred**, high confidence.

**Forging** (tech 67 → effect 67, 150 food, 50 s):

```
(4, -1, 6,  9, 1025.0)   → +1 attack, class 6 units
(4, -1, 45, 9, 1025.0)   → same for unit 45
... six such commands
```

`c = 9` = attack, and `d = 1025 = 1024 + 1`: the 1024 bit flags "add" for attack/armour
rather than "set". Loom's HP modifier needed no such flag. **Inferred** — the 1024 offset
is the least certain part of this decode (U2).

**Feudal Age** (tech 101 → effect 101, 500 food, 130 s):

```
(1, 6, 0, -1, 1.0)            → resource grant (age counter)
(3, 109, 71, -1, 0.0)         → TC 109 → 71
(3, 103, 105, -1, 0.0)        → barracks 103 → 105
(3, 68, 129, -1, 0.0)         → mill 68 → 129
(3, 45, 133, -1, 0.0)         → dock 45 → 133
... etc, 15 commands
```

`type 3` = **unit transform** (`a` old id → `b` new id). This is how buildings visibly
change appearance and stats on age-up. Whether it rewrites existing instances, and what
happens to units mid-production, is **unknown** (U10).

## What this buys us

Tech application is a small interpreter over 13 command types, not 800 special cases. Two
of the important ones are decoded with high confidence, one (type 3) is decoded, and the
rest are questions.

## Attribute ids used by `type 5` — decoded

The `c` field of a type-5 (multiplier) command is an attribute id. Four are pinned by
matching published tech bonuses:

| `c` | attribute | evidence |
|---|---|---|
| 0 | hit points | Loom's `type 4` uses `c=0` for +15 HP |
| 5 | speed | Wheelbarrow `×1.1` (published +10% villager speed) |
| **13** | **gather rate** | DBA `×1.2` wood, Gold Mining `×1.15` gold, Bow Saw `×1.2` |
| 14 | carry capacity | Wheelbarrow `×1.25` (published +25% carry) |

Frequency of `c` across all 1,091-ish type-5 commands: `0` (176), `13` (115), `100` (88),
`10` (45), `101` (41), `5` (38), `9` (22), `104` (13), `105` (14), `1` (11). Ids ≥ 100 are
likely resource-amount multipliers (farm food, building HP) — not yet confirmed.

## Open

- ~~**U1** — the missing Double-Bit Axe gather multiplier~~ — **resolved**: it is present in
  effect 195 as `(5, 218, -1, 13, 1.2)`. The earlier "missing" claim came from assuming
  tech 200 = Double-Bit Axe; tech 200 is **Bodkin Arrow**, DBA is tech **202**. Lesson
  recorded: match techs by cost and research time, never by assumed id.
- **U2** — remaining `type` semantics: 0, 2, 6, 101–103, and the 1024-add convention for
  attack/armour (`d = 1024 + n`).
- **U10** — application timing of type-3 upgrades.
- Tech names/descriptions are string ids into the language tables, not text in the dat
  (**known**; resolution is U9).
- Whether `required_tech_ids` encodes ages as pseudo-techs: the six slots likely mean
  "age + prerequisites", unverified.
