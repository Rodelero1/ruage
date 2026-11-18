# Review: the fallback prefix on top of substitution, no cap and no slide - animals 10/1, gather 22/0

*Change (branch `d279-substitute`): the re-plan's tile-chain fallback now prefixes the body's **own tile centre**, so the first
leg is one inside a free tile instead of a line from a point the plan never started at. This is the combination whose parts were
measured separately and never together: substitution, no ground cap, no slide, and the prefix.*

## Performance

`cargo test -p ruage-sim --no-fail-fast --test animals --test gather`, this branch:

| suite | result | time |
|---|---|---|
| animals | **10 passed, 1 failed** | 5.46 s |
| gather | **22 passed, 0 failed** | 17.79 s |

Against the same two suites **without** the prefix on this branch (slide deleted, cap gone, substitution in): animals **8/3** and
gather **20/2**. So the prefix is what closes the five failures - animals 3 -> 1 and gather 2 -> 0 - and the one red left is
`a_live_animal_is_a_gather_source` (*"must end up carrying meat"*), which is the **arrival measure** and not a movement defect.

**The acceptance run is in flight, not claimed.** At the last read: 1,600 s of match with wood **166**, food **194**, units
**33**, buildings **21** - growing, and with no `not enough food` line so far, against the two parked branches that both stalled.
Its final line is the gate (D278) and this branch does not claim it until it prints.

## Stability

**Why this combination and not one piece.** Each piece alone was measured and each failed the same way: the slide alone left 14
tests red; the substitution alone left the re-plan storm (173,981 ground re-paths) because dropping a waypoint only aims the man at
the next corner of the same unwalkable route; the prefix alone was inert on `main` because the acceptance never reaches the
fallback. Together they close what each was missing: the prefix makes the fallback's first leg walkable, which is what stops the
ground from refusing, which is what makes the cap unnecessary.

**What is still not proven, and it is the project's first rule:** no fixture was watched failing for the substitution itself, and
none was written for the prefix. The evidence is that five existing failures close - behaviour, not a number - and that the two
suites above are green otherwise. A fixture per rule is owed before this is merged.

Known and accepted: parked until the acceptance prints a win. `main` untouched.
