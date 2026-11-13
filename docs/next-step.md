# OPEN - register row 2, then the Feudal gap

**Ruling in flight: D189 is withdrawn, and its replacement is parked on a branch.** The owner: *"Adopt original game
behaviour. Don't make difference of friend or foe as in original game."* The collision block was read and has no owner
test (`docs/movement-in-the-binary.md`), so friends are walls. It is built on **`d189-owner-blind`** (commit `eaa2d98`) and
**it cannot land yet**: the acceptance match ends `players=2` where `main` ends `players=1` — with friends blocking, the
crowd machinery underneath is not good enough. The blocker is one number: our gather places are spaced one body apart
(0.40), which *is* the new reach, so a man cannot stand on the place next to a standing friend; the game spaces its men at
**0.66**. Full measurement and the three-step landing plan:
`docs/reviews/2026-11-13-owner-blind-collisions.md` (on that branch).

**State, 2026-11-13.** `main` at `a5812ad`, green (183 binaries, `EXIT 0`, clippy 0, fmt 0), acceptance hash
`91572e72c33cee93`. Two things closed since the last entry:

- **The standoff question is answered, and D192 was right.** The owner sent `tree`, `mine` and `bush` saves with eight men
  on each: every node carries **seven** bodies at **0.641-0.760** (edge, `0.5 + 0.2`) and **0.935-0.936** (corner,
  `0.5*sqrt(2) + 0.2`), mean 0.79-0.81, closest pair 0.661. The 1.156 "ring" was the tile grid - exact **1.000** and
  **1.414** readings round every node - and D186 is withdrawn (**D199**). `tools/savegame_positions.py` now defaults to
  the touching band, and its one-body `ValueError` is fixed test-first (5 tests).
- **craft-anything is evaluated** (`docs/reviews/2026-11-13-craft-anything-evaluated.md`): 1-2 s per identification query
  on the exe, 190 MiB, no Ghidra import; it does not recognise the dat or a save.

**The queue, in order, no decisions needed:**

1. **Register row 2, damage.** `native-function-callers-list 0x744d60` gave **5 call sites** (`0x6bb100`, `0x741a30` x2,
   `0x744ff0`, `0x745060`). Next: which of them is combat's path, then whether the per-entry drop (`0x744d60`) or our
   total floor at `combat.toml`'s `minimum = 1` is the game's rule. Test-first either way.
2. **The Feudal gap.** `oracle.md`: our Feudal at ~14:30 against the game's ~9:30 - a **55% slower** opening economy that
   no decision answers. Register rows 6, 7, 9 and 11, all `unknown-static`, all now cheap to locate by string trace.
3. **The gather capacity.** The game shows 7-8 bodies at **0.66** spacing; D192's geometry gives **eleven** places. Ours is
   the permissive side, so nothing fails - but the two are different claims, and the game's spacing says the limit is the
   ground, not a constant. Measure how many bodies a node takes before the next one stops working.
4. **`Vision` sub-struct** - the god-object split, no behavioural content. Last on purpose.

**Needs the owner:** `crates/save` + `--load` (licensing, `docs/licensing.md`, D4 = B), and anything that changes a rule the
owner set.

**Instrument notes, so a fresh session does not re-derive them.** craft-anything's CLI is
`/opt/data/tools/craft-anything/bin/craft-anything-cli`. The MCP entry is registered **deep**: args
`mcp --all-tools --deep`, **323 tools**, `connect_timeout` 180, and its `env` `PATH` carries the JDK 21 `bin` and both Ghidra
12.2 directories (without the JDK's `bin` on `PATH` the CLI reports java and Ghidra "not found" whatever `JAVA_HOME` says;
`~/.config/craft-anything/config.toml` holds `deep_mode = true` under the runtime `HOME=/opt/data/home`). Ghidra is
detected now, so the first Ghidra-backed call will pay the ~6.5 min import and 1.4 GB; the built-in engine needs neither and
**every identification number in the evaluation came from it**. `/reload-mcp` loads the entry into an open session. Its
function count is **19,484** where Ghidra's is **15,465**: a count is only comparable within one tool.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256
`6bc5f8a6…` (tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
