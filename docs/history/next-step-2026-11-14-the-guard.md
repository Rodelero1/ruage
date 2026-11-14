# OPEN - the guard's real cause: an open window skips the bodies, and the shoulders were never built

**Takeover, 2026-11-14 ~20:45, branch `d208-time-not-geometry` (at `e21b44d`), worktree `/opt/data/cache/scratch/a2h-d209`.**
The two open items from the last session are answered; two new defects were found on the way, and one of them explains a
"green" that was never real.

## 1. The trap that made `group_overlap` look green: a missing pack passes every world test silently

`config/gamedata` is **absent** in this worktree (`fresh()` returns `None`), and every fixture does
`let Some(mut world) = common::fresh() else { return; };`. So the whole file reported `ok` **without running a single
assertion** - I reproduced a "pass" for `a_mate_who_is_stopped_becomes_solid` and then the same binary, with the pack
copied in, failed at once. Any "green" taken in a tree without `config/gamedata` is worthless. `config/gamedata/` is
gitignored, so a fresh worktree has none: copy it (`cp ../Age2HD/config/gamedata/*.toml config/gamedata/`), never symlink
it - the pack-loader tests write into that directory and a symlink hands them the shared pack.

## 2. The trace: the re-plan *does* happen, and the route it builds is the bug

`trace_steps()` on that fixture, one line a tick (probe deleted after use):

```
t=167 a=6.142,2.318 b=6.506,2.135 dist=8154 a_step=Take     route=[(8.50,2.50)]
t=168 a=6.167,2.320 b=6.506,2.135 dist=7728 a_step=TakeAtRest route=[(5.37,2.32),(8.50,2.50)]   <- re-plan, anchor 0.8 back
t=169 a=6.142,2.320 b=6.506,2.135 dist=8168 a_step=Take     ...
t=233 a=6.389,2.380 b=6.506,2.135 dist=7844 a_step=TakeAtRest route=[(8.50,2.50)]               <- the detour is spent
t=236 ..              a_step=TakeOverlapping                                                     <- inside him, walks out
```

- the re-plan **is** asked for, and it **succeeds** (`repaths_body=1`): D212's re-seed picks the backwards direction
  and puts the anchor 0.8 tiles behind him;
- the second leg of that route, `(5.37,2.32) -> (8.50,2.50)`, passes **0.25 tiles** from the stopped man's centre
  (`clears_disc` says it is clear; it is not) - so the "detour" is a detour round nothing;
- `blocked.is_empty()` in `short_range_clear` **returns the straight line before `bodies` is ever consulted**. The window's
  blocked tiles are empty here because the stopped man shares the walker's own start tile, and the repath drops a tile that
  equals `start` (`step.rs`, the obstacle filter). So the disc is the *only* trace of him, and the fast path ignores discs.

### Two defects the trace exposed in the code that is already committed

- **`clears_disc`'s endpoint case is an OR.** When the perpendicular foot falls outside the segment the nearest point is
  the nearer *end*, so the segment is clear only when **both** ends are clear; the code reads
  `d(a) >= c || d(b) >= c`, which calls a segment clear whenever its *far* end is far - a line from inside a body's disc
  to a free point beyond him passes that test.
- **There are no shoulder nodes.** `short.rs:72` and `docs/reviews/2026-11-14-d211-clearance.md` both say the graph "gains
  eight shoulder points round every body". It does not: `grep shoulder crates/sim/src/short.rs` finds the doc comment and
  nothing else. The two `clearance.rs` tests pass for a different reason - the stander's *tile* is in the avoid set, so a
  whole-tile rectangle exists to bend round. The sub-tile mechanism was never built and never exercised.

## 3. A real defect found while tracing, **not** in this slice: `last_moved` is stamped on bodies that did not move

`move_bodies` writes `entity.last_moved = now` for every delta it is handed, including the `(0,0)` deltas of an idle body
(measured: a stopped, `Idle` villager's `last_moved` advanced on every tick). Consequences: `is_past` can never be true
(`created >= moved` with `moved` always the current tick), and `stander_bodies`/`stander_tiles` return **nothing** - so the
D210b/D211 stander mechanism is dead code today, and the only body ever modelled as an obstacle is the one a repath names
explicitly. That re-enables a rule measured at 2.3x, so it is **its own slice**, with its own test and its own timing; it is
not being fixed inside this one.

## 4. What is being built now, test-first

Two unit tests in `crates/sim/tests/clearance.rs`, straight at `short_range_clear` (no world, no pack):
a body alone in an open window must be rounded (not through), and a walker who starts *inside* a body must step **away**
from him first. Then the search gains: the straight-line shortcut tests the discs; an empty rectangle set is allowed when
bodies are present; the shoulder nodes the doc promised; `clears_disc`'s endpoint rule fixed; and the anchor keeps D212's
`2 x clearance` re-seed but only in a direction that moves *away* from the disc it starts in (a free point on his far side
is a destination, not a start - measured 3702 raw against 8000 when it was allowed).

Then: whole gate, timing against `main`, the decision renumber (our D210/D211/D212 collide with `main`'s D210 and D211),
the review note, and the merge.

---

## The state this work continues from (kept from the previous session)

**Where it stands, 2026-11-14, on `d208-time-not-geometry`.** Both halves of the original's collision rule are built:

- **no clamp**: a step is never refused because it would end inside `r1 + r2`. `bound_to_reach` is no longer called from `collide`.
- **time, not geometry**: the loser of the id comparison is paused 0.25-1.50 s, and a collider that has **not** moved since
  the route was planned is *past* (see defect 3 above - this half cannot fire yet).
- **standing bodies are search obstacles** (D210b): `stander_tiles`/`stander_bodies` - see defect 3, dead today.

**Measured on this branch** (`--seed 7 --ticks 48000`, release): `entities=158 players=2 hash=c2ed74016ee533ea repaths=508
made_room=0 repaths_body=174 repaths_ground=334`, against `main`'s `entities=160 hash=c3b10f4b09e17a43 repaths=324
made_room=147 repaths_body=83 repaths_ground=241`.

**The 18 failures the last gate reported** were taken in the worktree with the pack **symlinked**; two of them are that
symlink, and the rest need re-triage on this tree with a real pack. The list is in the last session's report and in the
review's triage section; re-run the gate rather than trusting it.
