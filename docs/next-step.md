# OPEN - D207 is on `main`: a group walks over itself. Next: the explicit group id (register), and D205's pause on top of it

**State, 2026-11-14.** **D207** is built and green: units ordered in the **same tick** are mates, a mate is not solid (the
step is taken, no reach, no wait, no repath), and the exemption is mutual - a man whose order is cancelled is solid again in
the same tick. Review with the numbers: `docs/reviews/2026-11-14-d207-group-overlap.md`.

**What is deferred, deliberately.** The owner's option 2 - an explicit group id on the command - is the register row. Until it
lands, "same tick" is not "same selection": a caller issuing eight separate moves in one tick gets group behaviour it did not
ask for, and a selection ordered across several ticks gets none. When it lands, `mates` is filled from the command instead of
from `ordered_ticks`, and the three tests in `crates/sim/tests/group_overlap.rs` keep their names.

**The parked branch.** `d205-pause` (`e251ca2`) holds D205 - the 0.25-1.50 s pause for the higher id, correct and measured -
plus three real fixes that the binary read now says were the wrong shape: the re-plan learnt about bodies (`short_range_around`),
the "past" exemption learnt to expire when a body moves, and `short_range`'s fast path stopped answering "walk through him".
Those three are **fidelity bugs in their own right** and worth re-landing on their own, without D205's pause, once the pause has
something to sit on. Not merged.

**The pause's missing half.** With D207 landed, a non-mate collision still has no run-time answer in our engine: the original
pauses the loser (D205, parked) and re-plans. The order that follows is: land D207's mechanism (done), then re-land the three
search fixes alone, then the pause on top - each with its own red test, because each of the last three attempts failed by
carrying the others.

**Open register rows.** (1) explicit group id on the command; (2) the three search fixes above, extracted from `d205-pause`;
(3) D205's pause, after those.

## The whole tree, and the next slice (D208, 2026-11-14)

**`docs/dependency-tree.md` is the map** - 64 items with the evidence for each status, the fourteen hollow "done" rows
(T1-T14), the five seams for the deferred mechanics, seven phases and a gate each - and `make plan-check` refuses it when
an open item is missing from the sequence. This file stays the short plan for the next few steps; D208 is the decision.

**Next: T1a + T1b, the attribute names (phase 0, the first of the live wrong numbers).** Compact, before any code:

- **Where it lives now.** The pack's attribute names come from the importer, `ruage-tools/genconfig/records.py:125`:
  `ATTRIBUTE_NAMES` maps **1 to `"range"`** and has **no 12**, so 177 commands in `config/gamedata/effects.toml` are
  `range` and attribute 12 becomes `other:12`. In the engine, `crates/data/src/techs.rs:96` classifies `1 => Attribute::Range`
  too. Range modifiers **are** consumed - `crates/sim/src/world/battle.rs:96` resolves `range` through `scale_fixed` - and
  line of sight is **never** modified: `los` is read raw at `lib.rs:3155`, `seed.rs:146`, `memory.rs:250`, `report.rs:128`.
- **What that measures today.** Town Watch and Town Patrol (effects 8, 280) add +4 to attribute 1 for classes 3 and 52, so
  a watch tower's **attack range** goes 8 -> 12 and its sight stays 10. Greek Fire (effect 518, attribute 12, fire galleys
  529/532/1103 at range 2.49) matches nothing.
- **Which tests pin what.** `crates/data/tests/techs.rs` (the attribute table and the packed attack), `crates/sim/tests/modifiers.rs`,
  `min_range.rs` (Andean Sling) and `range_boxes.rs` for what a modifier reaches; `vision.rs` for sight. **Watch them fail
  first**: a tower after Town Watch keeps range 8 **and** sees four more tiles; a fire galley with Greek Fire outranges one
  without.
- **The slice is two commits in two repos** (D12): the importer's dict plus its pytest and a regenerated `config/gamedata/`
  (not committed, D8), then the engine's classifier and a `line_of_sight` consumer in vision. Landing one alone leaves the
  pack and the engine disagreeing about attribute 1, which is the failure this compact exists to prevent.
- **Then, in phase 0 order:** T4 (farm base 175 from resource 36), T9 (U17's negative packing), T10 (elevation or a stated
  zero).
