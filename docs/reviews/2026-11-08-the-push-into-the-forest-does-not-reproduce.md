# The push into the forest does not reproduce here (D184)

*2026-11-08. The owner's reproducer: **"put scout as near to forest as possible, place lumber camp where the scout is, he should immediately jump to the
forest."** Written as a test before touching anything - and it **passes on the shipped engine**, twice, in two different shapes. That is the result, and it
is more useful than a fix would have been.*

## What the test does

`crates/sim/tests/eviction.rs`: a scout stands on a tile, a building is placed across it, and the test asserts three things about where he ends up -
he is not on a tile holding a resource or a building, he is not inside the new building's own footprint, and `body_clearance` at his new position is
not negative. Two fixtures: the scout on the building's anchor tile, and the scout on the **far** tile of a 2x2 building, which is where the eviction
has to look around the *unit's* tile rather than the building's.

Both **pass**. `work_position` already refuses ground that is not free and already asks `plan_path` whether the tile can be reached, and our forests are
**terrain** - `is_free_ground` answers *no* for every tree tile - so the eviction cannot choose a tree even by accident.

## Why the owner saw it, and we cannot

In the original, and in the owner's game, **a tree is an object standing on grass**: the ground under a trunk is walkable terrain, and what stops a body
is the trunk. Our engine currently has the opposite model - forests are blocked *terrain*, and resource tiles are impassable squares (the owner's own
rule, 2026-11-08). So the exact accident he photographed needs the model we have not built yet (scattered trees as round obstacles), or a resource whose
node stands on walkable ground.

**That is a fact about the models, not about the code being right.** It is recorded here because "we could not reproduce it" is a result, and because the
next person to see a unit inside a tree needs to know which of the two models they are looking at.

## What is kept

The test stays as a **guard**, not as a regression: the invariant it states - *a placed building never evicts a body onto a resource, a building, or
inside its own footprint* - is true today and must stay true when D183 replaces the teleport with a walk, and when scattered trees arrive with walkable
ground under them.

## Performance

**Measured**: the new test runs in **0.76 s**; the whole gate in **407,986 ms** (`gate109.log`) with no change to the tick loop - nothing in the
simulation was touched.

## Stability

- **No code changed**: the test is the whole diff, which is why the acceptance hash is untouched.
- The two fixtures cover both the anchor tile and the far tile of a multi-tile footprint, because those are different ring centres in `work_position`.
- Suite: **789 passed / 0 failed / 21 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
