# Review — what the original and its neighbours have that the tree did not (2026-11-14)

**What changed.** `docs/dependency-tree.md` gains **15 missing mechanics (M11-M25)**, **three debts (T15-T17)**, a
"verified absent from HD 5.7" table and a corrected seam (**R3**). `tools/plan_check.py` learns one thing: a citation
prefixed `genie:`, `openaok:`, `install:` or `ruage-tools:` names a file that is **not** in this tree, so it is skipped by
the prefix rather than by luck — without it, every external citation would read as a broken path.

## Performance

**No engine number: this is a survey and a document.** The measured numbers are the ones the survey is built from, all
counted on this machine today:

- **84 distinct source-file names** survive in `AoK HD.exe` (assert and log strings), which is the original's own partial
  map of itself — `act_misl.cpp`, `tact_cnv.cpp`, `tact_obt.cpp`, `trigger.cpp`, `rmm_*.cpp`, `group.cpp`, `motion.cpp`.
- **104 `.per` AI files** in `install:resources/_common/ai/`, whose action vocabulary is the game's own list of what a
  player can be told to do: `release-escrow` (1,531 uses), `sell-commodity` / `buy-commodity` (31 each),
  `commodity-buying-price`, `set-escrow-percentage`, `tribute-to-player` (4), `resign` (3), `players-stance` (16),
  `idle-farm-count` (8), `town-under-attack` (1).
- **String counts in the exe** for the candidates: `waypoint` 83, `tribute` 47, `formation` 42, `trade` 33, `relic` 26,
  `wonder` 17, `resign` 18, `diplomacy` 13, `follow` 12, `guard` 9, `treaty` 9, `regicide` 2, `doppleganger` 2, and the
  three that settle commands outright: `TRIBE_COMMAND_TOWN_BELL`, `TownBell(uId=%d, activate=%d)`,
  `TRIBE_COMMAND_SELL_COMMODITY`/`BUY` (read in `genie:players/market.rs`).
- **The pack**: `MONK` 8, `DOCK` 4, `TCART` 4, `COGXX`, `FTRAP` 2, `KINGX` 2, `RELIC` 1 — so all of it is HD 5.7's own
  data, not a later expansion's.

`plan_check` still runs in **266 ms** over the document (measured before this change: 3,304 ms with an unpruned index,
266 ms after). Its own suite is **11 tests in 86 ms**.

## Stability

**What was attacked.** Every candidate had to survive three filters, in the order `AGENTS.md` fixes: (1) **does HD 5.7
have it** — a string or a dat entry from the install itself; (2) **is it in the tree already** — a `grep` of `crates/`, not
a memory of the tree; (3) **is it a mechanic or a name** — a UI label or a DE-only class is not an item.

**What the filters killed, and this is the part that keeps the list honest.** `pasture`, `mule cart` and `empire wars` are
**zero hits** in `AoK HD.exe` and belong to DE's later expansions; `solidify` is zero too, which means genie's
`Wait_Solidify` action is a DE name and our foundation model (D183/D185) is already the HD behaviour. Genie's passive
ability kinds (generation 151, movement damage 152, terrain look 159, removal reward 161) are named but not adopted: its
own README says build 158041's `.dat` holds "only generation tasks", and the Feitoria and Keshik that carry them are
expansion civs this install does not have. Those are recorded as **verified absent** in the document, not as items —
"absent from HD" is a finding, and writing it down stops the next session adding a mechanic the game never had.

**What the survey corrected in the tree.** **R3** said "a roll is its own seeded stream". The original's answer is
stronger and different: `genie:math/rng.rs` and the determinism section of `genie:docs/ENGINE_SYSTEMS.md` describe
**eleven WELL512a streams that all take the frame seed at the start of every tick**, so a draw in one tick cannot reach
the next. Our engine has two PCG32 streams and **never reseeds them** — a silent divergence that no current test could
see, and now **T15**. The second is the **per-player turn** (`Turn`, schedule step 16): only the player whose turn it is
runs its unit AIs' full update, which is a different axis from T12's intra-tick order and is now **T16**.

**What the two projects could not settle, and is therefore left open.** Whether HD's treaty is a lobby option or a
scenario setting (`mTreatyLength` and three message ids prove it exists, not where it is set); whether any 5.7 `.dat`
carries a passive-ability task; and whether the market's price swing is the same as DE's. Each is named in the item that
needs it rather than guessed.

**My own error, found while writing.** The first draft of the tool change put the external-prefix check *after* resolving
the path, which would have flagged `genie:players/market.rs` as missing — the test
`test_an_external_source_is_named_and_not_checked_here` was written first and failed on exactly that, and the fix is the
prefix test at the top of the span loop.
