"""Which of the pack's units beats which, computed from the pack's own attack and armour classes.

This is the authoring half of **D91**: the counter relation is arithmetic over the data - `damage_between` already
computes it in the engine - so the numbers a `[[rule]]` is written from are *derived* here rather than remembered from
the game. With **D93** (rules name the unit and the observation that triggers it, no symbolic "counter to"), nothing in
the engine needs this table at runtime yet; that is why it is a tool and not a crate.

It mirrors the engine's formula exactly, including its two awkward parts: a class the target does **not** list
contributes *nothing* (the engine assumes a huge armour value there), and the total floors at `combat.toml`'s
`minimum`. Both are read from `combat.toml`; neither is a literal here.

    python3 tools/counters.py knight spearman pikeman halberdier
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
UNITS = REPO / "config" / "gamedata" / "units.toml"
ECONOMY = REPO / "config" / "economy.toml"
COMBAT = REPO / "config" / "combat.toml"


class UnknownRole(Exception):
    """A role the economy's `[role]` table does not name."""


@dataclass(frozen=True)
class Unit:
    id: int
    name: str
    hp: int
    reload: float
    costs: list
    attacks: list
    armours: list

    def armour_in(self, cls: int, missing: int) -> int:
        for target_class, value in self.armours:
            if target_class == cls:
                return value
        return missing


def damage(attacker: Unit, target: Unit, minimum: int, missing: int) -> int:
    """One hit, class by class, exactly as `damage_between` does it."""
    total = 0
    for cls, amount in attacker.attacks:
        total += max(0, amount - target.armour_in(cls, missing))
    return max(total, minimum)


def dps(attacker: Unit, target: Unit, minimum: int, missing: int) -> float:
    """Damage per second - `reload` is the seconds between blows, so a zero would be a unit that never stops."""
    return damage(attacker, target, minimum, missing) / attacker.reload if attacker.reload else 0.0


def resource_cost(unit: Unit) -> int:
    """The plain sum of what the unit costs. A market makes resources non-equivalent, so this is an author's yardstick,
    not a valuation - the report prints the per-resource split beside it for exactly that reason."""
    return sum(amount for _, amount in unit.costs)


@dataclass(frozen=True)
class Pack:
    units: dict
    roles: dict
    minimum: int
    missing: int

    def unit(self, role: str) -> Unit:
        if role not in self.roles:
            raise UnknownRole(f"{role!r} is not in the economy's [role] table")
        unit_id = self.roles[role]
        if unit_id not in self.units:
            raise UnknownRole(f"role {role!r} names unit {unit_id}, which the pack does not carry")
        return self.units[unit_id]

    def rank_against(self, target_role: str, candidate_roles: list[str]) -> list[tuple[str, float, int, int]]:
        """Candidates ordered by damage per resource second against `target_role`: `(role, dps, damage, cost)`."""
        target = self.unit(target_role)
        rows = []
        for role in candidate_roles:
            attacker = self.unit(role)
            cost = resource_cost(attacker)
            rate = dps(attacker, target, self.minimum, self.missing)
            rows.append((role, rate / cost if cost else 0.0, damage(attacker, target, self.minimum, self.missing), cost))
        rows.sort(key=lambda row: (-row[1], row[0]))
        return rows


def _units(path: Path) -> dict:
    with open(path, "rb") as handle:
        data = tomllib.load(handle)
    units = {}
    for row in data.get("unit", []):
        units[row["id"]] = Unit(
            id=row["id"],
            name=row.get("name", ""),
            hp=row.get("hp", 0),
            reload=row.get("reload", 0.0),
            costs=[tuple(pair) for pair in row.get("costs", [])],
            attacks=[tuple(pair) for pair in row.get("attacks", [])],
            armours=[tuple(pair) for pair in row.get("armours", [])],
        )
    return units


def load(units_path: Path = UNITS, economy_path: Path = ECONOMY, combat_path: Path = COMBAT) -> Pack:
    with open(economy_path, "rb") as handle:
        roles = tomllib.load(handle).get("role", {})
    with open(combat_path, "rb") as handle:
        combat = tomllib.load(handle)
    # The engine reads these two from `combat.toml`; a tool with its own copy would drift from it.
    minimum = int(combat.get("minimum", 1))
    missing = int(combat.get("armour_missing_sentinel", 10_000))
    return Pack(units=_units(units_path), roles=roles, minimum=minimum, missing=missing)


def report(pack: Pack, roles: list[str]) -> None:
    print(f"{'attacker':14s} {'target':14s} {'dmg':>4s} {'dps':>6s} {'hp':>4s} {'cost':>5s} {'dps/cost':>9s}")
    for attacker_role in roles:
        for target_role in roles:
            if attacker_role == target_role:
                continue
            attacker, target = pack.unit(attacker_role), pack.unit(target_role)
            hit = damage(attacker, target, pack.minimum, pack.missing)
            rate = dps(attacker, target, pack.minimum, pack.missing)
            cost = resource_cost(attacker)
            print(
                f"{attacker_role:14s} {target_role:14s} {hit:4d} {rate:6.2f} {attacker.hp:4d} {cost:5d} "
                f"{(rate / cost if cost else 0.0):9.4f}"
            )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Damage and cost efficiency between the pack's units.")
    parser.add_argument("roles", nargs="+", help="roles from the economy's [role] table")
    parser.add_argument("--units", default=str(UNITS))
    parser.add_argument("--economy", default=str(ECONOMY))
    parser.add_argument("--combat", default=str(COMBAT))
    args = parser.parse_args(argv)
    try:
        pack = load(Path(args.units), Path(args.economy), Path(args.combat))
        report(pack, args.roles)
    except (OSError, UnknownRole, tomllib.TOMLDecodeError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
