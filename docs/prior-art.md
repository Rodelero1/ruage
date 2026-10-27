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

## 2026-10-27 (second pass) — the effect kinds, named twice and confirmed in our own data

All twelve heads are the shas the last look recorded, so nothing was diffed this pass. What follows
came from reading those heads, and one of them answered three open questions.

**Two independent implementations name the same kind table.** `librematch/genie` (DE, AGPL-3.0)
`crates/engine/genie-core/src/technology/effect/mod.rs` has `Kind::SET 0, RESOURCE 1, ENABLE 2,
UPGRADE 3, ADD 4, MULTIPLY 5, RESOURCE_MULTIPLY 6, SPAWN 7, TECH 8`, and `TECH_COST = 0x65` (101),
`TECH_DISABLE = 0x66` (102), `TECH_TIME = 0x67` (103). `Zaydme/openaok` (AoK 1999, MIT)
`src/core/Rules.h:330` names 0–6 and 101–103 identically, and `src/ecs/Effects.cpp` gives the field
roles: `SetAttribute/AddAttribute/MultiplyAttribute` (a=unit, c=attribute, d=value), `EnableUnit`
(a=unit, b=1 on / 0 off), `Resource` (a=resource, b=0 set / 1 add, d=value), `TechCost` (a=tech,
b=resource, c=0 set / 1 add, d=amount), `TechTime` (a=tech), `DisableTech` (d=tech id).

**Our data confirms it without either repo.** Across the 97 kind-101 commands in
`config/gamedata/effects.toml`, `b` takes only 0, 1, 2 and 3 (37/17/1/42 — exactly the four
resources), `c` only 0 or 1 (58/39), and `d` is an amount, including negatives. The effects holding
them are the ones whose published behaviour is cheaper technologies: effect 10 "Lombards Technology
Tree" prices Feudal/Castle/Imperial at 425/680/850 food and then *adds* −200/−157/−250; effect 5
"Magyars Technology Tree" zeroes 67/68/75; effect 3710's `[101, 197, 0, 0, 0.0]` is a free tech.

**U16 closes.** Kind 101 is a tech cost and its `a` is a **tech id**. The census's "84 present in
both tables" is made of techs that happen to share an id with a unit (202 = double-bit axe, and a
unit id); no unit reading survives, because enabling a unit is kind 2. `Meaning::AmbiguousId` should
become a tech reference — test-first, in `crates/data`, against effect 10's five age prices and
effect 5's three zeros.

**U2's open kinds are answered**, subject to the same test: 0 set attribute · 2 enable unit ·
6 multiply resource · 101 tech cost · 102 disable tech (`d` = tech id, which matches U13's finding
independently) · 103 tech time. Kind 3 = upgrade unit and 4/5 = add/multiply attribute were already
pinned, so every kind our data uses is now named.

**New, and not in any U-row: the tens digit is the party.** genie's `Party::of_block(c.kind / 10)`
maps 0 to the player, 1 to mutual allies, 2 to mutual enemies, 3 to neutrals, 4 to Gaia. Our data
confirms it in place: the effect at `effects.toml:3770` carries `[11, 50, 1, -1, 1.0]` beside
`[1, 50, 1, -1, 1.0]` — the same grant to self and allies; `:3656`'s `[15, 82, -1, 13, 1.25]` is a
team-wide gather bonus; `:4028`'s `[11, 3, 1, 13, 500.0]` gives allies 500 gold. So kinds 10–49 are
the six base kinds retargeted at another party. openaok does not implement this at all
(`Effects.cpp`'s `default: break`), so the decoding is genie's, from the DE binary; our data is the
independent check.

**Gap this exposes.** 97 kind-101 and 25 kind-103 commands sit in `effects.toml`'s `commands` and
have no counterpart in `modifiers` — effect 10 carries twenty of them and only its kind-5 multiply
survived the import. The TOML keeps the raw command, so nothing is lost, but the schema has no
`tech_cost`/`tech_time` modifier and the effect interpreter cannot see them.

**U12 is still unanswered for HD.** genie's DE recordings carry `dt` per frame (genie-oracle runs
`frame.dt`; its fixtures use 20 ms), which is a property of DE's recording format, not of HD's
clock. openaok chose 16 ms by hand, commented `~60 Hz`, which is 62.5 Hz and 3.125 ticks per 50 ms
datum — its own second-based values land off the grid ours were chosen to land on (D16). Twenty
hertz stays a choice, and still the cheaper one. openaok's A* is likewise its own design, not
evidence about the original's pathers (U7).

**Divergence worth naming.** openaok targets macOS, Linux and Android, and not Windows. Its engine is
MIT while its build-time data tools link LGPL-3.0 `genieutils` — the boundary D12/D21 draw for us,
working in practice.

**Study list.** genie `docs/ENGINE_SYSTEMS.md` (nineteen systems, tick order, per-folder READMEs);
`crates/engine/genie-core/src/world/world_update.rs` — only the player whose turn it is runs its unit
AIs' full update, and the turn rotates each tick, so AI cadence in the original is per-player-turn,
not per-tick; openaok `src/verify/` — forty one-scenario-per-file checks, `Determinism.cpp` among
them, the shape our adversarial pass keeps reaching for.

**Licence.** genie is AGPL-3.0: read, nothing crosses. openaok is MIT and may be read freely; its
`third_party/genieutils` is LGPL-3.0 and must stay in tools. A field-role table is not
copyrightable, but the code that carries it is — "it was only a small function" is how that line
gets crossed.
