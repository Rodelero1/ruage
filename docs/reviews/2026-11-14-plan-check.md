# Review — `tools/plan_check.py`, the checklist's own gate (2026-11-14)

**What changed.** A new tool (`tools/plan_check.py`), its tests (`tools/test_plan_check.py`, 10 cases), a `make plan-check`
target that `make check` now runs, and the document edits the tool forced (`docs/dependency-tree.md`: the owner's six
answers in §12, the five architecture seams in §7b, and the six items the checker found unscheduled).

## Performance

**Measured, five runs, median 266 ms** over `docs/dependency-tree.md` (32,577 bytes, 64 items, 104 citations, 8 checks).
The first version was **3,304 ms** — an `rglob("*")` index that stat'ed every file under `target/` and then filtered it;
pruning the walk (`target`, `.git`, `.venv`, `node_modules`, `build`, `Age2HD`, `site`) and indexing only the extensions a
citation can name took it to **266 ms, 12× faster**. The ten tool tests run in **42 ms**.

Nothing here is in the tick: the tool is Python, reads two files, and is never linked into the engine. Its cost is paid by
`make check`, once per commit.

## Stability

**Seven mutations on the real document, each caught by its own check** (a mutation that only trips some *other* check
proves nothing):

| Mutation | Caught by |
|---|---|
| `world/sitez.rs:427 nearest_drop_site` | the path check — *cited and no such file* |
| `reserch.rs:221` (a bare name) | the same check, through the basename index |
| `techs.rs:99996` | the line check — *the file ends at 531* |
| a `DONE-ish` row's `**Hole: T3**` removed | the hollow-row check — *names no T or M* |
| `**Hole: T19**` | the id check — *referred to at line 59 and never defined* |
| `**MOSTLY**` in a status cell | the vocabulary check |
| the M3-M8 phase item deleted | the scheduling check — *M3: named by no phase* |

**What it cannot see, stated so a green run is not read as more than it is.** It checks that the document is *consistent*,
never that a status is *true* — a row that says `DONE` about a lie passes. That is §11's probes' job and the reviewer's.
It does not check globs (`tests/gather_*.rs`), paths into generated or installed directories (`config/gamedata/`, `build/`,
`Age2HD/`, `target/`, `docs/site/` — a fresh clone has none of them, D8), or a partial path that resolves by tail to more
than one file (the line is accepted if *any* candidate is long enough). And a cited line number drifts when somebody edits
above it: the check catches *past the end*, not *moved*.

**Two defects in the tool itself, found by mutating it rather than by reading it.** (1) The first version skipped every
backticked span containing a space — which silently excluded this document's most common shape, `` `file.rs:427 a
description` ``. (2) It deduplicated citations on the path alone, so `README.md` in one row and `README.md:99` in another
counted as one claim and the wrong line survived. Both are now tests.

**Six genuine misses in my own document, found by the first green-attempt run:** M4-M7 and T8/T13 were defined and named by
no phase (M4-M7 were hidden inside a `M3-M8` range — exactly the shape where one goes missing), and S11's hole was named by
`M1` while the check only accepted a `T`. The check was right on all six; the document was wrong.

**One process finding, because it cost real work.** While this slice was being written, another session in this shared tree
stashed and then cleaned the worktree twice; the tool, its tests and the document edits were deleted mid-flight and had to
be rewritten. Nothing was lost from git — the first half was already committed — but the recovery cost a full pass. The
rule is in `AGENTS.md` and it is worth repeating here: **commit each piece as it is finished**, because a document or a
tool that lives only in the worktree is one `git clean` away from gone.
