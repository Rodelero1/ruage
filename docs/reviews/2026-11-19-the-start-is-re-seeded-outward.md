# Review: an order the original accepts is not refused - the start is re-seeded outward (D277 addendum)

*The only real red on `main` (`b2accfe`) was `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`:
`NoPath { from: (6, 20), to: (19, 19) }` raised on **submitting** an attack-move - twelve militia ordered through one gap in
a wood.*

## Judged against the original first

- **The order layer never refuses a click.** The deck's own line (p. 96, D134) is that a walk or attack-move is
  *"every click is accepted"*, and the search's answer when the goal cannot be reached is **the closest point**, not
  `None`. The engine already has that mode (`plan_path_closest`) and the attack-move uses it.
- **So the failure was one layer lower**: `find_path_avoiding` returned `None` even in closest mode. The cause is the
  **standers** it is given - every stopping unit's tile is inserted into the search - and twelve soldiers ordered through
  one gap stand **shoulder to shoulder**, so all eight neighbours of the start were stander tiles. A search that cannot
  leave its own tile answers nothing.
- **The original has this case and its answer is named**: a mover that stands inside an obstruction is **re-seeded
  `2 x radius` outward, and only that far** (`FUN_007fb220`, the register's *start* row). Not a wider clearing, and not a
  refusal.

## What changed

`stander_tiles` skips any stander tile within **one tile** of the start - the search-free form of the game's re-seed,
which is `2 x radius` = 0.4 tiles for a 0.2-tile body, i.e. the eight neighbours and no further. The two ends were already
exempt; this extends the exemption to the ring the game re-seeds through, and nothing else.

## Performance

**Not attributable, and stated as such.** `the_acceptance_match_trains_more_villagers` read **58.84 s** on this run against
the **26.6 s** recorded at `a4331de`. That gap is **not** evidence about this patch: the host is a 2-CPU / 4 GiB cgroup
**shared with another live session** (a `cargo test --workspace` and a release simulation were running on it during these
measurements), and the 26.6 s figure was taken on a quiet box. A paired A/B on the same host at the same moment is owed
before any cost is claimed - and with the contention it will still be noisy.

## Stability

- `army_walk` **2/2** (the crowd walks 3,000 ticks and never stands on blocked ground), and the movement/gather families
  that the stander list also feeds: `movement` 4/4, `evasion` 6/6, `gather` **22/22**, `gather_motion` 6/6, `collision`
  3/3, `group_overlap` 4/4, `no_pushing` 2/2 - **49 tests, 0 failures**.
- The change is a *widening* of what a plan may stand on next to its own start, so the risk it carries is a body's first
  step entering a mate's tile. The crowd fixture is exactly that case and is green; the reach test at the step is
  unchanged, so a first step into a body still refuses and re-plans.

## The honest label on this patch: the rule is the game's, the **width is not**

`FUN_007fb220` re-seeds *the start point* by **2 x radius** - **0.4 tiles** for a 0.2-tile body. This patch instead exempts
**a whole tile in each of the eight directions** from the stander list, which is **2.5x** the game's reach, and it is a
different mechanism: the game moves the body's own start, this lets the plan walk through the bodies beside it. It makes
the crowd fixture pass, and it is **an approximation of our own** - so it is labelled here rather than left to read as a
ported rule. The faithful form is a re-seed of the start point (candidates on a ring at `<= 2 x radius`, terrain- and
body-checked, the route prefixed with the body's real position), which is the slice to make next; the one-tile ring comes
out then, whatever the fixture says.
