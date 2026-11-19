# The pathfinding gap register: nothing extra, nothing missing, nothing different

*Written 2026-11-18, on the owner's ask: evaluate the original's pathfinding and obstacle avoidance, compare this engine
against it, and plan the difference to zero. Sources: `AoK HD.exe` read at the addresses named below
(`docs/movement-in-the-binary.md`), the matching decompilation (`samuelcaldas/age-of-empires-2-decomp`, `src/zone/`,
`src/unit/`), and the first-party talk (`docs/pathfinding-from-the-talk.md`).*

## 1. What the original does, as one model

| layer | the original's rule | source |
|---|---|---|
| ground | a **point test on the tile**, against the unit's terrain mask - no radius in it | `FUN_008aa480` |
| objects | a **rectangular region of tiles stamped** with `0x4000`, a type and an id; per-tile, not a box | `func_001EE5B0` |
| passage | a **four-bit neighbour mask** built from the neighbours' passability and stamp; a step is legal only where the bit is set | `func_001D29F0` |
| plan | **two levels with a parent**: coarse nodes spanning `<< level` tiles, an edge refined to a tile on the boundary; a parent path installed when the low level fails | `zone_spatial_pyramid_search_hierarchical`, `AddFuturePath` |
| cost | **16** per cardinal step, an **octile table**, and `h = 25 x euclid` - a **weighted A\*** (weight ~1.56) that returns longer routes on purpose | `FUN_007ffc80`, `FUN_008007f0` |
| frontier | a per-tile best-cost array with a generation stamp **and** a binary heap | `0x38414`, `0x2db424`, `FUN_00801e90` |
| bounds | inclusive at both ends, clamped to the map; an out-of-map tile is **dropped, never costed** | `func_001D40E0`, `func_001D5B00` |
| termination | stop after **`h` expansions**, never more than **20,000** - and answer the best so far | `FUN_008007f0` |
| start | re-seed when the mover stands inside an obstruction, **2 x radius** outward only | `FUN_007fb220` |
| bodies | **AABBs** from the radius table, grown by the mover's radius, colliding on `<=`; distance is box-to-box with overlap clamped | `FUN_00840830`, `func_002690E0` |
| search obstacles | **standers only** (velocity zero), inserted per search, never into the shared grid | `FUN_007f9d80` |
| step | velocity applied **whole**; no axis dropped, no slide | `008a4360` |
| collision answers | target done / **non-past** becomes a repath / **pause the higher id 0.25-1.50 s** / past walked through / *"don't know what to do"* falls through | `008a4360`, `0x8aa6d0` |
| ignore list | built **at the search's entry point** (self + the order's target + the group's members, or the route's `isMoving` entries) and read **twice**: by the search as **passability** - a tile whose bodies are all on it is free ground - and by the collision classifier as a skip | `FUN_008007f0`, `FUN_00801a20`, `FUN_007fe010` |
| waypoints | snapped to and spent in the same update; **substituted** when not reachable "this or the next update" | `008a4360`, `mConsecutiveSubstituteCount` |
| no failure | a search that cannot reach answers **the closest point**; the order layer may still ask "as close as I can" | the talk, p. 83 |
| speed | the unit's own dat speed; **0.0 on a pause**; no ramp, no terrain factor | `getMaxVelFwd`, `008a2a30` |

## 2. Nothing extra (ours, and the game has no such rule) - delete

| what | where | why it must go |
|---|---|---|
| the **terrain slide** (an axis kept when a diagonal is refused) | `movement.rs` ground branch | *no axis is ever dropped*; it is the source of 10% steps (82, 81, 80 raw a tick, 567 of 1,398 ticks) |
| **`TakeAtRest`** - a step taken through a resting body | `movement.rs` | the epilogue has no such arm; an idle friend who is not past is a non-past collision |
| **rule 3** - "a body already inside is not an obstruction" | `movement.rs` | not in the binary; kept only because deleting it alone made overlaps worse (104,386 -> 167,025 pair-ticks) - it goes with the repath that *leaves* an overlap |
| **`MAX_GROUND_REPLANS = 2`** | `lib.rs` | the game counts nothing per route; it substitutes the waypoint |
| **`MAX_REPATHS_PER_TICK = 4`** | `lib.rs` | ours, invented to bound a storm the cap created |
| the **avoid set written into the search** | `step.rs` re-plan | the ignore list is written after the search and read only by the classifier |
| **same-tick mates** as the path-ignore list | `step.rs` `stamp_group` | membership is a **formation's**, re-tested on `isMoving` each tick; a paused mate stops being exempt |
| the **tile-chain fallback** when the geometry declines | `lib.rs` `corners` | the game has no point-route fallback: it expands the search area once, then answers the closest point |

## 3. Nothing missing (the game's rule, ours absent) - build

| what | source | fixture that must fail first |
|---|---|---|
| **two levels with a parent path** | `zone_spatial_pyramid_search_hierarchical`, `AddFuturePath` | `conformance::the_original_solves_a_long_walk_in_its_hierarchy` (red today) |
| **the expansion budget** (`h`, cap 20,000) with a best-effort answer | `FUN_008007f0` | a search whose goal is far behind a wall answers *something*, and expands no more than `h` |
| **waypoint substitution** | `mConsecutiveSubstituteCount` | a waypoint a re-plan keeps returning is skipped and the next is reached (branch `d279-substitute` has the mechanism, not the fixture) |
| **the octile table** (not a flat 23 per diagonal) | `FUN_007ffc80`, `DAT_00afd2a0` | a two-diagonal leg costs `table[2]`, not `2 x 23` |
| **the border cost nudge** (`cost_mod` 0 / -100) | `zone_spatial_pyramid_search_radius` | tie-breaks near a map edge; unread in detail |
| **the order's path vocabulary**: forced waypoint, teleport-to-waypoint, per-path hit | `ADDmove`'s fields | a forced waypoint is honoured even when the search would route another way |
| **`<=` on a body collision** (touching collides) | `FUN_00840830` | two bodies exactly at the pair's reach are a collision, not a brush |

## 4. Nothing different (same intent, different answer) - align

| what | ours | the game |
|---|---|---|
| planning for the body | `short.rs` grows obstructions by the radius (same idea), but a **place route** is planned by a point and the fallback by tile centres | every route is planned for *"the boundaries of where this unit's radii will go"* |
| arrival | a **ring/standoff distance** | the **box gap** against the order's `rangeDistance` (`func_002690E0`) |
| pause | drawn once, never re-drawn | `setPause` writes the timer **unconditionally** |
| moving-mate exemption | ends when the route is dropped | re-tested every tick on `isMoving` |
| re-plan after a collision | re-plans **from the body**, capped at two | re-plans, uncapped |
| route shape | tile A\* then geometry | the geometry **is** the search, at two levels |

## 5. The plan, in dependency order

1. **Route origin** - every route planned for the body's own radius against the ground, and the fallback's first leg made
   walkable (prefix, measured inert). *Fixture: a gather approach beside a footprint produces a route the disc accepts.*
2. **Arrival by box gap** - replaces the ring distance. *Fixture: a carrier at the wall banks; a man pushed out by his own width
   still works.*
3. **Waypoint substitution + delete both caps**, with the fixture step 1 owes. *Gate: the acceptance resolves.*
4. **Delete the slide**, which should then cost nothing. *Gate: the movement suites stay green and the acceptance stays won.*
5. **Delete `TakeAtRest`, rule 3, the avoid-set-in-search and the same-tick mates** - one at a time, each with the repath that
   replaces it, because each was kept for a measured reason.
6. **The expansion budget**, then **the octile table**, then the **border nudge** - the search's remaining constants.
7. **Two levels with a parent path** - the last conformance red, and the one that makes 1-6 unnecessary to fight.
8. **The order's path vocabulary** - forced waypoint, teleport, per-path hit.

**Already zeroed (2026-11-19, D284).** The ignore list's **plan-level** use: the search now skips the mover's order group, so
a group's route is planned through its own members (`FUN_00801a20`), and the group is formed before the first plan of the
batch. What remains of that row is the **membership** - same tick instead of a formation, and a mate exempted because it has
a route rather than because `isMoving` says so this tick (D207's stand-in, still open).

**The measurement that governs every step**: `players` (the match must resolve), no late resource wall, the suite green, and the
entity/re-path counts as information (D278). Today: `main` **131 / players=1**, and the working branch **150 / players=2** with
**149,254** ground re-paths - the storm the caps hid, **since measured**: half of it is the tile-chain fallback (step 1,
rows 1 and 26) and half is *bodies* (the collision block's epilogue, row 29), so neither step alone moves more than half
(`docs/reviews/2026-11-19-the-storm-split-by-route-origin.md`). Step 1 still owes a probe that
prints **the route's origin** per refusal, not the task.
