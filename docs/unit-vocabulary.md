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
| upgrade edges in the effect table (`[3, from, to]`, deduplicated) | **222** |
| bases with several variants (an age-up: normal) | 43 |
| units reached by several tiers **inside one effect** (a tech that skips a tier) | **22** |
| units reached across **different effects** (a real fork) | **2** (`ARRG3` (10) from 102 and 133; `WALL3` (155) from two effects, both from `WALL2`) |
| units on a **cycle** | **2** (`RMSHP` 527 and `SDGAL` 1104: the two fishing-ship effects upgrade each into the other) |
| effect fields carrying a bare `unit = <id>` | 0 (units are referenced only through `commands`) |

## What the numbers change about the proposal

I proposed a per-unit field, `upgrades_from = "militia"`. **My first reading of the pack said it could not work** — I
counted "units with two predecessors" and reported 30. **That was wrong, and the owner caught it:** the halberdier (359) is
reached from the spearman (93) *and* the pikeman (358), but **both edges live in effect 189, the Halberdier tech itself** —
the dat lets one tech replace *every* earlier tier of a line, so a chain that skips a rung still ends where it should. The
line is pikeman -> halberdier, exactly as a player knows it. Same shape for the hussar (441): effect 439 lists the scout and
the light cavalry.

Corrected by the inventory: **22** units have that same-effect fan-in (a tech skipping a tier), and only **2** are reached
from genuinely different effects (`ARRG3`, and `WALL3` which is a duplicated edge from `WALL2` through two effects). So a
per-unit field *could* express a line - it simply cannot express **the tech as the actor**, and cannot say which of two
different effects performed an upgrade.

**The edge model stands, for a better reason than I first gave:** in AoE2 the *tech* is what performs an upgrade ("Halberdier
replaces spearman and pikeman"), so the tech is the edge's owner. A per-unit field would be a lossy projection of that -
and the cycle (`RMSHP` <-> `SDGAL`, two effects, one each way) shows the relation is not a forest in principle.

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

- **A pack with a cycle** would hang `root`; the loader must refuse at load time with the cycle printed. One exists in the dat
  today (the two fishing ships, `RMSHP` and `SDGAL`), so the detector is not theoretical.
- **An upgrade performed by two different effects** (two units today) makes "the family root" ambiguous — the engine must ask
  "is this an ancestor of that", not "what is its root".
- **A missing name** (56 today) would leave a unit unreachable by any content rule; the generator must fail rather than emit
  an unnamed unit.
- **The hash** moving is the migration's whole cost, and it must not be absorbed into an unrelated change.
