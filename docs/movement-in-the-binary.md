# The movement module, read out of the binary without decompiling

Written 2026-11-03. Answers the owner's question ("can you read path finding and evasion from the game binaries?")
with the map first and the reading second. **The map is done and it was cheap** - the original asserts through calls
that carry its own source path, line number and a log string, so the exported pseudocode can be indexed by source file
without reading a single expression. `docs/re-toolchain.md` records the toolchain (REA on this box, D86); this file
records what the movement module *is*.

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
grep -ro --include=*.c 'move_obj\.cpp' . | wc -l   # 174 references
grep -ro --include=*.c 'motion\.cpp'   . | wc -l   #  91
```

`/opt/data/tools/rea/source-path-index.txt` already has the same mapping in text form for 94 source files; the counts
here are measured from the corpus today, and they disagree with the earlier "159 / 81 sites" note because that pass
counted only one of the two call shapes (the assert and the three debug-log variants all name the file).

**Measured: 265 references across 18 functions** - 91 in `motion.cpp`'s three, 174 in `move_obj.cpp`'s fifteen. The
whole corpus carries **1,029** `.cpp` path references, so movement is the densest module in the binary by a wide margin.

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
