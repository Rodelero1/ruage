# Review: the crowd goes to the tile it was ordered to, and it jams (D289)

*The owner, asked whether D271's fan-out stays or goes: **"Drop it and let the crowds jam. I want the start as close to the game as
sensible. Upgrades may only come later."** D271 (2026-11-18) had given every follower in a crowd a ring place of its own, so a group
ordered to one spot spread itself round the first man. The original does not: its own players describe men piling onto one tree and
standing pressed (`docs/gather-seating-evidence.md`).*

## What changed

`start_walk` no longer searches for a place per follower: **every man's goal is the tile that was ordered**. `crowd_ring_point` and
the `already_heading` count that fed it are deleted, along with the `wanted` tile it needed. D271 is withdrawn on the moving side;
its gather-side remnants belong to P8.

## Performance

- the cut **removes work**: `already_heading` was a scan of every entity per click and `crowd_ring_point` ran a ring search per
  follower, so a click on a 40-unit selection did that up to 40 times;
- the fixture that measures the change - `crowd_piles_up::a_crowd_ordered_to_one_tile_piles_up`, three men, 400 ticks - runs in
  **0.87 s**;
- the whole suite is compared by test name against `ee51745` so a status change cannot hide in a total.

## Stability

**The witness was written first and watched failing**: three men ordered to one tile stood **0.800 tiles** apart before the cut
(one tile each - D271's ring, measured) and pass after it, pressed inside half a tile. Its claim is the game's own description, not
a number of ours.

**Four fixtures asserted the fan-out and were re-pointed, each with its measurement in the code**:

- `group_move::a_crowd_ordered_to_one_tile_all_arrive_and_spread` - closest pair **5173 raw** inside an **8000 raw** footprint;
  the claim becomes "no deeper than a quarter of a footprint" (a crowd presses, it does not merge).
- `collision::a_crowd_sent_to_one_place_does_not_stack` → renamed **`..._presses_but_does_not_merge`** - **6746 raw** inside 8000.
- `overlap::eight_villagers_ordered_to_one_tile_never_walk_through_each_other` - the crowd settles at **0.172 tiles** against a
  **0.400** standoff; the floor becomes a quarter of the standoff.
- `click_spot::a_crowd_stands_round_the_unit_that_took_the_spot` - which man takes the spot is now a race, not an assignment; the
  claim becomes "somebody takes it and the rest close on it".
- `collision::a_walker_never_ends_a_tick_inside_a_friend` - the deepest point after contact is now **781 raw** deeper against an
  **800 raw** step, so its monotonance claim becomes "at most one step deeper", the whole resolution a tick has.

**What is now possible that was not**: with the fan-out gone, a crowd ordered to one tile *can* jam, which is the precondition for
P5 (`beside` and `finish_blocked_orders` cannot be witnessed while every man has a private tile). P5 is still blocked on its own
finding - the forty-tick counter carries the press-through guard, the head-on pause and the D204 re-target - and that is the next
slice.
