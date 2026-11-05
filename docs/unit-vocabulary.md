# Unit vocabulary: from dat ids to names

*Plan for the owner's request (2026-11-04): "I want to get rid of unit IDs. Instead it's like each unit having clear
parameters — starting availability and which unit it upgrades." Accepted as the next architectural step; this note is written
**before** any schema code, and it exists so a session that starts cold does not have to rediscover the numbers.*

`tools/vocabulary.py` is the inventory everything below is measured against; run it before changing the schema.

## The measurements (2026-11-04, `empires2_x2_p1.dat` pack)

| | |
|---|---|
| units in the table | **1,389** |
| units with **no name** | **56** |
| **duplicate names** | **28** (`BARRICADE` ×4, `BGAA` ×3, `THSWD`, `MPCAV`, `ARTCT`, `DOLP3`, `CARAV`, …) |
| roles in `economy.toml` (the vocabulary the engine already uses) | **40**, every one naming a unit that exists, no two roles on one unit |
| upgrade edges in the effect table (`[3, from, to]`) | **223** |
| bases with several variants (an age-up: normal) | 43 |
| **variants with more than one base** | **30** (e.g. the unit our roles call `pikeman` (358) is reached from 93 and from 358's own line) |
| **cycles in the upgrade graph** | **3** (527, 528, 1104) |
| effect fields carrying a bare `unit = <id>` | 0 (units are referenced only through `commands`) |

## What the numbers change about the proposal

I proposed a per-unit field, `upgrades_from = "militia"`. **The pack says no:** 30 units have two possible predecessors, and
there are three cycles. A single field per unit cannot express a graph, and would silently pick one parent.

So the **edge** is the unit of content, exactly as AoE2 does it:

```toml
[[upgrade]]
from = "barracks"            # the unit that is replaced
to   = "barracks_feudal"     # what it becomes
tech = "feudal_age"          # the effect that performs it; this is the condition, not a preference
```

and a unit carries only what belongs to a unit:

```toml
[[unit]]
name = "militia"             # the key. Unique. Names are the vocabulary.
hp = 40
available_from = "dark_age"  # or: requires = ["..."], or available = true
id = 74                      # provenance the importer writes; nothing reads it
```

**Modding requirement (the owner's question, answered in the schema):** the edge lives in the effect, so
*"scout cavalry is the upgrade of hussar"* is a one-line edit and needs no engine change. The engine must therefore treat the
graph as data: no assumption that a base has one variant, no assumption of acyclicity (**detect** a cycle and refuse with the
cycle printed, do not loop), and no assumption that the direction matches the dat's.

## Phases, each one landable and green

1. **The inventory tool** — `tools/vocabulary.py` (this step). Reads the pack, prints the table above, exits non-zero while
   the pack would block a name-keyed schema. No engine change.
2. **Names for everything** — 56 unnamed units and 28 duplicated names need keys, in the **generator** (`tools/genconfig`),
   not in the generated files. The rule to pick: the dat's own name; on a duplicate, the name plus the smallest distinguishing
   suffix the importer can derive (a numeric suffix is ugly but honest, and the pack is generated, so a hand-made word would
   be overwritten). `tools/vocabulary.py` must go green on duplicates and unnamed units.
3. **Edges into the pack** — the generator emits `[[upgrade]]` for the 223 edges with the tech that carries them; the cycle
   detector refuses the three cycles with a printed path. `units.toml` keeps `id` only as provenance.
4. **The engine's handle** — `Entity.unit` becomes a **pack key**: an index the loader assigns in name order. The tick keeps
   comparing integers (never strings), and `role_unit(name)` / `root(key)` / `role_name(key)` become pack lookups. `root`
   becomes "the ancestors of this key via upgrade edges" — a **set**, because 30 units have two.
5. **The hash migration** — the vocabulary is in the state hash, so every acceptance hash and every recording changes. Name
   it, land it alone, and re-baseline the acceptance test in the same commit so the change is visible rather than absorbed.

## What this deletes

- The bug fixed on 2026-11-04 (`role_name` naming a militia a champion): with keys that *are* names there is no family to
  disambiguate, so the class of bug disappears by construction.
- `units.root()`'s role as "the family base": it becomes an ancestor query over data, which is what the engine actually
  asks.
- The dat ids as the engine's vocabulary. They stay in the generated file as provenance and in the importer, which is the
  only place that may ever read them.

## Risks, named

- **A pack with a cycle** would hang `root`; the loader must refuse at load time with the cycle printed. Three exist in the
  dat today, so the detector is not theoretical.
- **A unit with two bases** makes "the family name" ambiguous — the engine must ask "is this an ancestor of that", not
  "what is its root".
- **A missing name** (56 today) would leave a unit unreachable by any content rule; the generator must fail rather than emit
  an unnamed unit.
- **The hash** moving is the migration's whole cost, and it must not be absorbed into an unrelated change.
