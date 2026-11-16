# D238 - "villagers still push": the seat shuffle, measured, and why it cannot be removed yet

**Scope.** `crates/sim/tests/overlap.rs` (one new fixture, `#[ignore]`d). **No production code changed**: three implementations
were built, measured and reverted in this session; their numbers are the evidence.

## Performance

**Acceptance match, seed 7, 48,000 ticks, release**, against D237's shipped baseline (155 entities, 497 repaths, `8cf433ad4e0205f4`):

| attempt | entities | `repaths` | the fixture that caught it |
|---|---|---|---|
| ignore any body with a route | **126** | 507 | `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` - **0.204** tiles apart |
| ignore non-ring-member walkers | 154 | **380** | the same - **0.204** |
| held seats excluded + release only at rest | (not gated) | - | the same - **0.204** |

**The second row is the interesting one for cost**: ring-members-only *improved* both numbers - **380** repaths against 497 is
**-23.5%**, and entities 154 against 155 is **-0.6%** - so the seat churn was doing searches, and yet the same 0.204 pile
remained. The first row is the disaster: **126** entities against 155 is **-18.7%** of the match's army and economy gone. So the cost of the shuffle is not where the defect
is; removing it does not make the crowd separate, it only stops hiding the overlap.

**Nothing on the tick is added by what landed**: the commit is a fixture and an `#[ignore]`. The acceptance hash stands at
`8cf433ad4e0205f4` because the tree is D237's, and that tree's own measured wall is **38.36 s** and **38.45 s** on this box
(two runs, 48,000 ticks). The three reverted attempts were **not** timed: none of them was kept, and timing a build that is
being thrown away would put a number in a table that no reader can use.

## Stability

**Attack 1 - "pushing" is a guess about a mechanism.** It is not: `reserve_gather_place` was instrumented to print every change to
a held seat with the bodies near the *new* point, and the defect is one line - `tick=756 worker=9901 old=(421103,276154)
new=(410625,276000) near=[11:Idle, 9900:Gather{node:11}, 9902:Gather{node:11}]`. The two neighbours are ring members **still
walking to their own squares**, which is the case the release test cannot tell from a body standing on the seat.

**Attack 2 - `velocity` is not a test of "is walking".** The first draft of the fixture asked `entity.velocity != 0` and measured
**0 walking ticks** on a man crossing the map: this engine does not maintain `velocity` (battle reads motion off the movement
itself, `world/battle.rs`). The fixture now compares positions between ticks, and the engine's release test uses "has a route",
which *is* maintained.

**Attack 3 - each fix was judged by the whole match, not by its own fixture.** All three would have looked correct against the
passer-by fixture alone (each made the seats stay put). The acceptance match and `three_gatherers_on_one_bush_do_not_pile_up`
killed them, and the third attempt - the honest rule, with held seats excluded so the churn is not needed - failing identically
is what **locates** the remaining overlap in the collision clauses rather than in the seats.

**Attack 4 - the fixture could pass for the wrong reason.** It absorbs a walker who stops *inside* the ring (a man parked on your
square is not a man walking past), by asserting only on ticks where his position changed, and it fails loudly if the walker never
crosses at all (`"the passer-by never crossed the ring"`). Absorbing stops at the moment he stops - which is the honest boundary
of the claim.

**Not attacked, and why.** The two collision clauses that let a body be crossed - D210's "past" test and "a body already inside
is not an obstruction" - were not touched in this session: fixing them changes the movement pass for every unit, and doing it in
the same commit as a seat-rule change would leave no way to attribute the acceptance movement to either. They are the next
slice, in that order, with the seats after them.
