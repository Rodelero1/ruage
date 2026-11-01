# The save reader: what it costs, and what was attacked (2026-11-01)

Scope: `tools/savegame.py` and `tools/test_savegame.py`, new. No engine code, no `config/`: the tick loop cannot see
this change, and the only artifact is analysis tooling that runs against the user's own install at runtime and writes
nothing. The format register it implements is `docs/save-format.md`.

## Performance

Measured on this container (2 CPUs, 4 GiB), best of five, in-process, nothing else building:

| what | number | how |
|---|---|---|
| inflate one 4.0 MB body from 263 KB on disk | **8.2 ms** | `savegame.inflate` on `brit1.aoe2spgame`, best of 5 |
| walk every string record in that body | **3.67 ms** | `list(iter_strings(body))`, best of 5, 6 records found |
| all 18 saves in the install | **652 ms** for 217 MB inflated (**333 MB/s**) | one pass, in-process |
| all 31 files (saves + the four recordings' header sections) | **0.69 s**, 31 read, 0 refused | one pass, in-process |
| the tool's suite | **70 ms**, 18 tests | `python3 -m unittest discover -s tools -p 'test_savegame.py'` |
| the whole `tools/` suite | **67 ms**, 29 tests | `python3 -m unittest discover -s tools -p 'test_*.py'` |

The walk is a `bytes.find` scan for the tag, so it is linear in the body: 4 MB costs 3.67 ms, and a 20 MB campaign save
costs about 18 ms. That is the right shape for a byte scan and it is not on any hot path. **zlib is the whole cost**,
which is why the reader does not attempt anything cleverer.

## Stability

Attacked, with the outcome of each attack run for real rather than asserted:

- **A `.dat` mistaken for a save.** `.dat` files are raw DEFLATE too and their body starts `VER 5.7`
  (`empires2_x2_p1.dat`: 2.26 MB on disk → 13.7 MB, body starts `VER 5.7`). The first version checked only the `VER `
  prefix and **accepted the database as a save**. It now requires the save line (`VER 9.3` / `VER 9.4`); the dat is
  refused with `'VER 5.7' is not a save-line version`.
- **An unbounded inflate.** A 204 KB stream inflated to **200 MB** of zeros in the probe. On a 4 GiB cgroup that is a
  hostile file away from an OOM, so `inflate` now streams against a **64 MiB** cap and refuses past it before
  materialising; pinned by `test_an_inflated_body_past_the_cap_is_refused_rather_than_filling_memory` and by a test on
  the shipped constant.
- **Truncated / trailing / random bytes.** `brit1` cut to 1 KB → "the DEFLATE stream ends before the file does";
  `brit1` plus 4 junk bytes → "4 bytes after the end of the stream"; 2,560 random bytes → "not raw DEFLATE"; empty file
  → "empty file". A recording's *container* (length prefix + deflate) is refused, because it is not what this reads.
- **A version the game itself would refuse.** `NaN`, `+inf`, `13.0`, `-1.0` are all refused, because the range check is
  the original's own (`FUN_005e6730` refuses above 12.5) rather than a guess - so the reader cannot claim to understand
  a format the game would not load.
- **A length field that lies.** A record whose length runs past the buffer, a zero length, a 300-byte length and a
  4-byte binary payload are all rejected; the walk advances one byte at a time on a rejected candidate, so it cannot
  skip a real record that overlaps a false one.
- **Determinism of the walk.** Two passes over the same 4 MB return the same list, in the same order
  (`test_the_walk_does_not_depend_on_what_it_has_already_seen`) - a cursor mutated on a rejected candidate would drift.

### My own errors, in this slice

- **The header was misread, and the misreading was in the tests as well as the code.** I read bytes 10-11 of a save as
  the ASCII `HA` and made it a required magic, because the binary's default-header initialiser writes `0x41480000` and a
  previous session's notes quote `VER 9.4\0\0\0HA\0`. It is not a tag: `12.5f` **is** `0x41480000`, whose upper bytes are
  `48 41`. The reader therefore **refused eight of the user's own saves** (`no HA tag at offset 10: b'\x8f\xc2EA'`) until
  the float reading was forced on it. The correction is recorded as errata in `docs/save-format.md`, with the two
  independent confirmations (12.36 in the eight older saves; 12.5 is the game's own ceiling).
- **The dat/save confusion above** was found by attacking my own reader, not by a user report.
- **The cap was absent** in the first version for the same reason: I had only measured files I trusted.

## Not attacked, and named

- The CLI holds the whole inflated body in memory (the cap bounds that at 64 MiB); there is no streaming mode and none is
  needed yet.
- The string walk's acceptance rule is "tag, plausible length, printable bytes". A body engineered to contain many
  `60 0a` pairs with printable payloads would produce false records; nothing downstream trusts the list structurally
  yet, so this is noise rather than a hazard.
- No fuzzing beyond the cases above; the reader's only parser-shaped surface is the header, and it reads four fields.
- **The eight 12.36 saves yield zero framed records.** That is a finding, not a defect (see `docs/save-format.md`), but
  it means the string half of this tool is untested against that half of the corpus - there is nothing there to find.
