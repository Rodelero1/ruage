# Configuration layer

Decision **D6**: game data lives in `config/` as TOML, and the engine reads TOML — not the
`.dat` — at runtime. Nothing numeric is hardcoded.

The trade-off that comes with this, and how it is mitigated, is at the end. Read that part
before changing anything.

## Three layers, in order of precedence

```
config/gamedata/*.toml   GENERATED from the .dat   (never edited by hand)
        ↓
config/overrides.toml    hand-written, path-keyed   (survives regeneration)
        ↓
config/sim.toml          hand-written behaviour constants
config/combat.toml
config/economy.toml
config/movement.toml
```

Precedence is bottom-up: a value in `overrides.toml` beats the same value in `gamedata/`, and
behaviour constants are separate keys that never collide with data keys.

## What is in each layer, and why

| Layer | Contents | Why it lives there |
|---|---|---|
| `gamedata/` | 1,166 units, 801 techs, 798 effects, 32 civs — stats, costs, attack/armour arrays, gather rates, multipliers | It is data. Generated so it is reproducible and cannot drift from the dat. |
| `overrides.toml` | Anything from `gamedata/`, by path | Experiments. Regenerating must not destroy them. |
| `sim.toml` | tick rate, fixed-point scale, RNG, determinism flags | Engine decisions, not game facts. |
| `combat.toml` | min damage, elevation, sentinels, attack-timing fraction | Constants the dat does not contain. |
| `economy.toml` | farm rate, build/repair rates, pop values, decay | Constants the dat does not settle. |
| `movement.toml` | tile scale, pathing weights, collision, formation | Unknowns (U5, U7) that must be visible, not buried. |

## Rules

1. **`gamedata/*.toml` is never hand-edited.** Each file starts with the source hash and
   `# GENERATED FILE — do not edit`. Regeneration overwrites it wholesale.
2. **Edits go in `overrides.toml`.** Every override is logged at startup with old → new
   value, so a diff of the effective config always shows what a run changed.
3. **Arrays are replaced, never merged.** `"units.83.attacks" = [[4, 3]]` replaces the whole
   array. Partial array edits are almost always a mistake, and silently merging hides it.
4. **Strict loading.** Unknown key or missing required key = startup failure, not a warning.
   A typo'd key that silently does nothing is how sims drift from their spec.
5. **Unresolved values are explicit.** `-1.0` means *not yet determined*. If the engine needs
   such a value for the scenario being run, it must refuse to start rather than substitute a
   default. Currently unresolved: `hp_per_second_per_villager`, `repair_hp_per_second`,
   `world_units_per_tile`.
6. **Every non-trivial value carries a `PROVENANCE:` comment**, one of:
   - `dat` — read from the game files (traceable to the generator)
   - `published` — community-documented, cited in `docs/sources.md`
   - `measured` — we tested it (none yet)
   - `unknown` — an open question; the value is a placeholder with a U-number
   - `placeholder` — no provenance at all; must be tuned before it is trusted

   The point of the tag is that a reader can tell a fact from a guess at a glance.

## Regenerating

```bash
cd /opt/data/workspace/Age2HD/tools
uv run python -m genconfig \
  ../Age2HD/resources/_common/dat/empires2_x2_p1.dat \
  ../config
```

The generator is **test-first** (D7, SPEC §11): 17 tests in `tools/tests/`, run with
`cd tools && uv run --with pytest pytest`. The suite includes a real-dat integration test that
asserts 1,166 units, knight 100 HP / 1.35 speed / 2-2 armour, and effect 195 carrying floats.

`config/MANIFEST.toml` records the dat's sha256, byte size, per-file sha256s and counts. A
startup check compares the manifest's source hash against the dat on disk: if the game has
patched, the config is **stale** and the engine says so instead of quietly using old numbers.

## Verified so far

- All ten TOML files parse with `tomllib`; counts match the manifest
  (units 1,166 / techs 801 / effects 798 / civs 32).
- Round-trip against the independent JSON pipeline (`build/derived/units-british.json`):
  **0 mismatches across all 1,166 units** on hp, speed and work_rate.
- Spot values survive intact: knight 100 HP, 2/2 armour, 60F 75G; archer 30 HP, 4 pierce;
  effect 195 = `[[5, 218, -1, 13, 1.2], [5, 123, -1, 13, 1.2]]` (DBA, floats not strings).
- `fixed_point_scale = 20000` at 20 Hz: all 56 distinct dat speeds land on exact integers.

## Two traps worth knowing

**Unit array ≠ unit count.** The per-civ unit array has **1,402 slots but 1,166 populated**
ids — 236 gaps where the game has holes (ids 0..1401, not dense). Anything that iterates
"1..N" instead of the populated set will read garbage. The generator emits only populated
units, so the config is the safe view.

**Generated data is derived game content.** Committing `config/gamedata/*.toml` to a public
repo distributes derived values from a licensed game. Fine for private use; if **D4** lands on
"publishable", `gamedata/` must be generated locally at first run and gitignored, with only
`gen_config.py` and the behaviour configs shipped. This is why the generator is a
standalone script rather than part of the engine binary.

## What is deliberately *not* in config

- **Display names** — they are string ids into the language files (U9), not text.
- **Sprites, sounds, terrain graphics** — asset references, not simulation values.
- **Derived quantities** (DPS, effective HP, walk time) — computed, never stored. Storing
  them invites two sources of truth drifting apart.
- **Per-civ unit copies** — the dat duplicates 1,402 units per civ; the config ships civ 1 as
  canonical and will layer per-civ deltas on top (a few keys per civ, not 32 full copies).
