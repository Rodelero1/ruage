# The harness tells the truth (D130), and two fixtures that had never run

*2026-11-06. The second external review's test-hygiene batch, taken first because it needs no decision from the owner -
and because a suite that reports `passed` for a test that never loaded its content is not evidence about anything else.*

## What the harness was doing

`crates/sim/tests` holds 87 test files. **36** of them loaded the pack through `ruage_data::load_*(..).ok()?` - **220**
loader calls, plus **62** `World::new` / `seed_from` / `load_*` calls written the same way. Every error became `None`; each
of those tests opens with

```rust
let Some(mut world) = fresh() else { return };
```

so a load failure was indistinguishable from "the generated pack is absent", and the test reported **passed**.

The other 46 files in the same directory already use `.expect("economy.toml")`. That is what makes the minority a defect
rather than a house style - and it means the fix had a convention to match instead of one to invent.

**Measured, before any change** (`config/economy.toml` replaced with `nonsense = [`):

```
running 2 tests
test a_standing_tree_blocks_its_tile ... ok
test a_chopped_tree_leaves_no_obstacle ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Two tests whose whole subject is that a tree takes time to fell, green, in **0.01 s**, against a pack that does not parse.

The drift the copies had already caused: **eight** distinct `fresh()` bodies, differing only in which tables they load, and
`farm_ring.rs` carrying a comment - *"Enough wood and food that a rule is refused for placement, never for price"* - over
`for slot in 0..4 { let _ = slot; }`. The stockpile it promised had been deleted; a no-op loop compiles and says nothing.

## The rule (D130), and the test that was watched failing

**The generated pack's absence skips a fixture; a pack that is present and will not load fails it**, with the file's path
and the loader's error. One loader, `crates/sim/tests/common/mod.rs`, with five entry points for the five shapes the eight
copies had (`fresh`, `fresh_plain`, `fresh_research`, `fresh_civs`, `fresh_with_resources`) and `present()` / `must()` for
everything else. Each converted file keeps a one-line delegate, so no call site moved.

| test | before the strictness | after |
|---|---|---|
| a pack that was never generated skips the fixture | passed | passes |
| a broken file in a present pack fails the fixture | **FAILED** - *"nothing: the loader returned instead of panicking"* | passes |
| a present-but-broken marker fails rather than skipping | written after the fix, passed first time | passes |

That is not a metaphor: the loader genuinely returned, and the test says so in the failure text.

**Suite-level A/B, same binary, same break:**

| harness | `cargo test --test depletion`, `economy.toml` = `nonsense = [` |
|---|---|
| lenient (this step's first slice) | **2 passed; 0 failed**, 0.01 s |
| strict (this step) | **0 passed; 2 failed**, panic naming `/opt/data/workspace/Age2HD/config/economy.toml` and `TOML parse error at line 1, column 14` |

## Two fixtures that had never run (D131)

The change's first dividend was immediate, and it is the reason this review is worth reading.

| file | what was wrong | after the fix |
|---|---|---|
| `footprint.rs`, `corridor()` | its inline map declared wood on **grass**; `load_map_str` refuses a wood node off a forest tile, `.ok()?` ate the refusal, and `a_head_on_pair_in_a_one_tile_corridor_passes` had never executed | the map has forest rows under its trees - and the test **fails**: the pair comes **7,372 raw units** apart at tick 66 against the **8,000** their footprints need. Ignored with that reason; the repair is D131's own step |
| `passage.rs`, `NODE` | the same: one wood node on grass | forest under the tree; the test runs and **passes** |

So the hole was not cosmetic: one dead test was hiding a real overlap bug, and one was hiding nothing.

## Performance

No engine code changed, so the honest measurement is the one that proves nothing moved plus the cost the harness itself
costs.

| | D129 (release, recorded) | this tree (release, three runs) |
|---|---|---|
| acceptance, 36,000 ticks, seed 7 | `12,383 ms` median | **`12,066 ms`** median (13,027 / 12,028 / 12,066) |
| state hash | `394c4892aac776d2` | **`394c4892aac776d2`** |
| entities / repaths | 141 / 184 | 141 / 184 (`made_room=187`) |

**The 2.6% is noise, not an improvement** - the spread across three runs is 8.3% (12,028 to 13,027 ms), and the hash and both
counters are identical, which is the part that matters: the same match, to the tick, so the harness change cannot have
touched the simulation.

*Debug* build, for completeness and explicitly not comparable: the same command took **197,264 ms** unoptimised. That is
what a debug binary costs on this 2-CPU box, and quoting it beside the release figure would be a false comparison.

Whole suite: **704 passed / 0 failed / 20 ignored**, `429.6 s` (`cargo test --offline --no-fail-fast`, debug, warm build,
130 result lines). The meta-tests cost **0.89 s**: two of them copy 1.1 MB of `config/`, the third is `0.00 s`. Every
fixture now calls `canonicalize()` once on the repository root - one syscall per test, and the panic message reads as a path
a human can open.

## Stability

| attack | result |
|---|---|
| a tree with **no `config/gamedata/`** (moved aside): the D8 rule | `depletion` **2 passed / 0 failed**, `fixture_rules` **3 passed / 0 failed** - fixtures skip, the suite stays green |
| ... and this attack **found a defect in this step** | before the gate was added, the two copy-based meta-tests failed without a pack - a fresh clone would have gone red on the tests that describe the skipping rule. They now skip like every other fixture |
| a broken **hand-written** file (`economy.toml`) | panics, naming the file and the TOML error; was 2 passed |
| a broken **generated marker** (`units.toml` present but garbage) | panics naming `units.toml` - the marker is existence, not parseability |
| the meta-test twice in a row | its temp copies are removed at the start and the end; the second run is green |
| a fixture whose content will not parse passing silently | gone: only an **absent marker** reaches a `return`, and that path is asserted by the first test |
| two test binaries in parallel | `common` is compiled per binary with no shared state; temp directories carry the pid and a per-test name |
| `canonicalize()` on the root | the root always exists; a symlinked checkout resolves, and the failure text is the resolved path |
| a fixture that now fails because its pack is broken | the point. Two were found in the first run (D131, and `passage`) |

## What this step did not do, and one finding it refuted

- **`crates/view/tests/enemy.rs` still loads through `.ok()?`** (14 sites). It is a different crate; giving it the same
  rule needs either its own copy of `common` or a small shared test-support crate. Named here rather than half-done.
- **The 50 `world.submit(..)` sites are not the defect the review said they were.** `World::submit` returns `()` - it is
  `submit_checked` that returns a `Result` - so nothing is being discarded. The residual issue is real but different:
  `submit` skips `validate`, so an order the engine will refuse is staged and the test reads the aftermath. Which of the 50
  should be `submit_checked` is a judgement per site (some are deliberately staging an illegal order), so it is a step, not
  a batch.
- Still open, unchanged: the ~25 loader-validation findings, the two new performance findings (`--dump` building one
  `String`, metrics sampling every entity each second), and **D131** - the overlap this step uncovered.
- The five questions from session 29 remain the owner's, unguessed.

## This step's own errors

- Ran `/usr/bin/time` on a box that has no `/usr/bin/time`; the run died with `EXIT=127` and the wall clock had to be
  taken with `date` instead.
- The first full-suite run had no `--no-fail-fast`, so it stopped at the first failing target and the aggregate I first
  read (407 tests) was a partial count, not the suite.
- Wrote the meta-tests without the D8 gate, making a fresh clone red - caught by the stability attack above, before the
  commit.
- Left a stray `//` line in `docs/next-step.md` from a draft; removed before the commit.

**704 passed / 0 failed / 20 ignored**; clippy and fmt clean.
