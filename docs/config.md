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
| `map.toml` | the authored 64×64 slice map (D15): terrain rows, resource nodes, town centres | Our own content, not extracted data, mirrored point-for-point so neither side is favoured. |

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

The generator lives in its **own repository**, [`ruage-tools`](https://github.com/Rodelero1/ruage-tools)
(D12, split 2026-10-26): it is GPL-3.0-or-later because it imports `GenieDatParser`, and this
repository is MIT OR Apache-2.0. It takes the dat and the output directory as arguments, so it
writes into *this* checkout without living in it.

```bash
git clone https://github.com/Rodelero1/ruage-tools ../ruage-tools
cd ../ruage-tools
uv run python -m genconfig \
  /opt/data/workspace/Age2HD/Age2HD/resources/_common/dat/empires2_x2_p1.dat \
  ../ruage/config
```

The generator is **test-first** (D7, SPEC §11), and its suite stays with it: 22 tests, run with
`uv run --with pytest pytest`. Two of them need this side of the boundary and skip without it:
`RUAGE_CONFIG=<this checkout>/config` for the cross-file checks, `RUAGE_DAT=<install>/…/empires2_x2_p1.dat`
for the real-dat integration test (1,166 units, knight 100 HP / 1.35 speed / 2-2 armour, effect 195).

`config/MANIFEST.toml` records the dat's sha256, byte size, per-file sha256s and counts.

**Planned, not implemented (corrected 2026-10-23).** A startup check is supposed to compare the
manifest's source hash against the dat on disk, so a patched game produces a "config is stale"
error instead of quietly reusing old numbers. Nothing reads the manifest today — `genconfig`
writes it and no code consumes it, and `crates/data` loads only `sim.toml`. The earlier text
here described that check in the present tense, which was wrong. SPEC §11 already lists
"manifest staleness" as a test that must precede its code.

## Scope of strict loading (corrected 2026-10-23)

**Updated 2026-10-23:** `sim.toml`, `economy.toml`, `map.toml` and `gamedata/units.toml` have
strict Rust loaders
(`crates/data/src/{lib,economy,map}.rs`); every loader's tests were written first. `combat.toml`,
`movement.toml` and `overrides.toml` still have no schema and no loader, and `overrides.toml`'s
path syntax remains documented but unimplemented — so the rule is enforced for four files of six.
Do not read a green `cargo test` as "the config layer is complete".

`economy.toml` also carries `[gather.resource_task]`: which gather task a node of each map kind
presents (our map's food is berries, so `food = "forage"`, while hunting and fishing exist as
variants nothing on this map triggers). The loader refuses a name that is not a key in
`[gather.variant]`, so a typo is a startup failure rather than a villager who walks to a tree and
never gathers.

`economy.toml` also carries `[start]` — **role names**, not ids: which role is the town centre,
which is the villager, and how many villagers each player begins with — plus `[role]`, the table that
turns a role into a unit id (`town_centre = 109`, `house = 70`, `barracks = 12`, `spearman = 74`).
The role table is the engine's whole vocabulary for units, so nothing outside content has to know
that a town centre is 109 (D17): `World::seed_from` places one town centre per map town-centre tile
and that many villagers on the nearest free walkable tiles, a scenario step names `role = "house"`,
and the engine's own free-tile search finds the centre through `start.town_centre`. The loader
refuses an empty role table, a role mapped to unit 0 (zero is a real unit id, so it cannot mean
"unset"), and a `[start]` naming a role the table does not define.

`units.toml` carries `variant_of` on the 190 units that are an *upgraded variant* of another: an age-up
replaces a building's id rather than changing its numbers (town centre 109 -> 71, barracks 12 -> 498,
blacksmith 103 -> 105). The relation exists nowhere in the dat except in the effects table's kind-3
commands, so the importer derives it, and the engine resolves a variant to its root wherever a rule
names a building: what a tech is researched at, where a carried load may be dropped, what a building
trains, and what a scenario's `role = "barracks"` means.

`[farm] food_resource = 36` names the dat's *internal* resource that farm upgrades add to: the dat has no
resource names, so an id is the only handle, and 36 is the farm food line (Horse Collar +75, Heavy Plow
+125, Crop Rotation +175). A tech whose effect adds to that resource raises every plot the player has that
still holds food, and every plot built afterwards. With no `food_resource`, farming never improves.

`[farm]` says which building is a farm and what working it yields - `unit = "farm"` (a role) and
`resource = "food"`. Those two names are the entire extent of the engine's farming knowledge: a farm is a
building it would otherwise treat like any other, and it becomes worth gathering from only because content
says so. Leave them out and no building yields anything. `use_villager_rate` decides who sets the pace (the
building's own `rate`, or the farmer variant's), and `depletion.food_per_farm` is the plot's whole budget.

`[start] teams = [1, 1]` puts players on teams: entry *n* is player *n+1*, the last entry covers any
further player, and **empty means a free-for-all** (every player is their own team, so the setting is only
written by a pack that wants allies). Two players on one team cannot attack each other (`AlliedTarget`);
the project's own pack names no teams, because its two players are meant to fight.

`[start] civs = ["British", "Huns"]` names the civilisations a match is played as, by the names
`civs.toml` gives them: entry *n* is player *n+1*, and the last entry covers any further player, so a pack
that names one civ means "everyone plays this one". The project's own pack names two, which is what the
bar rules are exercised against - player 1 is refused the Hussar and can build houses, player 2 is the
other way round on both counts.
The civ table is generated from the dat (`id`, `name`, `tech_tree_effect`, `team_bonus_effect`), and the
effect it names holds the techs that civ **cannot** research — a restriction list, per D23. Content never
names the effect id, only the civ; `World::load_civs` resolves the name and refuses an unknown one by
name. One civ for the whole match for now: per-player civs are a later step, and the civ id is hashed so
a replay that lost it cannot diverge silently.

`[start] stock = { food = 200, wood = 200, gold = 100, stone = 200 }` is the standard opening: resource
**names** to amounts, resolved through the pack at load, so a pack with five resources opens with its
own five. It is applied once - a second `load_resources` does not re-apply it and wipe what the players
have gathered since (`start_stock_applied` is state, and hashed) - and a name the pack has no column for
is refused with the name in the message rather than dropped, because a player who silently starts with
no gold cannot pay for anything and has no way to know why.

`[start] techs` names the techs a match begins with, by `[tech_role]`. The pack opens with
`techs = ["dark_age"]`, because every tech in the table lists the Dark Age (104) as a prerequisite: a
match that started with nothing researched could research **nothing at all**. The engine grants them
whichever order its tables are loaded in - `seed_from` and `load_research` both try - and a tech role
the pack cannot name is refused at load. The tables must carry the tech: `--techs`/`--effects` on the
headless runner, or `World::load_research` in a test.

`[[train.entry]]` says which building may create which unit, and it names **roles**, like everything
else that talks about a unit: `building = "town_centre"`, `units = ["villager"]`. The ids are resolved
through `[role]` when the pack loads, so an author who mistypes a role is told at load which name was
unknown; an integer is refused outright rather than accepted as a second spelling.

`[tech_role]` does the same job for **techs**, which the dat does not name at all (a tech carries a
string-id into the game's language files, so the importer has no word for one): `man_at_arms = 222`,
`loom = 22`, `feudal_age = 101`. It is optional — a pack with no research needs no table — but a
scenario step that says `tech = "loom"` needs the entry, and the loader refuses a role mapped to tech
0 for the same reason as units (tech zero is a real tech, "New Technology").

The map loader enforces the invariants an author can get wrong: a row must be exactly `width`
characters, terrain characters are `G` or `F` only, every resource tile is on the map, **wood
must stand on forest and everything else on open ground** (which is what stops "a tree you can
walk through" or "gold inside the trees"), and both players need exactly one town centre on open
ground.

**Known limits of the economy loader**, both deliberate:

- It cannot enforce "the base villager is never a gather variant": that needs unit-id knowledge,
  so the guard lives in `tools/tests/test_config_consistency.py`, which cross-checks against the
  generated data. The loader enforces only what it can see — one-to-one ids, no zero, non-empty.
- `rate_from`, `carry_from` and `drop_sites_from` are free-text selectors today, so a typo
  (`"dta"`) loads fine. They become enums with the code that reads them, not before — a closed
  enum for a field nothing consumes would be the speculative kind of strictness (Karpathy rule 2)
  rather than the kind that catches a real error.

## The two decimal grains (D10)

`sim.toml` declares what the data needs, and the loader refuses a scale that cannot meet it:

| key | value | what it means |
|---|---|---|
| `dat_value_decimals` | 4 | finest grain anywhere in the dat — `work_rate = 0.0111` on units 110 (TDWS) and 1021 (FEITO) |
| `per_tick_decimals` | 3 | finest grain among quantities integrated every tick — speed `0.737` |

Rules: `scale % 10^dat_value_decimals == 0` (no value is rounded on the way into state) and
`scale % (tick_rate_hz × 10^per_tick_decimals) == 0` (no per-tick displacement drifts). At 20 Hz
that gives exactly 20000 for the second rule, and 20000 also satisfies the first.

`crates/data/tests/corpus.rs` sweeps every speed, work_rate and los in the generated data and
fails if either declaration is too small. If a future dat introduces a value with more places,
that test says so by name instead of the engine rounding silently.

## Gathering data (settled 2026-10-23, D11)

The gap recorded by the adversarial review is closed on the data side:

- `gamedata/units.toml` now carries **`drop_sites`** per unit, read from
  `task_info.drop_site_unit_ids`. Measured: the lumberjack (VMLUM 123) drops at the town
  centre + lumber camp (109, 562); the miner (124) and gold miner (579) at 109 + mining camp
  (584); forage/hunt/fish/herd at 109 + mill (68). The base villager (83) lists 109 + mill,
  which is exactly why a base-villager lookup gives the wrong answer for wood.
- `economy.toml` carries the **task → variant** table (D6: the dat cannot supply it at
  VER 5.7.2, so code must not either). Rates are not duplicated there; they are read from
  `gamedata/` by variant id.
- `tools/tests/test_config_consistency.py` guards the join: every named id must exist, be a
  `VM*` variant, have a plausible gather rate, and list a drop site — and the base villager
  must never appear.

Still open: nothing in Rust reads `economy.toml` yet (see "Scope of strict loading" above), so
the table is data waiting for its loader, which is the first job of the gathering code.

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

## Changing the tick rate (2026-10-23)

`sim.tick_rate_hz` is a config value, and `minimum_scale()` in `crates/data/src/lib.rs` derives the
smallest scale that keeps both of D10's grains exact: `lcm(10^dat_value_decimals,
tick_rate_hz × 10^per_tick_decimals)`, i.e. `lcm(10000, 1000 × Hz)`.

| Hz | minimum scale | per-tick displacement |
|---|---|---|
| 10 | 10,000 | speed × 1000 |
| 20 | 20,000 | speed × 1000 |
| 25 | 50,000 | speed × 2000 |
| 30 | 30,000 | speed × 1000 |
| 60 | 60,000 | speed × 1000 |
| 100 | 100,000 | speed × 1000 |
| 144 | 720,000 | speed × 5000 |
| 240 | 240,000 | speed × 1000 |

**What does not change.** Every dat number is per second, so gather rates, hp, build times and
time-to-kill in seconds are stable across rates. The rate is sampling resolution, not accuracy: a
higher rate does not make an exact dat value more exact, it only reduces the quantisation of
tick-based events (damage cadence, build completion, deposit oscillations).

**What changes.** The state hash, whenever the scale differs; tick-scheduled scripts (D15's
opponent list is expressed in ticks); tests that pin raw integers; and CPU cost, which is linear —
a 10-minute game is 12k ticks at 20 Hz, 36k at 60 Hz, 86k at 144 Hz. §6.1's bands are expressed in
ticks, so they *tighten* in wall-clock as the rate rises: "1 tick" is 50 ms at 20 Hz and 7 ms at
144 Hz. A higher rate makes the acceptance bands harder to meet, not easier.

**Experimenting is cheapest now**, before there is a golden hash, a recorded trace or a scripted
opponent schedule to re-baseline. `crates/data/tests/rate_sweep.rs` proves twelve rates from 10 to
240 load at their derived scale, that a rate change without a scale change is refused, and that
five dat speeds land on a whole unit per tick at each rate.

## `resources.toml` — the engine's whole idea of a resource (2026-10-25, rewritten 2026-10-26)

```toml
schema = 1

[[resource]]
slot = 0
name = "wood"
dat_id = 1
site = "forest"
```

Four fields, and each answers a different question:

- **`slot`** — the stockpile column. Dense from zero in file order; the loader refuses a gap, because
  a gap is a column nothing can address, and a duplicate name, because a name lookup would silently
  take the first. The engine indexes this and nothing else.
- **`name`** — what map files, scenarios and the economy call the resource. It is the only vocabulary
  the content has, and the engine never invents one.
- **`dat_id`** — the *reference pack's* numbering, optional. Present, it is what a cost entry
  `[1, 25]` resolves against; absent (a pack of our own), every reference lookup misses, which is
  correct and is why callers handle `None` rather than defaulting to column 0.
- **`site`** — where a map may put a node of this resource: `"forest"` or open ground (the default).
  "Wood grows in forests" is a fact about wood, so it lives here, and the map loader enforces it in
  both directions — a forest resource off a forest tile, and an open-ground resource inside one.

**This file is now the only truth about resources.** `ResourceKind` — the four-variant enum that
used to *be* the pack, with `[[u32; ResourceKind::COUNT]; 2]` stockpiles — was deleted on 2026-10-26
along with its bridge test (`the_content_file_agrees_with_resource_kind`, which asserted the file
and the enum agreed on name, slot and dat id). What replaced it:

| Question | Answer |
|---|---|
| How many resources exist? | `Resources::count()`, checked at load against the engine's `MAX_RESOURCES = 8` — a stockpile row is an array, so the *width* is the engine's fact while the *count* is content's |
| What is a node? | `EntityKind::Resource(slot)` — a column, never a kind |
| What does a cost mean? | A cost names its resource: `[1, 25]` (reference id) or `{ resource = "timber", amount = 25 }` (name) |
| Which task works a resource? | `[gather.resource_task]`, keyed by **name** (`timber = "wood"`) |
| Where may it stand? | `site`, enforced by the map loader |

D17's rule — the dat's numbering must not reach `crates/` — now holds for resources: `dat_id` is
read in one place (`Resources::slot_for_dat_id`), and every other layer speaks columns, names and
costs. The test that pins the whole boundary is
`a_content_pack_with_our_own_resources_loads_and_spends` in `crates/sim/tests/build.rs`: a pack of
five resources called grain, iron, salt, wool and timber, a map whose wood is called timber, and a
house that costs `{ resource = "timber", amount = 25 }`, gathered, paid for and built to full hp —
with the slot-0 column asserted to stay empty throughout.

## Techs cost what units cost (2026-10-26)

`crates/data/src/cost.rs` holds one price type, and both tables use it: `[1, 25]` is the reference
pack's id and `{ resource = "timber", amount = 25 }` is a name the pack defines. That matters for
research, which is content like everything else: a tech's price resolves through `cost_slot`, so a
pack of our own can price a tech in its own resources, and a pack that cannot price it refuses the
order rather than starting it for free.

`techs.toml` and `effects.toml` are part of the **content pack** as of the same date - `PACK_FILES`
is eight files, not six - because a peer running different techs is playing a different game. The
content hash covers their bytes too, so a mismatch is caught at the handshake rather than when one
side's spearman upgrades and the other's does not.
