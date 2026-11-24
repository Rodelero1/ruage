# Review: C-rulings + collision IDs + OWNER normalization (2026-11-24)

Tools change (2nd slice of `live_migrate.py`), so gate sections apply.
No engine/config change. Bank untouched; live files regenerated.

## What landed

- `render_decision` + `section_titles` + `_render_collision`: bank collisions
  (D88/D89/D90/D167/D168, distinct titles proven by audit) render as D88a/D88b…
  with both titles and both bodies. Genuine same-title amendments keep the old
  `#### Amendment N` rendering.
- OWNER-tag normalization: D354's line kept verbatim plus
  `*Live verdict:* OURS` (schema stays 3 verdicts).
- `conflict-table.md`: all 15 conflicts RULED with evidence — C3/C5/C6/C8/C11
  mechanical chains, C1/C2/C4/C7/C9 compositions under existing owner rulings
  (no new owner judgment invented), C10/C12/C14 grill-granted, C13/C15 via this
  slice's generator.
- `divergence-register.md`: +1 DEFECT row (gather search bounds, C9) with source
  and closing step.

## Performance

Suite: 218 tests (+2), 10.8 s; new file 7/7. Regen 0.4 s, 364 decisions,
5 `Conformance needed` flags (burn-down 5, unchanged), D88a/D88b verified in
`simulation-core.md`, live verdict verified in `economy.md`.

## Stability

Attacked: collision-vs-amendment confusion (same id, same title → amendment path,
covered by existing Amendment test); second-section title loss (parser keeps bodies
only — recovered from raw text via `section_titles`, tested); OWNER branch
unreachable (audit TAG can't match OWNER — caught by RED, fixed, re-greened);
dangling-scan noise (live IDs D88a… read as dangling refs by the existing report —
informational only, same 2 pre-existing failures before and after, no new failure).
Own errors: the unreachable-else (fixed same session) and assuming TAG would match
OWNER (wrong — schema audit predates the tag). `main()` glue still verified by
`--check`, not unit test. Cargo unavailable in this box; no Rust touched.
Pre-existing failures re-verified identical: areas-wiring, plan dependency tree.
