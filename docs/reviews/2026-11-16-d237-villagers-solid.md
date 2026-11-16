# D237 - villagers are solid: the narrowing, the guard, and what it cost

**Scope.** `crates/sim/src/world/step.rs` (`stamp_group`: mates among soldiers only), `crates/sim/src/world/movement.rs` (the
near-goal clause: a body may be planned round inside the goal's own tile), `crates/sim/tests/overlap.rs` (the specification
fixture comes off `#[ignore]`).

## Performance

**Acceptance match, seed 7, 48,000 ticks, release, both binaries built from source, two interleaved pairs:**

| | entities | `repaths` (body / ground) | hash | wall |
|---|---|---|---|---|
| HEAD | 149 | 432 (90 / 342) | `2f1ab0ae663e2506` | 36.82 s, 37.97 s |
| this | **155** | 497 (139 / 358) | `8cf433ad4e0205f4` | 38.36 s, 38.45 s |

**+2.7%** wall (median 37.39 s against 38.41 s), **+15%** body repaths, and **six more entities** alive at the end. The
comparison is not apples to apples and should not be read as one: the two runs are *different matches* by design (that is the
point of a behaviour change), so the wall delta mixes the extra searches with the extra units that survived them. What it rules
out is a disaster: the D236 attempt cost 3.2x the repaths and **ten** entities; this costs 15% more repaths and gains six.

**Per-tick cost of the changed clause is zero when it does not fire**: `(!near_goal || a_unit)` is one boolean against values
already in hand, and the search it can now ask for is the same one every other body collision already asks for, under the same
two bounds (`stalled.remove` spends the wait, one per man; `MAX_REPATHS_PER_TICK`).

## Stability

**Attack 1 - the crowd fixture passes because the engine is slower, not because it routes.** The fixture measures first contact
and depth per pair, not speed: on the pre-fix tree it reports *"villagers (9700, 9702) got 0.346 tiles apart at tick 101, deeper
than the 0.372 they first touched at"*, which is a **position** claim. It is green here with the same two bodies and the same
tick count.

**Attack 2 - the fix is a one-line loosening that lets every man repath every tick.** It cannot: the near-goal clause only
decides whether a *body* collision may ask, and the ask is spent by `self.stalled.remove(&id)` - one search per man per wait -
and capped by `MAX_REPATHS_PER_TICK`. Measured: 432 -> 497 repaths over 48,000 ticks for 155 entities, not an explosion.

**Attack 3 - the mates narrowing silently disables the group rule for soldiers too.** It does not: `is_military` comes from
content's own `[metrics] military_roles`, and `collision::a_crowd_sent_to_one_place_does_not_stack` - three spearmen ordered to
one tile - is green, which is the fixture that caught the first attempt's mistake (its mates were being reverted by a
separation pass and it read 7,383 raw against an 8,000 reach).

**Attack 4 - the near-goal clause reintroduces the search storm D98a measured.** That storm was a *ground* stall re-planning
beside the goal (525 repaths in 600 ticks without an `adjacent` guard). Ground keeps its guard here; only a body collision
reaches the new arm, and its cost is bounded by the same two limits. Measured above.

**Attack 5 - my own diagnosis.** D236 stated the missing mechanism was "a route that goes round a body". That was wrong: the
pencil (D211) already bends round discs and the `Repath { body }` path already asked for it; the request was refused by a
near-goal guard written for ground. The correction is recorded in D237 rather than quietly fixed, because the tree's next
session should know that a *guard* can look exactly like a *missing feature* when its failure mode is starvation.

**Attack 6 - the fixtures that pinned the group rule were quietly made vacuous.** `crates/sim/tests/group_overlap.rs` had
**two fixtures written with villagers** (`a_group_ordered_together_walks_over_each_other`, `a_mate_who_is_stopped_becomes_solid`).
With villagers never being mates, both would still pass - by never forming the group they exist to test, which is the worst kind
of green. Both now spawn **spearmen**, and a `spawn_unit` helper exists so the role is a parameter rather than a copy. The
mutation that keeps them honest is the narrowing itself: reverting `stamp_group` to mate-everybody leaves the spearman pair
overlapping, so the fixture measures what it claims.

**Not attacked, and why.** The gather ring's own spacing (0.66 between seats against a 0.40 standoff) was left alone: the
fixture `a_newcomer_does_not_move_the_working_gatherers` pins that a re-lay keeps seats, and the acceptance match's six extra
entities say the ring works better with routing than without. Tuning the spacing now would be a second variable in the same
commit.
