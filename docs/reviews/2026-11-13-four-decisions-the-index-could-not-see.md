# Four decisions the index could not see

The corpus writes one evening's five answers under one heading - `## D77-D79 - The owner's five answers (2026-10-30)` - and then states
each answer as a bold line:

```
**D78 - Victory is conquest.** The match ends when a side has nothing left that keeps it alive...
```

The parser read only the heading's own id. So **D78, D79, D81 and D82** were reported as *dangling* - referenced by the rest of the
corpus, defined nowhere - while they were defined all along, in the file, in bold.

## Performance - measured

| Metric | Before | After |
|---|---|---|
| decisions the index parses | 198 | **202** |
| dangling ids | 12 | **7** |
| `docs/site/index.html` | 315,844 bytes | **318,442 bytes** |
| `tools/decisions.py --check` | - | **1.26 s** per run (five averaged), in `make check` |
| tool tests | 70 | **72** |

The index is a reader's tool, not a hot path; the figure that matters is that four decisions the project had already made were invisible
to it.

## What changed

1. **Inline decisions.** A bold line matching `**D<n> - Title.**` inside a section is a decision, with the section's date (the heading
   carries it: `(2026-10-30)`) and its text running to the next such line. A bold phrase that is not an id is left alone.
2. **The date is read from the heading as well as the body** - the five answers are dated in the heading, and an inline decision
   inherits it.
3. **`docs/dangling.toml` and a ratchet.** The seven that remain (D115, D122b, D124, D125b, D126, D136, D190) are recorded with what
   each is cited for - and the corpus itself already names four of them as *"never written down"*. `--check` now **refuses a dangling
   id that is not recorded**, so a new one is caught instead of joining a warning nobody reads.

## Stability

- **Test-first, twice**: `InlineDecisions` went red with `'D78' not found in {'D77': ...}` (the inline text was D77's prose), then red
  again on the date (`None`, because it searched only the body), then green; `DanglingIdsAreARatchet` went red with
  `AttributeError: check_dangling` before the function existed.
- **Every decision keeps its area**: four new areas were authored (D78 Victory & defeat, D79 Graphics, D81 Licensing & publication,
  D82 Graphics); `--check` is green at 202 decisions, 0 unclassified headings.
- **No engine change**: nothing outside `tools/` and `docs/` was touched, so the tick, the acceptance hash and the suite are untouched -
  the workspace gate stays where D193-D197 left it (181 binaries, clippy 0, fmt 0).
- **The ratchet is a ratchet, not a mute**: recording a gap requires a note, so the next session reads *why* D126 has no decision rather
  than a bare id in a list.
