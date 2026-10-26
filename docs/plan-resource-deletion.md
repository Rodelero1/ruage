# Plan: delete `ResourceKind` (D17)

Not started. It touches nine fixtures and every test that pins a stockpile, so it wants a session
that can finish it rather than an hour before the host's automatic reboot.

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
