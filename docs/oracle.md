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
| U7 pathing algorithm / tie-breaks | **no** — needs per-tick trajectories; the *module map* behind it is named by the binary (`docs/binary-class-map.md`: `path.cpp` and `pathsys.cpp` separately, `motion.cpp`, `move_obj.cpp`) |
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

## Recorded games carry sync records (found 2026-10-23)

`librematch/AoE-Kaitai` publishes Kaitai Struct definitions for DE files, and its `aoe2record.ksy`
covers the recorded-game **body — "actions, syncs, chat, post-game data"** — for DE save versions
12.97 to 68.9. That matters here for one reason: **a recording contains periodic sync records**,
the closest thing to a per-tick oracle that exists in a file.

Two caveats before anyone gets excited:

- It is **DE-only**, and this install holds **zero** `.aoe2record` files (measured with `find`), so
  it cannot be used yet. One recording made on the Windows machine would test it.
- Whether HD 5.7 recordings carry the same sync section is untested. If they do, a recorded game
  becomes a genuine (if coarse) trajectory oracle — cheap to obtain, unlike a driven game.

Worth noting as a technique rather than a data source: a `.ksy` is a *readable format spec* that
generates parsers for C#, Python, Java, JS, Go and Rust. For our own future readers
(`.aoe2spgame` in step 2b, `.scx` per D15), writing definitions is far cheaper than hand-rolling a
parser, and it produces the documentation for free.

## Deducing the tick rate from quantised fields (measured 2026-10-23)

The idea: if a duration is *produced* by the tick loop, it is an integer number of ticks divided by
the rate, so the greatest common divisor of many such durations is the tick period — clock recovery
from quantised data. Run over every float field in the generated unit table:

| field | distinct values | gcd | reads as |
|---|---|---|---|
| `reload` | 20 | **0.05 s** | a 50 ms grain |
| `speed` | 56 | 0.001 s | authored to 3 decimals, tiles/s |
| `work_rate` | 19 | 0.0001 s | authored to 4 decimals |
| `range` | 18 | 0.01 s | authored to 2 decimals |
| `los` | 19 | 1 s | authored in whole tiles |

**The one hit.** All 20 distinct reload times are multiples of exactly 0.05 s, and two of them —
`0.25` and `3.45` — are **not** multiples of 0.1. So the grid those values were quantised to is
50 ms, not 100 ms: at 20 Hz they are whole tick counts (0.25 = 5, 0.9 = 18, 1.7 = 34, 2.5 = 50,
3.45 = 69, 6.5 = 130, 10.0 = 200).

**What that proves, and what it does not.** It excludes 10 Hz and anything coarser, and it shows
the quantisation step is 50 ms. It does **not** prove 20 Hz: a designer writing reloads in 50 ms
steps produces identical numbers, and 0.05 s is also a whole number of 60 Hz ticks (3). The other
four fields are useless here — they show authoring granularity, and a gcd across all fields just
collapses to the coarsest one.

**To settle U12**, an observable the engine *computes* is needed rather than one a designer types:
a recorded game's periodic sync records (decisive, needs one `.aoe2record` made on the Windows
box), a save's task progress at a known game time (the save reader currently gets only player
names), or a decompiled constant.

**The lockstep *shape* is now named, by the binary itself** (`docs/binary-class-map.md`, D140): HD 5.7 carries 31
`Comm*` classes, and they are a turn-synchronised command stream — `CommPacketSyncTurn`, `CommPacketSyncCmdCount`,
`CommPacketPlayerCommand`, `CommPacketChecksum`, plus `CommPacketSpeedChange`, `CommPacketResign` and the pre-game state
packets. So the contract is: per turn, a batch of player commands, a count, and a checksum — which is the ensemble
lockstep of the era, not a snapshot delta. **What it still does not give is the rate**, and the recording's own numbers
say the sync is not per sim tick: 31,104 `SYNC` ops over 3,066.04 s is **10.15 a second**, i.e. one every ~98.6 ms —
which at a 20 Hz sim is every **second** turn. That is a hypothesis with one number behind it, not a settled rate: it is
equally consistent with a 10 Hz sim, and the 50 ms authoring granularity above is what rules 10 Hz *out* for the dat's
own values.

## Real HD recordings, and what the sync cadence says about U12 (measured 2026-10-25)

`happyleavesaoc/aoc-mgz` (MIT) ships recorded games as test fixtures, including
**`tests/recs/hd-5.7.aoe2record`** — a real HD 5.7 recording, the exact version family this engine
targets. Fetched to scratch (not committed: third-party game data, D8):

```
https://raw.githubusercontent.com/happyleavesaoc/aoc-mgz/master/tests/recs/hd-5.7.aoe2record
```

**Container layout** (decoded by hand; the library's own header parser fails on these HD files
across 1.8.20-1.8.51, so the container was read directly):

| part | encoding |
|---|---|
| bytes 0-7 | `u32 section_len`, `u32 0` — section_len counts from byte 0 |
| header section | raw DEFLATE (wbits -15); inflates to a `VER 9.4` payload |
| after it | the **operation stream, uncompressed** — 40-byte meta block, then 4-byte op ids |

The inflated header begins `VER 9.4`, i.e. **the same magic as the 13 save games** in step 2b. The
save reader's work applies to recordings, and vice versa.

**Op stream, hd-5.7:** 958,069 bytes, 68,011 ops — 31,104 `SYNC`, 31,104 `VIEWLOCK`, 5,792
`ACTION`, 11 `CHAT`, ending exactly at EOF. Elapsed time 3,066,042 ms (51 min).

**Sync cadence across four recordings:**

| file | syncs | elapsed | syncs / second |
|---|---|---|---|
| hd-5.3 | 30,516 | 2,805,957 ms | 10.9 |
| hd-5.7 | 31,104 | 3,066,042 ms | **10.1** |
| hd-5.8 | 31,701 | 3,692,244 ms | 8.6 |
| de-13.34 | 158,648 | 2,682,772 ms | **59.2** |

DE syncs ≈ 60/s, HD ≈ 10/s — a 6:1 ratio. DE at 60 turns/s syncing once per turn is the natural
reading (59.2 measured, not a suspiciously exact 60). HD at 20 turns/s would then sync once per
**two** turns, giving 10/s.

**This corroborates the 20 Hz assumption from a second direction.** The first was the dat grain:
20 distinct attack reload times all multiples of exactly 0.05 s, with 0.25 and 3.45 excluding a
0.1 s grid — and 0.25 s is 5 turns at 20 Hz but 2.5 turns at 10 Hz, so 10 Hz cannot quantise it.

**What it still is not:** proof. It excludes 10 Hz only if reload times are tick-quantised, and it
reads the cadence ratio against DE, whose rate is itself inferred. A decompiled constant remains
the only direct answer.

**What the recordings do settle:** they are the trajectory oracle step 2b wanted. `ORDER`, `MOVE`,
`BUILD`, `RESEARCH`, `QUEUE` actions carry millisecond timestamps, so the slice can be compared
against real games — arrival times, build completions, gather cadence — without a live Windows
machine. Action timestamps show no fixed grid (gcd 1-2 ms), so the timestamps are clock-stamped
rather than turn indices; per-tick state is still not recoverable.

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


## The AI-vs-AI farm: what to capture, and in what mode (2026-11-01)

The owner's plan - two AIs playing each other, recorded, with periodic captures - is the right way to close the rows that
static reading cannot (`docs/behaviour-diff.md` rows 3-5, 8, 10). The mode matters, and the answer is not "either".

**Random Map, not Death Match.**

- **RM exercises the pipeline ruage actually implements** - the dark-age economy, the age ramp, the tech path, the first
  army. Those are where our timings differ (the probe puts our Feudal at ~14:30 against the game's ~9:30), and they are
  exactly rows 2, 3, 5 and 6 of the register.
- **DM deletes that ramp by design**: instant resources, a post-Imperial start, no villager economy under pressure. Its
  number would be the *combat* rows only, and its AI plays unrepresentatively because it never has to earn anything.
- **DM is the better instrument for one class later**: targeting, formations, damage exchanges, because engagements start
  in the first minute and there are many per hour. Use it when the combat rows are the ones being worked, not now.

**Settings that make the data comparable**

- **Map**: Arabia (or a simple land map), **small**, two players - big enough for a real build order, small enough that
  games finish inside the hour.
- **Both AIs the same civilisation** (a mirror match). A civ difference is a confound in every row; the AI's build order at
  the hardest difficulty is already documented in `references/ai-of-the-game.md`.
- **Victory: conquest** (our rule, D78). **Population 200** - the AI's boom depends on it, so it must match the rule our
  cap models. **Standard speed**: the recording is tick-stamped, so speed changes only how much wall time a game costs.
- **Note the seed and the map** in the filename; a rerun of the same seed is what makes two artifacts comparable.
- **Record the game** (`.aoe2record`), **save every five game-minutes**, and screenshot at events (age-up, first military
  unit, first engagement, a building completing) rather than on a timer - an event screenshot carries a fact, a timed one
  carries wallpaper.

**What each artifact answers**

| Artifact | Answers |
|---|---|
| `.aoe2record` | the command stream with tick stamps: what the AI built, trained, researched and attacked, and when. HD 5.7 layout verified: `VER 9.4` header, 68,011 ops, 31,104 `SYNC` in 51 game-minutes |
| `.aoe2spgame` | exact state at one instant - every unit, position, HP, stockpile, tick. **Two saves five minutes apart answer the decay row outright**, and the gather rows by differencing |
| screenshot | the visual and HUD record: does the screen look like the game. Its numbers are far coarser than a save's, so it closes no register row on its own |

**Settled 2026-11-01, by the owner: both player slots can be set to AI**, so the farm is a plain all-AI match - no scenario
trick, no parked villager, no passive human third slot. (The shapes this file first guessed at - a scenario with player 1
given nothing, or one hidden villager keeping the match alive - are kept only as fallbacks if a particular build refuses an
all-AI lobby. An unanswered question is cheaper than a wrong workaround written as procedure.)

**Naming and storage.** `<date>-<map>-<seed>-<civs>.aoe2record` plus its saves, one folder per match, and **never
committed**: these are install-derived artifacts (D8/D22). What is committed is the finding and the artifact's SHA-256, so
a later session can prove which file a number came from.

### The first match to capture (checklist, 2026-11-01)

The one that answers the most rows per hour, and the one to compare our headless run against:

- **Random Map, Arabia, small, two players, both set to AI** (settled: both slots can be AI).
- **Mirror civilisation** on both sides; **hardest** AI; **conquest**; **population 200**; standard speed.
- **Record it**; **save at 5, 10, 15 and 20 game-minutes and at every age-up**; **screenshot** the age-ups, the first
  military unit, the first engagement and any moment a building completes.
- Write the seed, map and civs into the filename: `<date>-arabia-small-<seed>-<civs>`.

What it answers: the AI's real build order and age timings (recording), the decay row (two saves five minutes apart), the
gather rows (differenced saves), and the HUD/vibe check (screenshots). What it cannot answer: per-tick order and pathing
tie-breaks - those stay `needs-oracle` until a recording's SYNC stream or instrumentation is used.

## The farm delivered (2026-11-01) — and it settles the 5.8 question

`Age2HD/SaveGame/` now holds the first real capture set:

- **4 recordings**: `SP Replay v5.8 @2026.11.01 141408 / 141505 / 141539 / 142146.aoe2record`, 188 KB - 2.7 MB.
- **14 saves**: `brit1` … `brit13` plus `britXhuns`, 263 KB growing to 454 KB across the series.

Decoded by hand (raw DEFLATE, `zlib.decompressobj(-15)`, exactly the recipe step 2b established):

| file family | container | header | seconds |
|---|---|---|---|
| recordings | `u32 section_len`, `u32 0`, then the deflated header; the op stream after it, uncompressed | `VER 9.4\0\0\0HA\0` | 187,964-2,697,673 bytes; the header section is ~252-256 KB in all four, so the 2.7 MB one carries a far longer op stream |
| saves | raw DEFLATE from offset 0 | `VER 9.4\0\0\0HA\0` | 263,141 → 453,902 bytes, monotone growth across `brit1`…`brit13` |

**The `VER 5.8` question is settled.** `docs/re-toolchain.md` recorded that the binary holds no `VER 5.8` literal "so that
needs a real `.aoe2record` from your install", and `oracle.md` itself said mgz-fast supports HD 5.8 but "whether 5.7 differs
is unknown". These four files are named **v5.8 by the game itself**, so this build writes 5.8 recordings — and every one of
them still carries a `VER 9.4` payload, the same magic as the 5.7-family recording and the saves. The version string in the
filename and the version string in the body are two different numbers, which is worth knowing before anyone "fixes" one to
match the other.

**What the series is for.** `brit1` … `brit13` is a *timed* series, not a scatter of unrelated saves: the sizes grow
monotonically, which is what a world filling up looks like. Two consecutive saves are the differential oracle — resources
gained over a known interval give the gather rate directly, unit and building counts give the AI's build order, and a
building that is present in `brit7` and absent from `brit8` with nobody attacking it answers the decay row.

**Not yet known, and not guessed:** which two civs each file holds, whether both slots were AI, and the interval between the
numbers. `britXhuns` names its civs in the ownername; the numbered series does not, and the tick counter lives inside a body
we have no reader for yet.

### Two facts from the owner about the capture set (2026-11-01)

1. **Both slots were AI.** The farm needs no workaround and no human slot - confirmed for this batch, not just as a
   possibility.
2. **The interval between the `brit` saves was random**, not a fixed N minutes.

The second one changes how the series may be used, and it is the kind of thing that silently corrupts an analysis:
**consecutive files cannot be assumed to be equally spaced**, so a rate computed as "difference divided by the number I
thought the interval was" is wrong, and so is ordering them by filename number alone if a save was made out of sequence. The
tick counter inside each save is therefore not a convenience - it is the precondition for using the series at all, and it is
the first field the reader must find.

**First probe, negative and worth recording:** across `brit1`…`brit13` (inflated 3.96 MB → 4.57 MB), there is **no u32 in
the first 8 KB of the body that increases monotonically across all thirteen files** and sits in a plausible tick range
(500 … 5,000,000). Two readings: either the header contains variable-length parts (player names, map data) so a fixed offset
does not line up between saves, or the clock is not a plain little-endian u32 there. Both say the same thing - a reader
needs the format's own structure (a section walk), not an offset guessed from a scan. That is the next step, and it starts
from the same `VER 9.4` header the recording shares.


## Errata (2026-11-01): there is no `HA` magic, and the 8-of-13 puzzle is the save version

Two claims in this file are corrected by `docs/save-format.md` and `tools/savegame.py`:

1. **`VER 9.4\0\0\0HA\0` is a float seen as text.** Bytes 8-11 of a save are a **float32 save-game version**, and
   `12.5f` is `0x41480000` - whose upper two bytes are the ASCII `HA`. The tag is not part of the format; the eight
   older saves read `12.36` there (`0x4145c28f`, bytes 10-11 `EA`) and have no `HA` at all. A reader that required the
   tag refused eight of the user's own saves.
2. **The step-2b split "8 of 13 yield no name records" is the save version, not campaign-vs-random-map.** The eight
   files that yield no `0x0A60`-framed records are exactly the eight at version 12.36; every 12.5 file yields 4-15.
   Whether 12.36 stores names in another encoding or stores none is still unknown.
