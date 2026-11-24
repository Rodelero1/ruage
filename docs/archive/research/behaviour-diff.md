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
| 1 | dat version gate | importer writes what it read; loader accepts it | **exact string equality against `VER 5.7`** at `0x9db348`, one branch at `0x72d4fc` in `FUN_0072d1a0` | **same** (semantics now known; no numeric parse, no tolerance) | `5ba445c`, `docs/archive/research/re-toolchain.md` |
| 2 | damage: armour subtraction, bonus classes, minimum-1 clamp, order | `crates/sim::damage_between` - per class `max(0, atk - armour)`, summed, then the **total** floored at `combat.toml`'s `minimum = 1` | **`max(1, sum)` once per attack** - corroborated three ways: the Armor class page's Step 6 (`Damage = max(1, Damage4)`, applied **after all multipliers**, the sum taken over every armour class), the Attack page's `Damage = max(1, k_ele x SUM_i max(0, Atk_i - Arm_i))`, and the **HD 2013** combat guide's three notes (each difference can never be `< 0`, and "the final **sum** ... can never be `< 1`"). The binary agrees where it can: `sub_744d60` drops a per-slot remainder `<= 0` and `sub_745060` returns the sum with no clamp, so the clamp is the caller's - which is the shape every source describes | **same** (three sources; the per-slot-clamp reading is excluded) | D227, D228, this file |
| 3 | the per-tick unit update: speed integration, attack reload, gather tick, in what order | `World::step` - one **turn per player** now (D230), with the blow pass ordered `(player, id)` | **`BaseWorld::update` = `FUN_00738720` at `0x738720`**, named by its profiler string, its `aoeworld-at-*.txt` status log and its `baseworld.cpp` assertions: the players array at `+0x184..+0x188` walked from index 0 (Gaia, then 1-8), and inside a player's turn that player's object list (head `+0x48`, `next` `+0x5c`, owner `+0x60`) walked object by object - per object one virtual call decides removal (`+0xa0`; 0 answers, the node is unlinked and vtable slot 0, the destructor, runs) and otherwise one virtual call runs the object's own update (`+0x84`). Death is therefore applied **in the victim's own turn**, and `pturn` at `+0x100` advances by one per update | **read** for the order *between* objects; **unknown-static** for the order *inside* one; **needs-oracle** for the effect | D229, `docs/next-step.md` T12, `docs/mechanics/determinism.md` |
| 4 | age-up timing at a kind-102 gate | `advance`/effect interpreter | not yet located | **needs-oracle** | question 3, U10 |
| 5 | decay: does an unattended building vanish, and at what rate | `[decay] rate_from = "dat"`, `resource_decay_rate` semantics unverified | not yet located | **needs-oracle** — and the game answers it from two saves 5 minutes apart | `docs/next-step.md`, D85's review |
| 6 | gather rate and the fractional accumulator (0.0111, whole units) | `crates/sim` gather; `Gather::fell_hp_per_tick` | not yet located | **unknown-static** | U1 |
| 7 | heap/carry caps and rounding (10 × 1.25 → 13) | `scale_fixed` with rounding | not yet located | **unknown-static** | v1.12 |
| 8 | projectile lead (Ballistics), minimum range, blast radius | `combat.toml`, `Task::Fly` | not yet located | **unknown-static** | U4/U6 |
| 9 | building placement, footprints, overbuild/refund | D58/D60/D76 | not yet located | **unknown-static** | D76 |
| 10 | vision radius and fog memory | `explored` grids | not yet located | **unknown-static** | — |
| 11 | population cap and house arithmetic | D31/D34 | not yet located | **unknown-static** | — |
| 12 | match end (conquest: a razed town centre with villagers alive is not a win) | D59b/D78 | not yet located | **unknown-static** | — |
| 13 | path step cost and the octile estimate | `path.rs:16-17` **`CARDINAL = 10`, `DIAGONAL = 14`**, summed as `straight*10 + diagonal*14` (`:112`, `:284`), with an **admissible** estimate | **16 per cardinal step**; the diagonal is a load-time table at `0xafd2a0` (`table[min] + 16*(max-min)`, `FUN_007ffc80`), and the estimate is Euclidean **x 25** (`FUN_008007f0`: `*2500/100`, or `*0x19` on one branch - the same number twice), which **overestimates**, i.e. a **weighted** best-first search at about 1.56 | **different** in the search's shape, not only its constants (settled 2026-11-16: the "not admissible" puzzle was my assumption, not a contradiction in the game) | `docs/archive/research/movement-in-the-binary.md`, D239 |
| 14 | a collision is queried **at the new position, with the unit's radius** | `world/movement.rs::collide` tests the candidate point against `body_radius` | `FUN_00840440` over the object system, radius = the unit's own radius field times a multiplier, mask `0x19`, player sentinel `0xfffffffe` | **same** | `docs/archive/research/movement-in-the-binary.md` (2026-11-13) |
| 15 | a friend is a wall: no owner test in collision | D200 | no player, team or alliance comparison anywhere in the block; the only id test is against the order's target | **same** | `docs/archive/research/movement-in-the-binary.md` |
| 16 | a collision with the order's target is arrival | `WaitTarget` | \"That's a target collision\" → \"That collision is with our target, so we're done\" (0x1251, 0x127d) | **same** | same file |
| 17 | a non-past collision is answered by **re-planning**, not by waiting | `Repath { body }`, landed 2026-11-16 (D237) | \"We have %d non-past collisions and %d terrain collisions, repathing\" (0x128b); the avoid list is extended and the parent path installed | **same** (D98's \"wait and let pass\" was the divergence; it is gone) | D237, `docs/archive/research/movement-in-the-binary.md` |
| 18 | right of way: lower id proceeds, higher pauses; the pause keeps its path | `step.rs:1276-1298`: `rand()*1.25/32767 + 0.25` s from `sim_rng`, `Entity::pause_remaining` in ticks - and **a running timer is never re-drawn** | `[+0x2b4]` on both units, `cmp eax,esi / jle` → `setPause(rand*1.25/32767+0.25)` writing `path+0x28` **unconditionally**; the paused unit keeps its waypoint list | **different in the re-roll** (corrected 2026-11-16; the tie-break and the kept path are same) - `setPause` has no "already paused" guard, so a fresh collision re-draws, and ours stands the original draw | `docs/archive/research/movement-in-the-binary.md`, D239 |
| 19 | a body that stopped before this route was planned is **past** | `is_past`: `route.created >= other.last_moved` (D210, D213) | `path+0x10` creation time against the other's `+0x134 lastMoveTime`; past entries are dropped with no pause, no repath, no bound | **same** | `docs/archive/research/movement-in-the-binary.md` |
| 20 | which bodies a mover plans through | soldiers ordered in the **same tick** become mates (`stamp_group`, D237); villagers never | the route's **path-ignore list**: formation members, re-tested **each tick** on `isMoving`, plus the order's target; membership drops a collider outright (\"That's a path ignore unit\", 0x1242) | **different in trigger and in the ending** (corrected 2026-11-16): the original re-tests `isMoving` every tick, so a **paused** mate is not exempt; ours ends the exemption only when the route is dropped, and a paused man keeps his route | `docs/archive/research/movement-in-the-binary.md`, `docs/prior-art.md` |
| 21 | passage is a property of the **ground**, never of the body | D117: a `trebuchet_packed` (radius 0.5) walks a one-tile gap | the query **grows by the body's radius**, and the short-range planner shrinks the body to a point and expands every obstruction by its radius (FE talk, steps 1-2) | **different, in the graph layer only** (corrected 2026-11-16; superseded by D239): our tile graph is per-tile and a radius never enters it, but `step_is_walkable` already grows the ground by the body's radius (D176/D192), so the step layer is the game's and the divergence is one layer, not two | D117, D239, `pathfinding-from-the-talk.md` |
| 22 | a *ground* re-plan is capped at two per route | `MAX_GROUND_REPLANS` (D127, D135) | no such counter: every non-past collision re-plans | **different** (priced: uncapped, the acceptance ran 3.5 ms/tick against 0.3; adopted 2026-11-16, re-measured by D239) | D127, D135, D239 |
| 23 | an **exclusive goal**: a `Move`'s last step stops short of an occupied tile | D52 | no such rule read; the collision answers at the new position | **unknown-static** (the visible outcome is similar) | D52 |
| 24 | a contested work square | D204 as built: after 40 refused ticks the man takes the **nearest free node of the same kind within `gather_reach`** (LOS + 5 tiles, D149) and **waits** when there is none; the idle-body nudge is the other half | **no displacement anywhere**; the owner, 2026-11-16: *"if there is no other bush nearby, then worker waits for free spot, if there is another bush nearby, he will go work there"* - which is what D204 does | **same for the re-target; different for the nudge only** (corrected 2026-11-16: D239 read the earlier sentence as "circles the same bush" and that was wrong) | D204, `step.rs:2178-2212`, `gather.rs:64-110`, D241 |
| 25 | a node's ring when a man is refused | `maybe_respread` re-seats **every** man on the node, at most once per 20 ticks (D200) | incumbents keep their squares; the newcomer goes to a nearby node or waits (D241) | **different** (the D238 blocker; adopted, D239) | D238, owner 2026-11-16, D241 |
| 26 | a waypoint that cannot be reached in two updates is **substituted** | none | `mConsecutiveSubstituteCount`: \"We won't reach our waypoint either this or the next update, but we have a count of %d, faking it\" (move_obj.cpp:0x1203) | **different** (mechanism missing; adopted, D239) | `FUN_008a4360`, D239 |
| 27 | path structure | one A\* level (`plan_path`) plus the `short_range_clear` pencil | two levels with a parent: `AddFuturePath` builds a high-level path when handed a low-level one; \"We don't have a parent path, so we're stopping\" (0x12a3) | **different structure** (adopted, with a read in front: D239) | `docs/archive/research/movement-in-the-binary.md`, D239 |
| 28 | standoff between a worker and the node | **0.70** at an edge, **0.94** at a corner (D169, D199) | **measured** on the owner's saves: bodies at 0.641 / 0.702 x3 / 0.935 x3 round a tree, mine and bush | **same** (the strongest class - measured off the game, not read) | D199 |
| 29 | a body **at rest** that is not past | `collide` rule 6: `TakeAtRest` - the step is taken **through** him, unbounded, after a repath has been asked for | no such arm: the epilogue has four - target / non-past repath / pause / `"Don't know what to do about my collision(s)"` - and an idle friend who is not past is a **non-past collision** | **different** (found 2026-11-16 while ruling the batch; adopted, D239) | `crates/sim/src/world/movement.rs`, `docs/archive/research/movement-in-the-binary.md` |
| 30 | what a re-plan plans around | the bodies met are turned into **obstacle tiles** for the search (`step.rs:1325`) and kept for the life of the route, so each re-plan avoids a growing set of tiles | the ignore list is written **after** the search and read **only by the classifier**; the search's obstacle set is standers and the mover's mates, and the body it met is a **disc** | **different** (found 2026-11-16; adopted, D239) | `crates/sim/src/world/step.rs`, `docs/archive/research/movement-in-the-binary.md` |
| 31 | a path is **never a failure** - a **walk** order | `walk_goal`: a blocked or unreachable place becomes `nearest_reachable` (D134), decided once when the order lands | *"expand the search area once, otherwise give the closest point path"* (the talk, p. 83); every click is accepted | **same** (D134) - **but the mechanism is open**: `nearest_reachable` answers one step from the start, not the nearest tile (D247, `crates/conformance/tests/findings.rs`, queue R10) | `crates/sim/src/lib.rs:2395-2430`, `path.rs:314`, `pathfinding-from-the-talk.md` |
| 32 | a path is never a failure - a **thing** order (gather/build/attack) | `SimError::NoPath` - a tree behind a wall is **refused at order time** (`orders.rs:52`) | the search returns a closest point; what the task layer does is unread - **but the owner has ruled the behaviour** (D241): walk as close as possible and idle, and work the nearest same-resource node when the obstruction is other resource | **different** (ruled by the owner, not read) | `orders.rs:52`, D241 |
| 33 | the start is **re-seeded** when the mover stands inside a body | none. `short_range_clear` lets a route leave a disc it already stands in, but only **outward** | `FUN_007fb220`: offset the start by **2 x radius** along a fixed direction ring (`DAT_009e252c`), re-seed, and return **3** ("no low-level path") only when every direction is blocked | **different** (mechanism missing; our outward-only edge is the stand-in) | `docs/archive/research/movement-in-the-binary.md`, D239 |
| 34 | the pause is enforced **twice**: the path's timer **and** a velocity cap | one mechanism: `pause_remaining` skips the step (`step.rs:1169-1192`), so the body does not move | `[+0x2b0]` writes `path+0x28`, and the velocity accessor returns 0.0 - the string `ret 0.0f due to pause reason` is in the exe (grep: 1) | **same in effect**; different in construction | exe string verified 2026-11-16; sibling session's read of `008a2a30` |
| 35 | **straight line before any search** | `plan_path` asks `short_range_clear` first, which answers a straight line when the lane is clear | *"Looking for a STRAIGHT LINE path ... There are no non-target collisions there, so we can do the straight line path"* (move_obj.cpp:4079-4112; the string is in the exe, grep: 1) | **same** (ours is stricter: it also clears the terrain, not only non-target bodies) | exe string verified 2026-11-16, `lib.rs:2185-2200` |
| 36 | the order's **path vocabulary**: forced waypoint, teleport-to-waypoint, per-path hit | none of the three; patrol and attack-move exist (D47), and a `Stop` clears the route | `ADDmove`'s fields include **patrol**, **linear**, **forcedWaypoint**, plus `TeleToWP` and `HitTarget` | **different** (vocabulary narrower) | sibling session's read, 2026-11-16 |

**Row 2's arithmetic, read directly (2026-11-15).** `sub_744d60` and its 1.0 accessor `sub_745060` were decompiled rather than summarised, and five things are
now facts rather than readings: the attacker record carries **exactly three** `(class, amount)` slots at `+0x18c/+0x18e`, `+0x192/+0x194`, `+0x198/+0x19a`; each
amount is scaled by the multiplier at `[ebp+0x28]` and **truncated**; armour is a **float table at `this+0x3c` indexed by class**; a positive leftover is **folded
into class 3** when a slot is class 3 or empty; and a slot whose remainder is `<= 0` is marked `-1`, i.e. **dropped per entry**. What the accessor does **not**
do is clamp: it sets the multiplier to `1.0`, calls the arithmetic and returns `slot1 + slot2 + slot3`. The accessor is a **virtual method** - its address is
taken five times and it has no direct callers - so the 1-damage floor sits at an indirect call site and stays unproven.

Against `damage_between`, that gives three statements and one gap:

- **Same sum.** Dropping a non-positive term and clamping it at zero are the same arithmetic, so the total matches for whole-number inputs.
- **Different attribution, not different total.** The original moves a leftover into **class 3**; we leave it on the class it came from. Only the sum is read
  by the accessor, so this cannot change damage today - it would only matter to a rule that reads a per-class amount.
- **Armour is a float in the original and an integer in ours.** A fractional armour modifier is representable there and not here; no shipped modifier is
  fractional today, so this is a latent difference rather than a live one.
- **The floor is the open question**: `minimum = 1` is ours, and the original's clamp has not been found.

**Row 2 ruled `same` (2026-11-15, D228).** Three independent sources put the minimum **once per attack, on the total, after every multiplier**: the Armor class
page's step 6, the Attack page's formula, and - the decisive one for this project - the **Age of Empires II (2013) combat guide**, which states it as three
arithmetic notes on exactly this edition. The binary does not contradict them: `sub_744d60` drops a slot whose remainder is `<= 0` and `sub_745060` sums the three
with no clamp, so the clamp is the caller's, and "once on the sum" is what the caller must be doing.

Three latent differences are recorded rather than fixed, each because it cannot arise in this pack today:

- **A negative class damage is possible in the original and impossible here.** The published step 2 is `max(min(0, Atk_i), Atk_i - Arm_i)`: a *negative* attack keeps
  its (negative) value instead of clamping at zero. Our pack carries no negative attack that any reading explains - D216 refuses them by name - so the branch
  cannot be reached, and `max(0, ...)` is the same arithmetic for every value we ship.
- **Bonus-damage resistance** (`k_res`, a multiplier on incoming bonus damage, "usually 0") is not modelled. Nothing in the 5.7 effect table names such a modifier;
  when one does, this row gains a term.
- **The floor is after the multipliers in the original and before one of ours.** `damage_between` applies `minimum` and *then* the blow is multiplied by
  `elevation_factor`; the published formula is `max(1, k_ele x SUM)`. It cannot show today because every height is zero (D217 keeps the term at 1.0), and it is
  written here so that the slice which makes elevation real does not have to rediscover it.

**The count: 2 of 12 ruled.** That is the honest state, and it is a better answer than "we compared them" — the eleven open
rows are a to-do list with a definition of done each. Row 3 has since moved from *not yet located* to **read for its order
half** (2026-11-15, T12): the order between objects is 5.7's own now, but the row also covers the order inside one object and
the effect itself, neither of which is read, so it is not one of the two.

**Row 2, first half ruled (2026-11-13, craft-anything).** `native-function-callers-list 0x744d60` in **2 s** gives **5 call
sites**: `0x6bb100`, `0x741a30` **twice** (`0x741c3c`, `0x741db0`), `0x744ff0` and `0x745060`. Both candidates this file had
guessed at - `FUN_006bb100` (30 KB) and `FUN_00741a30` (6.7 KB) - are real callers, so the per-entry-drop arithmetic is
reachable from the two functions that could be the damage path. What is still unruled is *which* of them combat uses, and
that is reading, not locating.

## How the oracle feeds it

Static reading cannot close rows 3, 4 and 5, and cannot certify any row as *same*. The owner's plan — two AIs, a recorded
game, periodic captures — closes exactly those. `docs/archive/research/oracle.md` carries the protocol and the settings; the short version:

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
two functions `docs/archive/research/movement-in-the-binary.md` already names - in **2 s**. Its pseudo-C is a lifted listing (*"partly
unstructured"*, 18 inline-asm in a 1,128-byte function), so it is an instrument for *finding*, and the Ghidra corpus stays
the instrument for *reading*. The dat and the saves are outside it entirely. Full measurements, limits and errors:
`docs/reviews/2026-11-13-craft-anything-evaluated.md`.

**Per row, the query is now the same shape:** `core-feature-find --query "<the rule's own word>"` for rows 3-12, and
`native-function-callers-list <address>` where a candidate is already known (row 2: `0x744d60`).

## What this file is for

Each open row becomes a decision input: a row that rules **different** is a defect with an address; a row that rules
**same** is evidence the "AoE2 vibe" is real rather than hoped for; a row that needs the oracle is a reason to capture a
specific artifact, not a reason to stop. When a row is ruled, move it out of this table into `docs/archive/decisions-2026-11-24.md`'s
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

**Row 2, why the corpus has no callers: `0x745060` is a virtual method.** A byte search of `AoK HD.exe` for each address as a
little-endian literal:

| address | literals in the binary | reading |
|---|---|---|
| `0x745060` | **5** (file offsets `0x5db0bc`, `0x5dba58`, `0x5dbce0`, `0x5dbf24`, `0x5dc0c4`) | **vtable slots** - the wrapper is a virtual method of five classes, which is why no caller exists to find |
| `0x744ff0`, `0x741a30`, `0x744d60` | **0** | called directly; the decompiler's caller lists are complete for them, and `0x741a30`'s caller is not in the corpus |

So "which of the five call sites is combat's" becomes a **class** question, not a call-site question: the damage wrapper is reached
through a vtable, and the next instrument is to resolve those five slots to their owning class constructors - `native-xrefs` on the
slot addresses - and read which class the combat code holds.

## The movement rows, ruled (2026-11-16)

**The count as first written, and corrected by the recount at the end of this file: 28 rows, 18 ruled.** Rows 1 and 2 were ruled before today; rows **13-28** are all pathfinding or obstacle
avoidance, added for the owner's question of 2026-11-16 (*"review all my decisions on pathfinding and obstacle avoidance,
compare them with gathered information and list where my decisions diverged"*) and **every one of the sixteen carries a
verdict**: **7 same, 8 different, 1 unknown-static**, each citing either the corpus, a first-party talk, or a save.

**Same, and it is the larger half**: no pushing anywhere; a friend is a wall (D200); the target collision is arrival; a non-past
collision is answered by re-planning (D237, which removed D98's "wait and let pass"); right of way is the lower id and the
loser pauses 0.25-1.50 s from a seeded stream (D205, and `step.rs:1276-1298` implements the original's own formula); a stopped
body is *past* (D210/D213); the collision query is at the new position with the unit's radius; and the worker-to-node standoff
(D169/D199) is **measured off the owner's saves**, which is the strongest evidence any row in this file has.

**The eight divergent rows, in the order they cost the player something** (the last item bundles three mechanism gaps):

1. **Line 20 - which bodies a mover plans through.** Ours is "ordered in the same tick, stamped once"; the original's is
   "formation members, re-tested each tick on `isMoving`". D237 fixed the villager half of this; the *re-test* is still
   missing, so a mate that stops mid-route stays a mate for ours and stops being one for the game.
2. **Line 25 - the ring re-lay.** `maybe_respread` re-seats every man on a node when one is refused. The original does not:
   the owner, 2026-11-16 - *"In case there are several men around the bush, 1 other man joining them will be walking around
   until he finds a spot for him."* This is the live defect D238 measured (a seated man abandoning his square, `SEATMOVE`), and
   it is the one row where a single sentence of the owner's outranks everything we have read.
3. **Line 24 - a contested work square.** D204 sends the man to the **next node of the same column**; the owner's statement is
   that he keeps **walking around the same bush** until a square frees. Also: the nudge that D204 kept for a body with no work
   has no counterpart in the original - a body at rest is never displaced, for any task.
4. **Line 21 - passage is the ground, never the body (D117).** The game grows the query by the body's radius and shrinks the
   body to a point, expanding every obstruction by that radius; D117 says a 0.5-radius unit walks a one-tile gap. This is the
   owner's own rule and it is deliberate, so the row is a *record*, not a defect - but it is a real difference and it is the
   one that would show on a chokepoint map.
5. **Line 22 - the two-re-plan cap (D127/D135).** The original re-plans on every non-past collision and has no counter. Ours
   caps ground re-plans at two because uncapped it ran **3.5 ms a tick against 0.3**. Bounded scope, priced and recorded.
6. **Lines 13, 26, 27 - three mechanisms the original has and we do not**: cost 16 with a table rather than 10/14 with a
   product (a ratio difference, no behavioural cost today), the waypoint substitution counter (`mConsecutiveSubstituteCount`,
   "faking it"), and the two-level path with a parent. The middle one is the cheapest to copy and the most likely to fix a
   stall nobody can explain.

**The one row that should be read before it is believed differently:** line 23, the exclusive goal. It is the only movement
rule of ours that has **no counterpart** in the original and is also **not** a divergence: the original answers at the new
position, so its crowd behaviour and ours could still agree in effect. It stays `unknown-static` until a recording of forty
units ordered to one tile is measured - and that recording is cheap, because it needs no AI and no economy.

## Corrected the same day, and adopted (D239, 2026-11-16)

**The recount, taken from the table and not from the prose above: 36 rows, 25 ruled. Rows 13-36 are pathfinding or obstacle
avoidance - 24 rows, of which 23 carry a verdict: 10 same, 13 different, 1 unknown-static.** The morning's figures (7 / 8 / 1
across sixteen) were, in the end, three corrections, six rows that had never been written down at all (29-30 and 31-36), and two
rows the owner has since **ruled** rather than left to a reading (24 and 32, D241).

- **Row 18 was wrong to be `same`.** `setPause` (`0x8aa6d0`) writes `path->pause` **unconditionally**; `step.rs` refuses the
  draw while `pause_remaining > 0`, and the comment beside it claimed "a running timer is never re-rolled", which the read does
  not support. A fresh collision re-draws in the game; ours stands the first draw. The tie-break and the kept waypoint list are
  still `same`.
- **Row 20 was half wrong.** `is_mate` needs *both* routes to still list each other, so a mate who arrives stops being exempt
  the same tick, and `stander_tiles` already drops a body that moved within a tick. What is missing is the **paused** mate: he
  keeps his route, so we still walk over him where the game's `isMoving` re-test has ended the exemption.
- **Row 21 named no layer.** D117's "a radius never enters it" is true of the **tile graph**; `step_is_walkable` already grows
  the ground by the body's radius (D176/D192). The divergence is one layer deep, not two.
- **Row 29 (new): a body at rest that is not past is walked through.** `collide` rule 6 takes the step (`TakeAtRest`), unbounded,
  after a repath has been asked for - so the route round him is installed *after* he has been stepped through. The original's
  epilogue has four arms and this is not one of them: an idle friend who is not past is a non-past collision.
- **Row 30 (new): our re-plan plans around what it met.** The avoided bodies are turned into **obstacle tiles** for the search
  (`step.rs:1325`) and kept for the life of the route, so every re-plan plans round a growing set. In the original the ignore
  list is written *after* the search and read **only by the classifier**; the body that was met is a **disc**, and the search's
  own obstacle set is standers and the mover's mates.

**The ruling that follows, and it needed no further question: every divergent row is adopted from the game (D239).** The owner,
2026-11-16: *"Resolve the questions by adopting from the game. That is the source of truth."* So rows 18-22 and 24-30 become
slices, in the order `docs/next-step.md` sets; rows 13 and 27 are adopted **with a read in front** - the estimate-versus-step-cost
contradiction has to be settled before 16 and the octile table are ported, and `AddFuturePath`'s parent derivation before the
two-level path is.

**The one row that adoption cannot settle is row 23**, and it does not need the game's code, it needs **his recording**: forty
units ordered to one tile. Nothing we can read answers it, and no decision should be taken in its place.

**Row 31 and row 32 are one sentence split in two**, and the split is the finding: D134 already answers "come as close as you can"
for a **place**, and the tree refuses outright for a **thing**. The talk's rule is about the search, so it settles the first and
says nothing about the second - which is why one is `same` and the other stays `unknown-static` rather than becoming a defect.

**Rows 31-36 came from the sibling thread** (`#age2hd`, 2026-11-16: *"What are you able to copy ... in area of pathfinding and
obstacle avoidance?"*), and every one was re-checked against this tree before it was written down: 31 and 35 read the code, 34 and
35 grepped the exe for their own strings (`due to pause reason`, `Looking for a STRAIGHT LINE path` - one hit each, 2026-11-16),
and 33 and 36 carry a sibling session's read **labelled as not re-read here**. Two of that thread's items are deliberately **not**
rows: the per-path **continue counter** (how many times a route has resumed) is instrumentation with no rule reading it, and DE's
Collide & Correct is a **later build's** mechanism - both belong in the register only as "not copied, and why".
