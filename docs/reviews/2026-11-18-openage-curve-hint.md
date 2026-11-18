# 2026-11-18 — the openage curve-hint note (docs only)

A dated section in `docs/prior-art.md` for `SFTtech/openage` `e3080121` -> `4c04b6be`. No code in this tree changed, so
`tools/review_check.py` does not gate it; the note exists because the routine owes one either way (AGENTS.md).

## Performance

**Delta is zero lines of code, so the only defensible number is the tree the note lands on — measured here, not inherited**
from the previous note. `cargo test --offline` at **HEAD `873870b`**, 2026-11-18: **23 test binaries, 192 passed, 1
failed, 128 s wall** (warm target dir; the same suite from the state the run began in took 174 s). `cargo fmt --check`
clean; `cargo clippy --offline --all-targets -- -D warnings` clean.

Those three checks were reported **red** by the 2026-11-17 prior-art note at `e96ef3b`. Two of them are gone — the fmt
diff at `crates/sim/src/path.rs:109` and the two `type_complexity` errors at `path.rs:250`/`:278` — and the third is not:
`a_crowd_walking_through_a_gap_stays_on_free_ground` (`crates/sim/tests/army_walk.rs:122`,
`NoPath { from: (6, 20), to: (19, 19) }`) still fails, the fixture U27 names and what D262-D269 keep working. Verified
twice: in the full run, and alone in **1.16 s**.

The openage change is not timed. Building that C++ tree is not a cron-tick sized job, so the claim that the pre-fix
`last_before(10, p0)` answered `10` is a **reading of the two branches**, not a run — the one claim in the section a
reader should check by eye rather than trust.

## Stability

Attacked, in the order the note makes its claims:

- **"one query, two answers."** Reconstructed by hand against the pre-fix loop for all five hints the new test uses, on
  the container `[-inf:0, 0:0, 1:1, 10:2]`. Checkable by eye; not executed, per the paragraph above.
- **"no caller outside the container."** `git grep last_before` at `4c04b6be` over the whole tree: **0 hits** outside
  `libopenage/curve/`. The two in-file ones are `sync(other, start)` at `:528`/`:551`, both passing `container.size()`,
  which is the branch that was already correct — so the defect was latent, not live.
- **"we have no query that takes a hint."** `grep -rn hint --include=*.rs crates/` -> **11 hits**, every one
  `std::hint::black_box` (`crates/sim/tests/perf.rs`, `crates/sim/tests/pathing.rs`, `crates/view/tests/frame_cost.rs`,
  `crates/view/examples/step_cost.rs`). `interpolat|keyframe` matches two files, neither of them a curve. Nothing here to
  repair and nothing to port.
- **What the note does not cover.** A hint parameter added later for speed would reintroduce the class, and none of the
  above would notice. The reusable part is the test shape — one query, every hint, one answer — and it is worth writing
  at the first caller, not before.
- **Tree hygiene.** HEAD moved twice during this run (`f9e64f0` -> `6fbebb9` -> `873870b`), by another session:
  `6fbebb9` (09:01:57) commits exactly the `crates/sim` slice that was staged in the index when the run began. This run
  staged nothing of it and stages two paths by name.

**My own errors.** I first wrote the second openage test block as `:196-201`; the blob says `:199-203`, caught before the
insert, and the section carries the corrected range. The first repo-wide `git grep` hit the 180 s foreground cap while
fetching promisor blobs and was rerun with 400 s — that run returned 0 hits, so the timeout was mine and not a result.
Three tool invocations were refused by the cron sandbox before any of this (a `python3 -c`, a heredoc, a recursive
delete) and the work was done with `git clone --bare --filter=blob:none` plus `sed`; a rerun that hid that would read
smoother than the run was.
