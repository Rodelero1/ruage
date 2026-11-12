# A place on the ring belongs to the man — work in progress, not landed

**Status: parked on branch `d186-reserved-places`, not merged.** It fixes the gathering defect it was written for and breaks
building in the ten-minute scenario. Main is green without it.

## What it does

`GATHER_RING_SLOTS` places on the ring used to be re-chosen **every tick** by `standoff_point`: the place belonged to whoever
was nearest it at that instant. Three separate failures came from that one design — **0.001**, **0.20** and **0.243** tiles
between men who were all supposed to be working — and the owner's description of the result was *"it feels rather random"*.

The change gives a man a **place**, not a claim: `gather_place: BTreeMap<worker, (node, slot)>`, chosen when the order lands
(`start_gather_with`), read by the settle instead of re-deciding, and released by `place_is_live` when his task stops naming that
node. A reserved slot is treated exactly as a body is: `standoff_point` will not choose it for anyone else.

## Performance

Measured on `main` at `9b30e9c` against this branch, 36,000 ticks, seed 7, release build:

| | before | after |
|---|---|---|
| acceptance hash | `08a111f13b7c43ec` | `16fe573fcd55440f` |
| entities at 1,800 s | 150 | 146 |
| **repaths** | **352** | **62** (−82%) |
| **made_room** | **226** | **7** (−97%) |
| `World::step` | 6.9 µs | 7.7 µs (first cut) |
| `Snapshot::of` | 101.6 µs | 103.6 µs |

The two big falls are the point: a man who keeps his place does not re-plan a route to a place that moved, and does not shuffle
bodies out of his way. The step cost rose 12% because `gather_place_of` was doing a linear `find` per gatherer per tick to prove
the place was live; that was removed from the hot path (the caller already knows the task names the node) before the parked state,
which is the number above.

## Stability

**The regression.** `the_ten_minute_scenario_builds_trains_and_fights` fails on this branch and passes on `main` at `9b30e9c`:

    the step due at 500 s did not run: Refused("player 1 has no finished barracks")
    barracks: construction = Some(Construction { progress_ticks: 0, total_ticks: 1000, .. })
    units: 2 gather (node 105), 3 gather (node 105), 4 gather (node 17), 5 Idle

The barracks is **placed and never struck once**: `progress_ticks: 0`. The builder is not building — he is back at a gather
task with a nine-point route.

**The likely cause, not yet proved.** A place chosen *before* a foundation was placed is never re-validated. `standoff_point`
refuses slots that land on a site (`on_site`), which is why newly chosen places are safe — but a place already owned stays owned,
so the man keeps settling **onto the footprint** that is now a building site. `site_is_clear` cannot answer while he stands
there, so the site stalls at zero, and the builder — pushed aside by the site's own clear pass — is restored to his old gather
task by `resume_idle`.

**What it would take.** Re-validate an owned place against the same three tests a new one passes (walkable, free, not on a site)
before walking to it, with a reserved slot dropped and re-chosen once when it fails. That is one tile lookup per settled
gatherer per tick in the worst case, which the step budget (7.7 µs, ~1,300 entities) can afford, but it has to be measured
rather than assumed.

## Tests written for it

- `a_settled_man_is_not_displaced_by_a_later_arrival` — a second man placed beside a settled man's place does not take it.
- `six_settled_men_hold_their_places` — six men round one bush drift less than 0.10 tiles over ten seconds and no pair is
  closer than 0.39. **Mutation-checked**: removing the reservation (making `gather_place_of` return `None`) makes two of them
  **0.000** apart, so the test bites.
- `gather_room.rs::a_woodcutter_whose_tree_runs_out_joins_a_patch_that_already_has_men_on_it` was made timing-independent: it
  now waits for the join rather than for tick 2,400, because a tree running dry mid-load sends the man home first and the old
  fixed tick made the test a measurement of arrival time.

Both new tests pass on the branch. The suite is otherwise green (**171 of 172 binaries**; the one red is the scenario above).
