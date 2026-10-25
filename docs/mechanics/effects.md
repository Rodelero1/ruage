# Techs and effects: what the data says, and what it does not

Status: **loaded and censused, deliberately not interpreted yet.** The loader and the census are in;
the interpreter waits until each command kind's arguments are pinned by the data.

## Shape

A tech is a price and a pointer to an effect; an effect is a list of `[kind, a, b, c, value]`
commands.

```toml
[[tech]]                         # techs.toml
id = 2
effect_id = 454
research_time = 45
costs = [[0, 1000], [3, 500]]

[[effect]]                       # effects.toml
id = 454
commands = [[4, 83, -1, 9, 1.25], ...]
```

801 techs, 798 effects, **3,306 commands**. `effect_id = -1` occurs and means "applies nothing",
so the loader exposes `Option<u32>` rather than refusing the file or inventing a meaning.

## The census is the interpreter's to-do list

Command kinds actually used, by count:

| kind | count | pinned? |
|---|---|---|
| −1 | 36 | no |
| 0 | 106 | no |
| 1 | 84 | no |
| 2 | 117 | no |
| 3 | 227 | no |
| 4 | 1,091 | no |
| 5 | 576 | no |
| 6 | 2 | no |
| 11 | 2 | no |
| 15 | 1 | no |
| 101 | 97 | **no** — see U16 |
| 102 | 942 | **yes** — names a tech |
| 103 | 25 | no |

The plan is not "implement thirteen kinds". It is: pin the arguments of the kinds that matter, in
order of how much of the game they carry. Kind 4 and 5 alone are half the commands.

## How a kind gets pinned

By counting counterexamples in the data, not by looking it up.

**Kind 102 is pinned.** Its fifth argument resolves to a tech id in 941 of 942 commands. The one
exception is `[102, -1, -1, -1, -1]` in the Koreans Technology Tree — a no-op, and the same `-1`
sentinel that means "unresolved" everywhere else. So 102 disables a tech, and the test names the
exception rather than tolerating a mystery.

**Kind 101 is the interesting failure.** It looked like the unit counterpart of 102 — "enable or
disable this unit" — and the first test found 13 of 97 commands naming an id that is not a unit.
Investigation showed they name **techs** (202 is double-bit axe, 203 is bow saw), inside bonuses
like `C-Bonus, free lumberjack upgrade`. Worse, checking both tables shows the split is
**0 unit-only, 84 in both tables, 13 tech-only**: unit ids and tech ids share a range, so an id
cannot say which table a command means.

A `UnitTouch { unit }` reading would therefore have been right 87% of the time and wrong exactly
where it mattered — a free *upgrade* would have become a free *villager*. The type now has an
`AmbiguousId` variant that carries the id and nothing else, and the finding is registered as
**U16**. Whatever distinguishes the two uses is in the `b`/`c` fields, not the id.

## What the tests hold

- the project files load: 801 techs, 798 effects, >3,000 commands;
- every tech with an effect points at one that exists (none dangling, -1 excepted);
- kind 102 names a tech in 941 of 942, the exception being the Koreans no-op;
- kind 101's ids are **not** separable by table (0 unit-only, 84 both, 13 tech-only);
- a fractional command kind, a duplicate effect id, and a negative research time are refused;
- command values land in fixed point (D9), not floats.

## Kinds 4 and 5 are pinned, and the reading is not the one the fields suggest

The quintuple is `[kind, a, b, c, value]`. Read against techs whose result is published, one
reading explains every clean case:

| field | meaning |
|---|---|
| `kind` | **4 adds**, **5 multiplies** |
| `a` | a unit id, or −1 for "no single unit" |
| `b` | a **unit class**, or −1 |
| `c` | the attribute: **0 hit points, 1 range, 8 armour, 9 attack** |
| `value` | the delta (kind 4) or the factor (kind 5) |

| tech | commands | published result | reading |
|---|---|---|---|
| **Loom** | `[4, -1, 4, 0, 15]` | villagers +15 HP | add 15 HP to class 4 (civilians) |
| **Forging** | `[4, -1, 6, 9, 1025]` | infantry +1 attack | add **packed** 1025 to class 6 (infantry) |
| **Fletching** | `[4, -1, 0, 1, 1]`, `[4, -1, 0, 9, 769]` | archers +1 range, +1 pierce attack | add 1 to attribute 1; add packed 769 |
| **Masonry** | `[5, -1, 3, 0, 1.1]` | buildings +10% HP | **multiply** HP by 1.1 on class 3 |

**Kind 5 multiplies, which is how Masonry identifies itself.** If kind 5 added, Masonry would give
every building 1.1 extra hit points and do nothing — the multiplier reading is forced by the value,
not chosen.

**Attack is packed: `class * 256 + amount`.** Forging's 1025 is 0x401 — class 4 (melee), one point;
Fletching's 769 is 0x301 — class 3 (pierce), one point. The alternative reading is "+1025 attack",
and the class ids that fall out (4 melee, 3 pierce) are the same ones `docs/mechanics/combat.md`
pinned independently from knight and archer stats. Two unrelated roads to the same numbers.

### The gap: negative packed values (U17)

Six commands pack a negative attack, in exactly two effects:

```
Non-Research (217)   [5, -1, 13, 9, -136]  x3  (classes 13, 54, 22)
Shinkichon  (506)    [4, 280, -1, 9, -226] x3  (units 280, 550, 588)
```

A negative cannot be an unsigned `class * 256 + amount`. The likeliest reading is a signed amount in
the low byte with the class in the high byte (`-226` = 0xFF1E = class −1, amount 30), but nothing
else in the file corroborates it, so it is **not decoded**. The test names all six, so a change in
them is caught, and the gap is registered as U17. Guessing here would put a plausible-looking wrong
number into every Korean game.

## Next

Pin kinds 4 and 5 (attribute multiply and add) the same way — by finding effects whose published
effect is known and checking that the arguments line up — then implement them, then 102's disable
list, which is what a civilisation's tech tree actually is.
