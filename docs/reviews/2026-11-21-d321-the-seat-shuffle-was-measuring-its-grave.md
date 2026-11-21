# Review: the seat shuffle was measuring its own grave (D321)

*Change: `overlap::a_passer_by_does_not_move_the_seated_gatherers` comes off `#[ignore]` - **and it passes**. Test-only: no engine code
changed. The fixture's scene is rebuilt so its three gatherers are actually at work (it had been failing its own precondition and measuring
nothing), and its doc no longer describes a place book the engine deleted in D293.*

## What was found

D238 diagnosed the owner's *"villager still pushes another villager"* as the **seat shuffle**: a working man giving up his square when a
neighbour passed within reach of it. It was called **load-bearing**, because removing it let villagers stand **0.204** tiles apart - a walker
still walked through a seated man's square, so the shuffle was the only thing keeping bodies apart.

Both halves of that have since been deleted or completed, and nobody re-checked the fixture:

- the **place book is gone** (D293, the owner: *"Drop it and let the crowds jam"*) - there is no square to give up;
- the walker's **past** rule completes its chain (D320) - a collider who has stood still since the route was re-planned is walked through,
  and the pause stops the pair meeting again.

Measured, with the scene repaired: three men **on the node's ring**, one of them taking (`progress > 0`), a fourth walking through the ring
for 300 ticks - **no working man moves**, `walking_ticks > 10` asserted so a passer-by who never crossed cannot pass the test either.

## Why the fixture said nothing before

It spawned its three gatherers in a row **three tiles apart** and stepped 500 ticks. On today's engine they were still walking in at the
snapshot, so the fixture failed on `assert!(!seated.is_empty(), "no gatherer was at work beside the node before the passer-by; the fixture
measures nothing")`. That is the second fixture this work has found in that state (the first was D315's skirmish clock): a scene that drifted
out from under its own assertion, parked on `#[ignore]`, reading as "we know about this".

## Test-first

Not a new witness, and it is not claimed as one: this is a **guard** that was already written and is only now honest about what it measures.
The rule it guards - a man taking from a node keeps the point he stands on while someone walks past - is the owner's own invariant 4, which
`invariants::a_villager_who_is_gathering_does_not_move` also pins from the other side (a man's own settle must not move him). Two fixtures,
two causes, both green.

**It can fail**: it fails if anything writes a working man's position while a body passes, which is exactly the shape of D238's `SEATMOVE`
line. It cannot fail *for the reason D238 documented*, because that mechanism is gone - which is the finding, not a weakness.

## Performance

- **Test-only**: the acceptance match is byte-for-byte the D320 result (`c7191f531bd81b1b`, 141 entities, 7,125 repaths, 13.0 s) because no
  engine code changed. Nothing per-tick was added.
- Full workspace suite: **930 passed / 0 failed / 19 ignored** against **929 / 0 / 20** - one more pass, one fewer ignore, nothing moved in
  the other direction.
- The two fixtures that would show a pile if the shuffle's absence hurt - `invariants::villagers_never_close_into_one_body` and
  `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` - are green, which is the answer to D238's 0.204 measurement.

## Stability

- Nothing in `crates/sim/src` changed, so this cannot regress behaviour; its risk is interpretive - calling a gap closed on a scene that no
  longer reproduces the original complaint. The scene is therefore stated in the doc: three men **on the ring**, one taking, one walker
  crossing, positions compared on every tick the walker moves.
- **What is not re-measured**: D238's original scene (three men scattered three tiles apart). It is in the git history and it produced the
  `SEATMOVE` line; the mechanism it caught no longer exists, so re-running it would measure nothing either.

Known and accepted: `head_on::a_stopped_friend_is_not_walked_through` still exists as `replan::a_stopped_friend_is_only_walked_through_after_
the_replan` (D320) - the pair was renamed, not duplicated.
