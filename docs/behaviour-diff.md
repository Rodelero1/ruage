# Behavioural difference: ruage against `AoK HD.exe`

*Written 2026-11-01, after the owner's question: can the disassembly give the **complete** behavioural difference between
ruage and the game, "to rule one"? This file is the answer, the register and the method. Append to it; never re-derive
what is written here.*

## The answer to "complete"

**A complete behavioural diff is not computable, and the reason is not the cost.** Three facts, all measured today:

1. **Decompilation is cheap; identification is unbounded.** Measured 2026-11-01: a Ghidra **post-script against the
   persistent project** exported the whole binary — **15,465 functions, 72 MB of pseudocode, 10 min 15 s**, 39 ms per
   function average — where REA's own `decompile-all` costs ~6 s per function (20 in 824 s including a 7 min import), i.e.
   ~150x slower for the same output. So the corpus is free; **finding anything in it is the work**. The binary carries a
   **PDB reference with load state unknown**: no names, no types. Finding "the armour subtraction" means strings, xrefs
   and reading, with no stopping rule that proves every relevant function was found. *Complete* has no completion
   criterion.
2. **Code is not behaviour.** Two implementations can differ line by line and behave identically (inlining, refactor,
   compiler), and can look identical while one constant changes everything. Static reading can prove a **difference**; it
   can never certify **sameness**, and equivalence over all inputs is undecidable.
3. **A mechanical diff over everything is 99% noise.** Rendering, UI, networking, DRM and the original's AI dominate the
   procedure count. The owner has excluded functional gaps — correctly, because gaps are where a mechanical diff produces
   thousands of meaningless rows.

So the deliverable is not a diff. It is a **register**: one row per rule ruage implements, each ruled with evidence.

## What the binary can settle, and what it cannot

| Kind of question | Static verdict possible? |
|---|---|
| Arithmetic: modifiers, clamps, rounding, order of application | **yes** — read the site, compare the constants, write a test |
| Loader behaviour and gates (version string, section checks, limits) | **yes** — and one is already ruled below |
| Thresholds and costs the dat does not carry | **yes** |
| Tick ordering across subsystems (what happens before what in one tick) | **partly** — the call order is readable; the effect on state needs execution |
| Pathing tie-breaks, target selection among equals, AI decisions | **no** — needs per-tick trajectories (U7), i.e. the oracle |
| Anything where the original's own behaviour is buggy | **no** — the code shows what it does, not what it was meant to do |

## The four verdicts

- **same** — the binary's site and our code agree, and a test pins our side. Says: *this rule was found and read.* Says
  nothing about rules nobody looked at.
- **different** — the two disagree, with the address, the constant and our file:line. This is the valuable row; it becomes
  a test-first fix.
- **unknown-static** — the rule is real but its original site has not been located yet, or static reading cannot decide it.
- **needs-oracle** — the difference (or sameness) is only observable by running the game: saves, recordings, or
  instrumentation.

## The register

Every row is one rule ruage actually implements. `M` grows only as our own rule list grows, which is what makes the work
countable — "N of M ruled" — without pretending to have surveyed 15,766 functions.

| # | Rule | ruage | original | verdict | evidence |
|---|---|---|---|---|---|
| 1 | dat version gate | importer writes what it read; loader accepts it | **exact string equality against `VER 5.7`** at `0x9db348`, one branch at `0x72d4fc` in `FUN_0072d1a0` | **same** (semantics now known; no numeric parse, no tolerance) | `5ba445c`, `docs/re-toolchain.md` |
| 2 | damage: armour subtraction, bonus classes, minimum-1 clamp, order | `crates/sim::damage_between` — per class `max(0, atk - armour)`, summed, then the **total** floored at `combat.toml`'s `minimum = 1` | **candidate located, not ruled.** `FUN_00744d60` (0x744d60) reads **three `(class, amount)` pairs** from a unit record (`+0x18c/+0x18e`, `+0x192/+0x194`, `+0x198/+0x19a`), subtracts the target's per-class armour from a float table (`+0x3c`), folds any positive leftover into **class 3**, and then **drops every entry whose amount fell below 1** (sets its class to `-1`) — a *per-entry* drop, not a total floor. `FUN_00745060` calls it at multiplier 1.0 and returns the **sum** of the three. What is not yet known is whether the damage path uses this function or has its own arithmetic: the plausible callers are `FUN_00741a30` (6.7 KB) and `FUN_006bb100` (30 KB) | **unknown-static** (with a shape difference worth chasing: per-entry drop vs total floor) | this file, 2026-11-01 |
| 3 | the per-tick unit update: speed integration, attack reload, gather tick, in what order | `World::step` | not yet located | **needs-oracle** for the effect, **unknown-static** for the order | question 2 |
| 4 | age-up timing at a kind-102 gate | `advance`/effect interpreter | not yet located | **needs-oracle** | question 3, U10 |
| 5 | decay: does an unattended building vanish, and at what rate | `[decay] rate_from = "dat"`, `resource_decay_rate` semantics unverified | not yet located | **needs-oracle** — and the game answers it from two saves 5 minutes apart | `docs/next-step.md`, D85's review |
| 6 | gather rate and the fractional accumulator (0.0111, whole units) | `crates/sim` gather; `Gather::fell_hp_per_tick` | not yet located | **unknown-static** | U1 |
| 7 | heap/carry caps and rounding (10 × 1.25 → 13) | `scale_fixed` with rounding | not yet located | **unknown-static** | v1.12 |
| 8 | projectile lead (Ballistics), minimum range, blast radius | `combat.toml`, `Task::Fly` | not yet located | **unknown-static** | U4/U6 |
| 9 | building placement, footprints, overbuild/refund | D58/D60/D76 | not yet located | **unknown-static** | D76 |
| 10 | vision radius and fog memory | `explored` grids | not yet located | **unknown-static** | — |
| 11 | population cap and house arithmetic | D31/D34 | not yet located | **unknown-static** | — |
| 12 | match end (conquest: a razed town centre with villagers alive is not a win) | D59b/D78 | not yet located | **unknown-static** | — |

**The count: 1 of 12 ruled.** That is the honest state, and it is a better answer than "we compared them" — the eleven open
rows are a to-do list with a definition of done each.

**Row 2, first half ruled (2026-11-13, craft-anything).** `native-function-callers-list 0x744d60` in **2 s** gives **5 call
sites**: `0x6bb100`, `0x741a30` **twice** (`0x741c3c`, `0x741db0`), `0x744ff0` and `0x745060`. Both candidates this file had
guessed at - `FUN_006bb100` (30 KB) and `FUN_00741a30` (6.7 KB) - are real callers, so the per-entry-drop arithmetic is
reachable from the two functions that could be the damage path. What is still unruled is *which* of them combat uses, and
that is reading, not locating.

## How the oracle feeds it

Static reading cannot close rows 3, 4 and 5, and cannot certify any row as *same*. The owner's plan — two AIs, a recorded
game, periodic captures — closes exactly those. `docs/oracle.md` carries the protocol and the settings; the short version:

- a **recording** (`.aoe2record`) is the command log with tick stamps plus SYNC records (HD 5.7 verified: 68,011 ops,
  31,104 SYNC in 51 game-minutes) — it answers "what did the AI do, when";
- a **save** (`.aoe2spgame`, `VER 9.4`, raw DEFLATE) is the full world state at one instant — it answers "what was true at
  t", and two saves 5 minutes apart answer row 5 outright;
- a **screenshot** is the visual and HUD record — it answers "does it look right", and its numbers are far less precise
  than a save's.

Screenshots alone would not close a single row. Saves and recordings together close most of them.

## The instrument that changes the price of a row (2026-11-13)

`craft-anything`'s built-in engine answers the **identification** half of a row in **1-2 s at 190 MiB**, with no Ghidra
project and no import: raw string search (including literals Ghidra's inventory misses - it found `VER 5.7` at `0x9db348`
and its one xref in **1 s**), function boundaries (**19,484** where Ghidra gives **15,465** - never quote one without its
tool), function dossier, callers, and `core-feature-find`, which traced `repathing` to `sub_7f4330` and `sub_8a4360` - the
two functions `docs/movement-in-the-binary.md` already names - in **2 s**. Its pseudo-C is a lifted listing (*"partly
unstructured"*, 18 inline-asm in a 1,128-byte function), so it is an instrument for *finding*, and the Ghidra corpus stays
the instrument for *reading*. The dat and the saves are outside it entirely. Full measurements, limits and errors:
`docs/reviews/2026-11-13-craft-anything-evaluated.md`.

**Per row, the query is now the same shape:** `core-feature-find --query "<the rule's own word>"` for rows 3-12, and
`native-function-callers-list <address>` where a candidate is already known (row 2: `0x744d60`).

## What this file is for

Each open row becomes a decision input: a row that rules **different** is a defect with an address; a row that rules
**same** is evidence the "AoE2 vibe" is real rather than hoped for; a row that needs the oracle is a reason to capture a
specific artifact, not a reason to stop. When a row is ruled, move it out of this table into `docs/decisions.md`'s
provenance or the changelog row that landed it, and leave the evidence pointer here.

## The identification map: the binary asserts with its own source paths (found 2026-11-01)

The single most useful thing the corpus gives us is not the pseudocode — it is that **the original's assertions carry
`__FILE__` and `__LINE__`** through a helper (`FUN_005404f0(path, line)`), so a decompiled function often names the source
file it came from and the line number it was at:

```c
iVar3 = FUN_005404f0(
    "c:\\users\\jakes_workstation\\...\\aoc\\source\\game\\randommap\\rmm_obj.cpp", 0x343);
```

Scanned over the exported corpus: **939 path references, 931 distinct `(path, line)` assertion sites across 84 source
files, carried by 260 of the 15,465 functions.** That is a module map of the original engine, and its shape is exactly what
the register needs:

| source file | assertion sites | functions | what it is |
|---|---|---|---|
| `move_obj.cpp` | 159 | 15 | the moving object: movement, task state, attack plumbing |
| `motion.cpp` | 81 | 3 | the integrator — the per-tick motion of a body |
| `game\basegame.cpp` | 44 | 16 | the game loop, tick order |
| `game\comm\com_sync.cpp` | 38 | 4 | the sync records a recording carries |
| `game\world\worldplayer.cpp` | 34 | 8 | a player's economy and production |
| `game\baseworld.cpp` | 31 | 12 | the world: entities, spawning, removal |
| `stat_obj.cpp` / `t_b_obj.cpp` / `t_c_obj.cpp` | 7 / 2 / 4 | 9 | static objects, buildings, training |
| `path.cpp` / `pathsys.cpp` | 2 / 2 | 4 | pathing (U7) |
| `game\ai\*` | 40 | 18 | the AI: `aiuaimod.cpp`, `taitacmd*`, `taiuaimd.cpp` |

Consequences worth acting on rather than re-deriving:

1. **Rows about movement, the tick and pathing now have a known neighbourhood** (`move_obj.cpp`, `motion.cpp`,
   `basegame.cpp`, `path.cpp`) — the search is over a few dozen functions, not 15,465.
2. **The absence of a `combat.cpp` is information**: damage is not a module of its own in this build. It is either inlined
   into `move_obj.cpp`'s attack path or lives in the static-object code, which is where register row 2's search should go
   next.
3. **Line numbers are stable identifiers.** A row can cite `move_obj.cpp:0x4a1` and a later session can find the same
   function by string, even after Ghidra renumbers its own `FUN_` names.
4. The corpus itself is **install-derived and never committed** (D8/D22): 72 MB in `/opt/data/tools/rea/pseudo-aokhd`, the
   index in `/opt/data/tools/rea/source-path-index.txt`, both regenerable in ten minutes by the script named below.

**Row 2, call sites read (2026-11-13).** The five callers of `0x744d60`, and what each does with it:

| caller | call | shape |
|---|---|---|
| `0x744ff0` | `FUN_00744d60(param_1, 0, ...)`, multiplier `0x3f800000` (1.0) | thin wrapper: fills the three `(class, amount)` pairs and returns **their sum** |
| `0x745060` | same, multiplier 1.0 | the same wrapper with its arguments in a different order |
| `0x741a30` | two calls, one with a literal `0x26d` (621), both at 1.0 | a 6.7 KB function that builds records; the plausible combat path |
| `0x6bb100` | `FUN_00744d60()` with no arguments at all | almost certainly a mis-attributed call (the listing shows no argument setup) |
| `0x744d60` | itself | builds the picture: three pairs at `+0x18c/+0x18e`, `+0x192/+0x194`, `+0x198/+0x19a`, per-class armour from the float table at `+0x3c`, leftover folded into **class 3**, and every entry below 1 **dropped** (`class = -1`) |

**So the multiplier is 1.0 at every call site** - nothing scales damage here - and the sum of the surviving classes is what the
wrapper returns. The shape difference already registered stands: a **per-entry drop**, where `combat.toml` floors the **total**
at 1, so an attack wholly absorbed by armour deals **0** in the game and **1** to us. What is still open is only *which* caller is
combat's: `0x741a30` is the candidate, and the corpus has no callers for it, so the next step is a vtable or a virtual-call
resolve rather than another string search.
