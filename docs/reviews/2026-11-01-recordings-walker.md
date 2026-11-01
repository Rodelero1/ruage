# The recorded-game walker: what it costs, what it refuses (2026-11-01)

Scope: `tools/recordings.py` and `tools/test_recordings.py`, new. Analysis tooling only - no engine code, nothing read
at runtime by the game. The grammar it implements is recorded in `docs/save-format.md` ("The recordings walk to exact
EOF"), and the numbers it reproduces are what confirmed the save clock.

## Performance

Measured on this container (2 CPUs, 4 GiB), best of five, in-process:

| what | number | how |
|---|---|---|
| walk `142146` (2.70 MB file, 2.44 MB op stream, 111,494 ops) | **128.8 ms** | `split` + `list(iter_ops(ops))` |
| the same per op | **1.16 us** | derived from the above |
| all four recordings, end to end (the CLI) | **0.4 s** wall including process start | `python3 tools/recordings.py *.aoe2record` |
| this module's tests | **0.27 s**, 8 tests | `python3 -m unittest discover -s tools -p 'test_recordings.py'` |
| the whole `tools/` suite | **2.7 s**, 46 tests | `python3 -m unittest discover -s tools -p 'test_*.py'` |

One `struct.unpack_from` per op and no copies except chat text and action bodies, which are slices (views in CPython).
The stream is walked in a single pass and never buffered, so a 100 MB recording costs about 5 s.

## Stability

Attacked, with the outcome of each run:

- **A stream that stops mid-operation.** A leaf action header claiming 900 bytes in a 30-byte stream raises
  `BadStream("action body of 900 bytes runs past the end")`; pinned by a test. A truncated recording is refused, not
  silently reported as a shorter match - the distinction matters because the *elapsed time* is the number being
  trusted.
- **An unknown operation id.** `BadStream("unknown operation 99")`. Guessing a length for an op the grammar does not
  name is how a walk drifts and then "reaches EOF" by luck.
- **A short trailing fragment.** Fewer than four bytes left raises rather than being skipped.
- **A container that is not one.** Section length zero, a section length larger than the file, or a header section that
  does not inflate to `VER ` all raise; the length is bounds-checked before it is used as a slice end.
- **A checksum record with a sequence.** The 28-byte record plus 332 bytes of per-player state is consumed only when
  the record's first word is zero, and the sequence is read at +16 - the layout mgz's `sync` struct specifies. Both
  branches are pinned by synthetic tests, and the real file exercises both (111,494 ops to **exact EOF** is the proof
  the branch logic is right, because a wrong branch drifts immediately).
- **Exact EOF.** The walk is only "right" when `pos == len(stream)`; all four recordings satisfy it. This is the check
  the binary-format rules demand and the one mgz itself cannot supply here, because its own parser refuses these HD
  5.8 files.

### My own errors, in this slice

- **Two of my synthetic fixtures were built to the wrong framing** - a checksum record with one 4-byte group too many,
  and an action whose length did not include the `+4` the grammar uses. Both failed on the first run and both were the
  *test's* fault, not the walker's; the walker was written from the walk that already consumed the real file to exact
  EOF, which is why the real-file tests passed while the synthetic ones failed.
- **The container assertion in my first test compared an inflated header's length to the file's** (`6379110 != 2697673`),
  which is nonsense: the header *inflates* larger than the section it came from. It now asserts the ops are exactly the
  file's suffix and that the header is larger than the deflated section.

## Not attacked, and named

- Action bodies are yielded as raw bytes; their inner grammar (the action type byte, unit ids, coordinates) is not
  interpreted yet. That is the next slice, and it is where "what did the AI do, and when" comes from.
- The meta block is assumed to be exactly 28 bytes, which is true for these four files and is checked only indirectly
  (the walk reaching EOF). A different log version would shift it; the failure would be an unknown op at the first
  read, which is loud.
- No cap on op count: a malformed stream that alternates valid ops could in principle loop for a long time. It cannot
  loop forever (every op consumes input) and the walk is 1.16 us per op, so the exposure is bounded by the file size.
