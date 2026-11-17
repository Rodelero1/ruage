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

## The path ignore list, read (2026-11-16): the game never pushes, and a moving group does walk over itself

Asked because the owner ruled on 2026-11-16 that villagers **must never push each other - a villager reroutes, he is never
pushed away** - and because the engine's own answer has been three inventions (make-room, give-way, nudge) that were built,
measured, withdrawn and replaced on suspicion rather than on a reading. The two strings the section above calls "the nearest
thing to a claim" are the answer, and they now have their code.

**1. The list is built in the search's entry point** (`FUN_008007f0`, pathsys.cpp; the array is `local_404`, cap 255) and holds
three kinds of id: **self** (`param_5[2]`), the **order's target** (`local_45c + 8`), and then, in one of two ways:

- with no group on the route (`param_5[0x21] == -1`): every entry of the route's **own array** (`param_5[0x1a]`, count
  `param_5[0x1b]`) whose object answers **1** to slot `+0x150` **or** to `+0x2a0`. Slot `+0x2a0` is **`isMoving`**
  (`0x8aa290`: true when the velocity vector at `+0x130` has non-zero length) - the same slot the collision log prints as
  `isMoving=%d` (move_obj.cpp:0x123d), so the arithmetic and the identity check each other.
- with a group (`param_5[0x21] != -1`): every id in the group object `FUN_0072cf10(id)` (`+0xd0` count, `+0xd8` array), no
  per-member test at all.

**2. It is a passability input.** `FUN_00801a20(x, y, ids, count)` returns **1** when *every* body on that tile is in the list:
it walks the tile's object array (`+0x5a8c` rows, `0x11 + x*0x24` count, `0x18 + x*0x24` array), skips any object with zero
extent (`0x3c`), scans the id list for the object's id at `+8`, and returns 1 the moment one is found - otherwise 0, blocked.
So **a tile held only by path-ignore units is free ground to the plan**, which is how a group's route is planned *through* its
own members.

**3. It is a collision input too, and the collision answer is a skip.** In `FUN_008a4360`'s block, `FUN_007fe010(id)`
(`0x7fe010`, membership in `[this+0x24]` of `[this+0x20]`) returning 1 logs **"      That's a path ignore unit"**
(`.rdata:0x9ea438`, move_obj.cpp:**0x1242**) and the loop moves to the next entry: no pause, no repath, no non-past count, no
bound. The second form, **"      That's a highest linked path ignore unit"** (`.rdata:0x9ea458`, line **0x124a**), is the same
skip behind `FUN_007fdff0`.

**4. The epilogue never writes a position.** Target collision (`DAT_00de1998 == 1` and the entry is the order's target) →
"That collision is with our target, so we're done". Otherwise, if any non-past collision → "We have %d non-past collisions and
%d terrain collisions, repathing", the parent path is installed, and the **higher-id** unit of each surviving pair is told
`setPause(0.25-1.5 s)` (0x8a8107-0x8a8141). The one remaining arm is "    Don't know what to do about my collision(s)". There
is no branch anywhere in the block that moves the other body.

**What it settles, and what it does not.**

- **Pushing does not exist in the original, for anybody.** Our make-room / give-way / nudge family was an invention, and the
  owner's ruling of 2026-11-16 is not a divergence from the game - it *is* the game, which we had left.
- **Walking over a moving group-mate is real and is the game's own rule** - the game's words for such a body are "path ignore
  unit". D207 read it correctly, and the `isMoving` gate is the same fact the Forgotten Empires answer states from the other
  side ("units give up their formation and therefore block each other when: stop / click enemy / attack-move").
- **The exemption is route-local**: it is the *mover's* list. The same two units can overlap under one order and block under
  the next, so "may never overlap" cannot be read as a global rule of the game.
- **Unread, and the one thing a decision here needs**: what fills route `+0x1a`/`+0x1b` at order time (candidates: the
  selection's other members, or a formation object), and what the byte `+0xF4` behind slot `+0x150` means (`0x8ae170` returns
  `[+0xF4]` unless slot `+0x154` answers 6 or 7; `0x428c50` is not in the corpus). That decides whether the original would
  exempt a **box-selected crowd of villagers** ordered by a player - which is exactly what our window sends - or only a
  military formation, which is all the FE quote ever claimed.

## The cost model settled (2026-11-16): the heuristic is deliberately inadmissible

The open item in "What is still unread" was a contradiction: *"x25 against a 16-per-step cost is not admissible, so either those
four arguments are not tiles or 16 is not the per-step cost."* **Neither was wrong - the assumption was.** Three functions, read
in the corpus:

- **`FUN_007ffc80`** is the **octile cost**: `table[min] + (max - min) * 0x10` - 16 per cardinal step, with the diagonal share
  looked up in `DAT_00afd2a0` (runtime storage, computed at load).
- **`FUN_007ffcd0`** is the plain **Euclidean distance**: `sqrt(dx*dx + dy*dy)` on the same arguments, returned as a float,
  truncated by the caller. Its arguments are tile coordinates - the same pair that indexes the best-cost array at
  `(x*0x1e0 + y)*4`, with `0x1e0` = 480, the maximum map dimension.
- **`FUN_008007f0`** (the search) computes `h = (int)euclid * 2500 / 100`, and when `FUN_0074e020() == 1` it computes
  `h = (int)euclid * 0x19` instead. **2500/100 *is* 0x19**: both are **25**, in hundredths and in whole units.

So 16 is the step cost and 25 is the **heuristic weight**, and `25 > 16..22.6` - the octile cost of one tile of Euclidean
distance - means **h overestimates the remaining cost**. The original's search is therefore **weighted A\* / best-first with a
weight of about 1.56**, trading path optimality for speed. That is a decision in the original, not a contradiction in its
constants, and it explains why the pair looked impossible to port: it is impossible to port *as an admissible A\** because it
never was one.

**What this changes for us.** Row 13 of the register is not "10/14 against 16 plus a table" - it is "an admissible octile A\*
against a weighted one that returns slightly longer routes on purpose". Adopting it changes route *shape* everywhere, which is a
behaviour change worth its own measurement rather than a constant swap, and it is no longer blocked on an arithmetic puzzle.

**The frontier is a heap too, and the note that said otherwise is corrected.** The loop keeps a **frontier array** of
`(x, y, cost)` triples at `0x119414 + n*8` with its length at `0x2db41c`, and calls **`FUN_00801e90(index)`** on every push -
a sift, i.e. a binary heap - **beside** the per-tile best-cost array at `0x38414` and its generation byte at `0x2db424` (advance
by 8, wrap at 240, clear only on wrap). So the earlier summary - "a stamped best-cost array rather than a heap" - named one of the
two structures and denied the other: both exist, because they answer different questions (best cost per tile vs order of
expansion).

**Still unread, and named, and it is now sharper rather than settled:** `local_46c` counts **expansions** (zeroed before the
loop, `+1` after each node's eight neighbours are tried), and the break is `h < local_46c || 20000 < local_46c` - a **distance
compared with a count**. Either the original carries a unit mismatch in that guard or one of the two variables is not what it
looks like in the lifted listing; **it is not ported on either reading**, and the termination rule plus the equal-`f` tie-break
remain what a replay would depend on.

## The maximum forward velocity, read (2026-11-17): the speed is the unit's own, and a pause is zero

Asked whether a villager's speed can ever change mid-walk, with the game as the first rung. `getMaxVelFwd` is the only place a
mover's speed is decided, and it is small enough to read whole.

- **`008a2a30`** (move_obj.cpp 3,069-3,145) is the function; its own log strings are at `0x9e8b2c` (*"ret 0.0f due to pause
  reason"*), `0x9e8b50` (*"ret %s getMaxVelFwd"*) and `0x9e8b8c` (*"ret %s vel < 10.f"*), and its callee `0x7dd9d0` answers the
  pause reason (`2` -> 0.0f, `-1` -> the speed, everything else -> 0.0f).
- **The value returned is the unit's own dat speed**: `mov 0xc(%ebx)` is the unit definition, `movss 0xe4(%eax)` its speed.
  There is **one** multiplier: when `[move+0x14]` (a motion object) is non-null and its float at **`+0x6c`** is `> 0.0`
  (`comisd 0x9ae7a0`, a 0.0 double), the result is `speed * factor`. Otherwise the bare speed.
- **A pause returns exactly 0.0f** - a stop, not a slow.
- **There is no terrain factor, no per-tick ramp, and no partial-speed arm in it.** `007ff920` integrates velocity into position
  and carries an `acceleration` term, but nothing in the velocity path writes a fraction of the unit's speed.
- **`+0x6c` is written from one place**: the thiscall setter at `0x7f4130` (`movss %xmm0,0x6c(%ecx)`), which the .text scan finds
  as the **only** store to that offset. It has **no direct caller** - it is reached through a virtual slot, so **who writes the
  factor, and whether it is ever below 1.0, is unread**. The mechanism that needs one is the formation
  (`FormationParams(..., breakAutoFormationsBySpeed=%d)` at `0x807ad8`, `AutoFormation` at `0x80754e`), where villagers are never
  a member (`docs/prior-art.md`).

**What that supports, and what it does not.** Supported: a body moves at its own speed or at zero, and the pause is the zero -
which is the owner's rule (*"they may stop or stop to repath ... so either they walk, or stand"*). Not settled: whether a
formation factor could ever be applied to a villager (its writer is unread), and whether the `acceleration` integration in
`007ff920` ramps anything in practice (D206 keeps that as a register row; a recording settles it, a reading does not).

The engine's own side of the same question is measured in `docs/next-step.md`'s third question: 15 of 20,000 villager body-ticks
move between 0.11 and 0.95 of a step, and each one is a waypoint's landing, not a collision.

## How a blocked step is answered, and why there is no slide (2026-11-17)

Read from `doIntelligentMovementUpdate` (`008a4360`, `move_obj.cpp`) in the REA corpus at `/opt/data/tools/rea/pseudo-aokhd`,
whose asserts name their own source lines. The movement block is `move_obj.cpp` **4,645-4,920**, and its own log strings give the
order of events without needing an expression read:

| source line | string | what it means |
|---|---|---|
| 0x1305 (4,869) | `DOING MOVEMENT, velocity.z=%.10f` | the step is a **velocity**, applied whole |
| 0x1320 / 0x1325 | `Doing yaw of %.10f, limitedYawAngle=%d` / `NOT Doing yaw ... because we'll hit the point the next time` | heading, with a guard against overshooting the point |
| 0x12b6 (4,790) | `No collisions at new position` | the new position is **tested**, not adjusted |
| 0x1225 (4,645) | `We have %d ObSystem collisions at our new position, pathCreationTime=%d:` | how many, and the path's age |
| 0x122f | `Collision with terrain` | terrain is one class of collision |
| - | `That's a non-target collision` / `target collision` / `non past collision` | the other three |
| 0x128b (4,747) | `We have %d non-past collisions and %d terrain collisions, **repathing**` | the answer to a collision is a **new path** |
| 0x129b (4,763) | `**TOLD TO PAUSE BY UNIT %d**` | or: the lower-priority unit is told to wait |
| 0x12b2 (4,786) | `Don't know what to do about my collision(s)` | the fallback, with a log, because it should not happen |
| 0xdc0 (3,520) | `Teleporting to exact waypoint` + 0xdc9 `Incrementing waypoint for path ID=%d` | a reached waypoint is **snapped to and spent**, in the same update |

Three conclusions, and they are structural rather than stylistic:

1. **There is no slide.** The position update is "new position from velocity"; every collision answer is a **repath**, a
   **pause** ("TOLD TO PAUSE BY UNIT"), or a stop. Nowhere in the block is a step decomposed into axes and one of them dropped -
   which is what `StepDecision::Slide` does in our mover.
2. **The pause is a priority comparison**, not geometry: `(**(*unit + 0x2b4))()` is taken for both bodies and the smaller one
   is told to pause (0x129b). The slot's target was not read, so *which* value it is (speed, mass, order priority) is **unread**;
   what is certain is that one of the pair yields for the tick, and the other walks on.
3. **A corner costs nothing.** `Teleporting to exact waypoint` puts the body exactly on the waypoint and `Incrementing waypoint`
   spends it in the same update - i.e. the original does what `World::fold_the_step` now does, and never stands a man on a corner
   to save the rest of his tick.

So the slide is **ours**: D188 added it because a villager that could not slide round a wall wedged for 68 ticks, and the game's
own answer to that situation - a repath, with the loser paused - is a pair our mover does not have in full. The decompilation has
nothing to add: `src/unit/unit_action_collision.cpp` is 61 lines and walks a coordinate list for group children, with no
movement geometry in it (`docs/prior-art.md`).

## What may be stood on: a point test for ground, a box for things (2026-11-17)

Asked by the owner - *"how come that villagers couldn't stand close enough to work tree properly if they approach the tile with
tree to 1/2 diameter?"* - and the answer from the binary is that **they can, and our engine was the odd one**. Two functions:

**The ground is tested at a point, with no radius.** `FUN_008aa480(x, y)` (`move_obj.cpp` 0x51d) bounds-checks the tile, reads the
unit's terrain passability mask (`FUN_008427c0`), and asks whether the tile's bits, with that mask removed, are zero - i.e. *is this
tile passable for this unit*. Only then does it run the object query. There is no growth of the tile by the body's radius anywhere
in it, and the arguments are cast to `int` before use: they are tile indices.

**Things are boxes with a radius pair, and everything inside the box is distance zero.** The decompilation carries the game's own
distance function, `compute_distance_sq` in `src/unit/unit_action_build_77a0.cpp`:

```c
float radius_x = *(float *)((char *)entity_proto + 0x20);
float radius_y = *(float *)((char *)entity_proto + 0x24);
float delta_x = (dx <= radius_x) ? 0.0f : (dx - radius_x);
float delta_y = (dy <= radius_y) ? 0.0f : (dy - radius_y);
return (delta_x * delta_x) + (delta_y * delta_y);
```

So distance is measured **to the box**, not to the tile and not to the centre: a point anywhere inside a building's footprint is at
distance 0, and a body's overlap is `distance < radius`. That is where "no body inside a wall" comes from in the original - the
object query, not a grown tile - and it is also why a villager can stand at **half a diameter** from a tree: his centre is on the
next free tile, his box-vs-box distance to the trunk is exactly the sum of the two radii, and zero overlap is the contact point.

**What our engine does instead** (D176): it grows *every blocked tile* by the body's radius at stepping time. That is the
pathfinder's trick - "shrink the unit to a point, expand every other obstruction" - applied to the wrong question. It is stricter
than the game by construction, which is why a gatherer was refused a **164 raw (0.008 tile)** brush and stopped 267 ticks at
`WaitGround`, and why the terrain slide was load-bearing: it was sneaking men past a test the original does not have.

**The model to copy, then, is three lines**: ground = the tile under the point; things = their `(radius_x, radius_y)` box; overlap =
distance-to-box `<` the mover's radius. No threshold, no brush allowance, and nothing to slide.

## Proved: the collision is box against box, and overlap counts as touch (2026-11-17)

The owner: *"Proved and follow the original game and decomp."* Three functions, read end to end, settle what "walkable" means.

**1. The ground is a point test.** `FUN_008aa480` (`move_obj.cpp` 0x51d) bounds-checks the tile, reads the unit's terrain
passability mask, and asks whether the tile's bits, with that mask removed, are zero. Its arguments are cast to `int`; **no radius
appears in it**.

**2. Things are boxes, and the test is AABB against AABB.** `FUN_00840830` (the per-candidate test inside the query) builds the
object's box from its **radius table** - `*(float *)(obj + 0x20 + index*4)` for four direction indices, i.e. the dat's radius
quadruple - grows the *query* box by the mover's radius (`in_ECX+0x268`), and collides when the boxes intersect:

```c
if ((query_x1 <= obj_x2) && (obj_x1 <= query_x2) && (query_y1 <= obj_y2) && (obj_y1 <= query_y2))
    → collision
```

Touching edges collide (`<=`). So **a body may not overlap any object's box**, and every object - tree, mine, building, unit - has
one, taken from its own radii.

**3. Distance is box-to-box and overlap reads as zero.** `func_002690E0` / `func_00269190` / `func_00269220` in
`src/unit/unit_action_distance.cpp`:

```c
float dx = fabsf(other_x - x) - self_radius_x - other_radius_x;
if (dx < 0.0f) dx = 0.0f;      // overlap collapses to zero
```

which is `max(0, |dx| - radius_x)` on each axis, then the hypotenuse. That is the number the movement update compares against
`rangeDistance` - arrival is "the boxes are within range of each other", and **the owner's half-diameter rule is exactly range
zero**.

**What that says about our engine - and it corrects an earlier draft of this very section.** I first wrote that our trees were the
wrong size. **They are not**: `TREEA` in the pack carries `radius = [0.5, 0.5, 2.0]`, so its box *is* its tile, and a tree is
exactly as big as the game says. The tile grid's box for a tree and the dat's box are the same thing.

The **164-raw (0.008-tile)** refusal a gatherer hit was against the **town centre's footprint**, and by this reading the game would
refuse it too - the collision test collides on touching, and its search plans for the unit's radii. So the defect is not the box
and not the step test: it is the **route**, and the game's answer is its own log string, *"the boundaries of where this unit's
radii will go"*. Ours comes from a straight-line planner (`short_range_clear`) that never looked at a building.

The refusal against the **town centre** (269 ticks, `WaitGround`) is, by this reading, **correct**: the game would call that
collision too. What is wrong there is the route, and the game's answer is in its own log string - the search answers for *"the
boundaries of where this unit's radii will go"*.

So the slice that follows is narrow and named: **give resources their box from the pack's `radius`, index them by tile so the test
stays cheap, and let the step test and the search use those boxes instead of whole tiles.** Buildings already have exact boxes
(their footprints), units already have radii, and the ground keeps its point test.

## How the original stops a walk from cutting a corner: it turns first (2026-11-17)

Asked by the owner - *"how does original game and decomp address this?"* - after the finding that our mover walks a **straight line
at the next waypoint**, so a plan can be valid and the walk still clip a wall by a fraction.

**The unit moves along a *heading*, and the heading turns in fixed steps.** From the decompilation,
`src/zone/zone_spatial_entity_adjust_heading_step.cpp`:

```c
int count = ...; if (count < 8) count = 8;
const float two_pi = 6.283185307179586f;
float delta_rad = (two_pi * (float)a1) / (float)count;   // a1 = steps to turn (usually one)
float new_heading = self->heading + delta_rad;
... wrap to [0, 2pi) ...
self->methodC4(new_heading);                              // set the facing
```

So the facing changes by **at most one 45-degree notch per update** (count is floored at eight), and in the movement update the turn
comes **before** the step: `Doing yaw of %.10f, limitedYawAngle=%d` then `(**(code **)(*in_ECX + 0x29c))()` = "do the movement".
It even declines a turn it would overshoot: *"NOT Doing yaw ... because we'll hit the point the next time"*.

**That is the answer to the corner.** A body walks along the direction it faces. To go round a corner it must *turn*, and a turn
takes updates; it cannot move along a line it is not facing. There is no "straight line at the next waypoint" to clip anything,
because the step is always along the current facing.

**Correction, from the owner**: *"Does turn really take time? I remember turns being instant."* He is right, and this section's
first draft over-read the code. `FUN_008a3270` is **`calculateYawAngle()`** - `calculateYawAngle() objId=%d (interval=%.10f,
yawAngle=%.10f, maxYaw=%s, maxYawPerSec=%s)`, with `Did not limit yawAngle`, `Limited yawAngle to maxYaw of %s` and `Reversing
yawAngle` - so what `maxYawPerSec` limits is the **facing**, i.e. the sprite's rotation, over an `interval`; nothing in the movement
update scales the *step* by the yaw, and the yaw's result in that function feeds a flag, not a distance. **A unit moves in the new
direction at once; its picture turns at `maxYawPerSec`.** The 8-way `count` in `adjust_heading_step` is the number of sprite
facings, and `a1` notches are applied in one call.

**And the rest of the pair is already known**: the new position is tested with **boxes** (`FUN_00840830`), a collision is answered
by a **repath** or a **pause**, a reached waypoint is **snapped to and spent** (`Teleporting to exact waypoint`, `Incrementing
waypoint`), and the search plans for *"the boundaries of where this unit's radii will go"*.

**What that means for us.** There is no facing to add - the original's protection against a corner is **not** a turn rate, it is
that the **search keeps the body's radius clear** and the **waypoint is snapped to**, so a short straight step toward the next
point is a step along ground the body fits on. Ours plans for a point and lets the body drift off it, which is exactly how a
**164-raw** clip of a footprint appeared. So the slice is the two pieces already named: plan for the radius, and keep the body on
the route (the fold gives the snap).

