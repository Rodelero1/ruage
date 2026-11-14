# OPEN - D207 is on `main`: a group walks over itself. Next: the explicit group id (register), and D205's pause on top of it

**State, 2026-11-14.** **D207** is built and green: units ordered in the **same tick** are mates, a mate is not solid (the
step is taken, no reach, no wait, no repath), and the exemption is mutual - a man whose order is cancelled is solid again in
the same tick. Review with the numbers: `docs/reviews/2026-11-14-d207-group-overlap.md`.

**What is deferred, deliberately.** The owner's option 2 - an explicit group id on the command - is the register row. Until it
lands, "same tick" is not "same selection": a caller issuing eight separate moves in one tick gets group behaviour it did not
ask for, and a selection ordered across several ticks gets none. When it lands, `mates` is filled from the command instead of
from `ordered_ticks`, and the three tests in `crates/sim/tests/group_overlap.rs` keep their names.

**The parked branch.** `d205-pause` (`e251ca2`) holds D205 - the 0.25-1.50 s pause for the higher id, correct and measured -
plus three real fixes that the binary read now says were the wrong shape: the re-plan learnt about bodies (`short_range_around`),
the "past" exemption learnt to expire when a body moves, and `short_range`'s fast path stopped answering "walk through him".
Those three are **fidelity bugs in their own right** and worth re-landing on their own, without D205's pause, once the pause has
something to sit on. Not merged.

**The pause's missing half.** With D207 landed, a non-mate collision still has no run-time answer in our engine: the original
pauses the loser (D205, parked) and re-plans. The order that follows is: land D207's mechanism (done), then re-land the three
search fixes alone, then the pause on top - each with its own red test, because each of the last three attempts failed by
carrying the others.

**Open register rows.** (1) explicit group id on the command; (2) the three search fixes above, extracted from `d205-pause`;
(3) D205's pause, after those.
