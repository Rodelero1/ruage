# D271 - the crowd round the click: review

*The owner: **"1 b -- any single unit chosen has to go to the exact spot by moving the center of the circle to that exact spot. If
spot unreachable, try come as close as possible. If multiple villagers are chosen, then the first comes to the spot and others stand
around him."***

## What changed

`start_walk` (`crates/sim/src/world/sites.rs`) no longer gives a follower `tile_centre(slot)`. When nobody else is already heading
for the clicked tile the order's goal is the **clicked point**, which is what a single unit gets; when somebody is, the goal is
`World::crowd_ring_point` - a point on a ring **four times the body's radius** out (`0.8` of a tile for a villager), in eight
directions, **ordered by how near each candidate is to where this villager already stands** so a man arriving from the east stands
east of the spot, with the `N`-th best tried first so a crowd deals round the ring instead of stacking on one side. Every candidate
must be ground the body fits on, clear of every other body, and **clear of the point another man of the same crowd is already walking
to**.

The radius is a measurement: at the gather spacing (`0.65`) the chord between two ring slots is `9,950` raw, which is less than the
`8,000` their footprints need once they drift, and `group_move::a_crowd_ordered_to_one_tile_all_arrive_and_spread` caught it (two of
one order at **6,311 raw**). At `4 x radius` the chord is **12,240 raw**. The "claimed point" rule is the other half: without it two
men take the same slot and walk across each other, which `overlap::eight_villagers_ordered_to_one_tile_never_walk_through_each_other`
caught at **0.344 tiles** apart against the **0.367** they first touched.

`slot_for` - the tile-ring search that produced the up-left bias - is **deleted**: it scanned from `(-1, -1)` outwards, which is why
every follower landed up and to the left.

## Performance

- **Measured** (`click_spot::what_the_ring_costs_for_a_hundred_orders`, ignored, 120 villagers ordered to one point): submitting the
  **120 orders took 162.4 ms** - 1.35 ms each - and the **600 ticks that followed took 8,503.9 ms** (14.2 ms a tick for 120 bodies).
  The ring is at most **eight candidate points per order**, each a `clearance_at` plus a scan of the world's units, so ~960 checks in
  total; the 1.35 ms an order is dominated by the route's own A\*, which this change does not touch.
- The ring is computed **once per order**, never per tick.
- The acceptance match is unaffected by construction: its script issues single-unit orders, so it never takes this branch.
- Full suite: **200 binaries, 199 ok, 1 failed** - the pre-existing `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`.

## Stability

- Fixtures, written first and watched fail (both **unignored**):
  - `click_spot::a_single_unit_stops_on_the_spot_it_was_sent_to` - green before and after; it is the half that already held, kept so
    the pair cannot regress together.
  - `click_spot::a_crowd_stands_round_the_unit_that_took_the_spot` - red at *"a follower stands **25,455** raw units from the spot:
    he was sent to a square's middle, not round the man on it"*, green at **12,998 raw (0.65 tiles)** on both sides of the first man.
- The first man takes the spot **exactly** (`off = 0` in the fixture's own print) and the followers stand **east and west** of him at
  `±0.65` tiles - "around him", not behind him.
- Two followers at the old tile centres could be a whole tile apart and still inside each other's approach; the ring's own
  clear-of-bodies test is what keeps them out of one another, and the fixture asserts every pair is at least a footprint apart.
- **A note on the fixture's own construction**: my first version spawned the two followers two tiles off the open row, which put one
  of them inside a building's footprint; he then never arrived and the test measured the map. The spawns are now all on the single
  row the map is known to leave open.
