#!/usr/bin/env python3
"""Inventory the pack's unit vocabulary: what the engine would have to name if unit ids went away.

Reads `config/gamedata/{units,effects}.toml` and `config/economy.toml` and reports, without changing anything:

- units, and how many of their **names are not unique** (a name-keyed pack needs a unique key);
- the **roles** the engine actually uses, and whether each names a unit that exists and is unique;
- the **upgrade edges** the effect table carries, whether the graph is acyclic, and how the chain from a unit to its base
  looks - this is the relation the owner wants to express as "which unit it upgrades";
- effect fields that name a **unit id with no name**, which an importer could not translate.

Exit code 1 when something would block a name-keyed pack, so the tool can gate a future step. Nothing here touches the
simulation; it is the inventory the plan in `docs/unit-vocabulary.md` is written against.
"""

from __future__ import annotations

import sys
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def load(rel: str) -> dict:
    with (ROOT / rel).open("rb") as handle:
        return tomllib.load(handle)


def main() -> int:
    units = load("config/gamedata/units.toml")
    effects = load("config/gamedata/effects.toml")
    economy = load("config/economy.toml")

    table = units.get("unit", [])
    by_id = {unit["id"]: unit for unit in table}
    names = [unit.get("name", "") for unit in table]
    counts = Counter(names)
    duplicated = {name: count for name, count in counts.items() if count > 1 and name}
    unnamed = [unit["id"] for unit in table if not unit.get("name")]

    print(f"units: {len(table)}")
    print(f"  unnamed: {len(unnamed)}")
    print(f"  duplicate names: {len(duplicated)}")
    for name in sorted(duplicated)[:12]:
        ids = sorted(unit["id"] for unit in table if unit.get("name") == name)
        print(f"    {name}: {ids}")
    if len(duplicated) > 12:
        print(f"    ... and {len(duplicated) - 12} more")

    # Roles: the engine's vocabulary today. A role that names a missing unit is a hole; two roles naming one unit is a
    # question a name-keyed pack answers by construction (one name per unit).
    roles = economy.get("role", {})
    missing = {role: unit for role, unit in roles.items() if unit not in by_id}
    shared = defaultdict(list)
    for role, unit in roles.items():
        shared[unit].append(role)
    collisions = {unit: rs for unit, rs in shared.items() if len(rs) > 1}
    print(f"roles: {len(roles)}")
    print(f"  naming a unit the table lacks: {len(missing)} {missing if missing else ''}")
    print(f"  two roles for one unit: {len(collisions)}")
    for unit, rs in sorted(collisions.items())[:8]:
        print(f"    unit {unit} ({by_id.get(unit, {}).get('name', '?')}) is {rs}")

    # Upgrade edges, from the effect table: `commands = [[kind, a, b], ...]` with kind 3 = upgrade a -> b.
    edges: list[tuple[int, int]] = []
    for effect in effects.get("effect", []):
        for command in effect.get("commands", []):
            # A command with a negative field is the dat's "no target" (an age-up writes `to = -1` when it only enables),
            # so it is not an edge and must not be counted as one.
            if isinstance(command, list) and len(command) >= 3 and command[0] == 3:
                base, variant = command[1], command[2]
                if base >= 0 and variant >= 0:
                    edges.append((base, variant))
    # **Direction.** `[3, from, to]` is "from upgrades into to": the barracks (12) becomes BRKS2 by the Feudal Age, so the
    # *base* is the first field. A variant with **two bases** is the case a per-unit `upgrades_from` cannot express; a base
    # with two variants is just an age-up and is fine.
    parents: dict[int, int] = {}
    ambiguous: list[tuple[int, int, int]] = []
    outgoing: dict[int, list[int]] = defaultdict(list)
    for base, variant in edges:
        outgoing[base].append(variant)
        if variant in parents and parents[variant] != base:
            ambiguous.append((variant, parents[variant], base))
        parents[variant] = base
    print(f"upgrade edges (effect kind 3): {len(edges)}")
    print(f"  bases with more than one variant (an age-up: fine): {sum(1 for v in outgoing.values() if len(v) > 1)}")
    print(f"  variants with more than one base (a per-unit field cannot say this): {len(ambiguous)}")
    for variant, first, second in ambiguous[:6]:
        print(f"    unit {variant} <- {first} and {second}")

    # Cycles: follow parents at most len(units) hops.
    cyclic = []
    for start in parents:
        seen = set()
        cursor = start
        while cursor in parents:
            if cursor in seen:
                cyclic.append(start)
                break
            seen.add(cursor)
            cursor = parents[cursor]
    print(f"  cycles: {len(set(cyclic))}")
    if cyclic:
        print(f"    starting at {sorted(set(cyclic))[:8]}")

    # **The shape that matters:** the relation is `(tech, from, to)`, and a unit can be the *target* of more than one
    # edge (a barracks reaches its Feudal form by one tech and its Castle form by another). So a per-unit "upgrades_from"
    # field is lossy - 67 units have two parents in this pack. Print the edges for a few familiar lines so the plan is
    # written against the real graph.
    print("edges for a few lines (from -> to, by tech):")
    by_child = defaultdict(list)
    for child, parent in edges:
        by_child[child].append(parent)
    for name in ("MILITIA", "SPRMN", "SCOUT", "LTCAV", "HUSSAR", "BRKS"):
        ids = [unit["id"] for unit in table if unit.get("name") == name]
        for unit_id in ids:
            incoming = [
                (parent, by_id.get(parent, {}).get("name", str(parent)))
                for parent in by_child.get(unit_id, [])
            ]
            if incoming:
                print(f"    {name} ({unit_id}) is upgraded by: {incoming}")

    # Effect fields naming a unit, which an importer has to translate.
    unit_fields = 0
    for effect in effects.get("effect", []):
        for key, value in effect.items():
            if key == "unit" and isinstance(value, int):
                unit_fields += 1
                if value not in by_id:
                    print(f"  effect names unit {value}, which the table lacks")
    print(f"effect `unit = <id>` fields: {unit_fields}")

    blocked = bool(
        duplicated or unnamed or missing or collisions or ambiguous or cyclic
    )
    print(f"verdict: {'blocked' if blocked else 'ready'}")
    return 1 if blocked else 0


if __name__ == "__main__":
    sys.exit(main())
