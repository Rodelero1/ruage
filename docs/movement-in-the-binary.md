# The movement module, read out of the binary without decompiling

Written 2026-11-03. Answers the owner's question ("can you read path finding and evasion from the game binaries?")
with the map first and the reading second. **The map is done and it was cheap** - the original asserts through calls
that carry its own source path, line number and a log string, so the exported pseudocode can be indexed by source file
without reading a single expression. `docs/re-toolchain.md` records the toolchain (REA on this box, D86); this file
records what the movement module *is*.

**Rebuildable without the corpus**: `tools/exe_map.py` reads the same source paths out of `Age2HD/AoK HD.exe` itself
(84 of them, plus 541 RTTI classes) and writes `docs/binary-class-map.md` - so the map above rests on a file this project
owns rather than on a decompilation whose strings happen to carry it (D140).

## The method, so it can be repeated

The corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 decompiled functions, one `.c` per function, named by
address). Every assertion or debug log in the original is a call of the form

```c
FUN_0053cfe0("c:\\users\\jakes_workstation\\...\\aoc\\source\\motion.cpp", 0x180, 10,
             "GroupID=%d BMotion::updateMovement", ...);
```

so `grep` for a source file name gives its **functions**, and the string argument gives, in the original's own words,
what the branch does. No pseudocode has to be interpreted to build the map:

```sh
cd /opt/data/tools/rea/pseudo-aokhd
grep -rl --include=*.c 'move_obj\.cpp' .   # 15 functions
grep -rl --include=*.c 'motion\.cpp'   .   #  3 functions
grep -ro --include=*.c 'move_obj\.cpp' . | wc -l   # 174 string matches
grep -ro --include=*.c 'motion\.cpp'   . | wc -l   #  91 string matches
```

`/opt/data/tools/rea/source-path-index.txt` already has the same mapping in text form for 94 source files.

**Count distinct `(file, line)` sites, not string matches.** Several branches log twice at one source line (the assert
and its debug-log sibling), so the same module counts differently depending on the extractor - the rule this project has
already broken once:

| motion.cpp | move_obj.cpp | total | what it counts |
|---|---|---|---|
| **81** | **159** | **240** | distinct `(file, line)` sites - **quote this** |
| 91 | 174 | 265 | raw string matches |

Either way it is **18 functions** (three in `motion.cpp`, fifteen in `move_obj.cpp`), out of the **1,029** `.cpp` string
matches the whole corpus carries - movement is the densest module in the binary by a wide margin, and the earlier
"159 / 81" note was right on the site count.

## The map

References are to the original's own source lines, as its asserts name them.

| Address | File | Refs | Role, from its own log strings |
|---|---|---|---|
| `008a4360` | move_obj.cpp | 132 | **`doIntelligentMovementUpdate()`** - the per-object movement update: `objId`, position, radius, the path state loop, "Top of ... state loop", distance to target/goal, waypoint incrementing, gates and child objects, and the whole collision block (lines 3,995–4,750) |
| `007f4330` | motion.cpp | 89 | **`BMotion::updateMovement`** (its own line 384) - the motion layer under it: pause time, path validity, "Top of IMU state loop", "Setting position to exact waypoint", "Incrementing waypoint", target range |
| `008a1ed0` | move_obj.cpp | 13 | **`ADDMOVE`** (2,405) - the order entry point: `targetID`, `targetPos`, range, **patrol**, **linear**, **forcedWaypoint**; builds the waypoint list from user waypoints and appends start/goal points |
| `008a1b10` | move_obj.cpp | 6 | **`AddFuturePath`** (2,641) - a **future path stack**: builds a high-level path first when given a non-high-level one, and prints the whole stack |
| `008a2a30` | move_obj.cpp | 5 | **Maximum forward velocity** (3,069–3,145): "ret 0.0f due to pause reason", `getMaxVelFwd`, "vel < 10.f", "cap unit max vel" - the order of the caps |
| `008a1880` | move_obj.cpp | 4 | **`AddCurrentPath`** (2,587) - path, target id, range, path type, and the waypoint dump |
| `008a3d70` | move_obj.cpp | 2 | Path/waypoint dump at line 5,427 |
| `008a1ed0`-adjacent small ones: `008a3bf0`, `008a92a0`, `008aa480`, `008aa5a0`, `008aa6f0`, `008abfe0`, `008acba0`, `008acf20`, `008ad130` | move_obj.cpp | 1-2 each | Accessors and setters whose asserts carry no message (lines 214, 523, 550, 622, 624, 1,309, 1,342, 2,731, 1,413) |
| `007f4190`, `007f4200` | motion.cpp | 1 each | Small motion helpers (lines 245, 351) |
| `007fdab0` | path.cpp | 11 | **The path state printer**: id, path, waypoint level `x of y`, creation time, patrol/linear, linked path id and type, `IncWPNU`, `TeleToWP`, `HitTarget`, `State=%s`, range, pause time, continue counter |
| `007ff920` | path.cpp | 2 | An **acceleration/velocity integration step**: `worldTimeDeltaSeconds`, `acceleration`, `velocity`, position |
| `008005e0`, `008007f0` | pathsys.cpp | 1, 1 | Path-system entry points; `008007f0` is a 580-line function with a large stack frame - the most likely site of the search itself, unread |

## What the strings already settle - and it changes the evasion question

`doIntelligentMovementUpdate` carries an explicit collision decision, in order:

```
    Looking for a STRAIGHT LINE path:                       (move_obj.cpp 4079)
      There are no non-target collisions there, so we can do the straight line path    (4102)
      There are %d non-target collisions there, though, so we can't do the straight line path (4112, 4245)
    There are %d must-collisions:                           (3995)
      Collision with terrain                                (4655)
      Collision with Invalid Unit #%d                       (4663)
        That's a target collision                           (4689)
        That's a non-target collision                       (4694)
        That's a non past collision                         (4709)
      Don't know what to do about my collision(s)           (an explicit fallback branch)
    That collision is with our target, so we're done        (4733)
    We have %d non-past collisions and %d terrain collisions, repathing  (4747)
```

Three facts follow, and none of them is a guess:

1. **The original repaths when the new position is blocked** - "non-past collisions and terrain collisions, repathing".
   It does not simply wait, which is what D98 ("wait and let pass") has our engine doing.
2. **It distinguishes collision classes**: with the *target* means arrival ("so we're done"); with an invalid unit or
   terrain means re-plan; a **"past"** collision is not counted at all, so a unit may overlap where it came from.
3. **It has a straight-line shortcut**: if a straight line to the goal has no non-target collisions, it skips the path
   entirely. That is a performance property with a behavioural shadow - two units in the open move differently from the
   same two with one obstacle between them.

**This is a fork for the owner, not a change I should make quietly.** D98 ruled "wait and let pass" as the cheap,
deterministic answer, and the test for it is exact. The original's answer is a repath, which is more expensive, needs a
route invalidated mid-flight, and is not what our pather currently does. The default I will take if nothing is said:
**keep D98 for now** (it is built, tested and cheap) and record this evidence against it, because a repath is a slice of
its own and the owner's stated preference is bounded scope over maximal fidelity.

## What is still unread

- **The A\* cost model.** No string in the whole corpus mentions a heuristic, an open/closed list or a cost - so this
  cannot be read from assertions, only from the pseudocode. The candidate is `008007f0` (pathsys.cpp, 580 lines, a large
  frame), with `007ff920` beside it as the velocity step. That is the "one or two sessions" part of the estimate.
- **Tie-breaking and equal-cost ordering**, which is the part our replay actually depends on.
- **Which branch decides between waiting and repathing** when a repath is not possible - the "Don't know what to do
  about my collision(s)" arm names itself but its condition is not read yet.
- **The object-system collision query** ("ObSystem collisions at our new position, pathCreationTime=%d") is a callee we
  have not identified; it is the function that answers "what stands at this position", and it is the natural thing to
  read first when the evasion slice starts, because it is the same question our `blocked()`/occupancy scan answers.

## Where the next hour goes, if this is picked up again

1. Read `008a4360`'s collision block (move_obj.cpp 4,600–4,760) - the conditions are a dozen comparisons and they decide
   the whole evasion question.
2. Read the ObSystem query it calls; compare it against our `Walk` layer and the entity occupancy scan.
3. Only then `008007f0`, for the cost model and the tie-break.

## The collision classification, read at last (2026-11-13): it is owner-blind

`FUN_008a4360`'s collision block is now read line by line in the corpus
(`/opt/data/tools/rea/pseudo-aokhd/FUN_008a4360_008a4360.c`, `move_obj.cpp:0x1225-0x129b`). What it does, in its own order:

1. **The collisions are a global array**, `DAT_00de1998` entries of `DAT_00de19a0[]`, filled by `FUN_008a92a0` →
   `FUN_00840440(x1,y1,x2,y2, mask, &count, &array, 0x19, 0xfffffffe, 0, 0)`. The query is at the **new position** and its
   radius is `*(float*)(unit+0xc) +0x34) * param_7`, i.e. the unit's own radius field times a multiplier.
2. **An entry is terrain when its `+8` object pointer is null** (`"Collision with terrain"`, line 0x122f).
3. **Objects are classified by unit id, not by owner**: `piVar13[2]` (the collided unit's id at `+0x8`) is compared with
   `*(int*)(order+0x24)` (the order's target id; negative means none) - equal → `"That's a target collision"` (0x1251),
   otherwise `"That's a non-target collision"` (0x1256).
4. **"Past" is a timestamp rule**: the entry counts as a *non-past* collision only if the path's creation time
   (`+0x10`) is older than the other unit's own timestamp (`[vtable+0x2ac]`), plus a class test on `+0x16`
   (`== 3`, `0x1b`, `0x27`, `0x34`). `"That's a non past collision"` (0x1265).
5. **The answer to a non-past collision is a repath**, not a wait: unless the list is exactly one entry and that entry is
   the target, it logs `"We have %d non-past collisions and %d terrain collisions, repathing"` (0x128b) and pushes them into
   the avoid list.
6. **The tie-break is a priority number, not a player**: over the same list it compares `[vtable+0x2b4]` on both units and
   the lower one is told `"TOLD TO PAUSE BY UNIT %d"` (0x129b).

**There is no player, team or alliance comparison anywhere in this block.** The only id comparison is against the order's
target. The query's ninth argument is the sentinel **`-2`**, where another caller of the same function (`FUN_007f7460`)
passes a real player value (`in_ECX[0x2a]`), which reads as "no owner restriction" - **an inference, not a proof**: the
filter's own comparison has not been decoded, and the collision query reaches the object system through a virtual call
(`[esi+0x2c8]`) that neither craft-anything nor REA resolves to a target.

**Consequence for D189.** The rule that *your own units are not walls* has **no support in the binary**: the collision
handler cannot tell a friend from an enemy, so a friend is returned by the query and counted as a non-past collision like
any other, and the original's move is to repath around it. D189 was chosen to solve crowd delivery in our engine, not
because the game does it. What the original does allow is the *past* exemption - a body the route already accounts for does
not block, and a unit may overlap where it came from - which is the one part of our current behaviour it supports.

## The tie-break read at last (2026-11-14): **the priority number is the unit's id, and the loser pauses 0.25–1.5 s**

`[vtable+0x2b4]` is **not a data field** - it is a virtual call, and the move-object implementation returns the object's own id:

```
0x8a80ed  call *0x2b4(%edx)      ; iVar5 = unit->priority()
0x8a80f9  call *0x2b4(%ebx)      ; iVar6 = other->priority()
0x8a80ff  cmp %esi,%eax / jle    ; if iVar5 < iVar6 { "TOLD TO PAUSE BY UNIT %d" }
```

The move-object vtable is `0x9e82c0` (its constructor at `0x8a13b2` stores it and then zeroes exactly the offsets the collision
block uses: `+0x104` position, `+0x128` path, `+0x130` velocity). Slot `+0x2b4` = VA `0x9e8574` = **`0x4c8a90`, whose whole body
is `mov 0x8(%ecx),%eax; ret`** - `*(int*)(this+8)`, the id the same function prints as `objId=%d`. A sibling class implements
the same slot as `mov $0x7fffffff,%eax; ret`, i.e. "never has priority", which confirms the slot means priority rather than
identity-by-accident.

**So the rule is: lower unit id has right of way; the higher-id unit is told to pause.** Not a formation slot, not an order
sequence number, not a timestamp (that is the neighbouring slot `+0x2ac` = `0x4b0860` = `mov 0x134(%ecx)`, the value the log
calls `lastMoveTime`), and not a dat field.

**What "told to pause" does.** Slot `+0x2b0` = `0x8aa6d0`, whole body:

```c
if (*(int *)(in_ECX + 0x128) != 0) { *(undefined4 *)(*(int *)(in_ECX + 0x128) + 0x28) = param_1; }   // path->pause = arg
```

The argument is a duration, drawn at the call site as `rand() * 1.25 / 32767 + 0.25` - **0.25 to 1.50 seconds**, in real
seconds, not ticks. The paused unit **keeps its path**, advances no waypoint, does not move, and resumes where it left off
when the timer runs out (`"Pause time is %.10f, taking off %.10f"`, then `"Pause time is still > 0 (isMoving = %d), returning
MoveResultNone"`). The *pauser* is the one that repaths (`"We have %d non-past collisions and %d terrain collisions,
repathing"`). Two facts about the list matter: the pause loop runs **after** the repath decision and does **not** re-apply the
past filter, so a "past" collider can still be paused; and the only early-out is a list of exactly one entry that is the
order's target.

**The original says its own pause is not sync safe.** The `rand()` wrapper (`0x5404f0`) logs *"Usage of rand() has been
disabled because not considered sync safe. Permission denied triggered by %s(%d)"* - so a faithful reimplementation draws the
same uniform 0.25-1.5 s from **our** seeded stream, never from a global.

**Consequence for us.** Our movement pass already processes bodies in id order, so the *winner* of a head-on is the same man
the original picks - by accident, not by rule. What we do not have is the loser's **bounded pause with a resume**: we give
way, slide or nudge, and the pair can stand for ever (measured 2026-11-13: seven villagers on two tiles, `WaitOwnTile` for
the whole match, the acceptance bank at 0 food). A 0.25-1.5 s pause that keeps the path would have been *safer* than what we
had, because it is bounded by a countdown rather than by the other man moving.

### Follow-up (2026-11-14): what the id *is* is still open, and the tie-break is not arrival order

The owner asked the obvious next question - *"isn't it just identifier which unit came to tile first, first served first?"* -
and the answer is **no**, with one correction to the section above.

**Corrected: the constructors do not set the id.** An exhaustive scan of the move-object constructor (`0x8a1350-0x8a1560`) and
both base constructors (`0x88b070-0x88b130`, `0x8b12b0-0x8b13e0`) finds **zero** stores to `[reg+0x8]`; they set the vtable and
zero the collision fields. The getter (`0x4c8a90`) and setter (`0x4c8aa0`, COMDAT-folded beside it) exist separately, and the
setter's callers are **serializer and editor paths** (`0x4ca116`, `0x4ca6e5`, `0x4ccc6d`, `0x4cd1fb`, `0x4eabff`, `0x681e5a`) -
one of them reads the value out of a struct field and hands it to `setId`. So an id **can be pushed in from saved or editor
data**, and "creation order" is an inference, not a fact.

What is established about it:
- **`World::addObject` (`0x72aba0`, vtable `+0x40`) is the only consumer**: it reads `obj->[0x8]`, stores the object at
  `table[id]` (`world+0xe8`, count `+0xec`, published as `ds:0xde1160`), refuses negative ids in the main path, and **grows the
  table to fit an arbitrary id** (`newcap = id + id/10`) rather than assuming sequence.
- There are **two id spaces** - `0..9999` and a negative one `-1..-999` - and an explicit **release-by-id** routine
  (`0x735860`) that writes `table[id]=0` in either. A two-space handle table with release is not a sequence counter.
- **The function that allocates an id for a newly trained unit was not found**: it sits behind virtual calls
  (`Player::addUnit 0x74cf80` -> `world->addObject` through `[vtbl+0x40]`), and no store-to-`+8` attributable to unit creation
  survives in the corpus.

**The tie-break is re-verified byte for byte** and reads no time, no distance and no occupancy:

```
008a80ed call [edx+0x2b4]     ; self   -> esi
008a80f9 call [edx+0x2b4]     ; other  -> eax
008a80ff cmp  eax,esi / jle   ; pause only when other > self
008a8107 mov  esi,[ebx] / 008a8141 call [esi+0x2b0]   ; other->setPause(rand*1.25/32767+0.25)
008a815e lea  eax,[edi+0x8] … "TOLD TO PAUSE BY UNIT %d"    ; prints SELF's id
```

**The only time rule in the block is the past filter, and it runs one way** (`move_obj.cpp:2063`):

```c
uVar4 = *(uint *)(iVar5 + 0x10);   // our path's creation time
uVar14 = (**(code **)(*piVar13 + 0x2ac))();   // the other unit's lastMoveTime
if (uVar4 < uVar14) -> "That's a non past collision" (counted) else "That's a past collision" (ignored)
```

`+0x134` (`lastMoveTime`) is **motion recency, not tile occupancy**: it is refreshed to the world clock whenever the object's
position has changed by more than `1e-7`, on placement, and on a state change. It is per-object, never per-tile, and the `0x2b4`
comparison never reads it.

**No distance or claim rule exists**: the only geometry in the block is the radius-overlap query at the new position
(`0x843410(self_radius * 1.0)` plus a box clamp); every other distance is self->target/goal/waypoint. The nearest thing to a
claim is route-local - `"That's a path ignore unit"` / `"…highest linked path ignore unit"`, membership by id in the path's
ignore list.

**So the honest verdict is narrower than the section above**: the winner is whoever holds the **lower object handle**, a number
nothing on the movement path ever writes or compares against time. If ids are creation-assigned it is oldest-object-wins; if
they are recycled slots or imported from a save it is arbitrary. *First come, first served* is **not** a rule anywhere in the
binary, and "who stood here longer" cannot be asked of it at all.
