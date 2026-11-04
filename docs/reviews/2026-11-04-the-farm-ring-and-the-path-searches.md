# The farm ring, and the 1,336 path searches a tick

Owner's note, 2026-11-04:

> Houses are built too close to the town center, ideally, mill should be built 2 farms away from the town center, then it
> should be possible to build farms around town center and the around the mill wight next to them. And houses and othrer
> buildings should be constructed in a way to allow such farm placement.

## What changed

Three rules, all of them content's numbers:

- `[build] farm = "farm"` names the plot, and `[build] farm_spacing = 2` is the owner's "two farms away". The ring's reach is
  `farm_spacing x farm width + 1` tiles from the town centre (7 for a 3x3 plot), computed from the dat's own footprint rather
  than a constant.
- **A farm goes in the ring**, not beside the berries: `farm_site` searches outward from the town centre, then from the
  buildings that take **food** (the food worker's own `drop_sites` list, which is the pack's answer to "where does this go").
- **Everything else is pushed out of the ring** (`Ring::allows`): a drop site may stand in its outer half - the mill is two
  farm widths out, which is the point of the ring - and a house may not stand inside it at all.

The geometry is now visible in the match: at 9,000 ticks the houses stand on the four diagonals at (11,11), (11,21), (21,11),
(21,21) - 50 squared tiles out, the ring is 49 - the mill at (14,21) (29, inner edge 25), and the first farms at (12,16) and
(16,12), right beside the centre.

## The performance bug this step found, which is bigger than the step

The first version of this change turned a 36-second acceptance run into **104 seconds**. The adversarial review found it in
three layers, each measured:

1. **The camp rule was 840 ms per firing.** My own V1 change called `nearest_resource` sixteen times, and each call sorted
   every node on the map *and* path-searched to each of them. Fixed by sorting once, cheaply, with no reachability, and asking
   the walk only of a candidate that survived every cheap test: **840 ms -> 6 ms**.
2. **The farm search paid a path search per candidate**: 256 candidates of A* per order. Fixed by gathering the geometry
   first and walking to at most eight of them.
3. **The real one: 1,336 path searches in a single tick.** `nearest_like` - "the next node of the same kind" - asked
   `can_reach` (a path search) of *every* candidate node on the map, twice each, every time a worker's node ran dry. On a map
   with 118 nodes that is 236 searches for one villager. Sorted by distance first and walking only to the nearest eight:
   **1,336 searches a tick -> 0 in a normal tick**, and the run fell from **104 s to 11.9 s**.

The last fix is behaviour-preserving: the acceptance hash is **byte-identical** before and after (`4e5a6cf336d3bf29`), which is
what a pure performance change looks like. Slow ticks (over 5 ms) went from **830 of 36,000 to one**, and that one is 28 ms.

This was found with a throwaway probe - a path-search counter and per-tick timings behind `RUAGE_PROFILE_RULES`, both removed
before the commit - because nothing else was going to point at it: the tick's own cost was invisible from outside.

## Another thing the review caught

My first `farm_site` drew rings around **every drop site**, lumber camps included, and put farms at the far side of the base
with a long walk to anything that accepts food. It now uses the food worker's own `drop_sites`, so a ring is drawn around the
town centre and the mill, which is what the owner described.

## Tests, written first

`crates/sim/tests/farm_ring.rs`, three tests, all red before the implementation existed and each shown to bite by mutation:

- `a_farm_is_built_beside_the_town_centre` - fails when `farm_site` is stubbed out (`no farm was built at all`);
- `the_mill_stands_two_farm_widths_from_the_town_centre` - fails when `Ring::allows` is made to return `true` (the mill lands
  at (13,17), 10 squared tiles, inside the inner edge of 5);
- `a_house_is_not_built_in_the_farm_ring` - the same mutation puts a house at (13,16), 9 squared tiles, inside a ring of 7.

## Performance, measured

| | before | after |
| --- | --- | --- |
| acceptance, 36,000 ticks | 104 s (and 35 s before the ring) | **11.9 s** |
| path searches in a slow tick | 1,336 | 0 |
| ticks over 5 ms | 830 | 1 (28 ms) |
| camp rule, one firing | 840 ms | 6 ms |
| match outcome | `players=1 entities=144 hash=4e5a6cf336d3bf29` | identical |

## Stability

`farm_site` returns `None` after eight candidates and `Ring::allows` is O(1) once built; a full ring refuses with the engine's
own `no free tile for a farm` rather than searching forever. The camp rule keeps its sixteen-node bound. Nothing new is
allocated per tick: all of this runs at rule time.

Open: a settled base still costs a fraction more pathfinding per gather cycle, because farms and houses now occupy ground that
used to be a corridor (repaths 152 -> 271 in the same match). That is the price of the layout the owner asked for, and it is
37 ms of the 11.9 s - not worth a corridor rule until a player complains about a specific one.
