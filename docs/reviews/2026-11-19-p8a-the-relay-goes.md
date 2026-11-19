# Review: the re-lay goes, and the witness that would not bite (P8a)

*The owner: **"I want no deliberate differences to original game. If there is any, I want that adjusted to meet the original or
cut off."** P8 of `docs/parity-plan.md` is his own instruction - *"cut the book entirely and let collisions and pauses do the
spreading."* This slice is the first, smallest piece of it: the re-lay of a node's ring (`maybe_respread`, D200/D202).*

## What was cut

`maybe_respread` and everything it needed: the `respreads` bucket on the tick's work (`step.rs`, both `WorkQueue` and
`TurnWork`, plus `bucket!`/`keep!`), the `work.respreads.push(node)` at the gather arm's `NoRoom` branch, and
`World::respread_at`. The rule it implemented was **ours**: when a node refused a man, the node's ring was re-laid at the packed
spacing, at most once per twenty ticks, and every man who had not started yet was handed a new place. The original has no
equivalent: it picks a man's place **once**, when the order is given, and after that his body is solid and nobody re-seats him.

## The witness, and the honest result

`crates/sim/tests/working_men_never_move.rs` was written first, and it is the fixture P8 must be judged by: a man who is
**working** (`Gather` with `progress > 0`, D246's own definition of a seat that may not move) and has been **standing still for
twenty ticks** must not move a single raw unit again.

**It passes both with the cut and without it** - and that is the finding, not a failure of the slice:

- the first version of the witness went red (**54 moves, worst 800 raw - one step**) and the numbers looked like the re-lay. They
  were not: every mover had `progress 0`, i.e. a man still walking to his place, and his own step is not the engine's doing.
- with `progress > 0` the scene is clean **before** the cut, because **D246 already froze working seats**: `maybe_respread`
  deliberately skips any man with progress, precisely so a newcomer cannot displace him. The re-lay could only ever move men who
  had not started - and that is what it did, by re-pointing their routes every twenty ticks.
- fourteen men on one tree still does not reach it: `standoff_point` falls back from the comfortable spacing to the packed one
  (`lib.rs:3833`), so `Standoff::NoRoom` - the re-lay's only trigger - is not reached in any scene this suite can build.

So this slice is **a deletion of unreachable machinery, measured as a no-behaviour-change refactor**, and it is worth its own
commit for exactly that reason: the next one (the place registry and the angular ring, `ring_point`/`GATHER_PLACES`) *does* change
behaviour, and it should not be tangled with a deletion that cannot be watched.

## Performance

Nothing measurable on the tick, and that is the honest result rather than a claim:

- the cut removes a loop that only ever ran when a node refused a man **and** had no free place left, at most once per twenty
  ticks per node;
- `working_men_never_move::a_working_man_is_never_moved` - fourteen men, 1,600 ticks, the scene the loop existed for - runs in
  **1.65 s with the cut against 1.85 s without it**;
- the full suite is **888 passed / 18 failed / 21 ignored** at both commits, and the red set is the same eighteen names
  (`/opt/data/cache/scratch/after-p1.txt` against `/opt/data/cache/scratch/after-p8a.txt`), so no fixture changed status either
  way.

## Stability

The fixture that stays is the value here: `a_working_man_is_never_moved` now holds the line for the rest of P8. Any later cut -
the registry, the ring, the settle - that starts shuffling working men goes red on it, with the man's id, the tick and the raw
distance in the message.

**Not done, named:** the place registry (`gather_place`, `reserve_gather_place`, `gather_place_of`, `place_is_live`) and the
angular ring (`ring_point`, `GATHER_PLACES`, `GATHER_RING_MILLI`) still stand, and they are where P8 actually changes what a
player sees. They need their own witness first - a newcomer arriving at a crowded node must not move anybody, and a man's place
must be a point where his body touches the node, not a point on a 1.156-tile circle.
