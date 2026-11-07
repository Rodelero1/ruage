# A shot is measured to the wall (D142)

*2026-11-07. Step 3 of four from the prior-art watch. The finding: the original compares attack reach **box to box** — a unit
is its dat size box, a building its footprint — and we compared **centre to centre**, so a 4x4 town centre was hit from its
middle and every building target sat two tiles nearer than it looked. The digest's own caveat ("the missing datum is a
building footprint, which our schema has not got") **expired at D76**.*

## What was wrong

```rust
let in_reach = if range.raw() > 0 {
    squared <= range.raw() * range.raw()      // centre to centre
} else {
    adjacent || squared <= contact * contact  // melee, tile-based
};
```

For a **unit** target the centre is inside a 0.2-radius box, so the two readings differ by 0.4 tiles and nothing visible
happens. For a **building** it is the difference between its anchor tile and its wall: a town centre's footprint is 4x4, so
its wall is **two tiles** from the middle, and a skirmisher could hit it from two tiles outside its own range.

## The change

`World::gap_squared_to(from, radius, target)` — the squared gap between the attacker's box and the target's:

- a **unit's** box is a square of its own `radius` about its centre;
- a **building's** box is the rectangle `footprint_area` covers, which is **anchored at a corner** and therefore not centred
  on its position;
- the attacker's radius shrinks the target's box, which is the same arithmetic as growing the attacker to a point.

Both places that ask the question now ask it the same way: the tick's blow and the route-ending test (E1b). A rule and the
engine disagreeing about reach is the defect class this project keeps finding, and it costs one shared helper to avoid.

## The test, and the RED

`crates/sim/tests/range_boxes.rs`, on a bare 32x32 map with one enemy town centre at (20, 20) and a skirmisher (range 4.0,
radius 0.2) at (26, 22):

- the fixture asserts it stands **6.3 tiles** from the centre — so the test says which distance it means;
- then asserts `in_strike_range`, which was **false**:

```
the range is 4.0 tiles; the skirmisher stands 6.3 from the middle and 1.8 from the wall
test result: FAILED. 1 passed; 1 failed
```

A second test pins what must **not** move: two units 3.2 tiles apart are in a 4.0 range and two 5.0 apart are not. It
passed before and after — the box change must not disturb the case that was already right.

`in_strike_range` is public so the rule has a test that does not need a played-out match; `unit_range` was extracted beside
it. **The RED was a boolean, not a compile error**: the predicate shipped first, answering the old way, and the assertion
failed against it — the route the pitfalls file prescribes for a test that needs an API that does not exist yet.

## Performance

- **The acceptance match is unchanged**: `f328ebc8d4d34b78`, **141** entities, **188** repaths, **187** made-room, **574**
  refusal lines, **10.82 / 10.83 s** for 36,000 ticks over two idle-box runs.
- That is worth stating carefully rather than as a win: the hash did not move because **no decision in this scenario depended
  on the difference**. The ranged units in it shoot at units, whose box is their own radius; the buildings they fight are
  reached by melee, where the tile test already decided. The test proves the difference is real for a building target; the
  acceptance run says this scenario does not probe it.
- **Cost in the tick**: one `footprint_area` (a small `Vec` of tiles, for a building target only) and four integer
  comparisons, in the branch that already had the squared distance. For a unit target it is `body_radius`, a lookup the
  caller already makes for `contact`. No allocation added on the unit path; the building path allocates a `Vec` per
  attacker per tick, which is the one number to watch if a match ever fields hundreds of shooters against buildings.
- The route-ending site (E1b) does one extra `find(target)` per route *decision*, not per tick.

## One existing test had to be re-stated, and that is part of the change

`crates/sim/tests/combat.rs::fletching_adds_a_tile_of_reach` asserted the exact tile the archer ended on: **11** without
Fletching, **10** with. That was a proxy for the behaviour the test is about (Fletching is worth a tile of closing
distance), and the box margin legitimately moves it: an archer can now shoot from **0.4 tiles** further out, so it stops a
fraction of a tile earlier and the integer division floors to 10 in both runs. The test now asserts the **distance moved** -
at least half a tile without Fletching, less than half a tile with - which is the property, not a tile boundary. It failed
on the old assertion and passes on the new one, and it is named here so a later reader does not think the code drifted.

## Stability

- **No dead zone change**: `min_range` still pads by the two radii, and the melee branch is untouched — `adjacent ||
  squared <= contact²` — so a body stopped short by a friend's footprint is still in contact.
- **Overflow**: all arithmetic is `i64` on raw positions; the largest term is a tile coordinate times the scale squared,
  four orders inside `i64` for the map limits the loader already enforces (D128).
- **Degenerate cases**: a target with no unit data has a zero box, which is the point case and matches the old behaviour; a
  building with no footprint entry falls back to its anchor tile via `footprint_area`, so no `unwrap` and no panic.
- **The two tests disagree if either half drifts**: the building test fails if the box is ignored, the unit test fails if the
  box is applied to point-like bodies in a way that moves the threshold.
