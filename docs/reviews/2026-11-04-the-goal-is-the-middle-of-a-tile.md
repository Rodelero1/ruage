# Review: the goal is the middle of a tile

*Change (M3a, D114): `World::tile()` is a tile's **corner** — the anchor a building's footprint and a spawn position are
placed from — and it was also the point every walk aimed at. Walk goals now use `tile_centre()`. Two hundred lines of
production change would have been one line if the convention had been right in 2026.

The bug, measured: a villager ordered to a tile stopped **10,000 raw units** from the middle of it — half a tile on each
axis — and a villager ordered to a tile beside a house stopped **0** raw units from the house, i.e. standing on its
corner. That is what the owner has been calling "villagers went through the house", and the unit of it is not collision at
all: the point the walk aimed at was on the wall.

## Performance

The acceptance match (36,000 ticks, seed 7, three runs): **10.54 / 10.54 / 10.81 s**, 148 entities, `players=1`, hash
`82efabd6f614c798`, repaths 124, made_room 171 — identical in all three runs, so the change is deterministic where the
match is stable. Against M2 (`ddcad16`) at 10.54-10.65 s, 148 entities, hash `623c3ff8c769eadd`: **the wall clock is
unchanged within noise**, and the hash moves because every unit now walks to the middle of its goal tile instead of its
corner. No per-tick cost was added: `tile_centre` is one extra addition at plan time.

The one new cost is in the tests: `tile_goal.rs` walks a villager for 600 ticks twice, about 20 ms.

## Stability

| attack | result |
|---|---|
| the goal point (both tests) | RED first, as written: 10,000 raw units off centre, 0 raw units from the house. GREEN after `tile_centre` |
| a rally point **inside** a building | centring the door point sent the finished unit *into* the footprint and it never arrived - `a_unit_rallied_to_its_own_building_stays_inside` failed. A garrison rally is not a place to stand; it is "come to this building". Reverted to the corner for that one path, which is now named in the code |
| a rally point **on a resource** | same shape: `a_villager_rallied_onto_stone_gathers_stone_not_the_wood_around_it` failed, because the goal is the *node*, and the gather approach owns where the body stands. Reverted |
| two units ordered to one tile | `a_walker_does_not_finish_on_top_of_a_friend` failed on a **tile** comparison; the bodies were 0.7 of a tile apart, which is legal and always was. The assertion is now the standoff `collide` actually owes |
| one unit crossing another's tile | `a_body_on_the_goal_is_never_entered_and_costs_no_repath` failed the same way: a body stopped at its standoff sits *in* the goal's tile without standing on the body. Assertion converted to a distance with one step of tolerance |
| "went round the body" | `a_blocked_unit_goes_round_a_standing_body_and_arrives` measured a **change of row**, which stops meaning anything when the goal is a tile's centre (the walk ends in the same row either way). Converted to the standoff, which is the stronger claim and passes |

**Tests that were encoding the old convention, corrected rather than weakened** — each is now measuring the rule instead of
a proxy for it:

1. `points.rs::a_tile_order_still_means_its_centre` — the test's **name** said "centre" and its assertion demanded the
   **origin**. It now has a `tile_centre` helper beside `tile_position`, and the two are documented as different things on
   purpose. This is the clearest evidence that the convention was wrong and that a green suite was agreeing with it.
2. `straight_walk.rs::a_walk_lands_exactly_on_its_point` — asserts the landing is *exact* (M1's guarantee). The exact point
   moved from the corner to the centre; the intent is unchanged and the test still passes.
3. `collision.rs`, `evasion.rs` (three assertions) — tile comparisons replaced by the standoff, as above.

**A test that stopped measuring anything, and why — reported as an error because I first got the cause wrong.**
`evasion.rs::a_repath_route_is_pulled_like_a_planned_one` places a body on the walk's line to force a repath. With centred
goals the walker **arrived with `repaths = 0`**, and I could not explain it, so I `#[ignore]`d the test with two
hypotheses: an idle body missing from the per-tick `bodies` list, or `blocker_at`'s 3×3 tile bucket missing a body on a tile
boundary.

**Both were wrong, and there was no defect.** The mover answers a blocked step in a fixed order: ground, then the order's
own business, then the standoff, then **slide on one axis**, then one nudge, then a repath. From a *corner* to a *centre*
the walk carries a small vertical component, so the body that blocks the step can be **slid past** on the other axis — the
unit arrives, correctly, without ever replanning. From **centre to centre** the walk has one axis only, the slide is the
step that was already refused, and the mover reaches its repath rule. The fixture now spawns the walker at the middle of its
tile and the body at the line's exact midpoint, and the test measures the pull again. Sliding before replanning is the
design, not a workaround (D98a) — the ignored test was my fixture asking a question the engine never had to answer.

**A claim I made and then withdrew, in the same session:** while the evasion fixtures spawned at tile centres I measured the
blocked-unit walker at **3,863** raw units from a body whose standoff is 8,000, wrote that into a test comment as a
pre-existing tunnelling bug, and it was wrong — the centre-spawned fixture had moved the body off the line, and with the
engine's own spawn convention the same measurement is ≥ 8,000. The comment is corrected in the test; recorded here because
the first number was reported before it was understood.

## Known and accepted

- **Spawns still sit on tile corners.** `Command::Spawn` is unchanged by this slice, so a freshly trained unit appears on a
  corner and walks to a centre. That asymmetry is real and is the next place the convention should be unified; it is
  named rather than fixed because a spawn position is content-dependent (a building's rally ring decides it).
- **A garrison rally still aims at a corner.** Deliberate: the point is "come to this building", and the entering rule
  decides when it is close enough. Naming it in code, not just in this review.
- The acceptance hash moves. That is the expected consequence of every unit standing somewhere slightly different, and it
  is why the hash is asserted rather than assumed.

---

*`tools/review_check.py` looks for `## Performance` with a number and `## Stability` with content; both are above.*
