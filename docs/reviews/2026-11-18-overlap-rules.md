# D267/D268 - the overlap rules from the original: review

*Two changes: **adopted** the placement separation (`func_002699A0`), **tried and refuted** the removal of the "already inside" movement
clause. Both measured; the refuted one is not in the tree.*

## What landed

1. `World::place_apart(at, radius, salt)` in `movement.rs`, called from `Command::Spawn` for every unit. The decomp's rule, one for
   one: step out by the **pair's radii plus `scale/100`** (0.01 tiles - the pair touches, does not overlap), on **one of four
   sides**, with a random offset across that side, **fifty** tries, refusing a point off the map, on ground the body does not fit,
   or within another body's footprint. Nothing moves a body already standing.
2. Two **unignored** fixtures written first and watched fail: `spawn_apart::a_body_put_on_top_of_a_friend_is_set_beside_him`
   (red at *"stand **0** raw units apart, inside the **8000**"*) and `spawn_apart::a_group_put_on_one_point_all_stand_clear`
   (five bodies, every pair).
3. `collision::units_that_already_share_a_tile_are_not_pushed_apart` (D52) **amended**, with the owner's explicit word, to
   `collision::a_newcomer_steps_aside_and_the_body_already_there_does_not_move`: the stander's position is byte-identical after 20
   ticks, and the newcomer is no closer than the pair's footprint. D52's rule (*nothing pushes a body*) is unchanged; the assertion
   that could not tell the two rules apart was a **tile**, which the fixture's own comment had already called a proxy.

## Performance

**The acceptance match, 48,000 ticks, seed 7**: `entities=133 players=2 hash=336c60c17233edf3 repaths=431 (309 body, 122 ground)`,
**identical to before the change**, twice, so the placement rule is **provably inert in that scene** (no spawn there lands on an
occupied spot) and nothing about the match's trajectory moved. Wall **11,544 ms / 11,849 ms** - the same machine that read **17.0-18.4 s**
for the same binary an hour earlier, which is the 24% box noise already recorded in `2026-11-17-legal-standoff.md`; the wall clock is
not a number to quote from one run.

**Cost**: `place_apart` runs **once per spawn**, and only when the spot is occupied; the worst case is 50 candidates, each one
`tile_walkable` + `clearance_at` + a scan of the world's bodies. At 133 entities and a handful of spawns a second that is invisible,
and the acceptance wall agrees.

**The measurement that matters** (`collision_probe`, 36,000 ticks, 15,245,283 pair-ticks): **104,386 overlapping pair-ticks
(0.68%)**, deepest **0.400 tiles**, **51,676** deeper than half a footprint - **unchanged by D267**, which is the honest result: the
placement rule fixes a class of overlap that this scene does not contain. Classified by cause for the first time: **mates 0, past
43,315, neither 61,071**. The first offenders printed are two villagers walking to the same node, **7,116 raw** apart where their
footprints need **8,000**.

## Stability

- **Full suite, `--no-fail-fast`**: **200 binaries, 199 ok, 1 failed** - the pre-existing
  `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`. No new red.
- **The refuted half, and it is the useful part**: deleting rule 3 (D52) - the clause the original does not have - took the overlaps
  **up**, not down:

  | | rule 3 in place | rule 3 deleted |
  |---|---|---|
  | overlapping pair-ticks | 104,386 (0.68%) | **167,025** (0.88%) |
  | deeper than half a footprint | 51,676 | **75,380** |
  | pair-ticks inspected | 15,245,283 | 18,922,347 |

  Three fixtures also went red on it (`overlap::a_newcomer_does_not_move_the_working_gatherers`,
  `head_on::the_loser_stands_for_the_drawn_pause`, `lib::tests::a_spawn_in_the_same_tick_does_not_move_the_wrong_entities`), all of
  them for the same reason: a refused step is not a separated pair. Reverted, and the reason is written at the clause.
- **Determinism**: acceptance hash unchanged and identical across two runs; `place_apart`'s side and offset come from a xorshift
  seeded from the entity id, not from the clock, so two runs agree by construction rather than by luck.
- **An error of mine, kept**: my first statement of this work claimed the 103,408 non-new overlaps were "stacks the map seeded". The
  probe refutes it - the map's own seeded bodies start **0 pairs inside** (`how_close_do_the_seeded_bodies_start`), and adding
  placement separation changed the acceptance hash **not at all**. The overlaps are created during play; the classification is what
  says by whom.
