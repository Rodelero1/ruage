# Content provenance (D22)

### Files this creates (not yet built)

- `ruage-import <path-to-install> --out <cache-dir>`  - reads the player's copy, writes the content
  pack and provenance manifest into a cache directory. Refuses a version it does not know.
- content pack `[provenance]`: per-entry `original` | `ours`, so "can this run without the
  installation?" has an answer.
- First run without an installation: a refusal that names the expected dat file and version, not a
  silent skip.

The engine already refuses to invent defaults (`--units` has no default path), and the tests skip when
`config/gamedata/units.toml` is absent rather than fabricating data - both are this decision applied
early.
