# Review: the 255 cap is unobservable in any scene the game makes (D329)

*Change: **none**. The binary's cap on the body-avoidance list was measured instead of argued: same bodies, same route, with and without a
255-entry truncation. It closes the last parked item in the queue with a number.*

## Why this was still open

D310 read the binary's ignore list (`local_404[255]`, filling stops at `0xfe`) and our `standing_bodies` builds **no** cap and iterates in **id
order**. D310 declined to add the cap for two reasons: our selection order would be ours (the binary fills colliders, then the route's own
array, then the group), and "no witness can see it". The second reason was an argument, so it was testable.

## The experiment

A 64x64 open map, a wall of villagers in column 32 (every row, layered when the count exceeds 60 rows), a walker at (20, 30) ordered to
(40, 30) through the wall. The route was compared by a hash of its points, **with the current code and with a temporary 255-entry truncation
inside `standing_bodies`**:

| bodies | route points | route hash | with the cap |
|---|---|---|---|
| 0 | 1 | `1253b82b7464e5ed` | identical |
| 100 | 2 | `a4eec65f7425dbd5` | identical |
| 200 | 2 | `a4eec65f7425dbd5` | identical |
| 255 | 2 | `a4eec65f7425dbd5` | identical |
| 256 | 2 | `a4eec65f7425dbd5` | identical |
| 300 | 3 | `7947ee1a076dff5d` | identical |
| 400 | 3 | `7947ee1a076dff5d` | identical |

**Not one route changed.** The point count moves at 300 because the wall is physically wider there (five layers of bodies, not because of the
list), and 300 and 400 agree with each other.

## What that means, precisely

The cap can only change an answer when the body that decides the route is the one that falls past entry 255. To build that scene you would
have to place 255 bodies *within the span of the line* and then put the one body that matters in the gap, with a **higher id** than all of
them - an arrangement no player can produce, and one the acceptance match does not contain (its largest list is 17 entries, measured in
D323).

So the cap stays unimplemented, and now for a measured reason: **it is unobservable in scenes the game makes**. What remains unfaithful is
only the *selection order* (id order against the binary's colliders-first), which matters exactly as much as the cap does - not at all until
a list reaches 255.

## Performance

- No production change: acceptance `c7191f531bd81b1b` (141 entities, 7,125 repaths, 12.9 s), suite **931 / 0 / 19**, conformance **21 / 0 / 0**
  - all from the commit this note follows, unchanged.
- The probe itself was cheap: a 64x64 scene, seven body counts, two builds, about four minutes. The experiment that D310 deferred "because no
  witness can see it" cost less than the paragraph that deferred it.

## Stability

- The truncation was applied to `lib.rs`, run, and reverted from a backup; `git status` after the revert is **empty**, so the tree carries no
  trace of the variant.
- **The lesson recorded**: "no witness can see it" is a hypothesis like any other and costs one probe to check. D310's *first* reason - that
  our selection order would be ours, not the game's - was the stronger one and survives; the second should have been measured when it was
  written rather than repeated for three days.
