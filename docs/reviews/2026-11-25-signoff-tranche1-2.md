# Review: sign-off tranches 1–2 + compact (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `Status: Signed 2026-11-25` on ADRs 0001–0006; sign-off notes on 0002 (parked modding-docs future), 0003 (reading (a); (b)/(c) parked, (c) refused), 0005 (parked alias follow-up).
- `MANIFEST.md`: 6 rows Draft → Signed; 0007–0013 verified still Draft (a blanket replace overshot and was reverted row by row — see below).
- SPEC 1.223 → 1.224 with one changelog row.

## Performance

Zero engine diff by construction. Suite state unchanged from the trunk verdict: 522 passed, 2 pre-existing stale-data failures (base-identical), 6 ignored — no re-run, nothing on the tick changed.

## Stability

Attacked: (1) the blanket sed flipped all 13 manifest rows to Signed — caught by reading the manifest back, 7 rows reverted; (2) 0003's plural status line ("Old rows") needed a separate edit after the singular-pattern sed missed it — caught the same way; (3) SPEC changelog row quotes only signed things (0007–0013 named as drafted, not signed); (4) `docs/next-step.md` deliberately untouched — 14,901 of 16,384 bytes with a `--check` gate, and migration state has a durable home already (manifest + commits), so appending there risked the cap for no gain. Ruling ledgered.
- Own error count this slice: 2 (both caught by read-back before commit).

## Accepted / refused

- Accepted: sign-offs and SPEC row as the owner verdicts them.
- Refused: nothing. 0007–0013 stay Draft awaiting tranche 3–5 verdicts.
