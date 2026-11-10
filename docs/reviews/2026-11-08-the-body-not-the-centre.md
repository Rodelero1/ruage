# The body, not the centre (D177)

*2026-11-08. The owner, with a screenshot of a villager on the town centre's light yard, its white ellipse crossing the dark core's edge: **"I see
clear overlap."** He was right, and D176 had only fixed the *drawing*. The collision rule itself was still asking about a point.*

## The defect, reproduced

`crates/sim/tests/body_clearance.rs::a_villager_walking_beside_a_building_keeps_its_body_clear` orders a villager to the tile touching the town
centre's solid core and checks `body_clearance` - the distance from its centre to the nearest blocked tile, minus its 0.2 radius - **on every tick**.
Failing first:

```
the villager's body was 0.18 tiles inside the town centre's blocked tile at some point - the ellipse overlaps the diamond
```

0.18 of a tile, on a body 0.4 wide: nearly its whole half inside the wall, which is exactly the picture.

## The rule

The original's, from the Forgotten Empires pathfinding talk: *"shrink the unit to a point, expand every other obstruction"*. A disc of radius `r`
against a square is the square grown by `r`, so the ground test is a distance, not a tile lookup:

```
step refused when   clearance(landing) < 0   and   (clearance(here) >= 0   or   landing digs deeper)
```

The second clause is what keeps a unit mobile: one that is **already** inside a wall - pushed there by a crowd, or spawned touching one - may always
step *out*, and is never allowed to step *further* in. Without it, a body that arrives overlapping can never move again.

## The exception, and why it is honest

`a_unit_rallied_to_its_own_building_stays_inside` went red: a villager walking into its own town centre was refused by the same rule. A unit on its
way through a door is not a body looking for room, so while it is in `entering` the ground rule is the old one. That is one flag, next to the code
that sets it, rather than a hole in the rule.

## Measured, acceptance

| | before | after |
|---|---|---|
| entities | 153 | **138** |
| repaths | 24 | **529** |
| made-room | 16 | 125 |
| hash | `6d23d672eb18ef40` | **`661e970e58bc0c57`** |

**The repath rise is the cost, and it is stated rather than hidden**: the *planner* still smooths routes round corners without knowing the body's
radius, so a villager walking a diagonal past a house is now refused the corner step and re-plans. Twenty-two times the baseline is a real regression
in re-planning work (though 529 over 36,000 ticks is one every 68 ticks, not thousands). The fix is to teach the smoothing pass the same clearance -
`short.rs`'s line test - so the path never asks for a step the body cannot take. That is the next step, and it is a planner change, not a rule change.

The entity count fell by fifteen and the match now ends with player 2 eliminated, which is the honest consequence of bodies keeping clear of walls in
a base whose paths were laid against them.

## Performance

`clearance_at` scans the nine tiles round the point - nine lookups and a square root per step test, and a step test runs up to three times per body
per tick. **Measured**: gate 425,069 ms against 442,955 ms before (the machine's load varies more than that difference, so this is not evidence either
way); the acceptance run itself is unchanged in wall time. `body_clearance` is not on the tick path.

## Stability

- **The invariant is tested on every tick of a walk**, not at the end, because one frame of overlap is what the player sees.
- **The `entering` exception is narrow and named**; without it, garrisoning is impossible under a disc rule.
- Suite: **782 passed / 0 failed / 20 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
