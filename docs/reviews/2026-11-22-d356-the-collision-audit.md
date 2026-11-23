# The collision audit of the owner's pathfinding rulings

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Docs and decisions only** (no `crates/` change)

**The ask:** *"Review all my decisions focused on pathfinding and obstacle avoidance. Are any of them colliding with each other?
If yes, please list them and crosscheck with decomp and online sources to decide. If no decision, please ask me like I'm five."*

## What was read

170 rows in `Pathfinding & movement` and `Economy`, of which about **forty are his rulings** (quoted, dated). Seven sources were
cross-checked: `unit_action_resolve.cpp` (`func_002699A0`), `unit_action_collision.cpp` (`func_002697C0`), the findings recorded in
`docs/movement-in-the-binary.md` (`FUN_008aa990`, `FUN_008a4360`, `008a80ff`), `docs/prior-art.md` (Forgotten Empires' talk,
openaok, the DE 2025 update), and the three probes run today (`invariants`, `collision_probe`, the acceptance scene).

## The four collisions, and what settles each

1. **The overlap rule, stated three times by him, answered by machinery later deleted.** D181 (2026-11-08, *"visible overlap of
   villagers"*), D243.1 (2026-11-16, *"closer than 1 diameter -> FAIL"*), D351 (2026-11-22, *"HD never stacks up that much"*).
   Between them the engine deleted every mechanism that had answered it: the standoff (D293) and the seat book (D287/D293).
2. **D243.1 against D289/D290** — resolved in the game's favour on **both** sides: the game **places** bodies `ext+ext+0.01` apart
   and has **no press floor** while they move. Verdict: D290 stands as a fact about motion; the arrival must be a placement.
3. **D255 "no sliding" against D292 "only sliding and the pause rule act"** — a collision I created: the word "sliding" was in
   **my** fork text, and the binary has no slide. Not a ruling; a correction.
4. **Five rows that read as live rulings and are not**: D189 (withdrawn by D200), D181 (D293), D186 (D199), D271 (D289), D204
   (D239/D241). Marked today, so the index prints them as superseded.

## The two rulings his answers produced (recorded as D356)

1. *"The second man steps to the next free spot near the same tree/mine (the game's own placement rule), and only goes idle if
   there is none."*
2. *"Never slide: if the step is blocked he stops and re-plans."*

## Performance

- The audit itself is tool work: `decision_index.py --summary` **0.35 s** over 832,998 characters; `decisions.py --check` and
  `next_step.py --check` both **0** after the changes; `tools-test` **211 tests, OK**.
- The measurement that drove the verdicts, for the record: acceptance scene, 36,000 ticks, seed 7 - **144,534** overlapping
  pair-ticks of **15,926,363** inspected (0.91%), deepest **8,000 raw** (a full footprint), **129,512** pair-ticks deeper than half
  a footprint, one gatherer pair closing **7,830 -> 7,030 -> 6,230 -> 5,600** against the **8,000** needed, **12,544** re-paths.
- `cargo test --test invariants` (release): **7 passed, 0 failed, 0.83 s** - including `villagers_never_close_into_one_body`,
  which is why a green fixture beside a red scene had to be reported as a finding rather than trusted.

## Stability

- **No engine code was touched**, so no behaviour changed tonight: the suite was not rebuilt for this slice beyond the two
  probes, and the tree's numbers stand as D353 left them.
- The risk this slice *removes* is the one that cost the previous session: five dead rulings were being read as live, and the
  index did not say so. The marker convention (`Superseded by Dn`) is what makes it machine-visible; the index now prints
  **six** superseded rows where it printed three.
- One bug in my own tool, caught by re-running it rather than by reading: `SUPERSEDED` did not match *"Superseded **in part** by
  D239 and D241"*, so the two rows whose status is most misleading - half-live - were the two it missed. Fixed and pinned by the
  regenerated index.
- **Known and deliberate:** D243.1's fixture stays green while the acceptance scene violates it. It is left green rather than
  weakened, and the scene's number is recorded here and in D356 so nobody reads the green fixture as parity again.
