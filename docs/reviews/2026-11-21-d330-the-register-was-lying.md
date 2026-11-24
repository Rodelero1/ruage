# Review: the register was lying in both directions, and the owner's rule is now "blunt copy" (D330)

*Change: `docs/parity-plan.md` (register rewritten after checking every row against the code) and one stale comment in
`crates/sim/src/world/motion.rs`. No behaviour. The owner's instruction that prompted it: **"Please remove all deviations. I want this to be
blunt copy."***

## What the check found

Every register row was verified **by symbol** against `crates/sim/src`, and five had drifted in both directions:

- **Done but still open**: **P1** and **P2** (the budget and the octile tie-break landed 2026-11-19, conformance green); **P5** (neither
  `finish_blocked_orders` nor `beside` exists any more); **P8** (no seat book - D293/D294); the levelled half of **P11**.
- **Open but written as settled, or settled but written as open**:
  - **P3**: `motion.rs` still carried *"PENDING A RULING ... Until he says which wins, the step keeps the strict-overlap test it had"*
    **after D306 had ruled**. Worse, the register's own ruling note said *"a half-tile body stops at a one-tile gap"*, which is the
    opposite of what D306 records the owner measuring (a packed trebuchet threading a one-tile hole).
  - **P4**: marked *"BLOCKED on P8"*; the seat book was cut, so it is **unblocked** and untouched.
- **Unchanged and live**: **P6** (`Ground.occupants: Vec<u16>` is still a per-tile count), **P7** (`standing_bodies` still skips bodies
  within a quarter tile), **P9** (both fork rules still in the engine), **P10** (kept, deliberately), **P12** (unread).

## What the register now says

A status column per row (**DONE / LIVE / LIVE, unblocked / LIVE - forks forfeited / HALF / UNREAD / KEEP**), the step each live row needs,
and the two classes of blocker stated in the row rather than in a note at the bottom: the **100/600 cost rows** need a binary this box does
not have (D327), and **P12** plus **P11's polygon pathfinder** need reads the corpus may not hold.

**P9 is now a live row, not a fork.** It encodes two of the owner's own earlier rulings - arrival "beside a held goal" (D200) and the crowd
ring point (D271). His directive of 2026-11-21 overrides them, and the row says so in its own words rather than leaving a future reader to
infer it.

The `motion.rs` comment now carries the ruling instead of the question, and adds the constraint the ruling puts on P3's implementation: if
touching collides **and** a one-tile gap is passable (D306, measured), then the box that test grows is **not** the body's nominal radius.

## Performance

- **No behaviour**: the acceptance match is unchanged (`c7191f531bd81b1b`, 141 entities, 7,125 repaths, 12.9 s on the last run) and the
  suite is the **931 / 0 / 19** it was on `4547572`. The only compiled change is a comment.
- The check itself cost one pass of `grep` per symbol - cheaper than the alternative, which is what happened: a register that sent the next
  session to re-cut P5 and re-land P1/P2.

## Stability

- Nothing executable changed, so nothing can regress; the risk this removes is *misreading*, which is the failure mode this project has paid
  for repeatedly (a stale `#[ignore]`, a stale `PENDING A RULING`, a fixture measuring nothing).
- **The rule it establishes**: a register row is verified by symbol before it is used as a spec. Three rows had been quoted in
  `docs/next-step.md` as work to do, two of which were already done.
