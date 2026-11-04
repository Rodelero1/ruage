# A cleanup commit, and why the gate still wants a note

Three leftovers from the last two batches, none of them behaviour:

- `round_div` in `ruage-sim` - the rounding helper the reverted steering change needed. Dead the moment D109 reverted
  `step_towards`, and the owner's build log said so ("function `round_div` is never used").
- A doc comment in `ruage-view` with nothing under it (`/// The minimap: the whole map at one or two pixels a tile...`
  followed by a blank line and a section heading) - clippy's `empty_lines_after_doc_comment`.
- Six `as u32` casts in `tests/hud.rs` from `u32` to `u32`, because `banner_point` already returns `u32`.

## Performance

None measurable: a dead function is not compiled, a comment is not code, and six casts were no-ops. The numbers are the last
measured ones - **630 passed / 0 failed / 18 ignored**, HUD 176 us a frame, acceptance 11.9 s for 36,000 ticks.

## Stability

This is the stability work, in the small: the workspace is **clippy-clean under `-D warnings`** again, which is what stops a
warning from being read as noise the next time the gate really is red. The lesson recorded here is the one that produced the
work in the first place: a revert is not finished when the tests go green, it is finished when the leftovers are gone too, and
`cargo clippy --workspace --all-targets -D warnings` is the command that says so.
