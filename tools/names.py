"""Names: give every unit a readable identifier instead of the dat's five-letter code (D97).

The codes in `units.toml` are not names. They are also not reliable: the record for 283 says `PALDN` and is the
cavalier, 569 says `HEROC` and is the paladin, 7 says `XBOWM` and is the skirmisher. The trustworthy source is the
language string the dat itself points at: every unit carries `name_str_id`, and the install's string table turns that
into "Elite Cataphract".

So the table is **derived and reviewed, not typed**: this tool joins the dat's `name_str_id` to the string table,
normalises the result to a snake_case identifier, and writes `config/names.toml`. The identifiers are ours; the join is
mechanical; a human reads the diff.

Corpses are named by the animal or unit that leaves them (`dead_unit_id`), which is why `scout_corpse` needs no entry of
its own in the dat: 449 is what 448 becomes.

Run:
    uv run python tools/names.py --report          # what would be written, and what could not be named
    uv run python tools/names.py --write           # write config/names.toml
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
CONFIG = ROOT / "config"
DEFAULT_DAT = ROOT / "Age2HD" / "resources" / "_common" / "dat" / "empires2_x2_p1.dat"
DEFAULT_STRINGS = (
    ROOT / "Age2HD" / "resources" / "en" / "strings" / "key-value" / "key-value-strings-utf8.txt"
)
OUT = CONFIG / "names.toml"
# The importer's parser, cloned outside this repository (D4/D12: the generator is not part of this history).
DEFAULT_PARSER = Path("/opt/data/tools/src/GenieDatParser")

# The pile of strings the game uses for "unit name": `5068 "Knight"`.
STRING_LINE = re.compile(r'^(\d+)\s+"(.*)"\s*$')


@dataclass(frozen=True)
class Record:
    """One unit record, reduced to what naming needs."""

    id: int
    code: str
    name_str_id: int
    hp: int
    dead_unit_id: int


def load_strings(path: Path) -> dict[int, str]:
    """The install's key-value string table, id -> text. The file is UTF-8 with a BOM and CRLF line ends."""
    strings: dict[int, str] = {}
    with open(path, encoding="utf-8-sig") as handle:
        for line in handle:
            match = STRING_LINE.match(line.rstrip("\r\n"))
            if match:
                strings[int(match.group(1))] = match.group(2)
    return strings


def slug(text: str) -> str:
    """A stable identifier from a display name: "Elite Cataphract" -> "elite_cataphract".

    `&` becomes `and` rather than disappearing ("Spear & Shield" would otherwise read as "spear shield"), and every run
    of anything else becomes one underscore.
    """
    text = text.lower().replace("&", " and ")
    text = re.sub(r"[^a-z0-9]+", "_", text).strip("_")
    return text


def upgrade_targets(effects_path: Path) -> set[int]:
    """Units an upgrade turns *other* units into: the variants, not the base.

    Which member of a name collision gets the bare word matters. `Town Center` is both 109 (what a match starts with,
    and what every age replaces) and 71 (what it becomes) - and by id alone 71 would take the name and the one the
    engine actually uses would read `town_center_2`. So the unit that is nobody's upgrade target keeps the plain word.
    """
    if not effects_path.exists():
        return set()
    # Read as text rather than through `tomllib`: this tool runs in the importer's environment, whose interpreter is
    # older than 3.11. The generated file writes every command as `[kind, a, b, c, value]`, and kind 3 is "upgrade",
    # so the two numbers after it are the pair.
    text = effects_path.read_text()
    targets = {int(to) for _, to in re.findall(r"\[3, (\d+), (\d+),", text) if int(to) > 0}
    if len(targets) < 100:
        raise SystemExit(f"{effects_path}: only {len(targets)} upgrade targets found - the file's shape has changed")
    return targets


def name_table(
    records: list[Record], strings: dict[int, str], targets: set[int] | None = None
) -> tuple[dict[int, str], list[str]]:
    """`id -> identifier`, plus every problem found while building it.

    Collisions are resolved by suffixing the later id (`arrow`, `arrow_2`), because the game really does carry 54 units
    that all display as "Arrow" - one per archer's projectile - and refusing them all would throw away 54 ids. The
    suffix is by ascending id, so it is stable across runs.
    """
    problems: list[str] = []
    by_id: dict[int, str] = {}
    owner: dict[str, int] = {}
    # A corpse is named after what left it, so it is not named on its own account first: `scout_corpse`, not `scout_d`.
    corpse_of: dict[int, int] = {
        record.dead_unit_id: record.id for record in records if record.dead_unit_id > 0
    }
    # Base units before variants, then by id: the plain word belongs to the unit that is not an upgrade's result.
    targets = targets or set()
    order = sorted(records, key=lambda r: (r.id in targets, r.id))
    for record in order:
        if record.id in corpse_of:
            continue
        text = strings.get(record.name_str_id)
        # 5000 is the sentinel every unit without a name carries: the editor's props, scenery and scenario-only
        # objects. The game has no word for them, so the code is the only stable thing - slugged, and therefore
        # `terrc` rather than `terrain_control`, which is honest about where the word came from.
        if text is None:
            text = " ".join(part for part in record.code.split("_") if part) or record.code
        base = slug(text) or f"unit_{record.id}"
        name = base
        if name in owner:
            suffix = 2
            while f"{base}_{suffix}" in owner:
                suffix += 1
            name = f"{base}_{suffix}"
            problems.append(
                f"unit {record.id}: {text!r} collides with unit {owner[base]}, named {name!r}"
            )
        owner[name] = record.id
        by_id[record.id] = name

    # Corpses, from the thing that leaves them - and overriding a code-derived name if one already exists.
    for record in order:
        if record.dead_unit_id <= 0:
            continue
        base = by_id.get(record.id)
        if base is None:
            continue
        # Two animals can share one body (the boar and the javelina do), and the corpse belongs to whichever living
        # unit comes first by id - the base animal, not its variant.
        if record.dead_unit_id in by_id:
            continue
        name = f"{base}_corpse"
        if name in owner and owner[name] != record.dead_unit_id:
            suffix = 2
            while f"{name}_{suffix}" in owner:
                suffix += 1
            name = f"{name}_{suffix}"
        owner[name] = record.dead_unit_id
        by_id[record.dead_unit_id] = name
    # Anything left (a record that is neither living nor anybody's corpse): the code, or the id.
    for record in order:
        if record.id in by_id:
            continue
        name = slug(record.code) or f"unit_{record.id}"
        if name in owner:
            suffix = 2
            while f"{name}_{suffix}" in owner:
                suffix += 1
            name = f"{name}_{suffix}"
        owner[name] = record.id
        by_id[record.id] = name
    return by_id, problems


def read_dat(dat_path: Path) -> list[Record]:
    """The unit records, via the importer's parser. The dependency is the tool's, not the engine's (D4/D12)."""
    parser = Path(os.environ.get("GENIE_PARSER", DEFAULT_PARSER))
    if not (parser / "src").is_dir():
        raise SystemExit(f"{parser} is not the importer's parser: set GENIE_PARSER to its directory")
    sys.path.insert(0, str(parser / "src"))
    from sections.datfile_sections import DatFile  # noqa: PLC0415

    dat = DatFile.from_file(str(dat_path))
    seen: dict[int, Record] = {}
    for civ in dat.civilizations:
        for unit in civ.units:
            if not unit or unit.id < 0 or unit.id in seen:
                continue
            seen[unit.id] = Record(
                id=unit.id,
                code="".join(unit.name).strip("\x00"),
                name_str_id=unit.name_str_id,
                hp=unit.hit_points,
                dead_unit_id=unit.dead_unit_id,
            )
    return [seen[key] for key in sorted(seen)]


def render(table: dict[int, str]) -> str:
    head = [
        "# Readable names for the units, keyed by the pack's unit id (D97).",
        "#",
        "# Derived from the install's own string table through the dat's `name_str_id` - the codes in `units.toml` are",
        "# neither names nor reliable (283 says PALDN and is the cavalier; 7 says XBOWM and is the skirmisher) - then",
        "# normalised to snake_case. `tools/names.py` rebuilds it; `tools/test_names.py` checks it.",
        "#",
        "# This file is ours and is committed. The tables beside it are extracted game data and stay out of git (D8).",
        "",
        "[unit]",
    ]
    body = [f'{unit_id} = "{name}"' for unit_id, name in sorted(table.items())]
    return "\n".join(head + body) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build config/names.toml from the install's own strings.")
    parser.add_argument("--dat", type=Path, default=DEFAULT_DAT)
    parser.add_argument("--strings", type=Path, default=DEFAULT_STRINGS)
    parser.add_argument("--out", type=Path, default=OUT)
    parser.add_argument("--write", action="store_true", help="write the file instead of only reporting")
    args = parser.parse_args(argv)

    for path, why in ((args.dat, "the game's dat"), (args.strings, "the install's string table")):
        if not path.exists():
            print(f"{path} is missing: {why} is needed to derive names", file=sys.stderr)
            return 2

    records = read_dat(args.dat)
    strings = load_strings(args.strings)
    table, problems = name_table(records, strings, upgrade_targets(CONFIG / "gamedata" / "effects.toml"))

    populated = [record for record in records if record.hp > 0]
    unnamed = [record.id for record in populated if record.id not in table]
    print(f"records {len(records)} | populated {len(populated)} | named {len(table)} | unnamed populated {len(unnamed)}")
    for problem in problems[:10]:
        print(f"  ! {problem}")
    if len(problems) > 10:
        print(f"  ... and {len(problems) - 10} more collisions")
    if unnamed:
        print(f"  unnamed populated ids: {unnamed[:20]}")
    if args.write:
        args.out.write_text(render(table))
        print(f"wrote {args.out} ({len(table)} names)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
