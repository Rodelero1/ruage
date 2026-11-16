# Parked: the closest-point search and the game's cost model (both implemented, one test refuses)

**What was ruled.** The owner, 2026-11-16: *"1 follow game, 2 follow the game, 3 can you follow the game?"* - adopt the
closest-point answer (the search, not a helper, returns the nearest reachable tile) and the game's cost model (16 a step,
23 a diagonal, Euclidean estimate x25).

**What is built, on `wip/follow-the-game` (unpushed).** `find_path` answers the closest point; `find_path_strict` /
`find_path_avoiding_strict` keep the refusal for thing orders; `nearest_reachable` is deleted (the defect it carried is gone
with it); `plan_path_mode` plans once and takes the order's place from the route's end rather than planning twice. Conformance
set: **32 green, 1 red** - only the hierarchical long route (U25 c) is still open; the cost model and the closest point both
pass.

**Why it is parked, measured.** The full `ruage-sim` suite has **one** failure:
`army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground` - twelve soldiers spawned two deep submit twelve
attack-moves, and one (`(6,20) -> (19,19)`) is boxed in by **18 stander tiles** of its own mates: `short_range_clear`
declines and the tile search cannot take a first step, so the order is refused at submit. The baseline passes, and the
instrumented comparison is the finding: **in the baseline that order never reaches the planner at all**, so the difference is
not in the search but in *which* orders get planned - not yet established which check skips it there.

**The prerequisite it names.** A crowd that boxes its own member is exactly the game's two mechanisms we have not built:
the search's obstacle set excludes **moving** mates (`+0x130` velocity, row 30), and a start inside an obstruction is
**re-seeded** by `2 x radius` along a fixed ring (`FUN_007fb220`, row 33). A start re-seed was tried inside the tile search
and reddened `scenario::a_world_whose_town_center_role_is_renamed_still_finds_a_build_site` (a site that later reads as
walled off), which is why it is not in the parked diff.

**What lands instead:** nothing in `crates/`. The ruling stands, the work is kept, and the next slice starts from the two
mechanisms above - they are already rows 30 and 33 of `docs/behaviour-diff.md`, so the queue and this note agree.

## Performance

No engine change landed, so no tick figure is claimed. The measurements taken on the parked diff: acceptance (seed 7,
48,000 ticks, release) **125 entities, players=2, hash `0f0a37dc86bf1002`, 451 repaths** against the baseline's
**155 / players=1 / `8cf433ad4e0205f4` / 497**; the conformance suite runs 33 tests in 0.19 s.

## Stability

Attacked: the double plan (a click accepted by the first search then refused by the second - the crowd case above), the
boxed-in start (re-seed tried, reverted: it moved a body's route origin up to three tiles), and the strict/closest split
(thing orders still refuse, pinned by `the_strict_search_refuses_a_walled_in_goal`). Every one of those is a finding, not a
failure of the rule - the rule is right and the crowd mechanism is missing.
