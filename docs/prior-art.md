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
