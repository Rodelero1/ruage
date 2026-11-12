# OPEN - one save decides whether D192 is fidelity or a regression

**State, 2026-11-13.** `main` at `80e7dd3`, green (183 binaries, `EXIT 0`, clippy 0, fmt 0). The craft-anything evaluation is
written and landed: `docs/reviews/2026-11-13-craft-anything-evaluated.md`. It answers the owner's question with measured
numbers - its built-in engine reads the exe in 1-2 s at 190 MiB with no import, finds the `VER 5.7` literal Ghidra's
inventory misses, traces `repathing` to the two movement functions, and does **not** recognise the dat or a save.

**The one open question, and it needs the owner.** D192 moved the gather standoff to the tile's border (**0.70** from a
node's centre) on the strength of screenshots, while the project's own measurement of three saves
(`docs/reviews/2026-11-12-the-mine-and-the-ring.md`) puts gatherers at **1.10-1.19**, mean 1.156, spaced 0.400, and the owner
said *"tree, mine and berries are all the same in that aspect."* The fixtures that settle it (`bush`, `mine`, `test`) are no
longer in the tree and not in `SaveGame.zip`. **One save plus one screenshot of the same moment, five or more men on one
bush or tree**, ends it: radii from the save, touching-or-not from the screenshot.

**The queue while that waits** (no decisions needed):

1. **Row 2 of the register, damage.** `native-function-callers-list 0x744d60` (~2 s) says whether the per-entry-drop
   function is on the damage path at all; if it is, our total floor at `combat.toml`'s `minimum = 1` is a different rule and
   the fix is test-first.
2. **The Feudal gap.** `oracle.md`: our Feudal at ~14:30 against the game's ~9:30 - a 55% slower opening economy that no
   decision answers. Rows 6, 7, 9 and 11 of the register, all `unknown-static`, all now cheap to locate by string trace.
3. **`Vision` sub-struct** - the god-object split, no behavioural content. Last on purpose.

**Needs the owner, unchanged:** `crates/save` + `--load` (licensing, `docs/licensing.md`, D4 = B); whether the craft-anything
MCP may run in Deep mode with the native toolset (it cannot read the game otherwise); and anything that changes a rule the
owner set.

**Instrument notes, so a fresh session does not re-derive them.** craft-anything's CLI is
`/opt/data/tools/craft-anything/bin/craft-anything-cli`. The MCP entry is now registered **deep**: args
`mcp --all-tools --deep`, **323 tools**, `connect_timeout` 180, and its `env` `PATH` carries the JDK 21 `bin` and both Ghidra
12.2 directories (without the JDK's `bin` on `PATH` the CLI reports java and Ghidra "not found" whatever `JAVA_HOME` says;
`~/.config/craft-anything/config.toml` holds `deep_mode = true` under the runtime `HOME=/opt/data/home`). Ghidra is
detected now, so the first Ghidra-backed call will pay the ~6.5 min import and 1.4 GB; the built-in engine needs neither and
**every identification number in the evaluation came from it**. `/reload-mcp` loads the new entry into an open session; a new
session picks it up anyway. Its function count is **19,484** where Ghidra's is **15,465**: a count is only comparable within
one tool.
