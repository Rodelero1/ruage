# D245 - how the original meets the four body rules, and the one defect that gates three of them

The owner asked how the original meets his four rules and for them to be implemented. **Rule 3's fix was built and works on its
own fixture - and it cannot land yet**, because it re-times the crowd and the crowd piles. That is the finding: three separate
attempts (D236, D238, D245) now land on the same mechanism, the **seat shuffle**, and it is what keeps rules 1, 3 and 4 red.
Nothing in `crates/` changed in this commit; the tree is D243's, and the numbers below are from builds that were measured and
reverted.

## The original, rule by rule

**Rule 1 - never closer than one diameter.** The original writes **no** positions on contact: the walker's move is tested, and on
a hit it **stops and re-routes**; being refused is a legitimate outcome (`MovementSystem`'s epilogue at `0x79b730`-`0x79b7c6`
returns "not reached", the caller keeps the waypoint and sets the pause; openaok's `Navigation::Move` does the same). Our engine
agrees - `made_room` reads 0 on the acceptance match, `push_the_standing` was deleted in D207/D208/D210.

**Rule 2 - gather only from within half a diameter.** The original's arrival test is a distance to the object's **box** plus the
job's range (openaok `WorkerSystem.cpp::ApproachGround`: *"up to the resource's own box ... within the job's range"*). For a
villager and a one-tile tree that is `0.5 + radius`: the **0.70** edge stand and **0.935** corner stand measured from the owner's
saves (D192/D199). Ours matches - the closest gatherer measures **0.200** from the box in `invariants`.

**Rule 3 - drop only within half a diameter of the drop site.** The same box test, against the building. Ours failed at **0.281**
because the delivery check was three loose clauses: tile adjacency by index, the *pair's* reach (`0.4`) against the building's
**anchor point**, and `touches_footprint`, which allows a whole tile of slack. The fix is one test -
`footprint_gap(position, drop) <= radius + one step` - and it **passes** (`invariants`: 3 passed, 1 ignored, rule 3 un-ignored).

**Rule 4 - while gathering, a villager does not move.** The original hands a villager his place once, when he arrives, and while
the gather runs there is no move order to give him. Ours recomputes the place every tick and **steps** him: the fixture catches a
0.011-tile step at tick 11.

## What blocks rule 3, with numbers

The rule-3 fix moves every delivery in the match, and the first casualty is a body rule:

- `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` goes **red at 0.236 tiles** (bodies touch at 0.400). At HEAD the same
  fixture passes - so this is not a new defect, it is the **seat shuffle (D238)** exposed by new timing: carriers bank at **4,155
  raw** from the wall instead of walking in to the pair's reach, the stock arrives a little differently, the crowd round the bush
  forms a little differently, and two seated men drift to 0.236.
- Narrowing the "a collider that has not moved is past" rule (D210) so a **working gatherer is not past** did **not** change the
  number - **0.236 unchanged** - which rules out the passer-by explanation and confirms it is a **seated pair drifting**, i.e. the
  seat logic itself.
- Aiming the carrier's last stretch at the wall (a sub-tile point at `radius` from the footprint) is **not** a safe alternative:
  measured, a carrier cut across a body and a pair came **0.325** apart, and a body ended **0.145 tiles inside** a blocked tile
  (`body_clearance::no_unit_ever_stands_with_its_body_inside_a_wall`). Reverted.

**The geometry fact that makes the rule reachable at all**: a carrier's route ends at the approach tile beside the footprint and he
stalls at **4,155 raw** from the wall against a **4,000** radius - 155 raw he can never cover, a step being 758. So "within half a
diameter" is only true with **one step of slack**, in the engine and in the fixture, and both now say so.

## The one mechanism to fix next

`reserve_gather_place` recomputes seats and **moves a seated man onto a newcomer's geometry** (D238's `SEATMOVE` line), and the ring
round a node is *maintained* by stepping men who are already working. Every attempt to remove the stepping reddens
`three_gatherers_on_one_bush_do_not_pile_up` at 0.204-0.236 because the stepping is also what **arranges** the ring (D200/D202).

The design that follows from the four rules: **a node hands out a place, the man keeps it until he leaves, and a man who has no
place waits outside the ring at a distance that keeps every body 0.400 apart.** Then rule 4 (no movement while gathering) and rule
1 (never closer than a diameter) become properties of the placement instead of properties of a per-tick correction, and rule 3's
timing can move without a pile.

## Cost, measured

Rule 3's fix was measured on the acceptance match before being reverted: `entities=157 repaths=649 made_room=0 repaths_body=257
repaths_ground=392`, hash `b8378af1021a09e2` - **more** entities than D237's 155, with re-paths up 497 -> 649 (+31%) because
carriers re-plan to reach the wall. Wall time was not interleaved, so no percentage is claimed.

## Not done, and not pretended

Rule 3 is **implemented and not landed**; rules 1, 2 hold as fixtures; rule 4 is red. The seat redesign is the next slice and it is
what unblocks the rest - attempting it a fourth time without it being the explicit task would be guessing, and D236/D238 both paid
for that.
