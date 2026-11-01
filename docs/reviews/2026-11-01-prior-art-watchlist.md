# The prior-art watch: what the sweep costs, and what was attacked (2026-11-01)

Scope: `tools/prior_art/watch.py` and `tools/prior_art/test_watch.py`, as touched by the 2026-10-30 discovery
sweep and today's watchlist additions (`yanfengliu/aoe2`, `Divy1211/AoE2DE_UGC_Guide`). No engine code, no
`config/`: the only thing that changed at runtime is what the cron gate prints.

## Performance

Measured on this container (2 CPUs, 4 GiB), best of three, nothing else building:

| what | before | after | how measured |
|---|---|---|---|
| test suite | 9 ms, 16 tests | **467 ms, 46 tests** | `python3 -m unittest discover -s tools/prior_art` |
| gate, quiet day, 14 repos | 2.6 s | **2.90 s** | `python3 /opt/data/scripts/prior_art_watch.py --dry-run --status` |
| monthly sweep | — | **5 search calls, ~30 s, 31 candidates** | live run against `api.github.com`, 2026-10-30 |

The 458 ms of suite growth is the discovery tests, and it is deliberately *offline*: the first draft let the
pre-existing `GateTests`/`FailureTests`/`MixedRunTests` classes reach the network through `main()`, which showed up
as a **7.0 s** run and a written state file. After isolating those classes the suite is 0.47 s and writes nothing.

Rate budget, which is the number that actually constrains the design: the gate's per-repo reads come from the core
bucket (60/hour unauthenticated, **14 per run** → 4 runs/hour of headroom), and the sweep comes from the separate
search bucket (10/minute, **5 per month**). A sweep can therefore never starve the gate, and a day of manual fires
cannot exhaust the month's discovery.

## Stability

Attacked, and the outcome of each attack:

- **A sweep that cannot run** (403s, DNS, timeout) — one query failing used to be swallowable. Now each dead query is
  named on stderr as `DISCOVERY FAILURE` and the run exits 1, so a rate-limited month cannot pass as a quiet one. This
  was exercised for real: live GitHub 403s during development produced the failure path, not a silent empty tick.
- **A half-complete sweep** — the scan timestamp advances only when all five queries answered. A partial sweep writes no
  state and tries again next tick.
- **The eight-line cap** — the first version marked every candidate `seen`, so items 9..N were discarded forever. The cap
  now bites: only what was printed is remembered, and the overflow resurfaces next month. Test:
  `test_overflow_is_not_marked_seen_so_it_can_surface_next_month`.
- **A stranger's description as input** — descriptions are attacker-controlled strings. A tab or newline in one forged a
  column boundary and a second line in the report the agent reads. Whitespace is now flattened; test
  `test_a_description_with_a_tab_or_newline_cannot_break_the_line`.
- **Report stability** — equal push times used to order by dict insertion, so the same month could print in a different
  order. Ties now break by name.
- **Noise as a denial of attention** — the readme qualifier doubled the candidate list (35 → 57) and its top eight were
  a "point Claude at any game" mod pack, a decompilation directory, a starred-repos mirror and an emulator. Dropping it
  kept the target project and removed 22 distractors; curated lists, SEO drill pages and our own account are filtered too.
- **Clock rot in the tests themselves** — `test_unchanged_within_interval_is_silent` had been failing since 2026-10-30
  because the test pinned its own `NOW` while the gate read the wall clock. `main(now=...)` makes the clock injectable and
  the suite pins it. This is the defect that matters most here: a green suite that is green only on the day it was written
  is the same failure mode as a gate that reports nothing because it is broken.
- **State files** — both are written atomically (`os.replace` over a temp file) and both are gitignored, so the watch
  cannot dirty the tree it is watching; a corrupt or naive timestamp makes the thing due rather than cold.

Not attacked, and named: the sweep still trusts GitHub's search ranking within a query (`sort=updated`, 30 results), so a
project that is old, unpopular and rarely pushed can be invisible to it — precision was chosen over recall deliberately,
and the trade is recorded in `docs/prior-art.md`.
