# The save-game format, as far as it is verified (2026-11-01)

`tools/savegame.py` reads a `.aoe2spgame`'s container, header and named records. This file is the register behind it:
what is **verified** (and by what), what is **inferred**, and what is still **unknown** with the code that will settle it.
Append to it; do not re-derive what is written here.

    python3 tools/savegame.py Age2HD/SaveGame/brit1.aoe2spgame
    python3 -m unittest discover -s tools -p 'test_savegame.py'

Nothing in this document is read at runtime, and no file it describes is committed (D8/D22): the reader runs against
the user's own install.

## Verified

| Fact | Evidence |
|---|---|
| A save is **raw DEFLATE from offset 0**, no container header, and the stream ends exactly at EOF | 31/31 files inflate with `eof` set and zero `unused_data` (`inflate`, and `test_the_container_is_raw_deflate_that_ends_exactly_at_eof`) |
| Bytes 0-6 are `VER 9.4`, byte 7 is NUL | 31/31 files; the binary's own default header is built the same way (`FUN_005e60b0` writes `0x20524556`, `0x332e39`) |
| Bytes 8-11 are a **float32 save-game version**: **12.5** in the 2018 build, **12.36** in the older saves | 23 files read 12.5, 8 read 12.36; the loader `FUN_005e6730` refuses `< 0` or `> 12.5` (`"Invalid save game version %f"`) and branches its field list at **12.35**, **12.37** and **12.38** - so a version-dependent header is the format's own design, not a decoding choice |
| Bytes 12-15 are a second float, **1000.0 / 1005.0 / 1006.0** across this install | measured on all 31 files; **its meaning is not known** and it is exposed as `Header.unidentified` for that reason |
| A **string record** is `u32 (0x0A60 << 16 | length)` followed by exactly `length` bytes | Two independent sites enforce it: the reader `FUN_004340d0` tests `(word & 0xffff0000) == 0x0a600000` and takes the low half as the length (`"Error Loading String, signature bits not found"` on a mismatch), and the writer `FUN_004360e0` writes `length | 0x0a600000` under the diagnostic name `&theLen` before `m_Data`. Matches the file: `0x0a600009` + `Promisory` at 0xa4 |
| The save line is **9.3/9.4**, and that is what distinguishes a save from a `.dat` | `empires2_x2_p1.dat` is raw DEFLATE too and its body starts `VER 5.7`: 2.26 MB on disk, 13.7 MB inflated. A reader that accepts any `VER ` prefix calls the database a save (it did, until this test) - `test_a_dat_file_that_inflates_is_not_mistaken_for_a_save` |
| The world clock is **not** `old_time` | `FUN_00730600` (the BaseWorld loader, `baseworld.cpp`) reads `&old_time` into the world object and then **overwrites it with `timeGetTime()`** when it is non-zero. It is a wall clock for pause accounting, not a game clock. Chasing it produces a number that looks plausible and is wrong |
| The world's own turn counter is read 4 bytes at a time, early in the world section, into `world+0x100` | same function, line ~448 (`(**(code **)(*param_2 + 0xc))(&local_b0, world + 0x40, 4)`, in `int` units); the debug print two hundred lines later labels exactly that field `pturn` (`"load (temp_pause=%d, old_time=%d, pturn=%d)"`) |
| **The game clock is a `u32` in game milliseconds, whole seconds only** | found at **`0x270f52`** in the `brit` match by `find_game_clock`, and confirmed three ways - see "The clock" below. It is *not* at a portable offset: a second match in the same set has unrelated bytes there |

## The corpus, measured

31 files: 18 `.aoe2spgame` (13 `brit`, `britXhuns`, and the older campaign saves) and 4 `.aoe2record`, plus the 9
historical saves. Reading all 31 took **0.74 s**, 0 refused.

| save version | files | word at 12 | 0x0A60-framed strings |
|---|---|---|---|
| 12.5 | 23 (the four 2026 recordings, `brit1`-`brit13`, `britXhuns`, `Alaric`, `Bari`, `Random_map`, `khmer`, `sforza`) | 1006.0, except `Bari` and `khmer` at 1005.0 | 4-15 per file |
| 12.36 | 8 (`1`, `Berber`, `ETHIOP`, `Magyars`, `Magyars1`, `Persia`, `malinese`, `portuguese`) | 1000.0 | **0** |

**That second row closes an open question in `docs/oracle.md`.** It recorded that 8 of 13 saves "yield no name records"
and guessed at campaign-vs-random-map or patch revisions. The split is now exact and mechanical: **the eight files that
yield no `0x0A60` records are the eight at version 12.36.** Whether they store names in another encoding, or store none,
is not known - only that this framing finds nothing in them, which is a statement about the framing.

## Errata

- **`VER 9.4\0\0\0HA\0` was a misreading, and it was mine.** `docs/oracle.md` and the 2026-11-01 note both quote a `HA`
  word in the header, and the first version of this reader *required* it - which is how it came to refuse eight of the
  user's own saves (`no HA tag at offset 10: b'\x8f\xc2EA'`). There is no `HA` tag: `12.5f` is `0x41480000`, whose upper
  two bytes are `0x48 0x41` - the ASCII `HA`. The initialiser that made it look deliberate (`FUN_005e60b0` writing
  `0x41480000` into the third word of a default header) is setting a **default save version of 12.5**, not writing a
  magic. Reading the word as a float is what makes the *whole* corpus decode under one rule, and 12.5 is the exact
  bound the game's own loader enforces.
- A `PTR` to the string table is not the string table: `Alaric`, `Bari` and `sforza` carry 15 framed records and
  `Random_map` 6, so a count of records is not a count of players.

## The clock, and the recordings that confirm it (2026-11-01)

**A timed series, from the files themselves.** The COPY destroyed the save files' timestamps (all 31 are stamped
`2026-11-01 15:00:18.17`-`.26`, in alphabetical order - that is the moment they were copied here, not the moment they
were written), so the interval could not come from the filesystem. It did not have to: `find_game_clock` looks for a
`u32` that advances across the series in **whole seconds**, and exactly one offset survives for the thirteen `brit`
saves:

| build | clock | interval from the previous save |
|---|---|---|
| `brit1` | 10.0 s | - |
| `brit2` | 181.0 s | +171.0 s |
| `brit3` | 295.0 s | +114.0 s |
| `brit4` | 380.0 s | +85.0 s |
| `brit5` | 526.0 s | +146.0 s |
| `brit6` | 648.0 s | +122.0 s |
| `brit7` | 964.0 s | +316.0 s |
| `brit8` | 1 156.0 s | +192.0 s |
| `brit9` | 1 171.0 s | +15.0 s |
| `brit10` | 1 429.0 s | +258.0 s |
| `brit11` | 1 941.0 s | +512.0 s |
| `brit12` | 2 226.0 s | +285.0 s |
| `brit13` | 2 681.0 s | +455.0 s |

**Three independent confirmations that this is the game clock and not a counter that happens to rise:**

1. **The recording's own length.** `SP Replay v5.8 @2026.11.01 142146.aoe2record` is the same match, and its op stream
   sums to **2 681 138 ms**. `brit13` reads **2 681 000 ms** - 138 ms apart, i.e. the save was taken 0.14 s before the
   recording's last op. Two artifacts, one number.
2. **The AI's chat timeline.** The recording's first chat line is at **852.4 s** ("Military level: 0."). No save from
   `brit1` (10 s) to `brit6` (648 s) contains a single chat string; `brit7` (964 s) contains exactly one. A clock that
   was wrong by minutes would put a chat in a file that cannot have it.
3. **A discriminator that only the last two saves can pass.** The `Build-town-size` lines first appear at 1 664.9 s and
   reach 24/26/28/30/32 at **2 277-2 597 s**. `brit12` (2 226 s) contains **none** of them; `brit13` (2 681 s) contains
   all five. That pair is the check that no window-based or size-based coincidence explains, because the two files are
   one save apart.

**Two things it is not.** The offset is **not portable between matches**: `britXhuns` - a different match - holds
`0x01000000` there, which is neither seconds nor increasing, and adding it to the set makes the candidate disappear
(`test_a_second_match_in_the_set_removes_the_candidate`). And a byte-shifted read of the same four bytes also looks
like a clock (`v << 8` is still a multiple of 1000 whenever `v` is a multiple of 125), so `find_game_clock` drops
aliases rather than reporting one field twice.

**What this unblocks, and what it does not.** Any two consecutive saves now have a known interval, so gather rates
(row 6), the age-up times (row 4) and decay-by-absence (row 5) can be read as soon as stockpiles and entities are
readable - the interval was the precondition and it is paid. It does **not** replace the section walk: the clock is
found empirically, so every new match needs its own series (or a recording, which supplies the same anchor).

## The archive's timestamps, and what they confirm (2026-11-01)

`SaveGame.zip` arrived from the owner and carries the **original** modification times (a copy loses them; an archive
keeps them). All 32 entries are byte-identical to the files already here - **32 identical, 0 differing, 0 missing** - so
nothing that was measured before this archive existed changed, and the timestamps are now available:

| save | written | wall gap | in-file clock | game gap | game / wall |
|---|---|---|---|---|---|
| `brit1` | 14:22:00 | - | 10 s | - | - |
| `brit2` | 14:24:04 | 124 s | 181 s | 171 s | 1.38 |
| `brit3` | 14:25:30 | 86 s | 295 s | 114 s | 1.33 |
| `brit4` | 14:26:32 | 62 s | 380 s | 85 s | 1.37 |
| `brit5` | 14:28:16 | 104 s | 526 s | 146 s | 1.40 |
| `brit6` | 14:29:46 | 90 s | 648 s | 122 s | 1.36 |
| `brit7` | 14:33:30 | 224 s | 964 s | 316 s | 1.41 |
| `brit8` | 14:35:50 | 140 s | 1 156 s | 192 s | 1.37 |
| `brit9` | 14:36:06 | 16 s | 1 171 s | 15 s | 0.94 |
| `brit10` | 14:39:08 | 182 s | 1 429 s | 258 s | 1.42 |
| `brit11` | 14:45:02 | 354 s | 1 941 s | 512 s | 1.45 |
| `brit12` | 14:48:24 | 202 s | 2 226 s | 285 s | 1.41 |
| `brit13` | **16:44:48** | (6 984 s) | 2 681 s | 455 s | - |

**This is a second, independent confirmation of the clock** - the wall clock and the in-file clock were produced by
different things (the filesystem and the game) and they agree on the *shape* of every interval: ten of eleven ratios
sit in **1.33-1.45**, mean **1.39**. A field that was wall time would read 1.00 there; a field that was anything else
would not track eleven independent gaps. It also says the match ran at a little under the nominal 1.5x - consistent
with the save menu pausing the simulation and with frame slips, and not separable from either without a controlled run.

**Two timestamps are not save times, and the archive says so plainly:** `brit13` reads **16:44:48** and the recording
`142146` **16:44:58** - the minute the archive itself was written - so both were touched (re-written or copied) long
after the match. Every other file in the set is coherent. Their *content* is still dated: `brit13`'s clock is 2 681 s
and the recording's op stream is 2 681.1 s, which is why the clock was confirmed before the archive arrived.

**And one correction to how the recordings were read.** Their filenames are the moment the recording **began**, not when
it ended: `SP Replay v5.8 @2026.11.01 142146` started at 14:21:46 and runs 44.7 game-minutes, so it ends around 14:53 -
and `brit1` (game clock 10 s) was written at 14:22:00, seconds after that recording started. The recording and the save
series are the same match from the same starting moment, which is the third thing this archive settles.

**The other dates name the install's history** - the previous owner's campaigns, two years before this one: `1` and
`ETHIOP` 2015-11-07, `Persia` and `Magyars` 2015-11-25, `Magyars1` 2015-12-12, `portuguese` 2015-12-17, `malinese`
2016-01-18, `Berber` 2016-08-13, `khmer` 2016-12-23, `Bari` 2017-11-21, `Alaric` 2019-03-23, `sforza` 2019-04-17,
`Random_map` 2020-06-10. **The save-version split is a build split and it tracks the dates**: all eight `12.36` saves are
2015-11 to 2016-01, and the `12.5` saves run from `khmer` (2016-12) forward - so a patch between January and December
2016 raised the header to 12.5, which is what "version-dependent header" means in practice.

## The recordings walk to exact EOF (2026-11-01)

The op stream after the deflated header is: a **28-byte meta block** (log version, checksum interval, multiplayer,
rec owner, reveal map, sequence numbers, chapters) then `[u32 op][payload]`, with the payload grammar mgz documents
(MIT, `mgz/body/__init__.py`): action = `u32 length` + `length+4` bytes; sync = `u32 increment` + (if the next word is
zero) a 28-byte checksum record, +332 bytes when its sequence is non-zero; viewlock = two floats + a player id;
chat = 4 pad + `u32 length` + text. All four files consume to **exact EOF**:

| recording | ops | game time |
|---|---|---|
| `141408` | 652 | 24.7 s |
| `141505` | 261 | 9.6 s |
| `141539` | 9 664 | 372.9 s |
| `142146` | 111 494 | 2 681.1 s |

`mgz` 1.8.51 itself **fails** on these HD 5.8 files (`could not parse: unpack requires a buffer of 2 bytes`) because it
expects a different meta preamble - the grammar above is what its own body spec says, driven by hand and validated by
the EOF check. **It is in the tree now:** `tools/recordings.py` (`split`, `iter_ops`, `Op`) walks `142146`'s 111,494
ops in **128.8 ms** and yields chat text and action bodies; `tools/test_recordings.py` pins the grammar with synthetic
streams and the real file's op count, elapsed time (2 681 138 ms) and the AI's resignation as the last chats. A recording therefore needs no save reader to be useful, which is what dates a series and what supplies
the "known interval" when the copies have lost their timestamps.

## What is still unknown, and where the code that answers it lives

The world follows the scenario and the map in the same byte stream, in this order. All addresses are image-base
coordinates in `AoK HD.exe` (32-bit PE, 2018-08-22) and all quotes are from the exported pseudocode corpus
(`/opt/data/tools/rea/pseudo-aokhd`, 15,465 functions, see `docs/re-toolchain.md`).

| # | Section | Reader | What it holds |
|---|---|---|---|
| 1 | header | `FUN_005e6730` (`savegame.cpp`?) | version, then version-dependent: `guid` below 12.38, `mSaveGameName`/`dateTime`/`saveGameChecksum` at 12.38+, `mRandomSeed` |
| 2 | scenario | `FUN_00545460`, `FUN_00567630` (1,454 lines) | `&name`, `mPlayerActiveArray[i]`, `PlType[i]`, `PlCivilization[i]`, `player_info[i].{Food,Wood,Gold,Stone}`, victory conditions, disabled techs, `mPlayerAge[i]`, `checksum` |
| 3 | map / tiles | `FUN_00836830` | `map_width`, `map_height`, `world_width`, `tilesizes`, `num_borders`, `max_terrain`, then the per-tile arrays |
| 4 | world (BaseWorld) | `FUN_00730600` (1,418 lines, `baseworld.cpp`) | a version word, `mLocalPlayerIndex`, `numPlayers`, `tempGameState`, three campaign indices, **`pturn` at +0x100**, then the object loop |

**The world's turn counter is in section 4**, 4 bytes at `world+0x100`. Its byte offset in the file is still unknown,
and that needs sections 1-3 walked (`&player_info[i].Food` and friends are in section 2, which is what makes the
stockpile question the same walk) - but note that **the clock itself no longer needs it**: `find_game_clock` finds the
game time in milliseconds from a series alone (see "The clock" above). What the walk still buys is everything the
clock does not: stockpiles, population and the entity list.

**The scan that found it, and the two times it was dismissed.** Looking for a `u32` or a finite `float32` that
increases across `brit1`..`brit13` at one fixed offset, over the common 3.96 MB and at **all four byte phases**:

| byte phase | increasing `u32` | increasing `float32` |
|---|---|---|
| 0 | 0 | 0 |
| 1 | 9 | 9 |
| 2 | 9 | 9 |
| 3 | 2 | 2 |

**One of those phase-2 hits was the clock, and I had called it a coincidence.** `0x270f52` appeared in that list with
the values that are now the table above; I inspected its neighbourhood, saw different bytes in `brit1` and `brit13`,
and concluded that the layouts shift so the hit must be noise. The neighbourhood *does* differ - that is what a live
field in a growing world looks like - and the conclusion was wrong. Two lessons, both paid for:

- **"Increasing at a fixed offset" is evidence, not noise.** What it needs is a second, independent filter, not
  dismissal. The filter that separates the clock from the rest is that the game writes **whole seconds** (`v % 1000 ==
  0` at *every* file), which leaves exactly one candidate out of 5,466; and the confirmation is a second artifact (the
  recording's length), not a reading of the neighbourhood.
- **A shifting layout does not mean fixed offsets are meaningless.** It means a fixed offset cannot be *trusted* - so
  it is found per match and then checked, which is what `find_game_clock` does.

The other phase hits did not survive: `0x224c52`/`0x224d1a` rise in milliseconds rather than whole seconds, and
`0x270f51` reaches 686,336,020 ms - 7.9 days, past the 24-hour ceiling on a match. Those are the coincidences.

**For a new match, the anchor is one of two things** - and the `brit` set needed neither, because the recording was
already there: either a series of saves from that match (13 was enough; the whole-second filter leaves one candidate),
or a recording of it, whose total length pins the last save. **The old ask - two saves a known interval apart - is no
longer needed.** What is still owed is the *content* of the state: stockpiles, population and entities, which the
clock cannot supply and sections 2-4 do.

**What the save does not hold, and a load must build (2026-11-04, from genie).** `genie-core` splits the two loads
(`crates/engine/genie-core/src/world/load/world/loaded.rs`): a world loaded from a **recording's header** gets a fresh
objects-in-sight registry sized to its players, the hunted animals' origins and the watched (ghost) lists; a world loaded
from a **save** gets none of them, and genie's test asserts the registry's absence for the save case. The game's own load
builds them, so the bytes of a save are not by themselves a start state: if our reader ever resumes a match rather than
reproducing one, these are the structures it has to reconstruct, and they are not in the file.

## The tool

`tools/savegame.py` - `inflate`, `Header.parse`, `iter_strings`, and a one-file description on stdout. `inflate` refuses
a body past `DEFAULT_MAX_BODY` (64 MiB) because 200 MB from a 204 KB stream is a measured ratio of this format, and it
requires the **save line** (`VER 9.3`/`VER 9.4`) because a `.dat` is raw DEFLATE too. Its tests are
`tools/test_savegame.py`: synthetic fixtures for the framing and the header (so a checkout without an install still
tests the reader), plus two real-fixture classes keyed on committed SHA-256 values, skipped when the install is absent.

## Where the objects stand: 229-byte records, x and y as float32

**Found 2026-11-12** while measuring a berry bush, and it answers the question the order queue could not: the queue records a
**destination** and the issuer's position when the order was given, so a click made while men are walking yields approaches, not
standing places. The objects themselves are elsewhere in the world section: a run of records of **229 bytes** whose first eight
bytes are **`float32 x, float32 y` in tiles**, and `world_x = column + 0.5` (a tile centre reads as `64.500`).

The way it was found, because the same scan works on any save: take the window around the thing being measured, read every
offset as a float pair, keep those inside the window, and look for the **longest chain at a constant stride**. The bush save gives
21 records at stride 229 from `0x19fee8`; the same save's order queue gives 33 records of destinations.

**What a measured ring looks like**, from `bush.aoe2spgame` around the bush square at `(64, 78)`:

```
radii from (64.50, 78.50): 0.699 0.699 0.699 0.759 | 0.995 0.995 0.995 0.995 | 1.100 1.103 1.103 1.140 1.145 1.159 1.161 1.166 1.174 1.174 | 1.627 1.798 1.975
```

- **`1.10-1.17`, ten men**, closest pair **0.400**, mean nearest **0.401** - the working ring, and the same radius the tree save
  shows (1.105-1.189) and the same spacing (0.404). **A bush and a tree share the ring.**
- `0.995` four times - the neighbouring tiles' centres, exactly one tile out: men walking through, whose positions happen to be
  waypoints.
- `0.699` three times - the tile's corner distance `0.5 * sqrt(2) = 0.707`, i.e. bodies just off the corners, walking past.
- anything past 1.2 is somebody else's business.

`tools/savegame_positions.py` does the scan and the radii for a given square:

```sh
python3 tools/savegame_positions.py <save> --window 62 76 68 82 --centre 64.5 78.5
```

**All three node types share the ring (measured 2026-11-12).** Using `tools/savegame_positions.py` on the owner's three saves:

| node | save | men on the ring | radii | mean | closest pair |
|---|---|---|---|---|---|
| tree | `test.aoe2spgame` (2nd) | 6 | 1.105-1.189 | **1.156** | 0.404 |
| bush | `bush.aoe2spgame` | 10 | 1.100-1.174 | **1.143** | 0.400 |
| mine (gold) | `mine.aoe2spgame` | 8-9 | 1.041-1.188 | **1.114** | 0.24-0.40 |

The mine's closest pair is polluted: that save stores each object's position **more than once** (copies 0.10 apart, gaps of 68 and
30 bytes between them), so `--near` has to collapse them and two survivors can still be one body. Its *radii* are the sound number
and they sit in the same band. The owner's own summary - *"tree, mine and berries are all the same in that aspect"* - is what the
measurements say.
