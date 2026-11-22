# The divergence register: every place this engine knowingly differs from the original

> **This is the live register.** `docs/pathfinding-gap-register.md` is its predecessor and is marked superseded by D279 -
> read that one for the research, this one for what the engine does today.

*Written 2026-11-22, on the owner's instruction: **"purge our rulings which are not conformant with original game, decomp or
online sources once and for all"** - and because a ruling that says "the game does X" when the game does Y is the failure that
keeps costing him a session.*

**Two verdicts, and nothing else is allowed in this file:**

- **DEFECT** - the game's behaviour is read, ours differs, and the difference is ours to fix. A row is only DEFECT when a
  source is named. Each carries the step that closes it (`docs/next-step.md`).
- **OURS** - a deliberate divergence, usually the owner's ruling, and the row says *why* and *what the game does instead*. A
  row here is not a bug and must not be quoted as parity.

**The rule that makes this auditable**: every decision row may carry one line,
`**Conformance**: GAME - <pointer>` / `OURS - <why>` / `UNVERIFIED - <what would settle it>`,
and `python3 tools/conformance_audit.py` lists the rows that talk about the game while carrying neither a pointer nor a tag.
The bank is at **0 pending** as of this file; the disease it cannot catch is a *wrong reading of a correct citation*, and the
only guard for that is re-reading the binary before a slice is built on a row (D350's own correction is the example).

## 1. DEFECT - read, ours, scheduled

| mechanism | the game | ours | source | closes with |
|---|---|---|---|---|
| the plan's body test | a stander blocks **every tile its grown box touches** (rect clamped to tile indices, stamped) | a tile is blocked only when its **centre** is inside the box, so a body off-centre blocks less and a body on a corner blocks nothing | `FUN_007f9d80` -> `FUN_007f9b70` -> `FUN_007f6ff0` | `next-step.md`, port 1 |
| the tier's smoothing | the segment test walks the **object system's** buckets, so bodies are in it | `chain_leg` - objects and grid, **no bodies** | `FUN_008acf20` -> `FUN_00840c60` -> `FUN_00841020` | `next-step.md`, port 2 |
| the tier's refinement | the crossing is the first tile along the span where the direction is **passable**, and passable consults the non-ignore objects | the crossing is interpolated **by geometry**, so it can land inside a body | `FUN_007eeda0`, `FUN_00801a20` | `next-step.md`, port 3 |
| the arrival's place | a taken spot is resolved by stepping out `ext_a + ext_b + 0.01` on one of four sides, 50 tries, refusing any obstructed candidate | every man of a node is sent to **the same ring tile**, and the last step onto it has **no body test** | `func_002699A0`, `func_002697C0` | D351 (branch `d351-reseat`) |
| the collision test's bounds | four `<=` on the grown boxes, plus the `+0x268` inset when the byte at `+0x266` is set | strict `<`, no inset | `FUN_00840830` | P3 - the inset's *value* is unread |
| the plan's obstacle filter | `(not on the mover's own list) AND (isMoving() == 0)`, plus the caller's explicit list | `has_route && moved` - **a third clause the binary does not have**, kept deliberately (dropping it made every crowd unplannable, D349) | `FUN_007f9d80:45` | re-read when the ignore list's writer is found |
| the past test's gate | the whole test sits behind `local_e0 == 0 || FUN_007dd9d0() != 0 || class in {3, 0x1b, 0x27, 0x34}` | the test always runs | `FUN_008a4360` (~0x1265) | **unread**: what `local_e0` holds at that point |
| the terrain re-plan's rate | a re-plan on **every** terrain collision | `REPLAN_TICKS = 20` | `0x122f`, `0x128b` | priced, not scheduled (D332) |
| the levelled search's activation | the mover's window is `radius + margin`, margin 0 | a whole-walk tier above 16 tiles (D306/D338/D347) | `FUN_007f0be0`, `FUN_007ee980` | the margin at vtable `+0x12c` is unread (D348) |
| the inventory of a walk | a path object always exists; a reached waypoint is snapped to and spent | a `Task::Move` whose route is spent walks on the task layer's own direction | `008a4360` (waypoint epilogue) | mitigated by D349, not closed |

## 2. OURS - deliberate, and what the game does instead

| ruling | what we do | what the game does | why ours |
|---|---|---|---|
| **D289 / D293** | every man is sent to the tile that was clicked, no man owns a place, a crowd that does not fit jams | sends every man to the resource or the point; the *plan* keeps them apart by stamping stander tiles, and no seat is reserved | the owner, 2026-11-19: *"Drop it and let the crowds jam"*. **The stacking this produces at scale is our defect, not this ruling's**: the rows in section 1 are the difference |
| **D290** | no minimum separation between two bodies at rest | the same: no pushing, no floor | conformant by construction |
| **D246** | a villager working a node is never nudged off his place | the same: no pushing; its own placement separation runs once, when the spot is assigned | the owner. D351 reads it narrowly: a man standing *inside* another body is re-seated, a 0.011-tile drift is not |
| **D204** | a stalled gatherer re-targets within a resource **kind**, never across | the engine's next-source search is same-kind inside a radius; cross-kind repurposing belongs to the AI (D291) | the owner + `aoe2mc` |
| **D17 / D18** | the engine knows kinds, not particulars; content names the numbers | the original's own tables | the project's shape, D17/D18 |

## 3. What this register is for

When a slice is about to rest on "the game does X", the question is: **is X in section 1, section 2, or nowhere?** Nowhere
means it is a claim waiting for a source - and the row's `**Conformance**` line is where it goes, not a fixture.
