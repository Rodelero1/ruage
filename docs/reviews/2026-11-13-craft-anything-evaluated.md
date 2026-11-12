# craft-anything, measured on the owner's install

*2026-11-13. Written after the owner asked, having just gained the `craft-anything` MCP: can it help gather data from the
game, which decisions can it move, and which of our decisions contradict what the game itself shows. Every number below was
taken on this box today, against `Age2HD/AoK HD.exe` (7,521,280 bytes, SHA-256 `cbd10d81b936…`), one call at a time, with
no other build running.*

## The answer in one paragraph

**On the executable: yes, and it is faster than the path we already had.** Its own engine - no Ghidra, no import, no project
- parses the PE, lists every function, searches raw strings *including the ones Ghidra's inventory misses*, follows a string
to the functions that use it, and lifts a function to pseudo-C. Every identification query I ran took **1-2 s** and **190 MiB**
peak, against REA's 6 min 30 s import and 1.42 GB for the same binary, and 6 s per function for a query. **On the game's data
files: no.** It does not recognise `empires2_x1_p1.dat` or a `.aoe2spgame` ("a format we don't recognise yet"), and it has
nothing to say about them that `tools/genconfig` and `tools/savegame.py` do not already say better. Its second real
contribution is the **ledger** - brief, plan, hypothesis, finding, unknown, handoff, every claim carrying an evidence id -
which is exactly the shape `docs/behaviour-diff.md` has been trying to be in prose.

**As installed it cannot do any of this through Hermes yet**, for two reasons that are configuration, not capability:
the MCP entry runs in **Clean room** mode (`core_target_open` is refused: *"This looks inside the original (its code, files
or page sources), which Clean room never does"*), and it is registered with the **small core toolset** (30 tools, no
`--all-tools`), so the native tools are not in the MCP surface at all. Both probes below were run through the CLI with
`--deep`, which is the same engine the MCP would use.

## Performance - the probes, with times

| # | Probe | Result | Wall | Peak RSS |
|---|---|---|---|---|
| 1 | `core-target-open` on the exe (Deep) | PE recognised, SHA-256 computed, 6 sections | **<1 s** | - |
| 2 | `native-sections-list` | `.text` at `0x401000`, 5,943,212 B; `.rdata` at `0x9ac000`, 1,042,662 B | **<1 s** | - |
| 3 | `native-functions-list` (all) | **19,484 functions**, 818,717 bytes of listing | **2.0 s** | **190 MiB** |
| 4 | `native-strings-find` literal `VER 5.7` | **`0x9db348`**, referenced from `0x72d4cf` in `sub_72d1a0` | **1 s** | 190 MiB |
| 5 | `native-strings-find` regex `\.cpp` | **97** strings (vs 84 distinct paths `tools/exe_map.py` finds) | 2 s | 190 MiB |
| 6 | `native-function-show` `0x72d1a0` | **1,128 bytes, 44 basic blocks** - the same body Ghidra gave | 2 s | 190 MiB |
| 7 | `native-function-pseudocode` `0x72d1a0` | 315 lines, header says *"partly unstructured"*, **18 inline-asm** sites | 2 s | 190 MiB |
| 8 | `native-function-pseudocode` `0x8a4360` | **4,065 lines**, 383,117 bytes (`doIntelligentMovementUpdate`) | **2.3 s** | 190 MiB |
| 9 | `core-feature-find --query repathing` | **2 functions**: `sub_7f4330` and `sub_8a4360` | **2 s** | 190 MiB |
| 10 | `core-target-triage` on `empires2_x1_p1.dat` | *"isn't a format we recognise yet"*, kind `unknown` | <1 s | - |
| 11 | `core-target-triage` on `brit13.aoe2spgame` | *"isn't a format we recognise yet"*, kind `unknown` | <1 s | - |
| 12 | ledger round-trip | brief + two findings recorded, evidence ids checked, revision 3 | <1 s | - |

Two of these matter more than the rest.

**Probe 4 is a capability we did not have.** `docs/re-toolchain.md` records that `VER 5.7` at `0x9db348` is **not in
Ghidra's defined-string inventory** (8,966 entries), so `rea search` could not find it and a Python byte scan plus the PE
section table had to. craft-anything found the string, its section and its one xref in **one second**. The same query shape
is what every "which function reads this literal" question in the register needs.

**Probe 9 is the identification step, automated.** `repathing` is a string inside the movement module; the trace returned
`sub_7f4330` and `sub_8a4360`, which are exactly the two functions `docs/movement-in-the-binary.md` names as
`BMotion::updateMovement` and `doIntelligentMovementUpdate` - the pair that cost a corpus grep over 15,465 files to find.
**2 s** against a few minutes of grep plus reading.

**What the 19,484 means, and what it does not.** Ghidra found **15,465**; craft-anything's built-in engine found **19,484** -
roughly 4,000 more, consistent with its own `found_by: call` discovery admitting call targets Ghidra treats as data or
thunks. I did **not** investigate the difference, so the only safe statement is: **a count is only comparable within one
tool.** That is the same trap this project already recorded once for `(file, line)` sites vs string matches, and it is
repeated here deliberately.

**What its pseudo-C is not.** Probe 7's header says *"partly unstructured"* and the body is a lifted listing: `eax =`,
`ecx =`, `__asm("xorps xmm0, xmm0")`, `goto` labels. It is readable enough to answer "does this branch compare two
bytes" and much too rough to read 4,065 lines of movement. For reading, the Ghidra corpus stays the instrument; for
**finding**, this one is faster.

## Which decisions it can move

Against the twelve-row register in `docs/behaviour-diff.md` (1 ruled, 11 open). "Move" means: produce the address or the
constant that a row needs to stop being `unknown-static`.

| Register row | Can it move it? | The query | Cost |
|---|---|---|---|
| 1 dat version gate | already ruled `same` | - | - |
| 2 damage order / per-entry drop | **yes, the open half** | `native-function-callers-list 0x744d60` - is the per-entry-drop function on the damage path at all? | ~2 s |
| 3 per-tick order of speed / reload / gather | **partly** | `core-feature-find --query "reloading"` / the call order out of the game loop | ~2 s per string; the *effect* still needs the oracle |
| 4 age-up gate (kind 102) | **partly** | string trace for the age-up message, then the callers of the effect interpreter | ~2 s |
| 5 decay rate | **yes, the rule** | string trace `decay`, then the function that applies it | ~2 s |
| 6 gather rate / fractional accumulator | **yes** | `work_rate` is in the dat, but the accumulator is code: trace `carrying`/`resource` strings | ~2 s |
| 7 heap/carry caps, rounding (10 × 1.25 → 13) | **yes** | the same function as row 6, the clamp branch | ~2 s |
| 8 projectile lead, minimum range, blast | **yes** | `core-feature-find --query "ballistics"` / `"blast"` | ~2 s each |
| 9 building placement, overbuild, refund | **yes** | string trace `foundation`, `placement` | ~2 s |
| 10 vision radius and fog memory | **yes** | string trace `visibility` | ~2 s |
| 11 population cap and house arithmetic | **yes** | string trace `population` | ~2 s |
| 12 match end (conquest) | **yes** | string trace `resign` / `razed` | ~2 s |

The honest qualifier: **it finds the site, it does not rule the row.** A row becomes `same` or `different` only when the
arithmetic at that site is read and a test pins our side. What changes is the price of getting to the site - from "build a
Ghidra project and grep a corpus" to "one query, two seconds" - which is the step that has actually been the bottleneck
(`behaviour-diff.md`: *"decompilation is cheap; identification is unbounded"*).

**What it cannot move at all:** anything about our architecture (D17/D18 layering, the god-object split), the licensing
questions (D4, D8, D12, D13, and `crates/save`), the content-pack format (D6), the determinism rules (D9/D10), and the
choice of G1 over G2 (D1). Those are ours, not the game's.

## Which decisions the game's own findings contradict

This is the part the owner asked for, and it is shorter than the register because a contradiction has to be *evidence on both
sides*. Five, in descending order of how much they matter.

### 1. D192's standoff contradicts the project's own measurement of the same saves - **unresolved**

- **D192 says:** *"A worker stands with his circle touching the node's tile… His saves measure it - men at **0.70** from a
  resource tile's centre and **0.99** where two edges meet - against the 1.156 ring D186 read from the row behind them."*
- **The measurement it overrides says:** `docs/reviews/2026-11-12-the-mine-and-the-ring.md`, same day, three saves:
  tree **1.105-1.189** (mean 1.156), bush **1.100-1.174** (mean **1.143**), mine **1.041-1.188** (mean 1.114); closest
  pair **0.400** on the bush; and the owner's own ruling there: *"tree, mine and berries are all the same in that aspect."*
  `tools/savegame_positions.py`'s docstring still states it: *"men working one square stand at 1.10-1.17 tiles from its
  centre, spaced 0.40 apart, which is D186's ring."*
- **The other side:** D169 came from the owner's **screenshots** - *"jump towards the bush, pile up, forage from a
  distance"*, *"while the circle is not touching the square with bush"* - where a man foraging from 0.66 of a tile away
  from the tile's edge is the *complaint*. That is real observational evidence for touching, and it is why D192 exists.
- **Why this is unresolved rather than a defect:** the two readings can describe different rows of the same object array
  (positions vs task destinations - `docs/save-format.md` already records that the order queue holds destinations, not
  standing places), and the fixtures that settle it (`bush.aoe2spgame`, `mine.aoe2spgame`, `test.aoe2spgame`) are **no
  longer in the tree** - `find` over `Age2HD/` returns nothing for any of them, and they are not in `SaveGame.zip` either.
  I could not re-measure, and I will not call a decision wrong on a file I cannot read.
- **What would settle it in one minute:** one save and one screenshot of the same moment, with **five or more men on one
  bush or tree**. The save gives the ring radii; the screenshot shows whether the circles touch the square.

**Why it went unnoticed, and the rule it suggests.** The review that landed D192 (`2026-11-13-touching-standoff.md`) never
mentions the save measurement at all; the number 0.70 enters in the decision row, not in a review with the measurement in
it. This project already has the analogous lesson written down for the decision log (`check_shrink` compares the corpus
against **history**, because two halves can be lost together). The same rule belongs here: **a decision that reverses a
measured number must quote the measurement that reverses it**, or the reversal cannot be reviewed.

### 2. Damage: our total floor against the original's per-entry drop - **unknown-static, and it has an address**

`behaviour-diff.md` row 2: our `damage_between` sums per-class `max(0, atk - armour)` and floors **the total** at 1
(`combat.toml`'s `minimum`); the candidate original site `FUN_00744d60` **drops each entry whose amount fell below 1**
(sets its class to `-1`) and then sums. Those are different rules with different results against high armour: a knight
against 12 pierce armour takes 1 from us and 1 from the original, but a unit with three damage classes takes 1 from us and
possibly 0 from the original. Not a contradiction yet - **the question is whether the damage path calls that function at
all**, and `native-function-callers-list 0x744d60` answers it in about two seconds. This is the first row I would spend a
craft-anything query on.

### 3. D16's 20 Hz is not contradicted, but it is not confirmed either

The evidence is indirect in both directions: reload times on a 50 ms grid (excludes 10 Hz only if reloads are tick
quantised), and recordings at ~10.1 syncs/s (consistent with syncing every second turn at 20 Hz, equally consistent with
every turn at 10 Hz). `oracle.md` says a decompiled constant is the only direct answer. craft-anything can now find the
constant's site cheaply; I did not chase it today.

### 4. The Feudal age arrives at ~14:30 where the game's does at ~9:30 - **no decision covers this**

`oracle.md` records it from the AI farm (*"the probe puts our Feudal at ~14:30 against the game's ~9:30"*). That is a
**55% slower** opening economy, measured against the owner's own recorded games, and no decision row anywhere answers it -
the register calls it rows 6, 7, 9 and 11 (`unknown-static`) and the work stopped there. It is the largest known
behavioural divergence in the project and it is currently nobody's next step.

### 5. D186's nine-gatherer cap against the game's ten-eleven - **closed, then reopened**

D186 recorded the cap as nine and *"not because it is right… the game does eleven"*, attributing it to our packing rather
than a rule. D192 closed it with a capacity of eleven computed from the touching geometry - but that geometry is exactly
what item 1 puts in doubt. The cap and the standoff are the same question, and both wait on the same save.

## Stability

What adopting this could break, and what I checked.

1. **Two engines, two number sets.** 19,484 vs 15,465 functions is a live trap: a count quoted without its tool is
   meaningless here. Instrument names in this project now have to carry the tool as well as the units.
2. **A second Ghidra on the box.** craft-anything ships against Ghidra **12.2** (`/opt/data/tools/ghidra_12.2_PUBLIC`,
   JDK 21), while REA is pinned to **12.1.4** (JDK 25) because 12.2 is rejected as `config_drift` by REA's version check.
   They are separate installs and separate projects, so nothing collides today - but a future session that points one at
   the other's cache will get exactly the drift REA's pin exists to prevent. Do not wire Ghidra into craft-anything: the
   built-in engine answered every identification question above without it.
3. **The ledger can become a third decision log.** The project already paid for this once - `docs/next-step.md` grew to
   348,807 bytes and had to be capped at 1,230 (D195). The ledger is useful *per investigation* and must not become the
   record of decisions; `docs/decisions.md` stays authoritative, and a ledger finding enters it as a row with the evidence
   id quoted.
4. **Clean room is a feature and a wall.** Nothing was read until Deep mode was switched on for one CLI run. Leaving the
   MCP in Clean room means it cannot help with the game at all; switching it on gives an MCP process read access to the
   install, which D8/D22 already permit locally (nothing derived is committed) but which is the owner's call, not mine.
5. **Nothing in the engine changed.** No Rust, no config, no acceptance run: the hash is still `91572e72c33cee93` on
   `main` at `80e7dd3`, and the last full gate (183 binaries, `EXIT 0`, clippy 0, fmt 0) still describes this tree.

## My own errors in this pass

- **A wasted run:** the first Deep open was wrapped in `/usr/bin/time -v`, which this box does not have, so it exited
  **127** and produced nothing. Cost: one call and a few minutes. Re-run without it.
- **A probe that measured the wrong thing:** I wrote `build/tools/ringprobe.py` to recover the gather ring from
  `brit13.aoe2spgame` without a window, scanning every byte offset for float pairs. It reported **5,892 distinct
  positions**, a nearest-neighbour histogram dominated by artefacts (1,391 points exactly 1.0 apart), and **no rings at
  all** - the scan reads non-position bytes as coordinates, which is precisely why the original tool takes a window
  around a known square. I am recording it as a failed probe rather than as evidence, and it is not in the commit
  (under `build/`, which is gitignored).
- **An inconsistency I could not explain:** 97 `.cpp` strings found by craft-anything against **84** distinct source paths
  `tools/exe_map.py` extracts from the same exe. It might be `.h` files, duplicate strings or a difference in what counts
  as a path; I did not chase it, and it does not affect any number above.

## What I would do next, in order

1. **Settle the standoff** with one fresh save + screenshot from the owner (item 1). It is the cheapest open question in
   the project and it decides whether D192 is fidelity or a regression.
2. **Rule row 2** with `native-function-callers-list 0x744d60` - two seconds of instrument, one test either way.
3. **Spend an hour on the Feudal gap** (item 4) with rows 6, 7, 9, 11 located by string trace. It is the divergence a
   player would actually feel.
4. Only then the `Vision` sub-struct, which is a refactor with no behavioural content.
