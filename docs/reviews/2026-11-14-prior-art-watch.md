# Prior-art watch, 2026-11-14

Docs only: `docs/prior-art.md` and two cells of `docs/mechanics/README.md` (U18, U22). No `crates/`,
`config/` or `tools/` change, so there is no performance number of ours to report and none is invented.

## Performance

Nothing in the engine changed, so a like-for-like probe would measure the same binary twice. The
numbers below are measurements of the sources: **genie 42 commits / 208 files / +12,704 / −2,762**,
openage 8 commits / 15 files, openaok 2 commits / 12 files. The only engine cost figure the week names
is genie's own placement sort, which is `std::sort` over the edge points of one building's square —
bounded by footprint, not by map size.

## Stability

What was attacked instead of asserted:

- **"The world loop is untouched."** The compare API lists `world/step.rs` and `world/schedule.rs`
  nowhere, but its per-file stats are demonstrably lossy: it reports `world/player.rs` as `0 0` while
  that file's own patch in `a8b51dc6` is +26/−12. The claim was settled by sha256 at both shas —
  `0e6265e9…` (step.rs) and `bb276386…` (schedule.rs), identical on each side.
- **The U22 change.** Checked in the head's source, not only in the new test doc: `closest_resource.rs`
  line 150's `MasterRoles::SEARCH_FROM_FOOTPRINT` with the zone compare at line 165, and
  `gather_search.rs` lines 479-482's two-pass (footprint) vs one-pass (site tile, unit's LOS + 5)
  branches. A test doc states intent; the code is what gets cited.
- **The population-peak correction.** Read from `a8b51dc6`'s patches for `world/player.rs` and
  `world/load/world/mod.rs`, not from the commit message.
- **The register still parses.** `python3 -m unittest test_decisions` in `tools/`: 72 tests, OK, 12 open
  questions — unchanged before and after the two cell edits.

My own error, reported: I first read the compare's `0 0` rows as "files whose stats GitHub omitted"
and was one step from recording the world loop as verified by absence from a list. The list is the
weaker evidence; the sha comparison is what makes the sentence true.

Second, caught and undone: `make decisions` regenerated `docs/site/index.html` and the only diff was
another session's D204 `measured` tag, which their commit left unrendered. Reverted rather than
committed under this message. The site is one render stale for D204, and that belongs to whoever lands
in `decisions.md` next — regenerating it here would have put their work inside my commit.
