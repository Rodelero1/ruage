# Review: a packed attack value no reading explains is refused, not carried as silence

*Change: `UNEXPLAINED_PACKED` in `ruage-tools`' importer lists the **thirty-one** kind-4/5 commands whose value fits no
reading, and any *other* such command now raises `UnpackedAttack` naming the effect and the value, instead of shipping as an
`unpacked:` modifier the engine ignores. A data test pins the set that the shipped pack carries (nine effects, twelve
negative words). D216.*

## Performance

**No runtime path, and the pack is byte-identical.** The change is a set membership test at generation time plus a test that
reads `effects.toml` once: regeneration after the change produced `effects.toml` differing from the previous file by **2 bytes
of diff, both the `# generated:` timestamp**. So the shipped behaviour is bit-for-bit what it was, which is why no timing is
claimed - there is nothing whose time could change. For scale, a full regeneration still takes the same ~90 s it did before.

## Stability

- **The refusal has to be able to fire, and to name the thing it refused.** The importer test does both directions: effect
  506's `[4, 280, -1, 9, -226.0]` is listed and still produces `unpacked:9`, and the same command under an effect id nobody
  measured raises with `9001` and `-226` in the message. A guard that only refuses cannot be told from one that never runs.
- **What it found.** Not a new defect in the engine, but two defects in **my own numbers**, both caught by writing the
  assertion rather than trusting the reading: I had written "seven are negative words" in the decision and in the importer's
  comment, and the test failed on `left: 12` - twelve of the thirty-one carry a negative word (five distinct values,
  -1025/-770/-769/-226/-136), and the other nineteen are the positive-but-unexplained ones. Both comments now say twelve.
- **The existing fixture that broke was a real signal.** `test_effect_record_leaves_negative_packed_values_undecoded` built a
  *synthetic* effect id 217 carrying effect 80's -1025 command; it now uses effect 80, the effect that actually holds that
  command in the data. A test that invents a pairing is exactly how a lookup like this one rots.
- **The set is pinned from both sides.** The importer refuses anything new; the data test fails if the pack gains, loses or
  renumbers one of the thirty-one. Between them a regeneration cannot quietly move the boundary of what we do not understand.

Known and accepted:

- **These effects still apply nothing.** That is the point of a refusal, but it means a player researching one of the nine
  gets no number rather than a wrong one. Effect 506 is Shinkichon, whose **+1 range** is decoded and does apply - only the
  -226 attack command beside it is dark.
- **The decode is not attempted.** Recorded in D216 with the arithmetic observed (classes 4/11/16/34, amounts 100/120/225, the
  (4196, 1124, 8804) and (4321, 8929) pairs) and the place to look: the exe's kind-4/5 handler.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
