# Fixtures vs the decomp: are the tests in line with the game? (2026-11-19)

*The owner: **"Can we first focus on check of the unit tests whether they are in line with original game and decomp?"** This is
that check, for the 19 red fixtures and the two that name a mechanism. Four readers worked in parallel on the fixtures and
`docs/movement-in-the-binary.md`; the claims marked *(verified here)* were re-checked by running or reading the code.*

## The verdict table

| # | fixture | what it asserts | verdict | evidence |
|---|---|---|---|---|
| 1 | `collision::a_walker_never_ends_a_tick_inside_a_friend` | one step of overlap at first contact, never deeper after | **IN LINE** (name stale) | the step is never clamped (`0x8a7fe7` discards the retraction; no store to `+0x104`); the "no deeper" clause is the re-plan's outcome |
| 2 | `invariants::villagers_never_come_closer_than_one_diameter` | **no two villagers within the sum of their radii, any tick, 2,000 ticks** | **FIXTURE CONTRADICTS THE GAME** | a *past* collider is dropped (`0x1265`/`0x1270`, then *"Don't know what to do about my collision(s)"* `0x12b2` and the position stands); touching collides on `<=`; the owner's own D241 ruling says the fixture must assert "nobody is shoved off his square" |
| 3 | `invariants::a_villager_drops_only_within_half_a_diameter_of_the_drop_site` | a load leaves within own radius + one step of the drop footprint | **NOT A MOVEMENT RULE** | the movement layer only compares a box gap against the order's `rangeDistance`; the drop order's range is **UNREAD** |
| 4 | `group_overlap::a_mate_who_is_stopped_becomes_solid` | a stopped mate is solid: ≤ one step at contact, never inside the reach after | **IN LINE — engine wrong** | the ignore list is gated on `isMoving` (`+0x2a0`), so a stopped body blocks; the engine's 4,207 raw is the shared-tile planning hole (item 2 below) |
| 5 | `gather_standoff_rule::six_settled_men_hold_their_places` | a man keeps the place he took; no settle churn | **IN LINE** | nothing writes another body's position; all six routes predate the stops, so the past clause does not fire |
| 6 | `gather_standoff_rule::three_gatherers_all_work_the_same_square_on_its_ring` | many men per square at the touching clearance | **IN LINE** (doc cites the withdrawn 1.156 ring) | no capacity or retarget anywhere in the collision block; touching = `func_002690E0` box gap at range 0 (0.70/0.935 in the owner's saves) |
| 7 | `gather_standoff_rule::a_settled_man_is_not_displaced_by_a_later_arrival` | the seat belongs to the order, not the nearest claimant | **IN LINE** (its `between >= 0.39` half is stricter than the game here) | the incumbent cannot be displaced; but the newcomer's route postdates their stop, so the game lets *him* walk through |
| 8 | `gather_continuous::ten_men_work_one_bush_a_body_apart` | **`working.len() == touching_place_count(...)` — a hard capacity of seven** *(verified here)* | **FIXTURE CONTRADICTS THE GAME** | no occupancy count or limit in `FUN_008a4360`; and the assertion compares the engine to **the helper the engine itself calls**, so it cannot detect the cap being wrong. Its own name and doc say *ten* and *no cap* |
| 9 | `gather_continuous::a_man_who_cannot_fit_takes_the_next_source` | `on_bush <= places` then retarget to another source *(verified here)* | **FIXTURE CONTRADICTS THE GAME** (the retarget half is UNREAD) | the cap is contradicted as #8; the gather task layer is absent from both sources, so "the game retargets" rests on the owner's ruling (D204/D241), not a read |
| 10 | `overlap::a_newcomer_does_not_move_the_working_gatherers` | the incumbent keeps his **seat** when a newcomer arrives | **IN LINE** (asserted on the wrong object) | the behaviour is the game's; the fixture asserts our `gather_places()` book, which a faithful engine does not have — assert positions too |
| 11 | `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` | a walker never comes within a full diameter of a body **that was standing when the route was planned**, and arrives | **IN LINE — engine wrong** | the search puts a stopped body in as a **rect grown by the mover's radius** (`FUN_007f9d80`) and the query box is grown by `+0x268` (`FUN_00840830`); `0x8a7fe7` removes only the *step* clamp, so this is a plan-level rule. Our plan carries **tiles**, not radii — measured **4,619 raw against 8,000** *(verified here)* |
| 12 | `head_on::the_loser_stands_for_the_drawn_pause` | the higher id stands once for a draw of 0.25-1.50 s | **IN LINE** — engine off-by-one found | `setPause(rand*1.25/32767+0.25)` on the higher id, which keeps its path. Our write is `ticks.saturating_sub(1)` *(verified here)*, so a 250 ms draw = 5 ticks → 4 written = **0.20 s**, under the game's floor |
| 13 | `no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` | a man refused a work square retargets to another node | **NOT A MOVEMENT RULE** (game side UNREAD) | the "square" is our `touching_places` ring; the rule is the owner's (D204/D241); the binary has no read of a gather retarget |
| 14 | `building_passage::a_walk_past_a_house_keeps_its_bodys_clearance` | a walk past a house keeps the body's radius clear of every footprint tile | **IN LINE — engine wrong** | `FUN_00840830` AABB against a query box grown by the mover's radius, `<=` on touch; a footprint is static, so D281 does not touch it. Same engine hole as #11 |
| 15 | `gather_pick::a_walled_off_wood_is_tried_once_and_then_left_alone` | the picker takes the nearest node, remembers the refusal, refusals stay bounded | **NOT A MOVEMENT RULE** | `Runner::rule_refusals` is our harness's book; the villager never crosses the wall; an unreachable goal's answer is unread in the binary |
| 16 | `animals::a_live_animal_is_a_gather_source` | a live herdable is a gather source and working it yields meat | **IN LINE — engine wrong** | arrival is the **box gap** against the order's range (`func_002690E0`); ours is a centre distance equal to the reach, so the gather band is one fixed-point value wide (D262 measured "gets there and does not work") |
| 17 | `animals::the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` | the knife kills, 594→595 in place with its meat, gather resumes | **IN LINE — engine wrong** | same arrival measure on the carcass, and unlike the strike (`reach + step`) it has no tolerance |
| 18 | `reseed::a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot` | one entry pays one farm's price at queue time, one replacement, once | **IN LINE** (owner + D226, green with a mutation proof) | FARM `costs = [[1, 60]]`; its red is **unattributed** |
| 19 | `reseed::auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out` | the toggle is per-player, free to set, pays when the plot empties | **IN LINE** — but **DE-by-instruction**, not HD | D226 and the fixture header say so; not evidence about `AoK HD.exe` |
| 20 | `scenario::the_ten_minute_scenario_builds_trains_and_fights` | a barracks ordered at 320 s with one builder (70 s window for 50 s of work) is finished | **ENGINE WRONG — a stall; fixture fair** | **0 of 1,000** progress in 1,400 ticks = zero builder-ticks: never in the adjacency band (`step.rs:1610`) or `site_is_clear` refused every stroke (`build.rs:188`) |

## What this settles

**Three fixtures contradict the game, three are not movement rules at all, and fourteen are in line** (two of those with a
caveat). The three that contradict it: `villagers_never_come_closer_than_one_diameter` (a global no-overlap invariant the
game does not have), `ten_men_work_one_bush_a_body_apart` (asserts a capacity of seven under a name that says ten and no
cap) and `a_man_who_cannot_fit_takes_the_next_source` (the same cap, then a retarget whose game side is unread). The three
that are not movement rules: the drop-site half-diameter, the ring refusal book and the picker's refusal book. The two
caveats among the in-line ones: `a_newcomer_does_not_move_the_working_gatherers` asserts our `gather_places()` **book**
rather than the game's geometry, and `a_settled_man_is_not_displaced_by_a_later_arrival`'s `between >= 0.39` half is
stricter than the game in that scene. Rewriting the three contradicting fixtures is fixture work, not engine work, and it is
the only way that part of the red set falls.

**Two engine defects the audit named that the red list had mislabelled:**

1. **The plan carries tiles, the game carries radii.** `FUN_007f9d80` puts a stopped body into the search as a rect grown by
   the mover's radius, so "keep a diameter from a body standing when you planned" **is** the game's rule, at plan level —
   `0x8a7fe7` only removes the *step* clamp. Our `replan_around_bodies` avoids the collider's **tile**, which is why #11
   (4,619 against 8,000), #14 and #4 stay red. **This corrects `docs/next-step.md`'s earlier row**, which filed #11 with the
   crowd rewrites.
2. **The pause is written one tick short.** `setPause` is given 0.25-1.50 s; we write `ticks - 1`, so at 20 Hz the floor draw
   becomes **0.20 s** — outside the game's own band. #12's red is that off-by-one, not a rule.

**And the two animals reds are the arrival measure**, as recorded: the game's answer is a **box gap** against the order's
`rangeDistance` (`func_002690E0`, and its own log *"We're not in range (%.10f) of our target"*), ours is a centre distance
equal to the reach, so the gather band is one fixed-point value wide.

## What is still unread

- **The collision block's avoid array** `DAT_00de1dc8` (cap `0x100`, written at move_obj.cpp:2074) has **no reader in the
  corpus**, and the re-path arm installs the pre-existing *parent* path rather than a freshly stamped one. That decides
  whether the game *guarantees* the walker comes out round a collider (#1 and #4's second clause), as opposed to usually.
- **The gather task/action layer**: absent from both the decomp sources and the corpus (gathering appears only in AI
  strings), so a gather order's `rangeDistance` and any "cannot work this node" decision rest on the owner's saves (0.70 /
  0.935 = range 0) and on his rulings, not on a read.
- **The reseed reds' cause**, unattributed rather than guessed.
