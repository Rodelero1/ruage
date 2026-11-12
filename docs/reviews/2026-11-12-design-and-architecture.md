# The architecture, attacked

An adversarial review of the **design and structure** of the Age2HD engine, asked for in full rather than scoped to one slice. It is
not a code review: it does not go line by line, it asks whether the shape will survive the next two hundred decisions.

**Verdict.** The two things that are expensive to add later are both right: the simulation is deterministic by construction, and the
crate graph has no cycles and no leakage. The weaknesses are all of **scale** - a 76-field `World`, a 4,731-line `lib.rs`, a task
state machine smeared across 25 match sites, a test suite that pins *measurements* instead of *rules*, and documentation that has
outgrown the code it describes. One process failure is catastrophic in kind rather than in size, and it is mine: **186 decisions were
deleted and every gate stayed green** (§1).

## How this was measured

Every number below comes from a command run on this box at 2026-11-12 18:2x UTC, not from memory:

```
find crates -name '*.rs' -path '*/src/*' | xargs wc -l      # source lines per crate
find crates/*/tests -name '*.rs' | xargs wc -l              # test lines per crate
sed -n '/pub struct World {/,/^}/p' crates/sim/src/lib.rs   # 76 fields, 0 of them pub
grep -rn "pub fn \|pub(crate) fn " crates/sim/src | wc -l   # 238
grep -rn "Task::Gather" crates/sim/src | wc -l              # 25
python3 tools/decisions.py --check | head -1                # 192 decisions parsed
wc -c docs/next-step.md SPEC.md docs/decisions.md           # 348,807 / 341,007 / 390,921
du -sb docs                                                 # 2,332,894
ls crates/sim/tests | wc -l                                 # 117 files, 35k lines
grep -rn "#\[ignore" crates | wc -l                         # 22
grep -rn "HashMap\|HashSet" crates | wc -l                  # 0
python3 tools/decisions.py --check                          # 192 decisions, 12 dangling ids
cargo test --workspace --no-fail-fast                       # 176 binaries, ~15 min wall clock
```

## The findings, ranked

| # | Finding | Severity | Evidence | Effort to fix |
|---|---|---|---|---|
| 1 | The decision log can be truncated and every gate stays green | **critical** | 186 decisions lost in `12b1e35`; `--check` green | **done** |
| 2 | `docs/next-step.md` is 348,807 bytes - the "compact" rule is defeated | high | one file, 20x the largest source file | S |
| 3 | Tests pin measurements, not invariants | high | 11 tests died at once when the standoff moved | M |
| 4 | Movement decisions are not inspectable | high | 2 hours spent to find a step that was *allowed* | M |
| 5 | `World` is a god object | medium | 76 fields, 238 methods, 19 modules | L |
| 6 | The task state machine is spread over 25+ match sites | medium | 9 variants, `Task::Gather` alone 25 references | M |
| 7 | 59 error variants with no classification | medium | `SimError` 509 lines, one enum | S |
| 8 | The scenario/AI language lives inside the sim crate | medium | `script.rs`, 1,656 lines, in `ruage-sim` | M |
| 9 | Save and load have no home in the engine | medium | parser exists only in `tools/savegame.py` | M |
| 10 | Measurement debris left in the tree | low | 3 env-gated probes, 2 unconditional prints | **done** |
| 11 | Documentation outgrew the code | low | 779 KB of docs for 26,636 lines of Rust | S |

---

## 1. Critical, self-reported: the log lost 186 decisions and the gate said yes

**What happened.** `docs/decisions.md` went from **385,025 bytes and 187 decisions** at `f468e75` to **1,937 bytes and one decision**
at `12b1e35` - the commit that landed D188/D189. D1-D187 were gone. `docs/decision-areas.toml` had been cut back to match, so
`tools/decisions.py --check` was **green**: it checks relations (every decision has an area, every area has a decision), and a
*consistent* truncation satisfies both.

**Recovery.** `f468e75:docs/decisions.md` restored the corpus; D188 was rewritten from
`docs/reviews/2026-11-12-no-cap-and-pass-through.md` (which measured it at the time) and marked as recovered; D189 and D191 were
carried over from the truncated file. **192 decisions parse now, 390,921 bytes**, and `--check` passes.

**The guard, test-first.** New tests in `tools/test_decisions.py` (`TheCorpusNeverShrinks`): the first run was **RED** with
`AttributeError: module 'decisions' has no attribute 'check_shrink'` - the three tests written before the function existed. Then
`check_shrink(corpus, baseline)` and `committed_corpus()`: the working corpus is compared against `git show HEAD:docs/decisions.md`,
and a corpus with fewer decisions **fails `--check`** naming how many are gone and which. No history (shallow clone, no git) is not a
finding - an unavailable fact is not evidence of loss (D45).

**Why the original check could not see it.** It was relational. The lesson generalises: *every check in this project that compares two
artifacts to each other can pass while both are wrong; at least one check per artifact must compare it to history.*

## 2. `next-step.md` is 348,807 bytes

The owner's rule for every step is "compact first: persist state to files so a fresh session resumes without the conversation".
Compaction has produced one file **20x the size of the largest source file**: it is not a next step, it is a second decision log with
no index, and a session that reads it (this one did, at 101 KB and growing) spends its context on history instead of on work.

**Proposal (S).** Cap it at **16 KB** with a check in `make check`: `tools/next_step.py --check` refuses a file over the cap or one
whose first heading does not name the single open step. History moves to `docs/history/next-step-<date>.md` and stays in git, where
it is already preserved perfectly. Do not delete the prose - move it.

## 3. Tests pin measurements, not invariants

When the standoff moved from a 1.156 ring to the tile's border, **eleven gather tests went red at once**, every one of them asserting
the number `1.156` rather than the rule (a body touches its tile; two bodies are at least a diameter apart). The engine's behaviour
was *more* correct and the suite said it was broken.

The volume makes this worse: **44,885 test lines against 26,636 source lines** (1.69:1 overall, **2.08:1** in the sim crate), 117
files in `crates/sim/tests`, 176 test binaries, and a full gate of **~15 minutes**. A suite that costs a quarter of an hour per
iteration is a suite people stop running, and the **22 `#[ignore]` sites** are the symptom: 11 declare themselves measurements or
probes, 7 print timings, 2 are multi-hour soaks, and 3 are parked defects (D181, D98a, resource depletion).

**Proposal (M).** A rule in SPEC §11: *a test asserts a relation - `apart >= 2 * radius`, `distance_to_tile <= radius`, monotone
decrease - and may **quote** a measurement in its failure message; it may not assert one.* Then tier the gate: `make check` runs the
default set (fast), `make soak` runs `--ignored`, and the parked-defect tests move to a named list that names its decision
(`D181`, `D98a`) so the parked work is visible without grepping reasons.

## 4. Movement decisions are not inspectable

`collide` is one function with **nine numbered rules** returning a `(Fixed, Fixed)` pair, fed by a `Passing` struct that carries
bodies, moving, refused, repaths and made_room, and followed by an apply pass that can still change the answer. Today's deadlock cost
**two hours** and ended with the discovery that the movement pass *allowed* the frozen man's step (`walkable=true`, clearance
0.120 -> 0.129) and something downstream refused it. The decision the engine makes is not a value anyone can print, test, or compare.

**Proposal (M).** Return a value, not a pair: `enum Step { Take, Slide(Fixed, Fixed), Wait, MakeRoom(Fixed, Fixed), Repath(Tile),
Refused(Reason) }`, applied by one function that is the only writer of a position. Its immediate payoff beyond testability: a
`step_trace` ring of the last 64 decisions per unit, which turns "why is he stuck" from an hour of printing into one dump. This is the
cheapest large improvement in the list.

## 5. `World` is a god object - but a well-behaved one

**76 fields, 238 `pub`/`pub(crate)` methods, 19 `world/` modules, 11 separate `impl World` blocks.** The file split shows where the
seams want to be: economy (gather/build/train/research), space (movement/sites/seed), vision (memory), match (battle/garrison/gates),
bookkeeping (report/state/shares).

Two things are right and must not be lost: **no field is `pub`** (all access is through methods that can hold invariants) and the
tick is an explicit sequence of ~20 named phases, which is the determinism contract in readable form.

**Proposal (L, phased).** Extract `Economy`, `Space`, `Vision` and `Milieu` sub-structs **one at a time**, each carrying its own
invariants and its own `impl`, starting with `Vision` (13 methods, 2 fields' worth of state) and `Economy` last (it touches
everything). No behaviour change in any of those commits; the test suite is what makes that credible.

## 6. The task state machine is smeared

`Task` has **9 variants**, and `Task::Gather` alone is matched in **25 places**. Transitions are implicit: `resume_idle`,
`step_aside`/`resuming`, `retask_workers`, `settle_workers` and the task arms all write `entity.task`. Today's deadlock printout was a
list of tasks per unit, which was the most informative thing available - that is a hint about the missing abstraction.

**Proposal (M).** A `world/tasks.rs` that owns transitions (`fn set(world, id, task, why)`) and records the **why** in a small trace.
Not a redesign: a chokepoint. Every later investigation gets "who changed this task and to what" for free.

## 7. Fifty-nine error variants, no classification

`SimError` is 59 variants in 509 lines. The view cannot tell "the player cannot do that" from "the engine is wrong" without matching
all 59, so every UI path either swallows errors or prints internals.

**Proposal (S).** `impl SimError { pub fn class(&self) -> ErrorClass }` with three arms - `Refused` (legal command, not now/not here:
`NoPath`, `SiteNotClear`, `NotAFoundation`), `BadRequest` (malformed: unknown unit, overflow), `Internal` (invariant broken). The
match is mechanical, and the UI gets a rule instead of a list. `submit_checked` already returns `Result`; nothing else changes.

## 8. The AI and the scenario language live in the sim crate

`crates/sim/src/script.rs` is **1,656 lines** - a scenario language, a parser and a scripted AI - inside the crate that D17/D18 says
"knows kinds, not particulars". It is the largest file in the engine after `lib.rs`, and it means the headless runner, the batch
runner and the acceptance script all pull the AI's dependencies into the simulation's build.

**Proposal (M).** Move it to `crates/script` (a sibling of `sim`, depending on `sim`, never the reverse), with the sim exposing only
the tick hook and the command surface it already has. This also makes the importer's future home (D12: separate repo) obvious by
analogy: **content and scripts are not engine.**

## 9. Save and load have no home in the engine

The save format is well documented (`docs/save-format.md`: raw DEFLATE, VER 9.4, the 229-byte object records and their variable-stride
cousins) and implemented **only in Python** (`tools/savegame.py`, `tools/savegame_positions.py`). The engine cannot load a save, which
means it cannot compare its own state to the original game's - the only oracle this project will ever have that is not a screenshot.

**Proposal (M).** `crates/save` with the format only (no game semantics), used by both the tools and the sim, behind a feature flag
until the licensing review (`docs/licensing.md`) clears shipping it. First consumer: a `--load <save>` mode for `ruage-headless`
that loads, ticks, hashes - the determinism oracle the project has been faking with synthetic maps.

## 10. Measurement debris (fixed here)

Three env-gated `PROBE` blocks and two unconditional prints were left in the test tree by earlier sessions: `crates/sim/tests/build.rs`,
`gather_continuous.rs`, `gather_motion.rs` (a nine-slot mock that printed and asserted nothing) and `retarget.rs` (a print every 400
ticks). They are invisible in a normal run, which is exactly why they survived review. Removed in this commit; `cargo clippy
--all-targets` is clean afterwards. The general rule: **a probe is code, and code that cannot fail is code nobody removes.**

## 11. Documentation outgrew the code

`docs/` is **2,332,894 bytes** and `SPEC.md` another 341,007, against 26,636 lines of Rust - and the three prose files a session is
told to read (`next-step.md` 348,807, `SPEC.md` 341,007, `decisions.md` 390,921) total **1.08 MB** on their own, with the last two
overlapping in a way the changelog no longer reconciles. **192 decisions** is one per **139 lines of code**. That is not a criticism of
writing decisions down - it is a criticism of writing them down *twice*, and of a SPEC whose changelog is a second history.

Also concrete: **D190 does not exist** (a gap in the numbering), and `--check` reports **12 dangling ids** (`D78`, `D79`, `D81`,
`D82`, `D115`, `D122b`, `D124`, `D125b`, `D126`, `D136`, `D192`). D192 is the parked standoff and belongs to a branch; the rest are
old references to decisions nobody wrote. Either write them or record them in the U-register - an id that resolves to nothing is the
same failure as a decision with no area.

**Proposal (S).** SPEC keeps §1-§11 and the current rules; the changelog moves to `docs/history/spec-changelog.md`. Every decision id
referenced must resolve, enforced by the `dangling` list becoming a **failure** once the twelve are resolved - otherwise the list is a
warning that nobody reads.

---

## Performance

Measured on this box at 2026-11-12 18:2x UTC, `--release`, one run at a time:

| Metric | Value | How |
|---|---|---|
| `World::step` | **6.9 us** at 128 entities | `cargo run --release -p ruage-view --example step_cost` |
| `Snapshot::of` | **105.0 us** at 128 entities - **0.82 us per entity**, 15x the tick | same run |
| acceptance (36,000 ticks, seed 7) | **13.20 s** wall clock, 143 entities | `ruage-headless ... --ticks 36000` |
| repaths / made_room | 434 / **0** on the D192 branch (was 435 / 96) | same |
| full workspace gate | **176 test binaries, ~15 min** wall clock (940 s of waiting before it finished) | `cargo test --workspace --no-fail-fast` |
| test lines vs source lines | **44,885 / 26,636** (1.69x; sim alone 2.08x) | `wc -l` over `crates/*/{src,tests}` |

The extrapolation that matters for the view: 0.82 us per entity is linear, so a real 2,000-unit match would spend ~1.6 ms per frame
in `Snapshot::of` - one tenth of a 60 fps budget before anything is drawn. That is a **projection, not a measurement**, and the fix
(delta snapshots, or reusing the buffers) is cheap only while the entity count is small.

## Stability

- **The decision log is recovered and the gate now fails when a corpus shrinks.** The restore is verified by `--check` (192 decisions,
  0 unclassified headings) and by the three new tests; the guard compares against `git show HEAD:` and names the lost ids.
- **Determinism invariants are untouched**: 0 `HashMap`/`HashSet` in `crates/`, `iter_over_hash_type = "deny"`, fixed point through
  the tick, and `no_float.rs` still green by file name.
- **The probe debris is removed** and `cargo clippy -p ruage-sim --all-targets` is clean afterwards; the three touched test binaries
  pass (`build` 10, `gather_continuous` 2, `gather_motion` 3, `retarget` 0 run / 1 ignored).
- **Still red, and not hidden**: `build::a_content_pack_with_our_own_resources_loads_and_spends` on `d192-touching-standoff` - the
  step-aside deadlock this review ranks first in the backlog. `main` is green.
- **What this review does not claim**: the eight proposals below are *not* measured improvements. Only the two completed items
  (recovery + guard, debris) are verified; the rest are ranked by cost and risk, not by demonstrated speed.

## What must not change

1. **The determinism stack**: no `HashMap`/`HashSet` anywhere in `crates/` (measured: 0), `iter_over_hash_type = "deny"`, fixed-point
   throughout the tick, and `no_float.rs` naming every non-tick file that may parse a float - by name, so a rename is a test change.
   This is the single most valuable property in the tree.
2. **The tick's explicit phase order** (~20 named calls in `step()`), including the comment on *why* gates open before movement.
3. **Crate graph**: `fixed` <- `data` <- `sim` <- `view`, no cycles, and `sim` never depends on `view`.
4. **Refusal over bucketing** in the tools: an unplaceable heading is an error naming the heading, not an "other" row.
5. **Zero `pub` fields on `World`**: it is what makes §5 tractable rather than dangerous.

## Ranked backlog

| Order | Work | Effort | Why now |
|---|---|---|---|
| 1 | The parked D192 deadlock (step is allowed, something downstream refuses) | M | blocks ten measured gather improvements |
| 2 | `Step` enum + `step_trace` | M | makes every future stuck-unit report one dump instead of two hours |
| 3 | `next-step.md` cap + history move | S | every session pays the context cost |
| 4 | Test-invariant rule in SPEC §11, then convert the gather suite | M | 11 red tests for one correct change |
| 5 | `ErrorClass` | S | the UI becomes able to speak |
| 6 | `crates/script` extraction | M | D17/D18 compliance, smaller engine |
| 7 | `crates/save` + `--load` oracle | M | the only non-visual oracle available |
| 8 | Resolve the 12 dangling ids; close the D190 gap | S | the index is a promise |
| 9 | `Vision` sub-struct, then `Economy` | L | the god object, in safe slices |

## Errors of my own, in this review's scope

- **The truncation is mine** (commit `12b1e35`) and it was silent for two commits. Recovered, guarded, and named here rather than in
  a footnote.
- **My first recovery dropped D191**: I split the tail at `### D191` and kept only the part before it. Caught by `--check` reporting
  D191 as dangling, then restored from `eb93070`.
- **`crates/sim/tests/site_clear.rs` does not bite.** It was written as the RED for the D192 deadlock and it passes on the branch, so
  it guards the rule but not the bug. Reported when it happened; still true.
- **Probe debris removed here was also mine** (`build.rs`, `gather_continuous.rs`), alongside older blocks I did not write.
