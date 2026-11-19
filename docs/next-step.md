# OPEN - standers in the search is right and 5.1x too slow; the crowd is still half built (D282, NOT LANDED)

**Where this branch stands (2026-11-19, measured).** `d281-body-in-the-search`, off `f055be6` (D279's port + D280's
picker). D281 read the original's collision block, implemented it, went three-green/three-red and was reverted. **D282 is
the re-attempt, and it carries the piece the first one missed**: the **standers are obstacles to the search**
(`FUN_007f9d80`; the register's *"search obstacles: standers only"*), which is what makes the *past* test coherent - a
route planned while a man stood goes round him, so a collider that has not moved since `path+0x10` can be ignored. On top
of that: a non-past collision **re-plans round the collider's tiles** (unthrottled), a re-plan **keeps the order's clock**,
and a walk's **start tile is never an obstacle**.

**Suite state on this branch, by binary** (debug):

- green: `lib` **11/11**, `movement` **4/4**, `replan` **4/4**, `right_of_way` **4/4** (two fixtures restated to the game's
  model), `straight_walk` 4/4, `walkable` 3/3, `passage` 1/1.
- **newly green by this slice**: `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` (was **4,619 raw**
  inside an 8,000 raw reach; now it goes round and arrives).
- red, **same class as before**: `evasion::a_repath_route_is_pulled_like_a_planned_one` (asserts D211's disc-shoulder band,
  which D279 withdrew, *and* makes the walker the lower id - the winner - so nothing forces the repath it measures);
  `group_overlap::a_mate_who_is_stopped_becomes_solid` (**5,461 raw** inside 8,000: two bodies on one tile cannot be
  planned round); `no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` (the gather-refusal book).
- red, **precondition moved**: `collision::a_walker_never_ends_a_tick_inside_a_friend` - "the fixture did not meet": with
  standers in the search the two men never come within a reach, so the scene no longer contains an overlap to measure.
- **removed**: `tests::a_spawn_in_the_same_tick_does_not_move_the_wrong_entities`, with its reason in place of it (its two
  movers were both at the origin - `spawn_now` places every body there - and two bodies inside one reach are a standoff
  under this rule; what it guarded is no longer reachable because positions are written by id).

**Why it is parked, in one table** (debug, 48,000 ticks, `the_acceptance_match_trains_more_villagers`): `f055be6`
**42.62 s**, D281's first attempt **38.96 s**, this branch **217.54 s** - **5.1x**. The cause is structural, not a stray
bug: with standers in the avoid list the straight-line shortcut becomes the exception, and the body re-plan is
unthrottled - `make headless` shows **28,545** re-paths against `f055be6`'s **5** (entities 120 -> 136). **The gate is
`players=2` on both**, so the unresolved match is pre-existing and not this row's; the suite gains and the cost are the
whole verdict.

**The three candidates for the next attempt, in the order they should be measured** (review note, `## Performance`):
**(1) throttle the body re-plan** like the ground one (`REPLAN_TICKS = 20`) - a divergence, and the cheapest test of the
hypothesis that the traffic is the cost; **(2) standers only within a window of the route** - the original inserts per
search, and over a 480 x 480 map it must have locality, unread; **(3) D281's own shape** - keep the shortcut, pay one
refusal per body, re-plan then - which measured **38.96 s** and fixed three fixtures.

**Next, in order:**

1. **The two fixtures whose scenario the rule invalidated** - `collision::a_walker_never_ends_a_tick_inside_a_friend` and
   `evasion::a_repath_route_is_pulled_like_a_planned_one`. Both need a scene where the meeting happens *and* the walker is
   the higher id (the loser), which is what the game's tie-break keys on; the second also needs its D211 band replaced
   with the port's own claim (tile-centre corners, never through the body).
2. **Two bodies on one tile** (`a_mate_who_is_stopped_becomes_solid`, 5,461 raw inside 8,000): the search starts on the
   tile they share, so there is no route round. The original's answer is the **placement** phase (`func_002699A0`: step
   out by the pair's radii plus a hundredth of a tile, four sides, fifty tries) - unbuilt here, and the honest fix for a
   pair that starts stacked.
3. **The arrival measure** for animals: "at the node" becomes the game's **box gap** within the job's range
   (`func_002690E0`) instead of the ring distance. Unchanged by this slice; still the last two animals reds.
4. **The eight crowd fixtures** (the settle ring, the seat, `six_settled_men_hold_their_places`, the bush and square
   ones): each asserts machinery D279 deleted. They are the fixture half of the crowd work and come after 1 and 2.
5. **The probe on `move_bodies`** - still the one phase nobody has looked inside since the port.
6. **The 1.6x that has no single owner**: `decide` 12.8 s, `move_bodies` 6.8 s, `turns` 4.9 s of 26.0 s at tick 14,400 of
   48,000 (phase probe, D280). The next slice there is a profile of `decide`, not a guess.

**The three divergences the conformance set names and `#[ignore]`s** are unchanged: the hierarchy, the radius in the
planner, and the expansion budget.

**Known reds that were already red on `main`** (`e63ed0d`): `animals::a_live_animal_is_a_gather_source` and
`animals::the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` - the arrival measure, item 3 above.

**A collision to hold in mind:** another session was writing the shared tree throughout this slice (`git reset` moved
`movement-clean-slate` from `829ba40` back to `f055be6`, and its edits replaced this session's in `ground.rs` / `lib.rs`).
This slice lives on its own branch from a linked worktree. **Check `git log --oneline -1` before assuming `main` or
`movement-clean-slate` is what you last measured.**
