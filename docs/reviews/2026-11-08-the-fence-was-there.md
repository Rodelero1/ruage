# The virtual fence was there; it was painted the same colour as the yard (D162)

*2026-11-08. The owner, with a screenshot of three villagers standing on a town centre: *"I still see huge overlap. Is it possible to have a
virtual border around each building and no unit['s] center can cross it? Except for Farm and Town Center (for TC the virtual fence would be
around the impassable structure of the building in the top quarter of the diamond - highlighted in lighter gray). Buildings cannot overlap
another building or functioning farm. Also, the drop of the resources would happen when the unit center … touches this outlying diamond."*

## What the engine already does - checked, not assumed

| The owner's rule | Where it stands |
|---|---|
| A building has an impassable border no unit's centre crosses | `Walk` grid: every **solid** tile of a footprint is blocked, and a unit's position *is* its centre, so a centre on a blocked tile is impossible (`no_unit_centre_crosses_the_core`, new) |
| Except farms | A farm's floor is **walkable** while it lives (`a_farm_is_a_floor_while_it_is_built_and_after`) |
| Except the town centre, whose fence is around the structure in the **top quarter** | Exactly what the mask says: the town centre's 4×4 footprint blocks **4 tiles** - a 2×2 at the **north corner**, the anchor - and the other 12 are yard. Read out of the running world: `[true,true,false,false, true,true,false,false, false×8]` |
| Buildings cannot overlap a building or a functioning farm | `site_refusal` refuses blocked ground, and `a_building_cannot_be_raised_on_a_live_farm` already pins the farm half |
| Resources drop when the centre touches the border | the drop site is a **tile beside** the footprint, and the deposit runs when the unit arrives there - its centre adjacent to the border, never inside |

So three of the five were already true, and the owner's own description of the town centre ("the impassable structure in the top quarter of
the diamond") is *precisely* what D115 built. The report was right about what he **saw** and wrong about what the engine does - because the
picture did not show it.

## Why it looked like "huge overlap"

`BUILDING` was `[200,200,200]` and `BUILDING_YARD` was `[170,178,170]` - a 4-tile dark-ish patch inside a 12-tile light patch, both "light
grey" against light-grey ground. A villager standing in the yard (legal) was drawn on a surface indistinguishable from the wall, so it read
as standing in the building, and three villagers in the yard read as "huge overlap".

## The fix: the fence is now visible

- `BUILDING` (the impassable part) is **dark slate** `[96,100,108]`.
- `BUILDING_YARD` is **light** `[188,196,184]`.
- The gap between them is a **60+ luminance step**, which the new test asserts - so a future palette tweak that makes them read alike fails
  rather than quietly bringing the report back.

## The tests

`crates/view/tests/virtual_fence.rs`, two tests:

- **`the_impassable_core_is_dark_and_the_yard_is_light`**: the core is 4 tiles, the yard 12, each painted its own tone, and the two tones are
  at least 60 luminance apart.
- **`no_unit_centre_crosses_the_core`**: a villager walks a line that crosses the town centre; for 1,200 ticks its **tile** (its centre) is
  never one of the core's four.

One of my own test bugs, worth recording because it is the same class as D160: the first version sampled pixels by subtracting the diamond's
**pixel** radii from a **tile** coordinate, which put the samples tiles away and found nothing solid (`core = 0`). The offset belongs after the
projection.

## Performance

No change to the simulation: the acceptance hash is **byte-identical**, and the only code change is two palette constants plus the mask
plumbing D161 already added. **Measured**: the new suite runs in **0.95 s** (two tests, one 900×700 render and 1,200 simulated ticks), and the
gate's wall clock is recorded with this commit.

## Stability

- **The mask comes from the walk grid**, so "what the fence is" has one answer; the tone is presentation on top of it.
- **The luminance gap is asserted**, which is the only way a *colour* decision can be a rule rather than a preference.
- **What this does not fix**: two units can still overlap **each other** in a crowd - that is unit-unit separation (D131's give-way rules),
  a different mechanism, and the screenshot's three villagers were in the yard, not on the wall.
- Suite: **766 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
