# Prior-art watch

Awareness of open-source AoE2 engines and tooling, on an adaptive schedule, without burning tokens
on projects that have not moved.

## The rule (D14 amendment)

The owner's rule, implemented against **upstream** activity rather than our own calendar — our
counters drift, and a missed tick would silently age the whole list:

| project last touched | look at it |
|---|---|
| within 2 days | every 2 days |
| within a week | weekly |
| quieter than a week | every 2 weeks |

Plus one override that matters more than the table: **a project is due the moment its head commit
changes**, whatever its idle time. That is how a project that has been quiet for a year and then
comes back to life is caught on the day it moves, rather than up to a fortnight later.

## How it runs

- `tools/prior_art/watch.py` holds the watchlist, queries GitHub for each repo's head commit, applies
  the rule, and prints **only** the projects that are due. Empty output is the normal case.
- Cron job `a10883e59b0a` runs the script daily at 09:00 UTC. The script's output is the gate: when
  it is empty the agent replies with one line and does nothing else.
- State (last head sha, last look) lives in `build/prior-art-state.json`, which is gitignored, so the
  watch cannot dirty the tree it is watching.
- `/opt/data/scripts/prior_art_watch.py` is a **shim** that runs the repo copy. It exists because the
  scheduler requires its scripts in its own directory; it must stay a shim or the two copies drift.

## What is watched

Reference engine:
- `SFTtech/openage` — the asset pipeline (D18), the engine/content split, and a determinism posture
  that is a contrast rather than a model.

Deterministic reimplementation and formats:
- `librematch/genie` — active; format readers and a DE-targeting deterministic implementation (D13).
- `Zaydme/openaok` — from-scratch AoK (1999) engine, C++20/SDL2/EnTT, MIT; fixed 16 ms tick, its own pathers, reads `EMPIRES2.DAT` at `VER 5.7`. The nearest sibling to this project, and the one that answers questions about our own dat version.

Dat modelling we depend on:
- `SiegeEngineers/genie-rs` — **dormant since November 2022**, so it will not answer U15 or U17; kept
  on the list because it is still the cleanest crate-level model of the format.
- `SiegeEngineers/genieutils-py` — an independent reading of the same format.

Asset pipeline (D18):
- `SiegeEngineers/sld-extractor` — DE sprite extraction.
- `SiegeEngineers/WololoKingdoms` — HD-era file conversion (what changes between HD and DE).

Published-value cross-checks for open questions:
- `SiegeEngineers/aoc-reference-data` — updated most days.
- `SiegeEngineers/halfon`, `SiegeEngineers/aoe2techTree` — unit/building/tech references.

Later steps:
- `SiegeEngineers/rms-check`, `SiegeEngineers/rms-doc` — the random map script language.

## What a report must contain

1. **What changed**, one sentence per commit or release.
2. **Divergence**, on the axes that matter: asset pipeline and format (D18), engine/content boundary
   (D17), determinism and tick model, platform targets, language and build, licence. A better
   approach than ours is reported as better, with the cost of adopting it.
3. **Study list**, with pointers specific enough to open.
4. **Never copy** — GPL-3.0 sources are read for ideas; nothing crosses into `crates/`.
5. **Open questions** — any U-number a source now answers.

## The relevance filter

The long-term plan: an open-source engine for **Linux, Windows and possibly macOS**, keeping the Age
of Empires II feel and dynamics, running initially on the original assets and ready for original
ones. A project outside that (a different kind of game, unusable licence, graphics-only toolkit) is
reported only if it teaches something usable anyway — and then the divergence is stated, not implied.

## 2026-10-27 — the watch tool was missing, and now it is not

The daily job failed with `FileNotFoundError: tools/prior_art/watch.py`. The script had never been
committed, and the working copy was gone, so the shim could only fail. Two runs were spent surveying
by hand with state that could not advance.

Fixed, and the fix is the point:

- `tools/prior_art/watch.py` is committed, with `tools/prior_art/test_watch.py` (16 tests; `python3
  -m unittest discover -s tools/prior_art`). The failure mode of a watch is **silence**, so the
  tests pin what "due" means rather than what the code looks like: never-looked, head-moved,
  interval-expired, and the quiet case.
- A failed fetch now **fails the run** (exit 1, `WATCH FAILURE <repo>` on stderr) even when other
  projects are due. Before that, one failed repo was hidden by one due repo — the exact shape of
  silence this tool exists to avoid.
- A failed fetch does **not** advance that project's state, so the next run tries again.
- An unreadable or naive timestamp makes a project due, not cold.
- State keeps `previous_sha` beside `sha`, so a look can always diff against the sha the last
  report saw.
- `Zaydme/openaok` joined the list; it is the only other project reading the same 5.7 dat.

State still advances when the script fetches, not when the report is written. That is a deliberate
trade: if a run dies between the two, the project goes quiet for at most its interval instead of
being reported twice a day. The alternative — ack-on-report — makes a forgotten ack repeat forever,
which is noisier but no safer.
