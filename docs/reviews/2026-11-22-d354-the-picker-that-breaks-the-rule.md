# The owner's rule was already in the engine — the picker that breaks it is the one handing out work

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Read-only slice** (no engine change landed)

**Owner:** *"Villager should return to former pile and look for another pile. If found, work on it. If none found, idle. Iirc
I already made such decision."*

## What was established

1. **He did decide it.** D150.3, 2026-11-07, his words verbatim. The engine implements it: the depletion picks and the
   `pending_gather` carry-over choose the next source of the same kind, and the rule ("if one is found, work it; if none,
   idle") is what `nearest_like` answers.
2. **The acceptance scene's gold stall is somewhere else.** Measured with `--dump` at tick 48,000: gold frozen at **10** from
   ~1,200 s to 2,400 s with **1,011 gold** left in four tiles at `(24.5,11.5)` — **9.6 tiles** from the town centre against a
   reach of **9** — and `player 1 has no gold node to gather` firing **220** times. Wood and food keep flowing throughout, so
   it is gold alone, and it is the **assignment** picker: `crates/script/src/lib.rs:1431` measures the search **from the
   worker**, who is standing at the drop site. The script's other site (`:455`) passes `None`, where the script names a
   resource and that is an order (D149.5).

## The fixture, and why it was not kept

`spent_pile.rs` (20x6 map; one gold tile of **1** at `(12,2)`, a patch of **400** at `(15,2)` — three tiles from the spent
pile, ten from the drop site):

- **On the shipped code:** he banks **1 gold** and stands `Idle` at `(5.2, 1.1)`. That is the owner's rule not being honoured
  by the path the *engine* uses when the load is banked.
- **With `next_source_anchor`** (the node he left, falling back to his own position, at the four depletion sites): green.
- **Rejected anyway** — see Performance.

Preserved at `/opt/data/cache/scratch/age2hd-pending-fixtures/spent_pile.rs`, and the suite is green because it is not in the
tree (D8's convention for a fixture whose change does not land).

## Performance

| | wall (3 runs) | median | entities | re-paths | wood at 2,400 s |
|---|---|---|---|---|---|
| shipped (`ba0aba8`) | 10.10 / 10.05 / 10.35 s | **10.10 s** | 124 | 12,847 | 1,990 |
| with the node anchor | 19.20 / 18.98 / 18.93 s | **19.20 s** | 145 | 3,607 | **68** |

**+90% wall time**, and the signature says why it is not worth chasing as a slow-down: re-paths *fell* three-fold while wood
**froze at 68** from 1,200 s. Fewer paths and more time is a pick answering with a node that `start_gather` then refuses, asked
again every tick. Not diagnosed tonight, so not shipped.

## Stability

- Suite **945 passed, 0 failed, 25 ignored, EXIT=0** on the shipped tree after the revert (two full runs).
- The reverted change caused **no panic and no hang** — the acceptance scene completed all 48,000 ticks and resolved to a draw
  both with and without it; the regression is economic (wood frozen) and throughput, which is why it was measured before
  deciding rather than judged by the gold curve alone.
- **My own errors, in the order they happened:** a first fixture whose "second patch" was the *same* patch (adjacent
  same-kind tiles are one node, `resource_tiles`); a second that spawned resources with `unit: None` and therefore could not be
  worked at all (0 gold in 3,000 ticks); a mutation of `pending_gather` that did not move the fixture, which is what showed the
  isolation was wrong rather than the mechanism. Each was caught by a measurement, not by reasoning.
