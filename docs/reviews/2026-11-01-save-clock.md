# The save clock: what it costs, and what it survived (2026-11-01)

Scope: `tools/savegame.py` gained `find_game_clock`, `gaps`, `show_series` and `--series`; `tools/test_savegame.py`
gained two classes (11 assertions). No engine code. The finding it lands is written up in `docs/save-format.md`:
the `brit` series' game clock, the interval between the thirteen saves, and the three independent confirmations.

## Performance

Measured on this container (2 CPUs, 4 GiB), in-process, nothing else building:

| what | number | how |
|---|---|---|
| the clock scan over 13 saves (3.96-4.57 MB each) | **0.96 s** wall, including inflating all thirteen | `python3 tools/savegame.py --series brit{1..13}` |
| of that, the byte scan itself | **0.91 s**, 5,466 candidate offsets from the first file | timed loop over `range(len(brit1)-4)` |
| candidates surviving "whole seconds in every file" | **1** (of 5,466) | `find_game_clock` |
| candidates surviving *increasing* at one offset, before the whole-second filter | 20 of ~4M offsets (all four byte phases) | the phase table in `docs/save-format.md` |
| the tool's suite | **2.4 s**, 27 tests | `python3 -m unittest discover -s tools -p 'test_savegame.py'` |
| the whole `tools/` suite | **2.8 s**, 38 tests | `python3 -m unittest discover -s tools -p 'test_*.py'` |

The scan is one linear pass over the first body plus a handful of `unpack_from` calls per *candidate*, which is why a
4 MB file costs a second rather than a minute. It is an analysis tool run by hand, not a per-tick path; the honest
cost statement is "about a second per series", and a 20 MB campaign save would make it about five.

## Stability

Attacked, with the outcome of each attack run:

- **A single file as a "series".** `--series brit1` → `1 files: 0 offsets advance in whole seconds, expected exactly 1`,
  exit 1. A one-file series cannot have an increasing field and the tool says so rather than inventing one.
- **The files out of order.** `--series brit2 brit1 brit3` → 0 candidates, exit 1. Ordering is the caller's claim and
  the tool refuses a set that contradicts it, instead of reporting a negative interval.
- **A second match in the set.** Adding `britXhuns` removes the candidate (`0x01000000` there is neither seconds nor
  increasing), pinned by `test_a_second_match_in_the_set_removes_the_candidate`. This is the check that stops the
  offset being treated as a constant.
- **A byte-shifted alias.** Reading the same bytes one or two positions early sees `v << 8` / `v << 16`, which is also
  increasing and also a multiple of 1000 whenever `v` is a multiple of 125 - in the synthetic fixture both offsets 99
  and 100 qualify. The first version of the filter dropped the **real** field and kept the alias (an inverted
  comparison, caught by `test_a_byte_shifted_read_of_the_same_field_is_not_a_second_clock` failing on 99 rather than
  100); it now keeps the smaller-valued member of an alias pair.
- **A value past a day.** `0x270f51` - the byte-shifted view of the real clock - reaches 686,336,020 ms, and the 24-hour
  ceiling rejects it, which is what keeps the shifted alias out of the real data too.
- **A value that rises but is not in whole seconds.** `0x224c52`/`0x224d1a` rise in milliseconds and are rejected;
  `test_a_field_that_advances_in_milliseconds_is_not_a_save_clock` pins it, and the doc says why those two are the
  genuine coincidences.
- **A non-increasing field.** Pinned (`test_a_field_that_does_not_advance_is_not_found`); without it a constant
  multiple of 1000 would be accepted for a series of length 2.

### My own errors, in this slice

- **I had already dismissed the clock once.** The phase-2 scan in the previous commit listed `0x270f52` with exactly
  the values that are now the clock, and I wrote that "every hit is a coincidence of alignment" on the strength of a
  look at the surrounding bytes. The neighbourhood *does* differ between saves - that is what a live field in a
  growing world looks like - and the dismissal was wrong. `docs/save-format.md` now carries the correction rather than
  the original claim, because the next session would otherwise skip the field that works.
- **My first alias filter was inverted** (see above), so it deleted the answer and kept the shadow.

## Not attacked, and named

- The scan is linear in the **first** file only; a series whose first save is much smaller than the others would
  truncate the search, because offsets are bounded by the shortest body. That is deliberate (an offset must exist in
  every file) and it is written down here rather than fixed.
- `find_game_clock` assumes the calls' order is chronological. There is no check that the values are *plausible* as a
  match (e.g. that the first is under a minute); a wrong order is caught only because the sequence stops increasing.
- The recording side - the op grammar that walks to exact EOF - is documented but not in the tool yet. It was used to
  confirm the clock by hand; putting it in `tools/` is the next slice, not this one.
