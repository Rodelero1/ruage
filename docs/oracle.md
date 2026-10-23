# The oracle: save games, not a controlled game

This file answers D2 in SPEC §6. The short version: **we do not need to automate the game,
and Hermes does not need to run on Windows.**

## What was found

`Age2HD/SaveGame/` already contains **13 real save games** (`.aoe2spgame`, 2015–2020,
432 KB – 1.8 MB) from the previous owner's play sessions. They are not empty and not
corrupt.

Tested three of them (`Alaric`, `khmer`, `Random_map`):

| file | on disk | inflates to | eof | trailing |
|---|---|---|---|---|
| `Alaric.aoe2spgame` | 821,064 | 9,498,877 | ✔ | 0 |
| `khmer.aoe2spgame` | 431,913 | 4,487,895 | ✔ | 0 |
| `Random_map.aoe2spgame` | 985,451 | 11,053,366 | ✔ | 0 |

- **Raw DEFLATE from offset 0** — the same trick as the `.dat` files. `zlib.decompressobj(-15)`.
- Inner header is `VER 9.4\0\0\0HA\0` — saves use **version 9.4**, not the dat's 5.7.
  (The exe carries `VER 9.4`, `VER 9.3` and `VER 5.7` strings; 9.x is the save/scenario line.)
- Inflated size scales with how much is happening: a small campaign save is 4.5 MB, a
  random-map game mid-play is 11 MB. That is consistent with a full world-state dump.

**These are state snapshots.** A save contains the world at one instant: every unit with
its HP and position, every player's resources, the tick counter, and the map. That is an
oracle you obtain by pressing "Save", not by writing a game bot.

## What a snapshot oracle can and cannot answer

| Question | Answerable from saves? |
|---|---|
| U1 gather rates (before/after snapshots over a known interval) | **yes** — count resources at t and t+Δ |
| U4 damage order (scripted scenario, save before/after a single hit) | **yes** — exact HP delta |
| U5 unit speed / tile scale (position delta per tick) | **yes** |
| U6 rate of fire (shots fired over an interval) | **yes** |
| U8 civ `resources` array meaning (compare civs across saves) | **partly** |
| U2 effect-type semantics (research a tech, diff the save) | **yes** — the cleanest method available |
| U7 pathing algorithm / tie-breaks | **no** — needs per-tick trajectories |
| U10 age-up upgrade timing | **partly** — snapshots catch the aftermath, not the transition |

So D2 resolves to: **file-based oracle, sufficient for the constants and the economy, not
sufficient for behavioural identity.** That directly constrains D1 — see the recommendation
below.

## How to produce more

Cheapest to most expensive:

1. **Save a game at a known moment.** Play 60 seconds with 3 villagers on wood, save as
   `wood60`, drop the file in `Age2HD/SaveGame/` (or anywhere I can read). One fixture, one
   question answered.
2. **Scenario with triggers.** The scenario editor can spawn units, force attacks, and run
   timers — then save. This is how U4 gets a clean single-hit measurement: build a scenario
   with one pikeman next to one knight, no AI, save, then attack and save again.
3. **Recorded games.** HD writes `.aoe2record` (the community `mgz-fast` parser supports HD
   5.8; whether 5.7 differs is **unknown**). A recording is the *input log with tick stamps*,
   which answers tick rate directly and gives real command streams for replay testing.
4. **Running the game on Linux.** Wine or Proton plus Steam, with `Xvfb` for a headless
   display. Works in principle for HD (a DX9-era title), but it buys only what a save cannot:
   mid-animation observation. Not required for anything above.

## Why not move Hermes to Windows

- The oracle is a **file**, so the agent and the game do not need to share a machine — they
  need a shared directory. Copying a 1 MB save is a solved problem; porting the agent,
  workspace, toolchain and 202 MB of derived data is not.
- The work Hermes does here (parse, diff, decode, write Rust) is identical on either OS, but
  the Rust/Linux tooling is already set up and the toolchain on Windows would be a fresh
  install.
- The real obstacle was never the OS. It was believing the oracle had to be *driven*; it only
  has to be *sampled*.

## Recommendation for D1

Choose **G1 (own deterministic engine)**.

Reason: a snapshot oracle validates constants, formulae and rates — it can hold a
reimplementation to "the numbers agree" — but it cannot hold one to "the trajectory is
identical" except by luck. G2 (behavioural identity with HD) would demand per-tick
comparison, which saves do not provide and which would require memory instrumentation or
recorded-game replay against a reference engine that does not exist.

G1 with snapshot validation is a target we can actually measure. G2 is not, on this evidence.

## Step 2b status — partial, and stopping here

Attempted the `VER 9.4` reader. What is proven, across all 13 saves:

| Result | Evidence |
|---|---|
| Every save is raw DEFLATE, offset 0 | 13/13 inflate with `eof=True`, zero trailing bytes |
| Stream starts with an 8-byte version string | `b"VER 9.4\0"` in all 13 |
| There are length-prefixed name records | `u16 len`, `u16 tag 0x0A60`, ASCII bytes — found in 5 of 13: `Alaric` (15 records), `Bari` (15), `sforza` (15), `khmer` (9), `Random_map` (6) |
| Human player is the first record | `RodeleroCZ` (khmer), `Belisarivs` (sforza) |

What does **not** work, and why the effort stops:

- **The layout is not uniform across saves: 8 of 13 yield no name records at all**
  (`1`, `Berber`, `ETHIOP`, `Magyars`, `Magyars1`, `Persia`, `malinese`, `portuguese`).
  The split does not track file date (a 2016 save has names, another 2016 save does not), so
  the same extension holds more than one header shape — likely campaign vs random-map saves
  or different patch revisions.
- **The record framing is ambiguous.** The `u32` after a name reads `2` (human) and `4` (AI)
  in the clean cases but `0x60000000` elsewhere, which means the field before the length is
  sometimes `u16` and sometimes `u32`. Guessing wrong there shifts everything after it.
- **No resources, unit array, or tick counter were located.** Careful float and integer
  scans over the first 200 KB produced only noise.

The decisive problem: there is **no public spec for this format** (the community
`aoc-mgx-format` repo covers *recorded games*, and its header section is a stub) and **no
second implementation to cross-validate against**. The dat had three independent parsers;
the save has none. Every field would be a single-implementation guess, which is exactly the
failure mode this project exists to avoid.

Getting the unit array and resources out would mean reverse-engineering the save *writer*
in `AoK HD.exe` — a bounded but multi-session Ghidra task, per field.

## Recommendation: don't build it yet

Under **D1 = G1**, the save reader is not on the critical path. Its purpose was to make G2
(behavioural identity) measurable. G1 needs the *constants and formulae* — and those are
published, community-validated tables (`aoc-dev-resources`, the wiki's unit/tech data),
which are cheaper and better than a RE-derived guess.

Revisit save parsing only if D1 flips to G2, or if a specific snapshot question appears that
nothing published can answer.

The working artifact is `build/tools/save.py` (inflate + version + name records where the
framing is clean), output in `build/derived/saves-header.json`. It is a real tool, it is
just not an oracle.

