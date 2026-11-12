# The repaths were ground, not bodies

D192 landed with one number moving the wrong way and no explanation: **repaths 240 -> 409**. This slice built the instrument and
answered it.

## Performance - measured on the same box, one run at a time

The two sides are the same commit pair as D192's landing, run with **the same argument list** - the older binary does not accept
`--techs`/`--effects`, so both runs use the minimal set and are comparable (both end `players=2`):

| | pre-D192 (`b7d92f2`) | D192 (`e66dc05` + counters) |
|---|---|---|
| repaths | **153** | **166** (+8.5%) |
| of which **body** | **31** | **0** |
| of which **ground** | 122 | 166 (+36%) |
| made_room | 46 | 0 |
| entities at 1,800 s | 146 | 129 |
| hash | `dd31f4b01124f395` | `20ab52a041ba01d8` |

**The answer**: bodies stopped forcing re-plans altogether (31 -> **0**) - men standing on a tile's border instead of queuing on a ring
no longer bump each other out of their routes, which is the same fact `made_room` 46 -> 0 records from the other side. Every repath
the slice added is **ground**: 122 -> 166. The plausible reading is that a place now sits against the node's blocked tile, so an
approach that grazes it costs a re-plan where the 1.156 ring kept men clear of it; it is **not measured**, and each route is bounded
at two ground re-plans (`MAX_GROUND_REPLANS`).

The absolute size is small: 166 re-plans over 36,000 ticks is one every 217 ticks, against a tick of 6.7 us. The acceptance figure in
D192's review (240 -> 409) is the same phenomenon on the richer scenario (techs and effects enabled): ground up, bodies to zero.

## What was built

Two counters, `repaths_by_body()` and `repaths_by_ground()`, split at the single place `repath_count` is incremented, using the
`Repath.body` the struct already carried. Instrumentation, not state: never hashed, never read by the tick.

The headless line gains `repaths_body=` and `repaths_ground=` after the existing fields, so a parser that reads the prefix still works.

## Stability

- **Test-first**: `crates/sim/tests/repath_reasons.rs::a_body_on_the_line_is_counted_as_a_body` was written before the counters and went
  RED twice for real reasons: first `total 0, ground 0` because the enemy was placed at the midpoint of start and goal, which is **not**
  on the route (the route runs tile centre to tile centre), then `MissingEntity` because a spawn only lands on the next tick and the
  `Move` was submitted in the same tick. The final test samples the unblocked route first and puts the body on
  it, and asserts the split equals the total.
- **Fixture trap worth keeping**: two `Spawn`s in one tick take the same id (`next_id` reads the entity list, which the spawns do not
  reach until the next tick) - the same bug `evasion.rs` documents. The test now names its ids explicitly.
- **Full sim suite green** after the change; `clippy -D warnings` and `fmt --check` clean.
