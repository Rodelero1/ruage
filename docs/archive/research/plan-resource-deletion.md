# Plan: delete `ResourceKind` (D17)

**DONE, 2026-10-26.** All five steps landed, the enum and its bridge test are gone, and the finish
line below is a green test in `crates/sim/tests/build.rs`. Kept as the record of what was
load-bearing and in what order it was paid down; the commits are `sim: a cost resolves through the
pack…`, `sim: the stockpile's width is the engine's…` and `sim: the resource kinds are content now…`.

What the plan did not anticipate, and the deletion turned up:

1. **A resource's terrain was a match arm on the enum.** `map.rs` had `ResourceKind::Wood if tile !=
   Terrain::Forest`, i.e. "wood grows in forests" typed into the engine. It became `site` in
   `resources.toml`, enforced in both directions by the map loader.
2. **`[gather.resource_task]` was a four-field struct**, one field per variant — the same leak in
   another file. It is now a map keyed by the pack's names, so a pack with different names can say
   which variant works them.
3. **A cost could only be priced with the reference pack's ids.** The finish line needs "a building
   that costs timber", which is impossible while a cost is a dat id. `Unit.costs` is now
   `Vec<Cost>` with `CostResource::{ReferenceId, Name}` — untagged, so the generated 574 KB table is
   byte-identical and only the new form is new.
4. **The stockpile's width had to move first.** `MAX_RESOURCES` (8) with the pack's count checked at
   load retired the per-cost range check; the width is the engine's fact, the count is content's.

## Where the enum is load-bearing today

- `crates/data/src/map.rs`: `ResourceKind` (Wood/Food/Gold/Stone), `index()`, `dat_id()`,
  `from_dat_id()`, `COUNT`. Map nodes carry it: `EntityKind::Resource(ResourceKind)`.
- `crates/sim/src/lib.rs`: `stock: [[u32; ResourceKind::COUNT]; MAX_PLAYERS]`; deposits use
  `kind.index()`; **four** cost sites use `ResourceKind::from_dat_id(dat_id)` (lines ~1038, ~1063,
  ~1248, ~1301 at the time of writing).
- `crates/data/src/resources.rs`: already has `slot_for_dat_id`, which is the replacement for
  `from_dat_id`, and `slot_of(name)`.

## Order of work

1. **`World` takes the pack.** `seed_from(map, economy, units, resources)` - or, if the churn is
   unwelcome, `load_resources(&Resources)` mirroring `load_combat`. Nine fixtures call `seed_from`
   (build, combat, gather, pathing, perf, scenario, seeding, train, headless) and each must pass
   resources; that is the whole cost, and it is mechanical.
2. **Costs go through the pack.** The four sites become
   `resources.slot_for_dat_id(dat_id).ok_or(SimError::UnknownResource { dat_id })?`, so a pack that
   does not carry an id refuses the order instead of guessing a slot.
3. **Stockpiles widen from the pack, not the enum.** `stock` stays a fixed array (a const is needed
   for the type), sized `MAX_RESOURCES` with the pack's count checked at load against it. The pack
   already refuses a duplicate slot.
4. **Map nodes name a slot.** `map.toml` writes resource names; the loader resolves name → slot via
   the pack, so `EntityKind::Resource(usize)` replaces `ResourceKind` and the map file keeps its
   readable names.
5. **Delete the enum**, its bridge test, `index()`, `dat_id()`, `from_dat_id()`, `COUNT`. If a test
   needs "wood", it says `resources.slot_of("wood")`.

## The test that proves the boundary held

`a_content_pack_with_our_own_resources_loads_and_spends` - a pack whose resources are called
"timber" and "grain", a building that costs `timber`, and a full build completed with it. It cannot
be written while `ResourceKind` exists, which is exactly why it is the finish line.

## Outcome

`a_content_pack_with_our_own_resources_loads_and_spends` exists and passes. It builds its world
entirely from content of our own: five resources (`grain`, `iron`, `salt`, `wool`, `timber`) with
**timber in slot 4**, a map that calls its wood `timber` and puts it in a forest, a three-unit table
whose house costs `{ resource = "timber", amount = 25 }` — and it asserts the column at slot 0 stays
empty from the first tick to the last, so a fallback to column zero pays in the wrong goods and the
test fails rather than quietly balancing.

Three further tests guard the refusals, because each one is the difference between "wrong" and
"free": a pack that does not carry a cost's reference id (`UnknownResource`), a pack wider than a
stockpile row (`TooManyResources`, refused at load), and a cost naming a resource the pack does not
define (`UnknownResourceName`). The last of these was proved to bite by mutation: replacing the
refusal with `Ok(0)` makes it fail at the point where the house gets built for free.
