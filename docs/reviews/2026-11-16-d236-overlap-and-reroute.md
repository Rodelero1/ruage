# D236 - villagers walking through each other: the specification, and the two fixes that failed

**Scope.** `crates/sim/tests/overlap.rs` (new: two fixtures, one green and mutation-proved, one `#[ignore]`d as the
specification). **No production code changed** - two implementations were built, measured and reverted in this session, and
their numbers are the evidence below.

## Performance

**Measured on the acceptance match, release build, seed 7, 48,000 ticks.** The three builds are the same tree except for the
change named, and each was rebuilt before its run:

| build | entities | `repaths` (body / ground) | hash |
|---|---|---|---|
| **baseline (what landed)** | **149** | **432** (90 / 342) | `2f1ab0ae663e2506` |
| exemption narrowed to soldiers | 139 | **1,401** (22 / 1,379) | `efdb951fe00e707a` |
| + post-step separation pass | 139 | 1,401 | `efdb951fe00e707a` |

**The change made the tick 3.2x the pathfinding work and lost ten entities** - the colony starves (`not enough food: 50 needed,
5 in the bank`), which is the cascade: villagers that may not pass over each other spend their two-second stall (`STALLED_TICKS`)
standing, replan, and never reach the woodline. This is a *behaviour* regression, not a cost, and it is why nothing landed.

**Against the defect**: today's engine lets an ordered crowd reach **0.269 tiles** apart where two villager radii are 0.400 -
the measurement the owner's eye saw, and the number the ignored fixture asserts against.

## Stability

**Attack 1 - the ignored fixture is unproven.** It was run against the pre-fix tree and is red with a named pair and a named
tick: *"villagers (9700, 9702) got 0.346 tiles apart at tick 101, deeper than the 0.372 they first touched at"*. With both fixes
in it is green. It is `#[ignore]`d because the fix cannot land, not because the failure is in doubt - and the ignore message
says where the work is (`docs/next-step.md`).

**Attack 2 - the green fixture does not bite.** `a_newcomer_does_not_move_the_working_gatherers` was mutation-proved: a re-lay
that wipes seats (`self.gather_place.clear()` before the re-assign) reddens it with *"villager 9800 lost the place it was
working when a newcomer arrived at the node (was (396000, 294375), now None)"*. It also needed **two** drafts before it measured
anything: a three-man ring had free seats, so no re-lay fired (instrumented: **0** re-lays), and a newcomer still walking to the
node never settles. Placing the ninth man beside a packed node fires the re-lay three times (ticks 409, 429, 449) and the seats
survive - which is the claim.

**Attack 3 - the rule was made stricter than the game.** The first draft of the ignored fixture demanded 0.400 apart on every
tick, which the game does not: the original lets an encounter reach contact and answers it with a pause, never a position write
(`docs/movement-in-the-binary.md`; `collision::a_walker_never_ends_a_tick_inside_a_friend` already pins the pair's version).
That stricter bar also contradicted an existing green fixture (`a_crowd_sent_to_one_place_does_not_stack`, three soldiers), so
the bar is now "never deeper than first contact, and back outside the standoff once settled".

**Attack 4 - the separation pass is inert or wrong for mates.** It was exempting mates and still firing for everyone else, which
turned the gather ring's own approaches into pauses: 1,401 repaths. Deleting the pass and narrowing only the exemption gives the
same 1,401 from a different direction - the two mechanisms mask each other, which is exactly why the acceptance run, not a test
count, is the instrument here.

**Not attacked, and why.** No attempt was made to *tune* `STALLED_TICKS` or the seat spacing to buy the narrowing cheaply: the
numbers say the mechanism is missing (a route round a body), and a tuned constant would hide that behind a threshold nobody can
justify from the data.
