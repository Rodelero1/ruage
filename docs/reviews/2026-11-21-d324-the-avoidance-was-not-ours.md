# Review: the avoidance is not an invention after all - the removal is measured and reverted (D324)

*Change: **none**. The experiment removed `standing_bodies` from planning, measured the result, and the measurement refuted the premise that
prompted it. The tree is the `90fe436` state plus documentation.*

## What was tried

D323 left a hypothesis open: our planner **adds** standing bodies to the search as `Clear` obstacles, and the binary's list is an *ignore*
list that makes bodies **passable** - so the avoidance might be ours. The variant: `standing_bodies` neutered, every planning call given an
empty clear list. Built, measured, suite run, and then read against the binary again.

## The measurement

| | with avoidance (`90fe436`) | without |
|---|---|---|
| acceptance entities | 141 | 140 |
| acceptance re-paths | 7,125 | **8,861** (+24%) |
| acceptance wall, 48,000 ticks | 12.6 / 12.7 s | **8.4 / 8.5 s** (-33%) |
| acceptance hash | `c7191f531bd81b1b` | `d818560cf7bfe824` |
| full suite | 930 / 0 / 19 | **928 / 2 / 19** |

The two reds:

- `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` - *"the walker came 4619 raw units from the body, standoff 8000"*. The
  walker brushes **inside** the body instead of going round it.
- `build::a_content_pack_with_our_own_resources_loads_and_spends` - *"an own-content house must be built to full hp: left None, right
  Some(550)"*. A builder never finished: the walk that used to be planned round a body now cuts through it and the site is never worked to
  completion inside the fixture's window.

## Why the premise was wrong

`FUN_00801a20(x, y, ids, count)` returns 1 - **passable** - when **every** body on the tile is in the path-ignore list, and 0 otherwise.
The list is built in `FUN_008007f0` from self, the order's target, and the route's objects **whose `isMoving` answers 1** (D284 read that
gate and D284 was the change that added it). So:

- a tile held by a **moving** group member is free ground - the route is planned **through** him; and
- a tile held by a **stopped** body is **blocked** - the route is planned **round** him.

That is exactly what `standing_bodies` does, in the opposite inventory: it subtracts the mover and the moving friends (they "will not be in
the way", D310) and hands the rest to the search as obstacles. The binary does not carry an "avoid" array because it does not need one - its
passability function is **fail-closed**: anything not explicitly ignored blocks. Our API needs the list because our `fits_for` is
fail-open on bodies.

**So the avoidance is faithful to the flat search, and the removal is a regression**: it takes a clearance away from a walking body and
costs a builder his house. Reverted from the backup; `git diff` shows only `docs/decisions.md`.

## Performance, honestly

The variant is **33% faster** on the acceptance match and that number is real - it is what the game's *hierarchy* would do, since the levelled
search does not consult `FUN_00801a20` at all. It is also **+24% re-paths**, because bodies that are not planned round are collided with and
each collision buys a re-plan. A faster wrong answer is still wrong, and the number is recorded so it is not rediscovered as a tempting
optimisation.

## Stability

- The variant was built and run from a backup of `lib.rs`; restored, and the file is byte-identical to `90fe436` (`git diff` empty for it).
- Nothing else was touched; the two reds are the variant's, not the tree's, and the tree's suite stands at **930 / 0 / 19**.

## What this settles

- Our `standing_bodies` stays, and its doc comment now has the binary's authority behind it: **a stopped body blocks a plan, a moving one
  does not**, from `FUN_00801a20` + the `isMoving` gate.
- The next real question in this area is **not** "do we avoid bodies" but "does the levelled search's blindness to bodies matter" - and on
  this map it cannot, because the tier never answers (D323).
