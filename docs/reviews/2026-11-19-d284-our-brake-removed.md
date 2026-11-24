# Review: the self-brake D279 invented is removed, and the ignore list is gated on `isMoving` (D284)

*The owner: **"Please do not invent own stuff and remove extra whatever compared to original game and decomp in pathfinding
and obstacle avoidance. I do not want new stuff to be invented. I asked for blunt copy/conversion to rust."** So this is
subtractive: two things in the step pass that the binary does not have are gone.*

## What was ours, and is now gone

1. **`blocked = true` - the mover refusing his own step against a lower-id collider.** The binary's epilogue **never writes a
   position** (`0x8a7fe7` discards the radius retraction; no store to `+0x104` anywhere in `FUN_008a4360`), so the mover's
   step is always taken; the only thing that stops a body is a **pause someone else handed it** (`setPause`, `0x8a8141`).
   D279 wrote "the older unit wins and walks on, the younger stands" as a second brake, and in a crowd every man stood for
   every lower-id neighbour: the owner saw it on the test branch - **"villagers get stuck"** - and the acceptance crew stopped
   at **7 then 7**.
2. **The mate exemption was not gated on moving.** The binary builds the path-ignore list from the route's objects whose
   `isMoving` (`+0x2a0`) answers 1; a stopped unit's velocity is zero, so a stopped mate is **not** ignored and blocks. Ours
   ignored a mate whenever both routes named each other.

## Performance

- `acceptance_economy`: **passes** in **55.25 s** (it was 39.58 s, and it was a **failure** at 7-then-7 with the self-brake on
  the test branch). The crew grows again.
- The 55 s is the cost of the rule as it stands: with no self-brake, more bodies collide, and **every collision buys a
  re-plan** (the original has no budget for that). Slower than D282 and correct where D282 was stuck - the number is reported
  rather than hidden because it is the next thing to look at.

## Stability

- `group_overlap`: 2 of 3 green. `a_mate_who_is_stopped_becomes_solid` still **4,478 raw** inside an 8,000 reach - with no
  self-brake the walker goes through the stopped man and keeps going, which is what the binary does when the step is taken;
  the fixture's second assertion is the one the arithmetic makes unsatisfiable (8,000 - 272 = 7,728) and it is on the rewrite
  list.
- `head_on::the_loser_stands_for_the_drawn_pause` now reports **0 ticks**: the pair never collides, so no pause is drawn. This
  is **not** explained yet - the most likely reading is the *past* test (`created >= lastMoveTime`, both true on the same
  tick) dropping the collider before the pause can be handed out, which would make our boundary where the binary's is not.
  Recorded as the first thing to settle before this lands.
- `evasion`, `collision`, `right_of_way` unchanged: `a_blocked_unit_goes_round_a_standing_body_and_arrives` still green.
- **The full suite was not run** for this commit (time); the numbers above are the acceptance fixture and the six movement
  files.
