# The owner's own arrangement — a fidelity test for the gather ring

*No engine change. This is the check the project's method asks for and the ring did not have: the rule compared against the
numbers it was read out of, starting from **his** arrangement rather than a fixture written to suit it.*

## What it does

`crates/sim/tests/gather_ring.rs::the_owners_own_arrangement_lands_on_the_same_ring` puts six woodcutters down at the exact
positions his save records - 1.105, 1.142, 1.160, 1.170, 1.170 and 1.189 tiles from the source square's centre, shifted by
`(-47, -80)` so they sit round this fixture's tree at (10, 10) instead of his at (57, 90); a translation changes no distance and
no angle - orders them all to the tree, and then asks the engine two things:

- every man works it, and the **closest he gets while working** is within a twentieth of a tile of the 1.156 ring;
- no two of them stand inside each other - 0.39 tiles apart, against the two bodies' 0.40.

## What it caught, and it caught me twice

Both errors were in the **test**, not the engine, and both are the same mistake in different clothes: measuring something that
is not the quantity the rule is about.

1. **The reference point.** The first version measured from `world.position(node)` - the tree's *anchor*, which this fixture puts
   on a tile corner - and read 1.217 and 1.239 for men who were actually on the ring. Every distance in D186 is from the centre
   of the square a source occupies, so the test now adds the half tile: 1.146, 1.156, 1.162 for the same men.
2. **The moment.** The second version measured at the end of the run, when a gatherer is cycling home with a load: 1.923, and
   0.288 apart from the man in front of him in the queue to the town centre. A gatherer is only *at* his place part of the time,
   so the test now records each man's closest working approach and the position where it happened.

An earlier attempt also seeded his coordinates into a 22x22 fixture and was refused with `NoPath from (57, 89) to (10, 10)` -
his map is 144x144 and the fixture's is not, which is why the translation is part of the test's doc comment rather than a
footnote.

## Performance

**No engine code changed**, so the numbers are the ones the tree already had, re-read after this test:

- acceptance match, seed 7, 36,000 ticks: **150 entities, 352 repaths, made_room 226, hash `08a111f13b7c43ec`** - identical to
  the run that landed D186 addendum 2;
- `step_cost`, 128 entities: `World::step` **6.8 us**, `Snapshot::of` **91.8 us**;
- the suite gains one test in a binary that already took 2.2 s; the full workspace run is unchanged in shape.

## Stability

- **Test-only**, so nothing in the simulation can regress: the file is compiled into the same `ruage-sim` test binary as the
  other three ring tests and shares its fixture.
- **Deterministic**: fixed seed 7, integer positions built from `f64` constants at compile time, no wall clock, no randomness.
- **The assertions are the rule's own numbers**, not tolerances invented to pass: 1.156 is the mean of his six, 0.40 is two
  villager bodies, and the window is the ring plus the fiftieth of a tile the settle works to.
- **It guards the whole slice**: if a later change moves the ring, breaks slot ownership or stops a man working from his place,
  this test fails on his arrangement even when the synthetic fixtures still pass.
