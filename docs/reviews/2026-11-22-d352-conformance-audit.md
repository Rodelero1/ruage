# 2026-11-22 - D352: the conformance audit and the divergence register

A mechanism and a register, no engine change. The owner's instruction - *"purge our rulings which are not conformant with
original game, decomp or online sources once and for all"* - answered by measuring what is actually wrong with the bank before
purging anything.

## Performance

- The audit walks **239 decision rows in 0.37 s** (`make conformance-audit`), so it can run on every handover without being
  felt.
- The measurement it produced on the bank as it stood: **238 rows, 3 that talk about the game while pointing at no source, 0
  pending after they were tagged**.
- No engine code is touched by this commit, so the acceptance scene is unchanged by construction: hash `7782dd38d7557514`,
  134 entities, 37,204 re-paths, 12.11 s median of three at `3200565`.

## Stability

- **The audit is a list, not a gate.** Making it fail a commit would teach a fake `**Conformance**` line to get past it - which
  is precisely the failure the line exists to prevent (the `0%` lesson, D209). `make conformance-audit` is in the Makefile but
  not in `make check`.
- **The tool cannot see the disease it was written for**, and both the register and the row say so: a wrong reading of a correct
  citation passes every check. The four examples in D352 (D348, D341, D349, D267) are all rows that *did* cite their address.
  The guard for that class is the tree's existing rule - re-read the expression before a slice is built on a row - and the
  register makes the correction visible when it happens.
- **The register's own risk is staleness**: it is a prose table, so a defect that is fixed and a row left behind would read as
  live. Every DEFECT row names the step that closes it, and closing a row means deleting it from the table in the same commit -
  stated in the file's header so a future session does not have to infer it.
- `tools/conformance_audit.py` has no test of its own yet; it is a reporter with one regex table, and its output is checked by
  eye against rows that are known to be cited (D59's manual quote, D63's Liquipedia) and known not to be (the three tagged
  today).
