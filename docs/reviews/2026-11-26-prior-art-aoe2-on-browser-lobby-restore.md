# 2026-11-26 - prior-art watch: aoe2-on-browser's lobby restore, and a save layout that does not transfer

Documentation only: **48 lines added to `docs/prior-art.md`**. No `crates/`, `config/`, `tools/`, `build/` or `Age2HD/` file
touched; no test, no fixture, no production code. The only due project was `aesaganda/aoe2-on-browser`, whose head moved from
`3d5eb6d3` to `35361665`.

## Performance

- **The range is four commits, two of them merges, and four files**, all from the compare API (`ahead_by: 4`,
  `total_commits: 4`, `files: 4`): `src/linux/dplay_stub.c` +38/-6, `src/net/dplay_net.c` +33/-1, `tools/net/relay.py`
  +7/-4, `web/page.js` +43/-11 - **+121/-22**, a 327-line patch. Nothing is hidden behind the merges: `4115e738` merges
  `55c00872` (player names) and `35361665` merges `c7ce335b` (lobby restore), each a single substantive commit.
- **Cost to the engine: zero.** No code path in this tree changed. `cargo clippy --offline --all-targets -- -D warnings` is
  clean in **10.46 s**; the full suite is **228 test binaries, 946 passed, 0 failed, 25 ignored**
  (`/opt/data/cache/scratch/test-2026-11-26.log`, outside the tree). The 2026-11-23 watch reported 222/945, so the delta is
  other sessions' tests, not this one's: a docs-only change runs no new test.
- **The one measurement this note adds** is on our side, not theirs: `Age2HD/SaveGame/brit1.aoe2spgame` inflates to
  3,963,296 bytes and the two name-carrying blocks sit at `0xa4` and `0xfa` - **0x56 apart**, against AoC's fixed `0xA0`.
  One probe, one file, no dependence on the game's data being committed.
- **The base moved while this note was being written.** The gates above ran on `45a8b28`; `origin/main` then gained the
  ADR migration merge (`a0b1720`, docs-only - 20 commits moving research files under `docs/archive/research/`), and this
  commit was rebased onto it. The rebase needed one edit, not a re-run: `docs/archive/research/dependency-tree.md:152` is
  where M24's row now lives. `docs/reviews/INDEX.md` was regenerated **after** the rebase, so it counts 336 notes over 26
  dates rather than the 315 this tree had at `45a8b28`.
- **`cargo fmt --check` is red on HEAD, at `crates/sim/tests/gather.rs:527`**, unchanged from 2026-11-23 - the same
  `let Some(..) = ..` pattern in a file landed by `ba0aba8` (D353), another session's commit. Left alone again, for the same
  reason, and named here rather than silently reformatted.

## Stability

What was attacked:

1. **"Nothing was missed in the range."** The compare response and the served patch agree on commit count (4), file list (4)
   and file order; every file in the diff is one of the four cited. The two merge commits each have the substantive commit in
   the range, so no PR content arrived only through a `main` update.
2. **The citations.** Each sha was fetched and the note's line numbers read in the final file, not in the hunk:
   `dplay_stub.c:79-96` (`PRESET_SIZE`, `restore_preset` and its offsets), `:107-109` (the reply), `dplay_net.c:170-186`
   (`restore_slot`), `:828-830` (`sd + 64`), `relay.py:135-140` and `:262-264` (the room's `restore` field and its parse),
   `page.js:606-611` (`Host`), `:1106-1115` (`defaultPlayerName`). Six of these were **wrong on the first pass** - the diff
   hunks' `@@` numbers were used as line numbers, which is exactly the mistake the previous note's check 2 was written for; the
   numbers in `docs/prior-art.md` are the fetched ones.
3. **The struct is a layout and not a guess.** The offsets the note lists for `AgePresetData` are checked against each other:
   `m_sGameFilename[260]` at `0x17` ends at `0x11B`, where `m_bMapTypeAvail[37]` starts, which ends at `0x140`, one byte
   before the map-size field at `0x141`. Three members agreeing to the byte is evidence the reading is a layout.
4. **The one portable-looking fact, tested against our own file.** `0x5D4 + 0xA0*n` is AoC's player table. A raw dump of
   `brit1.aoe2spgame` shows names as `0x0A600000|len` framed strings in blocks `0x56` apart, so the stride does not transfer;
   the note says what is **not known** (whether HD has any fixed player table at all) instead of inventing a reader for it.
5. **"No U-number moves."** Read U12's and U23's rows: U12 is the tick model and a restore runs the existing replay machinery;
   U23 is map sides and a lobby preset's map-size *choice* is not a tile dimension. Neither is answered, and the note says so
   rather than trading on the proximity.

**The failure no check covers:** whether that `0x5D4` search is correct even for AoC - their own comment admits a one-letter
name matches the first block whose opening bytes match, and nothing in their code checks that the matched name is a player
rather than a duplicate string elsewhere in the header. It does not matter to us (we are not porting it) but it is why the
note records the AoC table as *their* reading and not as a verified fact.

**My own error this run.** `cargo fmt --check 2>&1 | head -20; echo "FMT_EXIT=$?"` printed `FMT_EXIT=0`, which is `head`'s
status, not rustfmt's; the first draft of this note would have called formatting green. Re-ran it unpiped into a log: exit
**1**, one diff, at `gather.rs:527`. A pipeline to `head` had already hidden a test total on 2026-11-23; this is the same
mistake in a different command.

## The AI review

`make ai-review` selects **0 files** for a docs-only diff (default `OCR_EXCLUDE=docs/**`), so it is not run for this tick; the
note rests on the five checks above rather than on a reviewer's findings, and says so instead of implying a review happened.
