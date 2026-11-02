"""Lines: unit families named by an AI script instead of unit ids (D95).

The AI never says "train a paladin". It says "train from the knight line, the generalist cavalry one", and the
resolver takes the last member the civ can actually field. That makes the civ's tech tree the only thing that decides
what a civ fields - Saracens simply never reach the end of the knight line - which is why D95 chose lines over a
scored "best unit" search.

The table itself is authored (`config/lines.toml`) because D95 says a civ override decides what answers a line, and an
authored answer can be reviewed. But authored data drifts, so this tool checks the table against the pack: every
consecutive pair in a line must be an upgrade in the effect table (`kind = 3`), and no line may contain a unit the
economy has no role for.

Run: `python3 tools/lines.py` (verify) or `python3 tools/lines.py --print` (show the chains the pack implies).
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
CONFIG = HERE.parent / "config"
EFFECTS = CONFIG / "gamedata" / "effects.toml"
ECONOMY = CONFIG / "economy.toml"
LINES = CONFIG / "lines.toml"

# Effect command kinds, as the generator writes them. Only `upgrade` makes a line.
UPGRADE = 3
# A "Non-Research" effect swaps a unit for free (an age-up or a scenario grant); it is not a tier a civ can buy, so it
# must not extend a line - otherwise the line would claim a tier nobody can field.
NOT_A_TIER = "non-research"


@dataclass(frozen=True)
class Edge:
    """One upgrade in the pack: `effect` researches `from_unit` into `to_unit`."""

    effect: str
    from_unit: int
    to_unit: int


def _as_positive(value: int) -> bool:
    return value > 0


def upgrade_edges(effects: dict) -> list[Edge]:
    """Every researchable upgrade in the effect table, in table order."""
    edges: list[Edge] = []
    for effect in effects.get("effect", []):
        name = str(effect.get("name", ""))
        if NOT_A_TIER in name.lower():
            continue
        for command in effect.get("commands", []):
            if len(command) < 3 or command[0] != UPGRADE:
                continue
            from_unit, to_unit = command[1], command[2]
            if not (_as_positive(from_unit) and _as_positive(to_unit)):
                continue
            edges.append(Edge(effect=name, from_unit=from_unit, to_unit=to_unit))
    return edges


def _children(edges: list[Edge]) -> dict[int, list[int]]:
    children: dict[int, list[int]] = {}
    for edge in edges:
        children.setdefault(edge.from_unit, []).append(edge.to_unit)
    return children


def chain_from(root: int, edges: list[Edge]) -> list[int]:
    """The longest upgrade path starting at `root`.

    The pack branches (a unit can upgrade two ways - one per civ), and a line is the family, so the longest branch is
    the one a line is written against. Ties break on the lower id, so the answer does not depend on table order.
    """
    children = _children(edges)
    best: list[int] = [root]

    def walk(unit: int, path: list[int], seen: set[int]) -> None:
        nonlocal best
        if len(path) > len(best) or (len(path) == len(best) and path[-1] < best[-1]):
            best = list(path)
        for child in sorted(children.get(unit, [])):
            if child in seen:
                continue
            walk(child, path + [child], seen | {child})

    walk(root, [root], {root})
    return best


def roots(edges: list[Edge]) -> list[int]:
    """Units nothing upgrades into: the base tier of every line."""
    targets = {edge.to_unit for edge in edges}
    return sorted({edge.from_unit for edge in edges} - targets)


def verify(lines: dict[str, list[str]], roles: dict[str, int], edges: list[Edge]) -> list[str]:
    """Every problem with the authored table, as sentences that name the line and the unit.

    Empty means the table agrees with the pack: each role exists, and each consecutive pair is an upgrade the game can
    actually research.
    """
    pairs = {(edge.from_unit, edge.to_unit): edge.effect for edge in edges}
    problems: list[str] = []
    for name, members in lines.items():
        if not members:
            problems.append(f"line {name!r} is empty")
            continue
        ids: list[int] = []
        for role in members:
            if role not in roles:
                problems.append(f"line {name!r} names role {role!r}, which economy.toml's [role] table does not have")
                ids.append(-1)
                continue
            ids.append(roles[role])
        for before, after, previous, role in zip(ids, ids[1:], members, members[1:]):
            if before == -1 or after == -1:
                continue
            if (before, after) not in pairs:
                problems.append(
                    f"line {name!r} puts {role!r} after {previous!r}, but the pack has no upgrade "
                    f"{before} -> {after}"
                )
    return problems


def derived(effects: dict) -> dict[int, list[int]]:
    """Every line the pack implies, keyed by its base unit id."""
    edges = upgrade_edges(effects)
    return {root: chain_from(root, edges) for root in roots(edges)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check config/lines.toml against the pack's upgrade table.")
    parser.add_argument("--print", action="store_true", help="show the chains the pack implies, then stop")
    args = parser.parse_args(argv)

    if not EFFECTS.exists():
        print(f"{EFFECTS} is missing: run the generator first", file=sys.stderr)
        return 2
    effects = tomllib.loads(EFFECTS.read_text())
    if args.print:
        for root, chain in derived(effects).items():
            print(" ".join(str(unit) for unit in chain))
        return 0

    roles = tomllib.loads(ECONOMY.read_text()).get("role", {})
    lines = tomllib.loads(LINES.read_text()).get("line", {})
    problems = verify(lines, roles, upgrade_edges(effects))
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"{len(lines)} lines agree with the pack")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
