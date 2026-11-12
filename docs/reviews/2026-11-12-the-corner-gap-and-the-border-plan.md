# The corner gap, and the plan for the border

A guard test, one decision recorded, and the owner's ruling written down as the spec for the next slice. No engine behaviour
changed in this commit.

## Performance

None to report as a change: `crates/sim/tests/corner_gap.rs` adds one test binary that runs in **0.82 s**, `World::step` is
untouched, and the acceptance run is not re-taken because no production line moved. Suite: `cargo test -p ruage-sim` **119
binaries, `EXIT 0`** with the test in place.

## Stability

**What the rule already did, and what it does not.** The owner: *"Block of forest is never walkable through. Ever. Even tiles with
tree on it touching by the corner are blocking path."* The pathfinder has refused a diagonal grid step between two blocked tiles
since D113, and the movement pass asks the same question for any step that crosses a tile. A stricter version - firing on every
change of tile in both axes - was written and **reverted**: the new guard test passes identically with and without it, so it fixed
nothing reproducible, and inert code is noise. That is recorded here rather than shipped as a fix.

**What is not reproduced.** A villager standing **on** a resource tile (his screenshot). Nothing in this slice touches it; the
guard pins the corner rule so a future change cannot quietly break it, and `docs/next-step.md` says what would settle the overlap:
a save, or the seed it happened on.

**What the next slice is**, in his ruling: standoff measured from the **tile border** (`0.5 + r`, rounded corner), places anywhere
along the outline one diameter apart, as many as fit, **per tile** - the notch between two adjacent trees is walkable ground, as
24 of his own 49 men demonstrate - no outer band, and a resource tile that can never be entered or clipped by a circle, vanishing
the moment it is exhausted.
