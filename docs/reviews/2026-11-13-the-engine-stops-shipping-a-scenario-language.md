# The engine stops shipping a scenario language

The architecture review's item 6: 1,656 lines of scenario language, scripted AI and rule runner lived inside `ruage-sim`, against
D17/D18 (*"the engine knows kinds, not particulars"*). This is the move, and what it found.

## Performance - measured

| Metric | Before | After |
|---|---|---|
| `ruage-sim` source lines | 16,780 | **14,856** (-1,924) |
| crates in the workspace | 4 | **6** (`fixed`, `data`, `sim`, `script`, `harness`, `view`) |
| acceptance run from the moved binary | `entities=142 hash=91572e72c33cee93 repaths=409` | **identical** |
| workspace gate | 181 binaries | **183 binaries, `EXIT 0`**, clippy 0, fmt 0 |
| acceptance run, 36,000 ticks | - | **10.88 s** wall clock (same binary layout, same hash) |
| `ruage-sim` dependencies | `data`, `fixed`, serde, toml | `data`, `fixed`, serde, toml - **and `ruage-script` only as a dev-dependency** for its own tests |

The acceptance hash being identical is the whole claim of a pure move: the tick did not change, only what is allowed to see it.

## The shape now

- `crates/script` (`ruage-script`) - the scenario language, the scripted AI, the rule runner, the personality loader, 1,613 lines.
- `crates/harness` (`ruage-harness`) - `ruage-headless` and `ruage-batch`, 635 lines, the two binaries that drive a match.
- `crates/sim` - the engine, which no longer knows what a scenario is.

`script -> sim`, `harness -> sim + script`, and **no arrow points back into the engine**.

## What the move found

A layering break described in prose is a suspicion; a layering break that will not compile is a fact. Three of them surfaced:

1. **The engine called three functions that lived in `script.rs`.** `start_villager_unit`, `start_town_center_unit` and
   `is_free_ground` were used by `metrics`, `movement`, `sites` and `shares` - the engine depending on its own "content" module without
   anyone noticing. They moved to `crates/sim/src/roles.rs`.
2. **Two private methods became public**: `nodes_of_slot` and `tile`, both needed by the runner. They are now documented as *"public
   for the scenario language"*, so the widening is visible at the call site rather than discovered later.
3. **`crates/sim/tests/headless.rs` could not stay**: `env!("CARGO_BIN_EXE_ruage-headless")` only exists in the package that defines
   the binary. It moved with the binary, which is the test's real dependency speaking.

## Stability

- **Behaviour unchanged, and checked by the hash rather than by eye**: `91572e72c33cee93`, 142 entities, 409 repaths, `made_room` 0 -
  byte-identical to the pre-move run.
- **The gate ran the whole workspace**, not just the moved crates: `EXIT 0`, 183 binaries. The one failure it caught was mine - a
  `use ruage_fixed::Fixed` left in `roles.rs` after the split - and clippy refused it before the commit.
- **The Makefile and `docs/scenario.md` were updated** to `-p ruage-harness`, because a move that leaves the documented command broken
  is a move half done.
- **`ruage-view` now depends on `ruage-script`** for its window binary (it runs a scenario). That is a real new edge and is recorded
  rather than hidden; the alternative - the window taking a prepared runner - is a design change nobody asked for.
- **Decision D198** records the move and the three findings; the log is green at **203 decisions**, `--check` clean.
