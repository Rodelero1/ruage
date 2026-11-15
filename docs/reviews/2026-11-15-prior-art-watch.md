# Prior-art watch, 2026-11-15

Docs only: `docs/prior-art.md` (one entry), `docs/mechanics/README.md` (U18 and U22 addenda, the new U24 row),
`docs/dependency-tree.md` (T18 and its place in Phase 5), `docs/decision-areas.toml` (U24's area) and the rendered
`docs/site/index.html`. No `crates/`, `config/`, `tools/` or `Age2HD/` change, so there is no engine number of ours to
measure and none is invented.

## Performance

Nothing in the engine changed, so a probe would measure the same binary twice. What was measured:

- **genie: 27 commits, 154 files, +10,179 / −2,762** over `62831fa8` → `18d3698f` (13-14 Nov) — of which **2,562 of the insertions are four new test files**: `construction/pack/tests.rs` 932 lines,
  `objects/idle_update/tests.rs` 872, `construction/housing/tests.rs` 509, `movement/move_to/install/tests.rs` 249. The
  port is paying for behaviour with tests rather than with prose, which is the one thing about it worth copying.
- **The docs tooling, so this note is not all upstream numbers:** `python3 tools/decisions.py` rebuilds the 354 KB site in
  **1.37 s** wall; the site grew **354,368 → 354,843 bytes (+475)** for one new U row; the tools suite is **202 tests in
  8.4 s**; `plan_check` reports **83 items, 114 citations, 8 checks**.

## Stability

What was attacked rather than asserted:

- **"The world loop is untouched again."** Pinned by sha, not by absence from a file list (this file's 2026-11-14 note is
  the reason): `world/step.rs` `b7153ee2…`, `world/schedule.rs` `e27a9958…` — identical at both shas across all 27
  commits. Same check, same result for U22's contract: `gathering/gather_search.rs` `754c7d20…` and
  `gathering/closest_resource.rs` `2161462d…`.
- **"Housing is a DE mechanic and not ours."** Attacked in the only place that can settle it: our own install.
  `strings -a "Age2HD/AoK HD.exe"` carries `.?AVTRIBE_Action_Housing@@`, `…_Pack@@`, `…_Unpack@@`, plus
  `housing-headroom`, `amount-building-housing-rate`, `packed-trebuchet-class`, `unpacked-trebuchet-class` and
  `housing.wav`. That makes the mechanic 5.7's.
- **"…and therefore attribute 62 is HD's id too."** Refused. The same dump is a name list, not a table:
  `amount-building-housing-rate` is its 53rd entry while the port names it 62. No id was cited from the strings; the row
  names the port's id and says whose it is.
- **"A house's five arrive late, as genie's timer says."** Refused as an assertion and registered instead. genie's own
  fixture needs `work_rate = 0.5` and says why ("the `.dat`'s is 0, which no update would run down"), and our `HOUS` row is
  `work_rate = 0.0` (`config/gamedata/units.toml:1897`) — so the routine that grants the five is not in that path in either
  build. U24 carries the oracle that settles it instead of a guess wearing a citation.
- **"A trebuchet's pack time is its train time."** Attacked against our own data: both `TREBU` 42 and `PTREB` 331 read
  `train_time = 50`, which cannot be a pack time, and `GenieDatParser` exposes class 54 (`UnpackedSiegeUnit`) and no pack
  field at all. T18 therefore states the defect (we train 331, nothing turns it into 42, so it walks and never fires) and
  names the pack time as unread rather than importing DE's rule.
- **The register and the tree still pass.** `tools` suite 202 tests OK; `decisions-check` 0 findings, 220 decisions and
  **13 open questions** (was 12); `plan-check` ok. The one check that failed first was real — T18 defined and named by no
  phase — and it was fixed by scheduling the item in Phase 5, not by loosening the check.

My own errors, reported: (1) I read `HousingTuning::cost = 50` as a house's price and went looking for a 50-wood house;
the house is `costs = [[1, 25]]` (`config/gamedata/units.toml:1897`), so the constant belongs to the install site, not to
`HOUS`. (2) I briefly read the `amount-*` dump as an id table, at position 53 against id 62.

Second, disclosed rather than reverted: `make decisions` also moved the site's `SPEC.md` line citations by one
(`D115` at `SPEC.md:661` → `:662`, and its neighbours) because `7e052a5`, which last touched `SPEC.md`, did not re-render
the page. That is a committed-state render of someone else's landed edit, not an uncommitted one — last run's D204 case was
the other kind, and reverting here would have committed a site that disagrees with its own corpus.
