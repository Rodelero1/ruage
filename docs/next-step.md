# OPEN - the gather seat book is the next slice (D292 landed, 2026-11-19)

**Where this branch stands, measured (`blunt-port`, `4072892`).** Five rulings landed today and each is in `docs/decisions.md` with a
review under `docs/reviews/`: D286 (one box ruler, a click names its object), D287 (the node re-lay deleted), D289 (a crowd goes to the
tile it was ordered to; D271 withdrawn), D290 (**no press floor** anywhere - the binary computes the separation and discards it), D291
(the engine never crosses resource kinds), D292 (`beside` and the forty-tick finish gone: no rule of ours stands a man down).
**Suite: 893 passed / 14 failed / 21 ignored** (`/opt/data/cache/scratch/after-p5c.txt`), `acceptance_economy` **30.34 s**.

**The next slice: the gather seat book** (step 2 of `docs/parity-plan.md`). A gatherer must walk at the node he was given and press
in - no `standoff_point` taken list, no per-tile seat search, no `settle_workers` placement - with only D204's **same-kind** re-target
(D291) able to move him. It owns **eight** of the fourteen reds: `two_woodcutters_on_one_tree_both_touch_it_and_stand_a_body_apart`,
`three_gatherers_all_work_the_same_square_on_its_ring`, `ten_men_work_one_bush_a_body_apart`, `six_settled_men_hold_their_places`,
`a_settled_man_is_not_displaced_by_a_later_arrival`, `villagers_never_come_closer_than_one_diameter`,
`a_man_who_cannot_fit_takes_the_next_source`, and the gather half of `no_pushing`.

**Do it in one slice, not piecemeal.** The registry surface is 24 references and the naive removal re-creates the bug it existed for -
men swapping seats every tick (measured historically at 0.001 / 0.20 / 0.243 tiles apart). The witness to write first: *a gatherer
whose node is free takes from it while standing pressed against a neighbour, and no man is ever re-seated by the engine* - which is the
gather-side twin of `working_men_never_move::a_working_man_is_never_moved`.

**Then, in order:** the same-kind re-target and the reseed payment (three reds), P4 (throttle, then the goal as a tile), P3 (touching
collides, once the plan and the step agree), P11 then P1 (the levelled search, and the budget with it), and the AI layer that D291 made
a requirement.

**Earlier slices' notes** (D203, D279, D281, D282, D285, D286 and the parked skirmish pair) are in
`docs/history/2026-11-19-next-step-before-d292.md`.
