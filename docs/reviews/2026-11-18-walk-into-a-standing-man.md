# A walker still reaches into a man standing in his way: what was measured, and what is left (D269)

*The owner: **"Villagers still try to push through standing villager instead of trying to repath around him."** Reproduced,
traced, two rule defects found and measured, neither landed: the fixture I built fails with **and** without them, so the real hole
is somewhere else and I am not landing a change I cannot pin.*

## The reproduction

A wall down the middle of an 18x14 map with **one** gap at (9,6); a villager walks from (5,6) to (13,6) - his route is planned
while the gap is empty - and then a second villager is put **in the gap** and left standing for 20 ticks. Per tick:

    tick=118 apart=9500 walker=(180500,130000) stander=(190000,130000)
    tick=121 apart=8000
    tick=122 apart=7500   <-- inside the 8000 the two footprints need

The walker closes **500 raw a tick** and passes **500 raw inside** the stander. Nothing stops him, nothing re-paths, and the
stander never moves.

## Two defects found, both real, neither sufficient

1. **The past exemption was a licence to walk through anyone who had been standing still.** `is_past` is D210's reading of the
   binary - `path->createdAt >= other->lastMoveTime` - and on its own that makes **every man standing for a while** passable for
   ever, because his last move is older than every route planned since. The binary prints `isMoving` beside `lastMoveTime` for the
   collider it is classifying, and the exemption can only sensibly be about a man **walking away** from you. Requiring the
   collider to have moved within the last couple of ticks: **overlapping pair-ticks 104,386 -> 68,718** (-34%), **deeper than half
   a footprint 51,676 -> 10,191** (-80%), and the `past`-explained overlaps collapse from **43,315 to 111** (36,000-tick
   acceptance `collision_probe`).
2. **The slide checks ground and not bodies.** `StepDecision::Slide` asks `step_is_walkable`, which knows about walls only, so a
   man pressed against a wall beside a man in the only gap slides into him. Adding the body test to the slide's landing is three
   lines.

**Neither fixes the reproduction**: the walker still reaches 7,500 with both in place. So the step into the stander is being
allowed **before** either rule is consulted - `blocker_at` is not returning the man in the gap at all, or the step is taken from a
path that does not go through it. That is where the next trace goes: print inside `collide` every step whose landing is within a
body's reach and which was nevertheless returned as `Take`.

## Why nothing is in the tree

The owner's rule is strict test-first and no ignored fixtures: a change whose own fixture still fails is a half-fix, and the
measured improvements above cannot be pinned without the fixture that fails for the *real* reason. They are recorded here and in
the fixture kept at `/opt/data/cache/scratch/walk_round-wip.rs`, ready to land together with the actual fix.

## The numbers to keep

- Reproduction: **500 raw** of overlap, **500 raw/tick** approach, stander's own position unchanged.
- `is_past` narrowed: overlaps **104,386 -> 68,718**, deep **51,676 -> 10,191**, `past`-explained **43,315 -> 111**.
- Baseline (unchanged, current `HEAD`): **104,386** overlapping pair-ticks of 15,245,283 (0.68%), mates **0**, past **43,315**,
  neither **61,071**.
