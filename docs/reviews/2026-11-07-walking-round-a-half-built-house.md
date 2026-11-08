# A villager walks round a half-built house (D152's first coverage gap)

*2026-11-07. The owner asked: "Is there a way to test that units don't walk over foundation of the building once construction
began (except farms)?" The answer was yes — tile-level, already — and one gap: the **live walk** case used a finished house.
This closes it.*

## What was already covered

- `building_passage.rs:230` `a_foundation_blocks_while_it_is_still_being_built` — a real `Command::Build`, the foundation
  exists and is asserted **still under construction**, and every footprint tile is not free ground.
- `building_passage.rs:286` `a_farm_is_a_floor_while_it_is_built_and_after` — the farm exception, during and after.
- `building_passage.rs:330` `the_town_centre_keeps_its_solid_core_and_its_yard` — the one **partly** walkable footprint.
- `foundations.rs:184 / 218 / 372` — the fog dimension: your own foundation is in the way, one you can see blocks you, one
  you cannot see does not.
- `building_passage.rs:188` — a villager walks round a **finished** house, followed every tick.

## What was missing, and is now there

`a_villager_walks_round_a_half_built_house`: it starts a real house site, waits for the foundation, **asserts it is still a
foundation**, orders a second villager straight through it, and then checks every tick that the walker stands on no tile of
the footprint — and that it still arrives. It also asserts that the site was still a foundation for at least one tick of the
walk, so the test cannot quietly turn into a test of a finished building.

## Proving the test can fail

**It passes on the engine as it stands**, so it closes a coverage gap rather than fixing a bug — and that claim was
**mutated**, not asserted: adding `&& entity.construction.is_none()` to the blocking condition in `seed.rs`'s `insert`
(which claims a building's tiles "from the moment it stands there") makes **both** this test and
`a_foundation_blocks_while_it_is_still_being_built` fail, while the finished-house and farm tests stay green. The mutation
was reverted.

## Performance

**No production code changed**, so no measurement applies and none is claimed. The test's own cost is bounded by design:
1,600 ticks of stepping in a 64x64 seeded world, and the whole `building_passage` binary runs in **4.4 s** for ten tests
(measured in this run: `finished in 4.37s` before the mutation, `4.44s` with it).

What the behaviour itself costs the engine is already recorded where it happens: blocking a building's tiles is done **once**,
at insertion (`seed.rs`, `solid_tiles`), not per tick and not per settled piece — so a foundation is as cheap as a finished
building to walk around.

## Stability

- The mechanism this guards is one line in `insert`: `if entity.kind == EntityKind::Building { … walk.block(tile) }`, whose
  comment already said *"a half-built wall is not walk-through either"*. The mutation showed that line is load-bearing for
  both the tile test and the walk test.
- **The farm exception is untouched** and still green: farms are the one building a villager crosses, during construction and
  after — and the same suite proves a live farm cannot be built over while an expired one can.
- Suite: **739 passed / 0 failed / 19 ignored**, clippy `-D warnings` 0, fmt 0.
