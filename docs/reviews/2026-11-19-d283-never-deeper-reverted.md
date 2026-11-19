# Review: "never step deeper" bounds the overlap and freezes the crowd - reverted (D283)

*The owner: **"go"** on the next item of the queue, "two bodies on one tile". The rule tried is the one the game's own
behaviour implies - it takes **one** step into a collider and answers the collision with a re-plan and a pause, and its
epilogue writes no position, so an overlap happens but is never *driven* deeper. It works, and it is not landable: it freezes
a crowd.*

## Performance

- The acceptance fixture ran **26.26 s** against **39.58 s** - because it **failed early**, `the crew did not grow between
  400 s and 800 s: 7 then 7`. That is not a performance win; it is the match stalling. Recorded here so the number is not
  mistaken for one.
- No plan or search cost was added: the rule is one squared-distance comparison per collider, in the step pass.

## Stability

**What the rule did, measured:** two men meeting head-on came **4,478 raw** inside an 8,000 raw reach before it and **7,728
raw** with it - 8,000 minus the **272 raw** contact step this file's own fixtures record. So it bounds the overlap at exactly
the contact, which is the game's behaviour, and `a_mate_who_is_stopped_becomes_solid`'s second assertion
(*"nothing inside the reach on any tick after the meeting"*) is **arithmetically unsatisfiable** - 8,000 - 272 = 7,728 - and was
restated to *"never deeper than the meeting"*. That restatement is worth keeping when the rule lands.

**Why it is reverted:** in a crowd a body stands inside **several** clearances at once, and a hard "never deeper" is refused
by every one of them at the same time, so the man has no legal step. The acceptance match says so in one line: the crew stopped
growing (**7 then 7**). A body inside three neighbours needs a rule with a *net* answer - deeper into one, further from
another - or the game's own radius query, not a per-collider veto.

**What was attacked, in order:** (1) hard refusal per collider - froze the crowd; (2) the fixture work that goes with it - its
second assertion is unsatisfiable and was restated; (3) the acceptance fixture as the witness - it is the thing that caught
the freeze, and it caught it in 26 seconds rather than after a suite run.

**Where the queue stands:** item 1 is unchanged except that it now carries this refutation, both halves measured. Nothing else
in the slice was touched, and the tree is exactly the D282 commit (`24715b8`) plus this note.
