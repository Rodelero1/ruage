# The second external review (205 findings): what was checked, and the five questions it raises

*`tool_1addba7f8001iwQ2fr4224VYC6`: 176 files, 205 findings (2 critical, 28 high, 120 medium, 55 low; 80 bugs, 74
maintainability, 29 test, 17 performance), produced by `kimi-k2.7-code` in 42m45s, **partial** - 107 of 176 selected items
failed to complete. The owner's warning was accurate: the model had no decision record, so several findings are a correct
observation against a deliberate choice.*

This is a triage, not a verdict on all 205. What follows is what was **checked against the tree**, then the shape of the rest.

## Checked, and right

| finding | what the tree says | action |
|---|---|---|
| **[16]** a `f64` multiply per worker per tick in the simulation path | `crates/data/src/economy.rs:252` is called from `crates/sim/src/lib.rs:8606`, **inside the tick**, per chopping worker - and it *multiplied* the per-second rate by the tick rate instead of dividing it, so a 1.5 hp/s chop became **30 hp/tick at 20 Hz** and every tree fell to its first blow | **fixed**, D129 |
| **[70]** `--dump` CSV interpolates `task` without escaping | comma in a task name splits the row | deferred, trivial |
| **[71]** `players` reports the highest player id, not the count | true for a sparsely populated pack | deferred |
| **[74] [173] [184]** `NaN`/huge `duration_seconds` or `at_seconds` through `f64 as i64` | saturates silently; a `NaN` duration plays a zero-length match | confirmed; wants one load-time check |
| **[112]** `if let (Some(techs), Some(effects))` skips research when only one flag is given | confirmed at `headless.rs:165` | deferred, trivial |
| **[125]** `find_path_avoiding` has no node budget and no negative cache | confirmed; a far goal on a sealed map expands the whole grid | deferred with [134] |
| **[131]** `Step`/`Rule` accept unknown keys | confirmed - and the suggested `deny_unknown_fields` **cannot work** here: `Step` uses `#[serde(flatten)]`, which is incompatible with it. The real remedy is a hand-written check or a tagged enum | deferred, remedy wrong |
| **[134]** `nearest_resource` runs A* per candidate node | confirmed - **and the first review said the same thing independently**, which is the strongest signal in either review | see question 5 |
| **[135]** `nearest_nodes` picks without reachability | confirmed - and deliberate: the file records that asking per node cost **37x the rest of the run** | see question 5 |
| **[177]** `line_clear` starts at the `from` tile | true; a body standing on an occupied tile is already "blocked" | deferred |

## Checked, and wrong

| finding | why |
|---|---|
| **[3] [101]** `src/bin/batch.rs` is not built because `Cargo.toml` has one `[[bin]]` | **False.** `cargo build -p ruage-sim --bins` produces `target/debug/batch` alongside `ruage-headless`; an explicit `[[bin]]` adds to auto-discovery, it does not replace it |
| **[8]** `Civ::bars` always returns `false` | true, but the function has **no caller** in the workspace: a dead API, not a bug |
| **[32]** the start stockpile gives stone 200 where "standard AoE2" gives 0 | **the premise is wrong**: the standard AoE2 opening is 200 food, 200 wood, 100 gold, **200 stone**. Our stockpile matches the game |
| **[36]** `"British"` should be `"Britons"` | the pack uses the dat's own civ name (`civs.toml:20`), and `economy.toml` joins on it consistently. The game's *display* name is a different string (D123) |
| **[45]** FNV is unsuitable for a multiplayer handshake | there is no multiplayer, and the hash is a determinism check, not a security boundary |

## Checked, and a real content/comment drift (the owner's call)

| finding | the comment says | the value says |
|---|---|---|
| **[27]** `config/ai/base.toml` | the army attacks at **eight** soldiers, and the barracks line stops at **twelve** | `army_at_least = 3`, `army_at_most = 11` |
| **[34]** `config/economy.toml:121` | four wall types (72/117/119/155) | `blocking_roles = ["palisade_wall"]` - only one |
| **[0]** `config/map.toml:1` | "mirrored point-for-point" | the loader checks bounds and sites, not mirror symmetry |

The `army_at_*` one matters most: it is the owner's own item 8, and the two numbers that implement it disagree with the prose
that explains it. Content is the owner's to decide, hence the question.

## What was not checked

**Roughly 190 of the 205 were not verified line by line.** Their shapes are consistent enough to sort:

- **59 findings live in test files** - the recurring one is `world.submit(...)` returning a `Result` that the test discards,
  so a *refused* order reads as a passing test; and `fresh()` converting **every** load error to `None`, so a broken
  `economy.toml` silently **skips** a whole file's tests instead of failing them. The second is the "a piped grep is not a
  green suite" pitfall in another costume, and it is worth a batch fix on its own.
- **~25 are missing validation in the loaders** - negative `los`/`speed`/`train_time`, `min_required > required.len()`,
  duplicate civ ids, two nodes on one tile, negative `hp`. All cheap, all the same shape: a `ConfigError` that does not exist.
- **17 are performance** - most already have a comment saying we know (aggro scans, HUD, the node lookup); two are new and
  real (`--dump` accumulating one `String`; metrics sampling every entity once a second).

The review's **partial** status matters for how much to trust the absence of a finding: 107 of 176 items did not complete,
so "it did not mention X" is not evidence about X.

## The five questions for the owner

1. **The chop pace.** With the arithmetic fixed, a 20 hp tree now takes **13.3 s** at 1.5 hp/s - seven blows. Is that the
   pace you want, or was the number a placeholder?
2. **`army_at_least = 3` / `army_at_most = 11`** against prose that says eight and twelve. Which pair is the decision?
3. **`Map::is_walkable` accepts only Grass**, while `Terrain::walkable()` accepts grass, dirt **and forest**. D117 says
   passage is the ground's; trees block as entities. Which is the rule for a town-centre site and for seeding?
4. **`config/acceptance.toml` says "a thirty-minute game" and `duration_seconds = 2400.0`** - forty minutes. The canonical
   command passes `--ticks 36000` (thirty) so the documented match and the batch tool disagree.
5. **Reachability in the node lookup.** Two independent reviews want it; our own measurement says `can_reach` per node cost
   37x the run. Revisit with a bounded design, or keep the speed?

## Performance

The one performance finding that was measured, because it was also the one that was a correctness bug:

| | measured |
|---|---|
| the chop rate before the fix | **30 hp/tick** at 20 Hz, **60** at 40 Hz (probe on the shipped pack) |
| the intended rate | **1.5 hp/s = 0.075 hp/tick**, a 20 hp tree in 13.3 s |
| the tree's life after the fix | **tick 267** at 20 Hz = 13.35 s |
| acceptance match, 36,000 ticks | **12,383 ms median** (12,347 / 12,383 / 13,763) against **12,391 ms** before - one map lookup added, one `f64` multiply and `ceil` removed per chopping worker per tick |

The remaining 17 performance findings are **not** measured here. The two that are new and plausible - `--dump` accumulating a
whole CSV in one `String`, and metrics sampling every entity once a second - are recorded for a step of their own; the rest
already carry a comment saying we know (aggro scans, HUD cost, node lookup, the 37x reachability measurement).

## Stability

Not a code change, so nothing was attacked. What the triage establishes instead is which findings were checked and how:
**9 verified against the tree and confirmed, 5 verified and refuted with evidence** (the batch binary *is* built; AoE2 *does*
open with 200 stone; `British` is the dat's own civ name), and **~190 not verified line by line**, sorted by shape rather
than claimed as read. The review itself was **partial** - 107 of 176 items failed to complete - so its silences are not
evidence, and this file says so rather than implying a clean bill of health.

The one change this step did make has its own performance and stability sections, with its own attacks, in
`docs/reviews/2026-11-05-a-tree-took-one-blow.md`.
