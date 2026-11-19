# The game's start rule is in; our one-tile ring is out; the crowd fixture is red and that is the measurement

*Owner, on the previous commit: **"Follow the game, do not invent own stuff."** So the invention went and the reading
went in. It does not make `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground` pass - and the failure now says
something specific instead of hiding behind a wider ring.*

## What is in the tree now, and its source

1. **A start standing inside an obstruction is re-seeded `2 x radius` outward, and only that far** (`FUN_007fb220`): eight
   candidate points at exactly `2 x radius`, first free one wins, the route prefixed with the body's own point so it walks
   out rather than starting where it is not.
2. **A body with somewhere to go is not a stander** - the original's `isMoving` (velocity non-zero). The recency test we
   had (`last_moved + 1 >= tick`) misses the case that matters: a crowd ordered in one tick has not moved yet, so all
   twelve read as standers.
3. **Our one-tile stander exemption is deleted** - it was 2.5x the game's reach and a different mechanism.

## Stability: what these two rules do and do not buy

- `movement` 4/4, `evasion` 6/6, `gather` **22/22**, `gather_motion` 6/6, `collision` 3/3, `group_overlap` 4/4,
  `no_pushing` 2/2 - unchanged and green.
- **`army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground` is RED**, `NoPath { from: (6, 20), to: (19, 19) }`.
  With the game's rules alone, twelve men ordered through one gap still cannot leave their own tiles - and the reason is
  **not** the re-seed being too small, it is *when we plan*: this engine plans each order **as it is submitted**, so the
  first man is planned while the other eleven have no route and are genuinely standers. The original plans when the order
  is executed, with the group already formed, which is why its ignore list can hold the whole group re-tested on
  `isMoving`.

## Performance

The eight suites above ran in **32.7 s** in total (2.0 + 2.0 + 3.0 + 20.9 + 2.8 + 2.0 + 1.8 + 1.2) with these rules in
place, against **31.9 s** for the same eight immediately before them - a difference of **0.8 s**, i.e. none. That is the
whole of the cost claim: these two rules change *candidate starts* only and the stander set **shrinks**, so no search gets
bigger. The acceptance fixture's own timing is **not** quoted here, because the host is shared with another live session
and a paired A/B is owed (`docs/reviews/2026-11-19-the-start-is-re-seeded-outward.md`).

## The next slice, named

**Form the group before planning, or plan the order at execution.** Two shapes, both the game's: (a) a tick's orders are
collected, the group is written onto every member's route, and only then each is planned - which is what makes the first
man see the other eleven as `isMoving`; or (b) the order is submitted without a route and planned in `move_bodies` at the
first step, where the group already exists. (a) is smaller and testable against this fixture; (b) matches the original's
order of operations more closely but moves planning into the tick.
