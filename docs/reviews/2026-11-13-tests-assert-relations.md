# Tests assert relations: the rule, and the docstrings that still lied

The architecture review's item 3, and a smaller thing it turned up: five test files whose **docstrings** still described the 1.156 ring as
the current rule, three weeks after it stopped being one.

## Performance - measured

| Metric | Value |
|---|---|
| the five touched binaries | 1.63 s, 1.65 s, 2.27 s, 0.77 s, 2.64 s - **9.0 s total**, unchanged |
| tests red when the standoff moved (D192) | **11 at once**, every one asserting `1.156` rather than the rule |
| assertions converted | **0** - they were already relations after D192; what was wrong was the prose |
| comments corrected | **5** files |

Nothing in the engine changed in this slice; the numbers are the suite's, quoted so the claim "no behaviour moved" is checkable.

## The rule, in SPEC §11

`### Tests assert relations, not measurements — D193`, with the rule and the counter-example side by side:

```rust
// yes: the rule
assert!(distance_to_tile(worker, tile) <= radius, "MEASURED {distance:.3} from the border; his circle is {radius}");
// no: last week's number
assert!((distance_to_tile(worker, tile) - 0.70).abs() < 0.02);
```

and the evidence: eleven gather tests went red when the standoff moved from the ring to the border, all of them asserting the
measurement. A measurement stays in a test when the measurement **is** the claim (a save-game fixture saying what the owner's file
holds); it goes when the claim is what the engine does.

## The stale docstrings

- `gather_ring.rs` opened with *"A gatherer stands on a ring 1.156 tiles out, not 0.70"* - the **opposite** of the rule since D192.
  It now states the rule (the circle on the tile), keeps the save's six radii as the history they are, and says which decision owns
  which number.
- `gather_continuous.rs` described the place as a point on the 1.156 ring; it is a point on the tile's border.
- `gather_standoff.rs` announced *"The villager ends up on the measured ring"* above an assertion that checks the border.
- `felling.rs` budgeted two seconds *"walking out to his ring"*; he now walks out to the tile.
- `pathing.rs` explained its 1.3-tile arrival bound as *"the ring's reach"*. It is now named for what it is: **a coarse bound**, which
  admits both rules, so it does not have to move when the rule does.

A docstring that describes a rule the assertion no longer checks is worse than no docstring: it is the sentence the next session
believes.

## Stability

- **The five binaries pass** (3 + 2 + 4 + 1 + 5 tests), `fmt` and `clippy -D warnings` clean; the change is comments plus one SPEC
  section, so nothing in the tick could move.
- **No assertion was loosened.** The only assertion touched (`pathing.rs`'s 1.3-tile bound) kept its value; its comment changed to say
  what it does and does not claim.
- **The rule is now checkable in review**: a diff that adds `assert!((x - <literal>).abs() < ...)` next to a measured constant is the
  pattern SPEC §11 forbids, and the review gate already asks for the review note that would have to justify it.
