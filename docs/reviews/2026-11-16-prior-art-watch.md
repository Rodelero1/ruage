# Prior-art watch, 2026-11-16

Docs only: `docs/prior-art.md` (one new dated section), `docs/mechanics/README.md` (U24's description and method cells)
and `docs/dependency-tree.md` (T18's description and evidence cells). **`docs/site/index.html` is not in the commit**: a
render of this change is byte-identical to the committed page (`59e403dc…`), because the U-register's blurb is truncated
before the appended sentence. No `crates/`, `config/`, `tools/` or `Age2HD/` change, so there is no engine number of ours
to measure and none is invented. The pre-commit review gate is not triggered; this note is owed anyway, and it is mostly a
record of what was *disproved*.

## Performance

Nothing in the engine changed, so a probe would measure the same binary twice. What was measured is the work that produced
the claims, all on this box (4 GiB / 2 vCPU):

- **The dat probe that settles T18:** `GenieDatParser` over `empires2_x2_p1.dat` parses in **0.62 s** (two runs: 0.63,
  0.62), and the scan of `dat.unit_data.tasks` — **1,402 slots, 1,943 tasks, 28 distinct action types** — takes **7 ms**.
  The whole check that the trebuchet's transform is in our own data is under a second, which is why "unread" was the wrong
  answer to give last run.
- **Upstream:** genie `18d3698f` → `6eb73570`, 13 commits, **97 files, +5,761 / −1,066**, of which the Unpack alone is
  `construction/unpack/` **+1,978** (`mod.rs` 297, `unit.rs` 458, `tests.rs` 1,181, `README.md` 42). openage
  `5f042b9c` → `e3080121`, 2 commits, **2 files, +11 / −6**.
- **The docs tooling:** `python3 tools/decisions.py` runs in **1.51 s** and, on a clean checkout plus this change, writes a
  page **identical** to HEAD's (389,861 bytes, `59e403dc…`); `decisions.py --check` **1.54 s**; `plan_check` **0.28 s**
  (**84 items, 120 citations, 8 checks**); the tools suite **202 tests in 8.8 s**.

## Stability

What was attacked rather than asserted:

- **"`GenieDatParser` exposes no pack field."** *My own claim from last run, and it is false.* The parser has
  `transform_unit_id: i16, min_ver 5.7` at `sections/civilization/type_info/building_info.py:30`, and the probe returns
  `42 → 331`, `331 → 42`, `479 → 280`. The claim failed because I looked for a field called *pack*, and the field is the one
  a gate uses for opening and closing. T18's "fresh unknown" is struck in the row rather than kept with a citation.
- **"The 50 is a duration, so the pack time is 50 s."** Attacked by reading the timer's arithmetic in the port
  (`Unit::building_work`: `(work_rate_powerup + master.action_part().work_rate) * dt.seconds()`, accumulating *up* to the
  budget) and by checking our own row, which already carries `train_time = 50` and `work_rate = 4.5`
  (`config/gamedata/units.toml:1144`) — **50 / 4.5 = 11.1 s**. A 50 s pack would make a trebuchet unusable, which is the
  sanity check that should have stopped last run's sentence.
- **"Attribute 62 is HD's id."** Refused as before, and now settled from our own row instead of the port's XS name: index 62
  of the 211-value `resources` array is **20** in every playable civ, index 65 is **5.0**, index 63 is 32000. The ids are
  measured; that 65 is the per-house headroom is *not*, and U24 says so in those words.
- **"The Housing action grants a house's five."** Killed twice, from two directions: **0 of 1,943 tasks** in our dat carry
  action type 0x7b, and genie's own commit says the same of DE's build 158041; and its timer starts at attribute 62 while
  every `HOUS` row's work rate is 0.0, so it could not run out even if installed. The port's fixture work rate of 0.5 was
  the smell, and this run followed it.
- **"The world loop and the resource search are untouched again."** Pinned by blob sha at both commits, not by absence from
  a file list: `world/step.rs` `b7153ee2`, `world/schedule.rs` `e27a9958`, `gathering/gather_search.rs` `754c7d20`,
  `gathering/closest_resource.rs` `2161462d` — the same four blobs at `18d3698f` and `6eb73570`.
- **The foreign-change hazard, caught in the act.** `python3 tools/decisions.py` in the shared tree grew the page by
  **+1,696 bytes** — 8 lines of **D234**, which is another session's uncommitted work, not mine. The page was restored to
  HEAD, the render re-run in a `git worktree` at `ad4ebac` carrying only my three files, and the result was byte-identical
  to HEAD: nothing to commit. **A generated file is the easiest place to commit someone else's work**, and the only reason
  this did not happen is that the diff was read before staging rather than after.
- **The checks still pass, and what they reported was inherited:** `tools` suite **202 tests OK**; `plan_check` **ok**;
  `decisions.py --check` **0 findings, 239 decisions, 13 open questions, 7 dangling ids** (D115, D122b, D124, D125b, D126,
  D136, D190) and **42 with no stated basis**. Neither set is this commit's: both are read from `docs/decisions.md`, which
  this change does not touch.

My own errors this run, reported: (1) the probe read `task_info.tasks` first, which `GenieDatParser` refuses at 5.7.2
("tasks is not supported in struct version v5.7.2") — in 5.7 the task lists live at `dat.unit_data.tasks`, indexed
parallel to units, and the two are easy to confuse because `TaskInfo` exists on every unit anyway. (2) A first draft of the
section wrote a broken path (`ruage-tools/gengenconfig`); corrected to `ruage-tools/genconfig` before committing. (3) The
section's first wording said the Unpack task is what "the work command" installs; the port's own README says the installs
are `unpack-install-sites`', so the sentence now says the task path and does not name a factory it has not read.

## AI review

`make ai-review` on this commit returned **`skipped — 0 files, 0 comments, 0 tokens, 0 s`**: its exclusion list is
`docs/**,build/**,target/**,config/gamedata/**,**/*.html`, so a docs-only commit is outside what the reviewer reads. No
machine finding was available to accept or refuse, and saying so is more useful than implying one was folded in; the
attacks above are mine, and the one that failed (last run's "no pack field") is reported there rather than dropped.
