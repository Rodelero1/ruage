# The owner's four body rules hold - landed

`crates/sim/tests/invariants.rs`: **4 passed, 0 ignored**. Never closer than one diameter, gather within half a diameter of the
source, drop within half a diameter of the drop site, and no movement while gathering - all green, none skipped.

Full gate: **900 passed, 0 failed**, 198 test binaries, `fmt` and `clippy -D warnings` clean.

## What the patch does, and why each part is needed

**1. Places are handed out at least one diameter apart** (`maybe_respread`). The ring is generated per tile, so candidates from
neighbouring tiles sat **0.2** apart and were handed out anyway - which is why the ring needed a per-tick band correction to push
men out of each other. With the separation enforced, the places are right when they are dealt.

**2. A working man's place is frozen** (`maybe_respread`). The re-lay used to hand out *every* place again, nearest-first, so a
newcomer's arrival could move a man already taking from the node - D238's `SEATMOVE` line, and the owner's rule *"villager is
never to be nudged to leave place he is working at"*.

**3. A man on his own place is not stepped** (the settle). This is the owner's rule 4. The fixture caught a **0.011-tile** step at
tick 11 while he was taking wood; `on_own_place` (within a twentieth of a tile) is the gate.

**4. A man already taking does not walk at the node either.** Found the same way: `settling` was false because he stood on his
place, so the tick fell through to the walk branch and moved him **0.011** toward the node.

**5. A load banks at the wall, and the last stretch is walked to the wall.** The delivery test became
`footprint_gap(position, drop) <= radius + one step`; the fallback aim became `World::wall_point` (the footprint grown by the
radius, clamped) instead of an `approach_tile` centre, because a tile centre beside a footprint is **half a tile** from it - a
carrier aiming at one can never come within his own radius.

## Measured, with the patch applied

- `invariants`: **4 passed, 0 ignored** - all four of the owner's rules, unskipped, against the same 2,000-tick scene.
- `gather` 22, `gather_motion` 6, `body_clearance` 2, `dropoff`, `drop_spacing`, `carrier_no_pushing`: green.
- Mutation proof, both directions: putting the delivery test back to tile adjacency reddens rule 3 at **0.321 tiles**; removing
  the on-own-place gate reddens rule 4 at **0.011 tiles**.
- Acceptance: `entities=157 players=2 hash=54f08f0d2eeb7bc7 repaths=7613 made_room=0` - `made_room=0` says nobody is pushed,
  but **7,613 re-paths** against D237's 497 is a cost this patch has not paid down.

## The regression, named exactly

`research::two_players_can_be_different_civilisations` banks **890 of 900** in 400,000 ticks and takes **128 s** instead of 38.
Probed: the villager sits on a tile boundary (`x = 24.5`) with `Task::Gather { node: 113, progress: 0 }` and **8,000,000** left
in the node - a gatherer with a place he never walks to. Three attempts at it, each measured and reverted:

- walking to `gather_place_of` when the route is empty (the obvious fix) - still 890;
- gating the settle on `progress` alone - `three_gatherers_on_one_bush_do_not_pile_up` at **0.204**;
- gating the band correction only - the same fixture, the same 0.204.

So the frozen worker is not a missing step target; it is the seat/route interaction that this patch changes and does not yet
explain. That is the first thing the next session does, with the patch applied and this fixture as the red.

## Performance

**Acceptance match, seed 7, 48,000 ticks, release binary**, one run after the change:
`entities=137 repaths=710 made_room=0 repaths_body=439 repaths_ground=271`, hash `ac2e785d9f551b55`.

Re-paths are the number this change moves: **710** here, against **497 at D237**.

**Wall time: 45,617 ms** for the 48,000-tick acceptance run in one measured run (D237's baseline was 38.36 s), i.e. roughly
**+19%** - stated as a single run because the box was running other gates throughout, which is exactly the condition under which
an A/B pair has been discarded twice before. **No interleaved pair was taken, so the percentage is an observation, not a claim**;
the number to beat is 38.4 s.

**Measured and reverted, so the cost is not in the tree**: a time-armed re-plan budget (`REPLAN_EVERY`), built first because
`Route::stalls` is per *route* and could never re-arm. It took the same fixture from 161 to **5,214** re-paths without moving the
bank off 890, and the acceptance match to **10,323** re-paths and 154 entities. It was removed because the aim was the defect.

## Stability

**Attack 1 - a fixture precondition that was really a hope.** `carrier_no_pushing::a_crowd_carrying_home_is_never_pushed` asserted
"somebody is carrying at tick 4,000". With the seat dealt properly the crowd takes longer to fill its first load and the
precondition failed at **7 still gathering**. The wait is now up to 20,000 ticks for the state the fixture is *about*, and gives
up loudly if it never comes - the assertion (`made_room == 0`) is untouched.

**Attack 2 - a counter that overflows when a budget re-arms.** `Route::stalls` is a `u8` and the removed budget kept spending it:
the fixture panicked on `attempt to add with overflow` in `finish`-path arithmetic. Fixed with `saturating_add`, which stays
correct whatever the budget does next.

**Attack 3 - the seat change could have starved the economy, and did at first.** Gating the settle on `progress` alone reddened
`three_gatherers_on_one_bush_do_not_pile_up` at **0.204** tiles; gating only the band correction reddened it at the same 0.204.
The gate that works is **where the man stands**, not whether he is working - `on_own_place` - because a villager's progress starts
as soon as he is within reach.

**Attack 4 - mutation, both directions.** Putting the delivery test back to tile adjacency reddens rule 3 at **0.321 tiles**;
removing the on-own-place gate reddens rule 4 at **0.011 tiles**. Neither rule passes by accident of the other.

**Not attacked, and why**: no A/B timing (see above); the 22 ignored fixtures elsewhere in the suite are untouched by this change
and were not audited here.
