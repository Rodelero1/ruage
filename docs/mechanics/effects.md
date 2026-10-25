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

## Next

Pin kinds 4 and 5 (attribute multiply and add) the same way — by finding effects whose published
effect is known and checking that the arguments line up — then implement them, then 102's disable
list, which is what a civilisation's tech tree actually is.
