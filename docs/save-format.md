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

**The clock is in section 4**, 4 bytes at `world+0x100`; what is missing is only its **byte offset in the file**, and
that needs sections 1-3 walked (`&player_info[i].Food` and friends are in section 2, which is what makes the stockpile
question the same walk).

**A scan that finds nothing usable, with the numbers.** Looking for a `u32` - and separately a finite `float32` - that
increases across `brit1`..`brit13` at one fixed offset, over the common 3.96 MB and at **all four byte phases**:

| byte phase | increasing `u32` | increasing `float32` |
|---|---|---|
| 0 | 0 | 0 |
| 1 | 9 | 9 |
| 2 | 9 | 9 |
| 3 | 2 | 2 |

So the honest statement is **not** "there is no such field" - it is that every hit is a coincidence of alignment. The
layouts shift between saves, so the same offset holds a different structure in `brit1` and in `brit13`: the three hits
inspected (`0x224c52`, `0x270f52`, `0x22662e`) each have a *different* neighbourhood in the two files, and the 17
increasing values are unrelated binary that happened to land in order. **Do not build on them.**


**The experiment that settles the clock in one run**, and it is cheaper than the rest of the walk: two saves from one
match a known interval apart (60 s of game time, nothing else changing). The differential then has one field that moved
by exactly `60 x tick_rate` and the guesswork disappears. The owner can produce this; nothing on this box can.

## The tool

`tools/savegame.py` - `inflate`, `Header.parse`, `iter_strings`, and a one-file description on stdout. `inflate` refuses
a body past `DEFAULT_MAX_BODY` (64 MiB) because 200 MB from a 204 KB stream is a measured ratio of this format, and it
requires the **save line** (`VER 9.3`/`VER 9.4`) because a `.dat` is raw DEFLATE too. Its tests are
`tools/test_savegame.py`: synthetic fixtures for the framing and the header (so a checkout without an install still
tests the reader), plus two real-fixture classes keyed on committed SHA-256 values, skipped when the install is absent.
