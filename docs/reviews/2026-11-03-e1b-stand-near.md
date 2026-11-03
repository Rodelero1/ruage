# Review: E1b, step one - an order's route ends where the blow can land

*Change: `start_attack` no longer plans a route **to** a target whose own tile is blocked (a building, a node). It asks
`stand_near`: free ground, reachable, within the resolved `range` (adjacency when the range is melee) and outside
`min_range`, sorted by distance **from the attacker** with a tile-order tiebreak, skipping tiles another unit already
heads for. A unit already in reach gets an empty route; one with nothing reachable keeps the old behaviour. New test
`evasion::an_arms_route_ends_where_it_can_strike`.*

## Performance

**Measured cost, and it is a regression:** the acceptance run at 36,000 ticks takes **15.3 s** where S1 took 12.66 s
(+21%). The cause is order-time, not tick-time: `stand_near` spends up to one path search per candidate tile, and the
AI's attack rules re-issue their orders on a cooldown. The economy lines are **byte-identical to S1's** (5 wood / 10 food
at 400 s, 398 food at 1,600 s), which is the proof that the cost is confined to orders: nothing a villager does changed.
The outcome is unchanged as well - `players=2`, hash `a596e97fc9a47ec9` - so this step buys correctness for armies, not
throughput.

## Stability

- **A unit already in reach pays nothing**: an empty route, no search.
- **Nothing reachable falls back to the old route**, so a target walled in still gets the old "walk as close as possible"
  behaviour rather than a refusal.
- **Crowds spread**: the candidate sort is by distance from the attacker, not by ring order, because ring order hands
  every attacker the same tile when their orders land on the same tick - none has a goal yet, so nothing looks claimed.
  That was measured: with ring order, twelve attackers still queued behind the first arrival.
- **Deterministic**: the sort key is `(distance, tile)` and the skip test reads `goal_of` in entity order.

**This slice's test-first evidence is weak, and it says so rather than implying otherwise.** The RED that drove it was
the parked `garrison::a_damaged_building_ejects_its_garrison` - twelve attackers, five strikeable tiles, `cooldown_ticks:
0` - and that test is **still parked**, because five tiles cannot hold twelve bodies and the fixture needs an
open-perimeter target or ranged attackers. The new `an_arms_route_ends_where_it_can_strike` passed on *both* sides of
the change (a lone attacker already closed), so it is a guard, not the red-then-green pair. Named as a violation of D7's
rule rather than dressed up: the change is right by the original's shape and by the parked diagnosis, but the failing
test that proves it is still owed with the fixture.

Known and accepted: `stand_near` composes with the collision rule rather than replacing it - a unit walking to its stand
tile can still be blocked by a body and repath, which is E1a doing its job.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
