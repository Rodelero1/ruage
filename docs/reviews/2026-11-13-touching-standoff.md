# Standing with the circle on the tile (parked)

**Branch `d192-touching-standoff`, not merged.** The standoff the owner asked for is built and every gather test asserts it; one build
test still fails, and that is what parks it.

## What changed

- `standoff_point` no longer samples a 1.156 ring round the tile's **centre**. It asks `tiles::touching_places` for the node's
  **tile** - the points where the worker's circle touches the border without overlapping - and picks the nearest that is walkable, off
  a foundation, a body apart from every other man and every reservation, and within `GATHER_WALK_SLACK_MILLI` of walking straight at
  the node.
- The working test is `distance_to_tile(position, node_tile) <= radius + 1/50`, i.e. **touching**, instead of a radius band. The buried
  -ring fallback (a tree deep in a forest) is kept via `ring_open`.
- The capacity is `touching_place_count` - **eleven** for a villager on one tile - computed from the geometry, not stored.

## Performance

| | `main` (`ffe5001`) | this branch |
|---|---|---|
| acceptance hash | `ef7cdba92a83f173` | `ac7cfd180946dfe6` |
| outcome | `players=1` | `players=1` |
| entities at 1,800 s | 142 | 143 |
| repaths | 435 | 434 |
| **made_room** | **96** | **0** |
| `World::step` | 7.2 us | 6.9 us |
| `Snapshot::of` | 98.0 us | 105.0 us |

`made_room` at **zero** is the headline: with men touching the tile instead of queuing on a ring, nobody has to be shoved aside any
more. Repaths are unchanged, the tick is slightly cheaper, and the snapshot is 7 us dearer - it draws every entity anyway, so that
last figure needs a second look before this lands.

## Stability - what is green and what is not

**Green:** every gather suite that asserts the standoff, the spacing, the capacity and the owner's own arrangement, rewritten from the
1.156 ring to the touch - `gather_touching` (new: a woodcutter touches his tree; ten men touch one bush, none overlapping), `gather_ring`,
`gather_standoff`, `gather_standoff_rule`, `gather_motion`, `gather_continuous`, and `gather`.

**Red, and it is what parks the branch:** `build::a_content_pack_with_our_own_resources_loads_and_spends` - the house sits at **hp 1**
after 2,000 ticks because a villager ordered **off** the ghost (step-aside) oscillates at `x = 4.95` for the whole run, on the ghost's
edge, never reaching the tile outside it. The footprint's tiles are walkable, his route has one waypoint, `resuming` is set the entire
time, and he is blocked by the **builder** - who is walking to the same approach tile. Two of the player's own units fighting for one
tile, each refusing to pass the other, is a movement problem this slice exposed and did not fix; the branch's `collide` change (a
friendly body never blocks) was tried and does **not** clear it.

**What the next session should do:** reproduce the oscillation in a two-unit test (a walker on a ghost, a builder beside it), decide
whether step-aside should be a *stable placement* rather than a walk that can deadlock, and only then land the standoff. Everything
else here is measured and ready.
