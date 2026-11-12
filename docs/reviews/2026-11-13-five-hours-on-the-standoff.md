# Five hours on the standoff: what landed, what parked, and the numbers

A session record: the tile geometry landed, the touching standoff is built and parked, and one movement deadlock is named with the
measurements that identify it.

## Performance

`main` is green at `ffe5001`; the branch's acceptance run reads **`tick=36000 entities=143 players=1 hash=ac7cfd180946dfe6
repaths=434 made_room=0`**, against `main`'s `ef7cdba92a83f173`, 142, 435 and **96**. `World::step` 7.2 -> **6.9 us**; `Snapshot::of`
98.0 -> 105.0 us (flagged, not explained). No timing was taken while another run was in flight.

## Stability

- **Landed on `main`:** `ruage_sim::tiles` (D191) - corners, `distance_to_tile`, `touching_places` - with four mutation-checked tests.
- **Parked:** D192, the standoff moving from the 1.156 ring to the border. Eleven places on a tile, every gather suite rewritten, and
  **`made_room` falls to zero** - nobody is shoved aside when men stand on the border instead of queuing behind one another.
- **The one red:** a villager told to step off a ghost oscillates on its edge at `x = 4.95` for 2,000 ticks while the builder walks at
  the same approach tile; the house stays at hp 1. Diagnosed by printing the site's occupants (`standing [(4, Move, (4,4), resuming)]`),
  the footprint's walkability (all true), and the body that blocks him (the builder, friendly, moving). A friendly body never blocking
  was tried: it does not clear it.

Nothing was merged that failed a test; nothing is uncommitted.
