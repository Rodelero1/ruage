# The suite that was green because it did not run

**Step:** A5 and A6 of the owner's queue (2026-11-03). Make the test gate honest, then fix the tests that were red when it
was told the truth.

**What last session claimed, and what was true.** The report said "8 red tests left: `research.rs` (7) and `walls.rs` (1)".
The honest number was **14** - `tech_tree.rs` had six more, and they were red *because of my own repair*: I changed every
fixture to ask `site_fits`, but `tech_tree.rs`'s `a_free_tile` still asked `is_free_ground`, which answers for the anchor
tile only. The town centre's walkable yard passed that test and the build order then refused with `SiteOccupied`. **I never
ran the whole suite before that commit.** The count in the report came from an earlier HEAD comparison, not from the tree
I shipped.

---

## 1. The gate that lied (A5)

Every data-driven suite returns early when `config/gamedata/` is absent. That is D8's decision and it is right: a
stranger's clone must build and run without the game's data. The price was never priced: **`cargo test` on a worktree
without the generated files is green while running almost nothing.** Forty real failures hid behind it for a day.

`tools/require_data.py` (seven tests in `tools/test_require_data.py`) refuses to call a run meaningful:

- a missing **or zero-byte** file fails the gate (a failed import leaves the file behind, empty, and treating that as
  present is how a hole hides);
- the message names the files, says the suite would skip everything, and gives both ways out - import the data, or say
  `RUAGE_ALLOW_NO_DATA=1` and be told the result proves nothing;
- `make test` now depends on `require-data`, and `make check` runs the tools' own suites too (96 tests).

Measured: `python3 tools/require_data.py` -> `require-data: 4 generated files present`, exit 0; the same script pointed at
`/tmp` -> exit 1 naming all four files.

## 2. A building that appears did not evict the units on its ground (A6, first cause)

`Command::Spawn` inserted a building over whatever stood there. A villager left inside a blocked footprint can never plan a
route again - A* refuses to start on a blocked tile - so it stands there for the rest of the match. Same family as the farm
husk: *a command-made entity was not quite a map-made one.*

And the eviction that did exist matched **exact positions**, while a unit stopped by collision stands *inside* a tile and
rarely on its origin. So it almost never fired for the units that needed it.

Two changes:

1. `push_units_off` matches **by tile**.
2. A building spawn evicts units inside the new footprint, and decides *where* they go **before** the tiles turn solid
   (`escape_from`, a ring search on the pre-insert grid). A unit in the middle of a 3x3 has eight blocked neighbours;
   asking after the insert finds nowhere to stand.

Test first: `a_spawned_building_pushes_a_unit_off_its_ground` (`occupancy.rs`) - red with "the villager was left inside the
spawned building's footprint at (15, 15)".

**The performance mistake I made here, because it was mine:** the first `escape_from` scanned a *square* of radius up to the
map size per dispossessed unit - a quarter of a million tile tests - and took `research.rs` from 21 s to **82 s**. Rewritten
as a ring search that stops as soon as no further ring can beat what it has.

## 3. "Home" was the anchor, not the edge (A6, second cause)

The reason six `research.rs` tests were red was not the eviction: it was that a carrier's route home was planned **to the
drop building's own blocked anchor tile**, so A* had no goal at all, no route was stored, and the villager stood holding a
load until the test's tick cap. Every acceptance run ended with the same sentence:
`no walkable route ... to (16, 16)`.

`approach_tile(from, building)`: the nearest walkable tile of the footprint (a town centre's yard), else the nearest
walkable tile touching the footprint.

Test first, in `dropoff.rs` - which had carried this as an open question in its own header:
`a_load_comes_home_past_a_building_beside_the_town_centre`, a barracks at (14, 14) exactly as the research tests stage it.
Red with "no load came home in 2400 ticks ... Return { drop: EntityId(1) }".

### The regression this caused, and how it was caught

**With `approach_tile` as the route for every trip, the acceptance match stopped being won.** Measured against HEAD
(`04a53a9`) in a separate worktree with the generated data copied in, same seed and tick count:

| run | entities at 36,000 | outcome | repaths | made_room | wall clock |
|---|---|---|---|---|---|
| HEAD `04a53a9` | 176 | **won** (`players=1`) | 220 | 238 | 29 s |
| approach for every trip | 127 | drawn (`players=2`) | 27 | 32 | 29 s |
| + `touches_footprint` | 155 | drawn | 122 | 144 | 29 s |
| **anchor first, approach only as a fallback** | **160** | **won** | 256 | 235 | 29 s |

Two things were wrong, and the second is the interesting one:

1. Aiming every carrier at the building's edge changed the flow of a whole match. The fallback shape - the anchor's route
   first, the footprint's only when that route does not exist - keeps the old flow and still fixes the boxed-in case.
2. Even a walk into the yard was not enough, because the **deposit test measured distance to the anchor**: a villager
   handing in from the yard is two tiles from it and never banked. So the approach and the deposit rule disagreed, and the
   path fix alone lost a third of the economy. `touches_footprint` (on or beside the building's *ground*) makes them agree;
   `a_villager_in_the_yard_is_on_the_town_centre` pins it.

The remaining 176 -> 160 is the widened deposit rule and the spawn eviction moving units differently - a real behaviour
change, reported rather than smoothed over. The match is won, and the `no walkable route` sentence is gone from the tail.

## 4. A control test whose premise had died, and a comment that lied

`walls.rs`'s `a_house_does_not_block_the_way` asserted that a **ring of houses is walk-through** - true when D54 made
`wall` the only blocking role. D76 gave every role a footprint and the ring closed. The test was asserting a fact that
stopped being true, and it was red and assumed to be a staging problem.

Restated as `a_house_blocks_its_own_tile_and_the_way_goes_round`, and it immediately caught a second lie: I wrote the
assertion from `config/economy.toml`'s comment ("a role with no entry blocks its own tile") and it **failed** - a house
blocks its whole 2x2. The code (`solid_tiles` -> `footprint_area`) was right and the comment was wrong, so the comment was
corrected, not the code.

## Performance

- **`escape_from` was O(map²) and I shipped it that way for one build:** a square scan per dispossessed unit took
  `research.rs` from **21 s to 82 s**. The ring version is O(radius) and stops as soon as no further ring can hold
  anything nearer: `research.rs` is now **34-40 s for 20 tests** (it was 21 s for 20 tests when four of them failed
  fast, so the comparison is with the same file doing real work).
- **`touches_footprint` costs one `footprint_of` lookup and four integer comparisons** per carrier per tick, against the
  `reach_raw` arithmetic it joins; it adds no allocation and no search.
- **Acceptance: 29 s at 36,000 ticks**, the same as HEAD's 29 s - the engine changes cost nothing measurable at match
  scale. `repaths` rose 220 -> 256 and `made_room` 238 -> 235, which is the eviction doing its job rather than a slowdown.
- The full suite is **~9 minutes** wall clock on two cores, dominated by `research.rs` (40 s) and the acceptance-shaped
  sim tests; the gate adds 5 s (96 tool tests).

## Stability

- **The deposit rule and the approach rule disagreed, and the match proved it rather than a test:** with the approach as
  the route for every trip the acceptance went from 176 entities and a win to 127 and a draw. Caught by running the same
  binary at HEAD in a separate worktree with the generated data copied in - not by reasoning. Both numbers are in the
  table above.
- **A widened tolerance can reintroduce the pile-up the C2 rule was tuned for.** `a_crowd_all_banks_its_loads` (six
  villagers, one bush, one drop site) still passes, which is the guard that exists for exactly this.
- **Eviction before insertion, not after:** asking "where does this unit go" once the tiles are solid finds nothing for a
  unit in the middle of a 3x3 - eight blocked neighbours. The order of the two steps is the fix, not a detail.
- **A unit is moved to a tile the grid says is walkable, chosen from the pre-insert grid**, so an evicted unit can never be
  placed inside another building; if no tile is found within the map's size it stays put rather than teleporting.
- **The gate is honest in both directions:** with the data absent it fails, with `RUAGE_ALLOW_NO_DATA=1` it passes *and
  says the run proves nothing*, so a CI on a machine without the install cannot quietly claim coverage it does not have.
- **Two tests changed meaning and are called out rather than quietly deleted:** `walls.rs`'s house control (its premise
  died with D76) and `tech_tree.rs`'s `a_free_tile` (it asked about the anchor, not the site).

## 5. Numbers

- `cargo test --offline --workspace --no-fail-fast`: **584 passed, 0 failed, 17 ignored**.
- `tools`: **96 tests, all pass**.
- `research.rs` 20/20 in 34-40 s (was 7 failed); `tech_tree.rs` 10/10 in 8.6 s (was 6 failed); `dropoff.rs` 3/3;
  `occupancy.rs` 4/4; `walls.rs` 3/3.
- Acceptance, seed 7, 36,000 ticks: **won**, `players=1`, hash `ab314fb1969fde95`, 29 s, 160 entities - against HEAD's
  `c12bdc86aab31513`, 176 entities, 29 s.

## 6. What is still open

- The window still plays player 1's side for the player (A1, postponed by the owner) and still draws no HUD (A3, next).
- `can_work` is not wired into `nearest_resource`.
- Zero mining camps and the farm refusals of the AI runs are untouched here.
- `bold`-free assertion: the deposit rule now accepts "inside the footprint", which is a wider tolerance than the C2 rule
  was tuned for; the crowd test (`a_crowd_all_banks_its_loads`) still passes, so no pile-up was reintroduced.
