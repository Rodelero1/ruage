# Review: Phase B calibration — Vision & fog + Victory & defeat (2026-11-24)

First Phase B slice (small areas first, as agreed). Two micro-refinements land
via generator tables; everything else in both areas keeps. No engine change.

## Candidate diff (the calibration loop's output)

- **D59 → `superseded by D59b (ruled)`.** The bank never wrote the C6 chain's last
  link in machine form (D46 superseded ✓, D59a superseded ✓, D59 "decided").
  Evidence: C6 ruling + D59b's "supersedes D59a's property". Scaffolding only;
  bodies untouched.
- **D78 → `GAME - the manual's conquest rule, quoted in D59 (live refinement)`.**
  The flagged `Conformance needed` line is replaced: the manual quote in D59 is
  the source pointer (victory = side-has-nothing-left; TC not special). Bodies
  untouched.
- **KEEP, reviewed:** D50 (engine fog design, silent per schema — it claims our
  engine, not the game), D50a (19x-tick perf record), D214 (dat attributes, sourced
  via `stats_of(1, TOWER)` call-shape + review cite), D46/D59a (superseded chain),
  D59b (dat classes + parse, the chain's end), all bodies verbatim.

## Method point (needs one owner ruling for the rest of Phase B)

Refinements land as **tested generator tables** (`STATUS_OVERRIDES`,
`CONFORMANCE_ADDITIONS`) — bank bytes never touched, regen stays the applier,
`--check` still proves 364/15/0. This covers scaffolding only. The day Phase B
needs a *body* change, live files become hand-maintained and the generator
retires to audit-trail (headers updated that same slice). No body change in this
slice, so the question is asked now rather than guessed later.

## Performance

Suite 230 tests (+2), 11.7 s. Regen 0.4 s. Calibration areas: 3 + 5 rows read
in full, 2 refinements, 0 drops, 0 merges.

## Stability

Attacked: override shadowing a truer bank status (overrides are additive maps
keyed by id, rendered with "(ruled)" provenance — remove the row, the bank status
returns); addition masking a real gap (the tag is `GAME - <pointer>`, checkable,
plus "(live refinement)" marker — not a silent unflag); duplicate `lines =`
assignment left from an earlier edit (removed in this slice); my own test-file
slip that deleted a method line (repaired before GREEN, recorded). Same 2
pre-existing failures. Cargo unavailable; no Rust touched.
