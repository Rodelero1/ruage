# No cap, and your own units pass through each other

Two changes in one slice, both the owner's call: *"I think that capping is pointless"* with his rule, and *"1"* when asked how a
crowd should be delivered - bodies overlap while walking, as the original does.

## Performance - measured on the same box, back to back

| | `main` (`fa52c6e`) | this branch |
|---|---|---|
| acceptance hash | `ef7cdba92a83f173` | `8999f1baa7e7b897` |
| outcome | `players=1` | `players=1` |
| entities at 1,800 s | 142 | 139 |
| **repaths** | **435** | **240** (-45%) |
| **made_room** | **151** | **96** (-36%) |
| `World::step` | 7.2 us | **6.7 us** |
| `Snapshot::of` | 98.0 us | 100.7 us |

Both runs end with one player standing, so the counters are comparable. Repaths nearly halve: a man no longer stops or re-plans
because a friend is in his path - he walks on, which is what the original does.

## What changed

**The cap is gone.** `GATHER_RING_SLOTS` and its nine-place table are deleted. The 1.156 ring is sampled every two degrees
(`GATHER_PLACES = 180`) and a place is the nearest sample that is walkable, not on a foundation, and **one body (0.40) clear** of
every other man and every reserved place. `ring_capacity()` derives the count from the circumference - `2 * pi * 1.156 / 0.40 = 18`
- and is used only as a cheap "might there be room" bound; the settle decides sample by sample. Trig is a 46-entry quarter-sine
table, so no float enters a tick.

**Your own units pass through each other on the way.** A unit with **more than one waypoint left** ignores other friendly bodies
and takes the step it planned; the **last step still respects a body**, because the final approach is where order matters. Enemy
bodies are unchanged (`bound_to_reach`), which is the pass D131 was about.

**The settle walks a man exactly onto his place.** `off_place` is any deviation rather than a tolerance: a 0.125-tile tolerance let
two men finish **0.376** apart and a 0.025 one let them finish **0.345**. Exact is what makes the reserved spacing real.

## Stability

**The measurements that shaped it.** With bodies solid, ten men sent at one bush could not be delivered: every settle policy
failed one of two ways - two men **0.144** (then **0.345**) tiles apart, or a man stranded **1.543** (**1.962**) tiles out, never
working. The pass-through rule was then found to break **building**: a builder stacked on an idle villager and its house sat at
**hp 1** after 2,000 ticks. That is why the exemption is the *last step* and not the whole walk - the house now finishes.

**The tests that changed, and why.**

- `gather_ring::the_ring_holds_nine_woodcutters_and_the_tenth_takes_nothing` -> `the_ring_holds_eighteen_woodcutters_at_a_body_each`:
  the nine was the rule being replaced.
- `gather_ring::the_owners_own_arrangement_lands_on_the_same_ring` now measures the **places**, not the closest approach: since
  men may walk through each other in transit, a closest-approach reading counts a crossing, not a crowding (it read **0.357** for
  two men whose places are 0.408 apart).
- `evasion::two_villagers_crossing_never_share_a_tile_and_both_arrive` -> `..._pass_through_and_both_arrive`: the old assertion was
  the old rule.
- `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` and `a_repath_route_is_pulled_like_a_planned_one` now put an
  **enemy** on the line: a friendly body no longer forces a detour, so the behaviour those tests are about needs a hostile body.

**Suite:** `cargo test -p ruage-sim` **118 binaries, `EXIT 0`**; the workspace gate is in the log of the landing run.
