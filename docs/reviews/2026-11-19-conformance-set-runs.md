# Review: the conformance set runs again (13 green, 3 ignored, 0 red)

*Not a behaviour change: one call site in `crates/conformance/tests/cost_model.rs` still used `Ground::search(from, to)`,
the signature D279 replaced with `search(start, goal, scale, avoid)`. The crate is excluded from the workspace gate
(`crates/conformance/Cargo.toml` carries its own `[workspace]`), so nothing noticed for a week: `make conformance` stopped at
a compile error, and the "13 green / 3 ignored" in `docs/pathfinding-conformance.md` was a verdict from D279's tree, not from
this one.*

## Performance

**The set now runs: 5 files, 16 tests - 13 passed, 3 `#[ignore]`d (the three layers the port does not carry), 0 failed.**
Wall clock for the whole run: **62 ms** (cargo's own per-binary lines sum to under 0.1 s; the 62 ms is the shell's
`date +%s%N` either side, on a warm build). The three ignored ones are unchanged and say what they cost: the levelled hierarchy, the expansion budget
(`expansions > h`, cap 20,000), and the planner's radius - the last of which **D286 has since implemented**, so its ignore
reason is now stale and the row needs re-pointing rather than un-ignoring (as written it asserts a one-waypoint route through a
one-tile gap, which passes with or without a radius and therefore measures nothing).

No engine code changed, so no tick and no route changed: the acceptance fixture and the suite are unaffected by construction.

## Stability

**What was attacked.** The fix is a missing argument (`SCALE` and an empty clearance list, which is what the old two-argument
call meant) plus the `SCALE` import. The risk it addresses is the real one: a *verification* instrument that silently stopped
compiling, so a red rule would have been invisible.

**Found while reading it, and named rather than fixed:**

- `docs/pathfinding-conformance.md`'s header still describes the set as "13 green and 3 ignored" without saying that two of the
  three are still true and one is not, and its body still describes the pre-D279 set.
- Nothing in the gate runs this crate, by design (D249/D279). So it must be run by hand, and the note that says so is the only
  thing that will make anyone run it.
