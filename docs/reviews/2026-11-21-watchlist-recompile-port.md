# Watchlist: the recompilation port (2026-11-21)

Scope: one `WATCHLIST` entry in `tools/prior_art/watch.py` plus its test. No engine code, no `config/`, nothing
that runs outside the daily gate.

## Performance

Measured on this container, best of three: test suite **479 ms, 47 tests** (was 467 ms / 46 — one more case, no
change in shape), gate unchanged at **2.90 s for 15 repos** (one extra core-bucket read per run). The addition costs
one GitHub request a day and nothing at runtime; `main` stays single-pass and the sweep is still monthly.

## Stability

- The new entry is a repo with **no licence file**: nothing about it can be copied, so the role line says "the method,
  not the code" and the doc records the all-rights-reserved status. The gate itself only reads a commit sha, which is
  a fact, not an expression.
- The entry is *new* (created 2026-11-18, one commit-day old), so its 2-day backoff will report it often at first.
  That is the shared rule working as designed, not a special case; if it turns noisy the answer is the interval, not
  an exemption.
- Nothing here changes the failure paths: an unreachable repo still fails the run loudly (`WATCH FAILURE`, exit 1) and
  leaves that project's state untouched.

## What was verified before adding it

Repo metadata and tree read through the API (113 files, Python/C/JS, `docs/adr/`, `tools/oracle/`, `docs/phase*.md`);
`README.md`, `docs/adr/0002-binary-and-derived-code-policy.md` and `tools/oracle/replay_check.sh` read in full. The
determinism claim is theirs and machine-checked on their side; it is **not** independently verified here, and the
watch entry does not repeat it as fact — the gate will report what changes, and a claim like that earns a study slot
only if it is ever acted on.
