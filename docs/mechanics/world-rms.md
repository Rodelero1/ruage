# World: terrain, maps, AI

Source: install inventory under `Age2HD/resources/_common/` plus the dat.

## Asset counts — **known**

| kind | count | where |
|---|---|---|
| `.slp` sprites | 3,623 | `resources/_common/` |
| `.rms` random map scripts | **122** | 73 `drs/gamedata_x2`, 34 `drs/gamedata_x1`, 15 `random-map-scripts` |
| `.per` AI scripts | 74 | 31 `drs/gamedata_x2`, 34 `drs/gamedata_x1`, 10 `ai/Economy` |
| `.per2` (newer AI) | 30 | AI dirs |
| `.scx` scenarios | 49 | |
| `.wav` | 2,246 | |

There are **no `.drs` archives** — the directory is literally named `drs/` but contains
loose files. Nothing needs unpacking.

## Terrain

- 100 terrains, 42 terrain borders, 22 terrain restrictions in x2 (**known**, parsed).
- Each terrain carries name, colours, and pass-graphic data; terrain *restrictions* gate
  which units may walk where (a unit's `terrain_restriction_id` points into this table).

The mapping from restriction table → "this is water, only ships" is **unknown** and is pure
data lookup: it decides what a unit can path over. Cheap to establish by inspection, worth
doing before pathing work.

## `.rms` — random map scripts

These are a small declarative language, not compiled code. Example (`drs/gamedata_x2/Arabia.rms`):

```
<PLAYER_SETUP>
  random_placement
    ai_info_map_type ARABIA 0 0 0

<LAND_GENERATION>
  start_random
  ...
```

Section-driven: `<PLAYER_SETUP>`, `<LAND_GENERATION>`, `<TERRAIN_GENERATION>`,
`<CONNECTION_GENERATION>`, `<OBJECTS_GENERATION>`, with `start_random`/`end_random`
choosing among weighted alternatives. 122 scripts, all readable text.

**Consequence:** map generation is reimplementable as a parser + interpreter over these
files, and it is the cheapest route to playable maps without RE. It is also a
determinism landmine: `start_random` implies the RNG *is* the map, so map RNG must be part
of the seeded stream or two clients will generate different maps.

## `.per` — AI scripts

Fact/action rule language, e.g.:

```
(defrule
    (difficulty equal easiest)
=>
    (set-strategic-number sn-consecutive-idle-unit-limit 60)
    (set-strategic-number sn-initial-attack-delay 1200)
    (disable-self)
)
```

Rules are `(condition...)=>(action...)` with `(disable-self)` for one-shot rules. A few
hundred lines of interpreter covers it. **known** that this is the language shape;
**unknown** the full fact/action vocabulary (it is large — dozens of facts, ~100 actions).

Implication for the sim: AI is a separate interpreter over a fixed vocabulary, and the
stock `.per` files are the specification of that vocabulary. It can be built incrementally
(`(difficulty …)` and `(set-strategic-number …)` first) without blocking on anything else.

## Open

- Terrain restriction id → passability class (**unknown**, needs inspection).
- `.rms` RNG stream: does `start_random` share the game RNG or its own? (**unknown**,
  determinism-critical).
- `.per` vocabulary completeness; which files are actually loaded for HD's AI.
- `.slp` decoding is a separate format question — not needed until rendering.
