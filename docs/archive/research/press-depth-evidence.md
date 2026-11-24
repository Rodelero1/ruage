# How deep may two friends press? The binary answers, and the answer is "no limit"

Asked by the owner, 2026-11-19: *"Does the original game or decomp give any info?"* - about the press depth the P5 slice needs
(our `group_overlap` fixture measures a stopped friend walked into **4576 raw** inside an **8000 raw** footprint, i.e. 0.23 of a
tile). This note is the reading; the anchors are the corpus at `/opt/data/tools/rea/pseudo-aokhd`.

## The resolver: it exists, and it is a proper depenetration

`FUN_008aa990` (`move_obj.cpp`) takes the unit's own box and a **candidate position**, walks every colliding object, and for each:

- tests the two boxes on both axes (`fVar10 > fVar12` etc., four `<=`-style comparisons);
- computes the overlap on each axis and **takes the smaller** (`local_30` vs `fVar11`), which is the separating-axis answer;
- moves the candidate out by that overlap **scaled to the movement vector** (`local_30 / fVar1`), and if the fraction is `>= 1.0`
  it **snaps the candidate to the fully separated position** (`local_14 = *param_1`);
- loops over every collider and, when it resolved anything (`0 < iVar5`), writes the resolved point back into the candidate
  (`*param_2 = local_14`).

So the original *computes* exactly the rule the question is about - separate the boxes along the axis of least penetration, all the
way out.

## And then it throws the answer away

`FUN_008aa990`'s only call site is `FUN_008a4360` at `move_obj.cpp` **0x127d-0x12b2**, and its `param_2` there is **`local_64`** -
the candidate new position computed by the virtual at `+0x2d4`. In that whole function `local_64` appears **three times**: it is
declared, it is filled at 0x11df's neighbourhood, it is logged (`"  newPos=(%.10f, %.10f, %.10f)"`), and it is handed to the
resolver. **Nothing ever reads it back into the unit's position.** The unit's own coordinates (`in_ECX[0xf]`, `in_ECX[0x10]`) are
never assigned from it anywhere in the function.

That is the same finding D210 recorded from the other end (*"its only radius-against-radius retraction has its result discarded at
the movement site, `0x8a7fe7`"*), now with the mechanism named: the resolver is called, the answer is computed, and the movement
path ignores it.

## What actually stops a unit, then

Two things, and neither is a distance:

1. **the pause** - the higher-id unit is told to pause, `setPause(0.25-1.50 s)` (`0x8a8141`), and `getMaxVelFwd` returns exactly
   `0.0` while it holds (`008a2a30`);
2. **the re-path** - *"We have %d non-past collisions and %d terrain collisions, repathing"* (`move_obj.cpp` 0x12b2), with the
   colliders' ids added to the ignore list at `0x1265` (`local_e4 + 1`, `&DAT_00de1dc8 + local_114 * 4`).

## So the answer to the question

**The original has no press floor at all.** Its bodies may overlap to any depth; what it does about a collision is pause one of the
pair and re-path the other. Our **4576 raw** of press is *less* than the game permits, not more - and the mechanism that produces it
is the same one that produces the game's: the two bodies step into the same gap in one tick, each landing legal against the other's
*previous* position, and nothing corrects the result afterwards.

**Consequence for P5:** no floor should be added. The fixture that calls this "a stopped body walked over" is describing the game's
own behaviour, and the honest re-point is *"friends may press; the crowd is separated only by the pause and the re-path"*. The
quarter-footprint idea in `docs/archive/research/parity-plan.md` is withdrawn - it would have been a floor the original does not have.

**Not established, and said so:** whether the discarded resolver is dead code in a *retail* build or was live in an earlier one (the
corpus is a single HD build), and how the pause's 0.25-1.50 s roll is drawn - both are separate questions, and neither changes the
answer above.
