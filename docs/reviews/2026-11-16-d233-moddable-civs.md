# Review: a civilisation is config — the attribute vocabulary is closed

*Change: `crates/data/src/techs.rs` (a two-list vocabulary, `knows_attribute`, and a refusal in the modifier loader: +58
lines) and `crates/data/tests/techs.rs` (three fixtures). What it does: a pack that names an attribute outside the vocabulary
is **refused at load**, naming the effect and the attribute, instead of loading a modifier nothing will ever read.*

## Performance

**+0.0% on the pack load: 81.4 / 82.1 ms before the check, 79.8 / 80.8 ms after, two interleaved pairs** of
`perf::what_a_match_loads_and_installs_costs` (release). The after-runs being marginally *faster* is noise - the two spreads
overlap by 2% - and it is the shape the change predicts: one `&[&str]::contains` per modifier, **~1,300 modifiers over 798
effects**, once, at load, against a probe that also reads and hashes 11 pack files.

**The first pair I took read 80.3 ms against 120.2 ms** and would have been reported as a +50% regression; the re-run showed
it was a cold page cache and a loaded box (load average 2-3), so the pair is discarded rather than quoted. That is the same
trap this tree recorded on D214, met again.

Nothing runs per tick: the check is load-time only, and the state the loader builds is byte-identical.

## Stability

*Gate: 193 test binaries, **883 passed, 0 failed, 20 ignored** (880 before this change, +3 fixtures), `fmt --check` 0,
`clippy --workspace --all-targets -D warnings` 0, 202 tool tests OK, `next-step --check` OK (13,745 bytes),
`plan ok: 84 items, 121 citations`, `decisions --check` clean.*

- **The two new fixtures are red on the old code** - `attackk` and `other:7` both loaded silently, which is the defect - and
  the third fixture pins the other side: the shipped pack carries **exactly 29 distinct attribute names**, so the closed list
  cannot drift away from the data it must accept.
- **The refusal is the point, not the validation.** The engine matches modifiers by attribute *string*
  (`modifiers_for(player, unit, "range")`), so an unknown name is not an error anywhere today, it is a **no-op** - and for the
  owner's requirement (a civ added in config) a no-op is the worst outcome, because the civ loads, plays, and quietly lacks
  its bonus. The error text says that in one sentence rather than naming a rule number.
- **The list has two halves on purpose.** The 16 `other:<id>`/`unpacked:<id>` names are the data's *recorded gaps* (D216's
  shape) - they are in the vocabulary because the shipped pack carries them, and a **new** one is a change to this list, so a
  future importer cannot quietly add an attribute nobody reads.
- **No engine knowledge was added.** The vocabulary lives in `crates/data` (the config layer, D6/D17) as *names*, and the
  names are the interface the engine already matches on. Nothing here knows a civ, an id, or a bonus.
- **The loader's other refusals still fire in order**: empty attribute, non-finite value, unknown `op` - each has its own
  message, and the closed-vocabulary check sits after the empty check so the "a modifier needs an attribute" sentence still
  wins for the simplest mistake.

Known and accepted:

- **The check is closed, not documented as open.** A mod author who wants a genuinely new attribute must add a consumer and a
  name; that is the D17 boundary working as intended, and the alternative - accepting any string - is what the tree had until
  now.
- **The seam (R6) is untouched.** This step closes the vocabulary; the five-walks-of-three-tables problem the owner's
  requirement exposes is still open, and `docs/civ-bonuses.md` now records that the requirement raises its priority.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
