# The standoff is measured: 0.70 at an edge, 0.94 at a corner

*2026-11-13. The owner sent three saves and a screenshot: eight men on a tree, eight on a gold mine, eight on a bush, and
"pretty sure more men could fit in". This settles the question D192 raised against its own project's earlier measurement.
It also produced a defect in `tools/savegame_positions.py`, fixed test-first.*

## Performance

The measurement is a one-shot tool, not the tick path: **1.0-1.1 s per save** for a 7x7-tile window over a 3.26 MB inflated
body (`tools/savegame_positions.py`, unchanged cost), and **0.02 s** for the five new unit tests. Nothing in `crates/`
changed, so `World::step` and the acceptance hash (`91572e72c33cee93`) are untouched.

## The measurement

Each save is the same scenario with eight villagers on one node. The nodes were found without a guess: exactly three
tile-centre pairs appear nine times each in the coordinate stream of every save - `(76.5, 61.5)`, `(82.5, 66.5)`,
`(87.5, 71.5)` - and they sit ~7 tiles apart on a diagonal, matching the three clusters in the screenshot.

| save | node | radii of bodies round it (tiles) | on the touching band |
|---|---|---|---|
| `tree` | (76.50, 61.50) | 0.641, **0.701** x3, **0.935** x3; then 1.000 x4, 1.055 | **7**, mean 0.793, closest pair 0.661 |
| `mine` | (82.50, 66.50) | 0.641, **0.702** x3, **0.935** x3; then 1.000 x4, 1.055 | **7**, mean 0.793, closest pair 0.661 |
| `bush` | (87.50, 71.50) | **0.702** x3, 0.760, **0.936** x3; then 1.000 x4, 1.054 | **7**, mean 0.811, closest pair 0.663 |

- **0.70 is `0.5 + 0.2`**: the worker's body touching the node tile at an edge. **0.935 is `0.5*sqrt(2) + 0.2 = 0.907`** plus
  the same jitter a real click produces: his body touching it at a corner.
- **The exact 1.000 readings are the node's four neighbouring tile centres** and **1.414 the four diagonals**. Those are
  structural, not bodies: 1.000 and 1.414 come out of the scan bit-exact at every node in every save.
- The one body at **1.054-1.055** is a diagonal just outside the touching envelope. Unexplained; recorded rather than
  smoothed. It is not one of the four artefacts (those are exact) and not a ring member (one, not eight).

**So D192 is right and D186 was wrong.** The 1.156 "ring" that D186 read, and that
`docs/reviews/2026-11-12-the-mine-and-the-ring.md` repeated for three nodes (means 1.114-1.156), was **the artefact row**:
the old tool's default band was `1.05-1.20`, so it counted the neighbouring tile centres and diagonals and called them men.
That is the "row behind them" D192 suspected, and this is the first measurement that shows it directly.

**The owner's read is confirmed too.** Seven to eight bodies, spaced **0.66** apart on a ring of 0.70-0.94, is not a full
ring: the touching envelope of a one-tile node holds more, which is why "more men could fit in". Our engine's eleven
places (D192) is *more* permissive than the eight the game shows, so the capacity is not yet matched - but it is on the
permissive side, and the game's own spacing (0.66, not one body 0.40) says the real limit is the ground, not a number.

**Fixture provenance**, since these files must never be committed (D8/D22): read from
`/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…` (`tree`), `b9455053…` (`mine`), `bbc92744…` (`bush`), save
version 12.5, 283,966-284,548 bytes on disk, 3,260,876-3,263,704 inflated.

## The defect this found, fixed test-first

`ring_fit` computed every body's nearest gap with `min(... for j, q in enumerate(ring) if j != i)`, which raises
`ValueError: min() iterable argument is empty` when the band holds **exactly one** body. A lone worker on a node is the
commonest case there is, and the tool crashed on it:

```
$ python3 tools/savegame_positions.py .../bush.aoe2spgame --window 84 68 91 75 --centre 87.5 71.5
17 object positions (stride 303 x8)
radii from (87.50, 71.50): 0.000 0.702 0.702 ...
  File "tools/savegame_positions.py", line 70, in ring_fit
ValueError: min() iterable argument is empty
```

RED first, five tests in `tools/test_savegame_positions.py`: `test_one_body_on_the_ring_is_a_number_and_not_a_crash`,
`..._no_body_...`, `..._two_bodies_report_their_true_gap`, `test_the_default_band_is_the_touching_band`,
`..._corner_is_outside_the_edge_but_inside_the_band`. The first run was **`FAILED (errors=3)`**, two of them this
`ValueError` and one `AttributeError: no attribute 'DEFAULT_BAND'`. Then the fix: a one-body band returns
`(1, radius, 0.0, 0.0)`, and the default band moved from `1.05-1.20` to `0.60-0.98` - it was reporting the artefact row.
**5 tests pass** now.

**My own error, twice, in the tests themselves.** The first version of the one-body test placed the point at 1.70 with a
0.60-1.00 band, so it asserted `0 != 1` - a failure that proved nothing about the crash. The second version asserted that
1.000 should be *outside* a band whose upper bound was inclusive 1.00. Both are the same mistake the project has a rule
against: a test that fails for the wrong reason is not a red test. Corrected before the fix, and the corrected version is
the one that showed the `ValueError`.

## Stability

- **The tool's answer changed by moving its default**, not by changing how it reads bytes: the same window now reports 7
  bodies at mean 0.79 where it reported ~10 at 1.14. Anyone holding an old number must re-read it; the docstring, the
  `--band` help and `docs/save-format.md` now say which band is which and why 1.00 is not a body.
- **The windowed scan still reads non-position floats** - it is why the 1.000/1.414 artefacts appear at all. It is
  adequate for a small window round a known tile and useless without one: a whole-map scan invents thousands of
  positions. `tools/savegame_positions.py` documents the window as required input, and this review is the evidence for
  why.
- **Nothing in the engine moved.** D192's touching standoff was already the code; this confirms it rather than changing it.
- **Capacity is now the open number**, not the geometry: our eleven places against the game's observed seven-to-eight
  bodies at 0.66 spacing. Recorded in `docs/next-step.md`.

## What this changes in the record

1. `docs/decisions.md` **D199** - the standoff verified, D186's ring withdrawn.
2. `docs/reviews/2026-11-12-the-mine-and-the-ring.md` - erratum appended (that file says append, never re-derive).
3. `docs/save-format.md` - the "working ring" paragraph gets an erratum: the 1.10-1.17 band is the tile-centre row.
4. `docs/next-step.md` - the standoff question is closed; the queue moves to register row 2 and the Feudal gap.
