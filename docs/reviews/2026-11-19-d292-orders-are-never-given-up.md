# Review: an order is never given up by the engine (D292, P5)

*The owner's rule: **"no deliberate differences to original game. If there is any, I want that adjusted to meet the original or cut
off"**, and on the jam fork, chosen from the binary: *"no engine help: the order stays live and the man stands; only sliding and the
pause rule act."* This slice removes the last three ways our engine stood a man down on its own.*

## What was cut

- **`beside`** (`world/step.rs`, the `Task::Move` arm): a walk whose goal held a body was called finished once the walker came within
  1.25 tiles of it (D200).
- **`finish_blocked_orders`** (D200): after `STALLED_TICKS` (40) refusals the order was replaced with `Idle`. Its whole function and
  its call are gone.

Both are ours. The binary pauses a body (`getMaxVelFwd` returns exactly `0.0`) and re-paths it; nothing replaces an order because a
body was in the way, which is why a wedged villager is the game's most-reported bug (`docs/gather-seating-evidence.md`).

## One guard, added, and one that was tried and withdrawn

**Added:** a mover re-armed his friend's pause on **every** tick he was refused, because the pause is written *after* the movement pass
and `pause_remaining` is still zero while it runs. Measured before the guard: **1,876 ticks (93.8 s)** of standing against the
original's 0.25-1.50 s draw. The guard checks both (`pause_remaining == 0 && !pauses.contains(&id)`).

**Withdrawn:** a "no two bodies on one point" guard. D289's shared goals let a crowd press to **0 raw** apart, which reads like a
stack; the guard refused it, and `right_of_way::the_higher_id_pauses_...` then **never finished its crossing in 2,499 ticks** - the
pair could not pass because neither could occupy the other's point. Removed, and the fixtures ask for what the binary actually
provides: **convergence**, not separation. That is also what D290 found at source: the original's depenetration is computed and
discarded, so there is no press floor at all.

## Performance

- `acceptance_economy` (both acceptance matches, including the 48,000-tick one): **30.34 s** (33.0 s wall) - the previous commit's
  figure for the same file was 34.25 s, so the cut costs nothing on the work that matters;
- `right_of_way` (4 tests, the pause and the crossing): **1.63 s**; `head_on` (2): **0.97 s**; `retarget`: **0.91 s**;
- the full workspace suite is **893 passed / 14 failed / 21 ignored** against D291's **890 / 17 / 21** - three names moved from red to
  green, none the other way, and no name changed status except by the re-points recorded below.

## Stability

**Three fixtures re-pointed to the press model, each with its measurement in the code:**

- `group_overlap::a_mate_who_is_stopped_is_pressed_into_but_not_walked_past` - the walker comes **4,576 raw** from the stopped man
  inside an **8,000 raw** reach; "walked over" is what the game does, so the claim is "never on the far side".
- `collision::a_crowd_sent_to_one_place_presses_together`, `group_move::a_crowd_ordered_to_one_tile_all_arrive_and_spread`,
  `overlap::eight_villagers_ordered_to_one_tile_press_and_never_coincide` -> `press_and_converge` - the crowd closes inside the
  footprint instead of being kept outside it.
- `no_pushing::a_man_who_can_press_in_keeps_the_node_he_was_given` - with no press floor, "no square for him" no longer moves him,
  and D291 means it never moves him to another kind either.

**One fixture rebuilt rather than relaxed:** `head_on::the_loser_stands_for_the_drawn_pause` could not measure a pause in open ground
(the pair never touched: the re-plan went round) and could not measure it on a shared tile (the winner is idle at his goal, so the
"stand" measured 93.8 s of idling). It is now **two knights in a one-tile corridor** - radius 0.25 each, exactly the corridor's
width, so nothing can slip past - and it measures the drawn stand correctly, with the window starting **after** the contact tick.

**Still red, and not this slice's:** the gather-seat fixtures (`two_woodcutters...`, `three_gatherers...`, `ten_men...`,
`six_settled_men...`, `a_settled_man_is_not_displaced...`, `villagers_never_come_closer...`, `a_man_who_cannot_fit...`), the two
`reseed` ones, `gather::a_depleted_node_sends...`, and four unexplained ones (`both_sides_field_an_army`,
`the_skirmish_only_refuses...`, `a_walled_off_wood...`, `a_villager_drops_only_within_half_a_diameter...`). Fourteen in total, all
listed in `docs/parity-plan.md` with the slice that owns them.
