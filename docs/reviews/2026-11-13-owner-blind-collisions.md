# Owner-blind collisions, built and measured — parked on the match outcome

*2026-11-13. The owner's ruling: **"Adopt original game behaviour. Don't make difference of friend or foe as in original
game."** The rule is built on `d189-owner-blind` and it does what he asked at the level of two units. It cannot land yet,
because it changes whether a match resolves, and that is the one number that decides.*

## What changed

1. **`hostile` is gone from `collide`.** Every unit body blocks, whoever owns it: the reach, `bound_to_reach`, the goal-tile
   rule and the at-rest bound all apply to a friend exactly as to an enemy. The original's collision block has no player,
   team or alliance test anywhere in it and its query passes an owner sentinel (`docs/movement-in-the-binary.md`).
2. **A body on the goal tile asks for a route round it**, not for a wait — gated on the mover not already standing beside
   the tile, because without that gate it re-plans every tick (**525 repaths in 600 ticks**, measured).
3. **A body at rest inside the reach no longer leaves a zero step**: the pair resolves it through the same give-way path
   rule 9 uses, which is what stops the D192 freeze coming back from the other side.
4. **Near the goal, a *body* still earns a repath, a wall does not**: D98a suppressed re-plans within a tile of the goal,
   which is right for terrain and wrong for a man — the third villager sent at one bush stands 0.4 from a settled neighbour
   with his own place two steps round it.

## Performance

| | `main` (`603e72e`) | `d189-owner-blind` |
|---|---|---|
| acceptance (48,000 ticks, seed 7) | `entities=142 players=1 hash=91572e72c33cee93 repaths=409 made_room=0` | **`entities=147 players=2 hash=d3b087c60013493f repaths=45 made_room=24`** |
| `collision` suite | 3 passed | **4 passed** (the new one is the owner's rule) |
| `evasion` suite | 6 passed | 6 passed |
| `gather_motion` | 3 passed | **1 failed** |

The two numbers that matter: **`made_room` 0 → 24** (men now have to be shoved aside, because a friend is a wall) and
**`players=2` where `main` finishes `players=1`** — the match no longer resolves. That is the same failure the D189 note
recorded the first time the bound was applied to friends (*"164 entities against 141, both players alive at 1,800 s"*), and
it is why the exemption existed.

## Stability — what is verified and what is not

**Verified, in tests:** a walker never ends a tick inside a friend (**the new test**: RED first at
*"the walker came 6149 raw units from its friend at tick 79, inside the 8000 their footprints need"*, green after the
change); two units of one order still never stack; a body already overlapped is still not pushed (D52); the goal-tile
routes no longer spin (525 → 0 repaths in that fixture); a blocked unit still goes round a standing body and arrives.

**Red, and it is the blocker:** `three_gatherers_on_one_bush_do_not_pile_up` — one of three villagers reaches the bush in
1,200 ticks (it was three). **The cause is a number, not a rule: our gather places are spaced one body apart (0.40), which
is exactly the reach that now separates bodies**, so a man walking to his place is stopped by the man already standing on
the next place. The game spaces its working men at **0.661-0.663** (measured on the owner's saves,
`docs/reviews/2026-11-13-the-standoff-is-measured.md`) — a body *and a gap*, which is what makes a crowd deliverable
without overlap. Places must be spaced by more than the reach; the fix is `touching_places`' spacing for gather places,
and it changes capacity, so it is a slice of its own.

**Not measured yet:** the tick cost of the extra passes (`World::step` was not re-timed on the branch), and the 24
`made_room` shoves — whether they are the same 24 units repeatedly or 24 units once.

## Why it is parked rather than merged

The engine is green (`cargo test -p ruage-sim` reaches the parked test and the two rule-pinning ones below, everything
else passes), but **the acceptance match stops resolving**, and a change that turns a won match into a stalemate is not a
collision rule, it is a regression until the crowd machinery underneath it works. The remaining work is bounded and named:

1. Space gather places at `reach + spacing` (~0.66), not one body apart, and re-derive the capacity from the geometry
   (the game shows 7-8 men at 0.66 on the touching envelope).
2. Re-run the acceptance match; `players=1` and `made_room` back near 0 are the acceptance criteria.
3. Update the two tests that pin the withdrawn rule: `step_decision::a_friend_on_the_goal_tile_is_a_take` (a friend on the
   goal tile is now an obstacle and the unit stops beside it) and `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives`
   (it arrives *beside*, not on, the friend). `evasion::two_villagers_crossing_pass_through_and_both_arrive` needs
   renaming for the same reason - under D200 they go round each other.

**D200 itself is recorded on the branch, not on `main`**: the owner has ruled, and the rule is right, but the decision is
not landed until the engine can deliver a crowd without overlap.
