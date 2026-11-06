# Passage is the surface (D132), and the match is thirty minutes (D133)

*2026-11-06. Two of the owner's answers from the six-question list, landed the same evening. Both were written as tests
first, and both tests were watched failing.*

## What changed

| | before | after | why |
|---|---|---|---|
| `Map::is_walkable` (a town-centre site, a seeded body) | `Grass` alone | `Grass \| Dirt` | the owner: *"grass and dirt passable, wood impassable"* - and dirt is the surface left after a tree is cut and gathered |
| `Terrain::walkable()` (a route) | grass, dirt, forest | **unchanged** | terrain is ground and the **tree** is the obstacle (D84). A cleared wood is walkable the moment its node goes, because `Walk` counts occupants |
| `config/acceptance.toml` `duration_seconds` | `2400.0` (forty) | `1800.0` (thirty) | the file's own prose said thirty and the canonical command plays `--ticks 36000` = 1,800 s at 20 Hz |

The two predicates answer different questions and had been confused. Their doc comments now say which is which: **may
something be placed here** versus **is this surface ground**.

## The failing runs, as written

| test | before | after |
|---|---|---|
| `a_town_centre_site_may_be_grass_or_dirt_and_never_wood` | **FAILED** - `player 2 town centre at (1, 0) is not on open ground` | passes |
| `a_town_centre_in_the_woods_is_refused_by_tile` | (fails for the same reason: the dirt site is refused first) | passes - refusal names `(2, 0)` |
| `the_acceptance_match_lasts_the_thirty_minutes_the_command_plays` | **FAILED** - `left: 2400.0, right: 1800.0` | passes |

## Performance

The change is inert on the shipped pack, and that is measured rather than argued:

| | before this change | after |
|---|---|---|
| acceptance hash, 36,000 ticks, seed 7 | `394c4892aac776d2` | **`394c4892aac776d2`** |
| entities / repaths / made_room | 141 / 184 / 187 | 141 / 184 / 187 |
| decider (`"the match is over"` first refusal) | 1,527.6 s | **1,527.6 s** |
| wall time, release, three runs | 13,027 / 12,028 / 12,066 ms (median **12,066**) | 10,936 / 11,346 / 11,345 ms (median **11,346**) |

The 6% wall-time difference is **not** a claim: the identical hash means the identical match, and this box's three-run
spread has been ~8% all evening. What the timing does establish is that the change costs nothing.

Whole suite: **707 passed / 0 failed / 20 ignored** in `463.2 s` (`cargo test --offline --no-fail-fast`, debug, warm
build) - three more tests than the 704 of D130, which is exactly the three written here.

## Stability

| attack | result |
|---|---|
| a town centre standing on dirt | loads; `is_walkable` true (test) |
| a town centre standing in the woods | refused, and the message names the tile (test) |
| a tile outside the map | `terrain_at` is `None` -> not walkable, unchanged |
| the shipped pack's seeding | hash identical, so no body moved: the seeder's candidate tiles were grass already |
| a wood that has been cleared | unchanged and already correct: `Walk` counts **occupants**, so the tile frees when the node goes; no terrain mutation exists or is needed |
| every caller of `is_walkable` | two, both read: map loading (town-centre sites) and seeding (candidate tiles). No third caller to surprise |
| a match shorter than the game | the match is decided at **1,527.6 s**, inside the 1,800 s limit, so nothing is truncated - and the refusal log before and after the change is identical line for line |
| the acceptance file against the command | a test now ties 1,800 s to the 36,000 ticks the documented run passes, so the two cannot drift apart again |

## This step's own errors

- I expected the "dirt after the tree is cut" rule to need a terrain change in the engine, and went looking for somewhere
  to mutate `Forest` into `Dirt` on depletion. Reading `Walk` showed the tile already frees itself when the occupant goes,
  so the change is one predicate, not a mechanic - the owner's sentence described behaviour the engine already had.

## What this step did not do

- **D131** is untouched and still needs the owner's choice (bound the give-way step, let bodies slip past, or bound the
  invariant at `reach - one step`).
- The chop pace and the army-size pair were **answered with evidence rather than code**: the rate is already the dat's
  (`VMBAS` `attacks = [3, 3, 6, 0]`, `reload_time = 2.0` -> 3 hp / 2 s), and the game's own scripts put an attack group at
  `10..80` soldiers (`Military/General Military.per`) or `16..36` in four groups (`Dr3/shared.per`), not at 3/11.
- Reachability in the node lookup: priced in the reply, not implemented.

**707 passed / 0 failed / 20 ignored**; clippy and fmt clean.
