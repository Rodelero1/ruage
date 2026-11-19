# Test branch: "never step deeper" - built for the owner to look at (D283)

*This branch exists so the owner can **run** a change that the suite says is not landable, and judge it himself. It is one
commit on top of D282 (`24715b8`) and **nothing here is proposed for `movement-clean-slate`**.*

## The change

A body that meets another inside its reach takes **one** step into it and no more: a step that would leave it closer to a
body it is already inside is refused. That is the game's behaviour as read - it takes one step through a collider, answers
the collision with a re-plan and a pause of 0.25-1.50 s, and its epilogue writes no position, so the overlap happens but is
never driven deeper.

## Performance

- A head-on pair's deepest approach: **4,478 raw** before, **7,728 raw** with it, against an 8,000 raw reach - i.e. the
  **272 raw** contact step and nothing more.
- `group_overlap`: **4 passed, 0 failed** - `a_mate_who_is_stopped_becomes_solid` is green here because its second
  assertion was restated to what the arithmetic allows ("never deeper than the meeting"), as 8,000 - 272 = 7,728 makes the
  old "nothing inside the reach after the meeting" unsatisfiable.
- A new fixture, `a_meeting_swallows_no_more_than_the_contact_step`, pins the bound.
- The acceptance fixture runs in **26 s** against D282's **39.58 s** - and that is **not** a win: it is the stall described
  under Stability.

## Stability

`ruage-sim --test acceptance_economy` **fails**: *"the crew did not grow between 400 s and 800 s: 7 then 7"*. In a crowd a
body stands inside several clearances at once and every one of them vetoes the step, so a man has no legal move. The
fixture's own 26 s (against 39.58 s) is that stall, not a speed-up.

**What to look at in the window:** villagers in a group that do not move at all, or a group at a tree that never seats a second
man. If it *looks* right - men jostling and stepping out of each other rather than standing - the freeze may be narrower than
the acceptance scene suggests, and the next step is a net-motion rule rather than a per-collider veto.
