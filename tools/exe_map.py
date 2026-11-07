"""The install's own module map: source paths and RTTI classes inside `AoK HD.exe` (D140).

**Why this exists.** `docs/movement-in-the-binary.md` built the movement map from the decompiled corpus's embedded assert
strings. That corpus was produced by a toolchain on this box; the strings it carries come from *the binary we own*. This
tool reads them out of the install directly, so the evidence is ours end to end and the map can be rebuilt without the
corpus at all.

What it extracts, and what each is good for:

- **source paths** (`...\\aoc\\source\\path.cpp`) — the original's own module list, including the build root
  (`age_of_empires_2_de\\phoenix\\age2hd_archive_5.7_with_hotfix`), which is provenance: HD 5.7 was cut from the DE tree.
- **RTTI type descriptors** (`.?AVCommPacketSyncTurn@@`) — the class taxonomy. This is the part the source paths cannot
  give: MSVC emits one descriptor per polymorphic class, so the netplay family names its own packets.

Neither is code and neither is decompiled: a class name is a fact about the program, the same kind of fact as a dat field
name. Nothing here is copied from another project (see `docs/licensing.md`).

Run:
    uv run python tools/exe_map.py                  # counts, families, and the Comm* list
    uv run python tools/exe_map.py --json           # the same report, machine-readable
    uv run python tools/exe_map.py --write          # docs/binary-class-map.md
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DEFAULT_EXE = ROOT / "Age2HD" / "AoK HD.exe"
OUT = ROOT / "docs" / "binary-class-map.md"

# A run of printable ASCII, the way `strings(1)` finds one. Six is deliberate: shorter runs are noise from the loader, and
# the shortest source file name we care about (`path.cpp`) is eight.
MIN_RUN = 6

# `c:\users\...\aoc\source\path.cpp` - a build path, not a file name mentioned in prose. The `\\source\\` component is what
# separates the two: the exe also carries extension and font paths that end in `.h`-looking fragments.
SOURCE_PATH = re.compile(r"^[a-zA-Z]:\\.*\\source\\.*\.(?:cpp|h|c)$", re.IGNORECASE)

# MSVC RTTI: `. ?AV<name>@@` for a class, `. ?AU<name>@@` for a struct. Both are `TypeDescriptor` strings.
RTTI = re.compile(r"^\.\?A([VU])([A-Za-z0-9_@]+)@@$")

# One run of printable bytes, at least `MIN_RUN` long: the whole of `strings_of`, in one pass of the C engine.
STRING_RUN = re.compile(rb"[\x20-\x7e]{" + str(MIN_RUN).encode() + rb",}")


def strings_of(blob: bytes) -> list[str]:
    """Every run of printable ASCII of at least `MIN_RUN` characters, in file order.

    A regex over the raw bytes, not a byte-at-a-time loop: the loop was the obvious way to write it and measured **535 ms**
    on the 7.5 MB exe against **123 ms** here for the identical 16,666 strings. The pattern is `strings(1)`'s own rule,
    so an assert-string, a path and a class name all come out the same way in both.
    """
    return [match.group(0).decode("ascii") for match in STRING_RUN.finditer(blob)]


@dataclass
class Map:
    """What the binary says about itself, deduplicated."""

    paths: list[str] = field(default_factory=list)
    classes: list[str] = field(default_factory=list)
    structs: list[str] = field(default_factory=list)

    @property
    def roots(self) -> list[str]:
        """The build roots the paths share, longest common prefix per drive, so provenance is visible at a glance."""
        seen: Counter[str] = Counter()
        for path in self.paths:
            # `c:\users\x\documents\projects\age_of_empires_2_de\phoenix\age2hd_archive_5.7_with_hotfix\aoc\source\...`
            marker = "\\aoc\\source\\"
            at = path.lower().find(marker)
            seen[path[:at] if at > 0 else path] += 1
        return sorted(seen)

    def families(self, limit: int = 20) -> list[tuple[str, int]]:
        """Classes grouped by their leading word: `Comm` 32, `TRIBE_` 50, `RGE_Ac` 17."""
        counts: Counter[str] = Counter()
        for name in self.classes:
            match = re.match(r"([A-Z][a-z]+|[A-Z]+_)", name)
            counts[match.group(1) if match else name[:6]] += 1
        return counts.most_common(limit)

    def of_family(self, prefix: str) -> list[str]:
        """Classes whose name **starts** with the family word. The next character has to be a capital, or `Comm` swallows
        `CommandLineHandler` - a command-line parser is not a netplay packet, and a list that claims 32 netplay classes
        when it has 31 is the kind of number a later reader trusts."""
        return [
            name
            for name in self.classes
            if name.startswith(prefix)
            and (len(name) == len(prefix) or name[len(prefix)].isupper())
        ]


def read(exe: Path) -> Map:
    """The map of one binary. Raises `FileNotFoundError` rather than returning an empty map for a missing file: an empty
    map and "the file is not there" must never look the same, which is the D130 lesson applied to a tool."""
    blob = exe.read_bytes()
    found = Map()
    seen_paths: set[str] = set()
    classes: set[str] = set()
    structs: set[str] = set()
    for line in strings_of(blob):
        if SOURCE_PATH.match(line):
            key = line.lower()
            if key not in seen_paths:
                seen_paths.add(key)
                found.paths.append(line)
            continue
        match = RTTI.match(line)
        if match:
            kind, name = match.groups()
            (classes if kind == "V" else structs).add(name)
    found.paths.sort()
    found.classes = sorted(classes)
    found.structs = sorted(structs)
    return found


def report(found: Map, exe: Path) -> dict[str, Any]:
    return {
        "exe": exe.name,
        "bytes": exe.stat().st_size,
        "source_paths": len(found.paths),
        "build_roots": found.roots,
        "classes": len(found.classes),
        "structs": len(found.structs),
        "families": found.families(),
        "netplay": found.of_family("Comm"),
        "simulation": found.of_family("TRIBE_"),
    }


def markdown(found: Map, exe: Path) -> str:
    summary = report(found, exe)
    lines = [
        "# What the install's own binary says about its modules",
        "",
        "*Generated by `tools/exe_map.py` from `Age2HD/AoK HD.exe`; do not hand-edit. The evidence is a file this project",
        "owns, read at the string level: no code is decompiled and nothing is copied from another project (see",
        "`docs/licensing.md`).*",
        "",
        f"**{summary['bytes']:,} bytes.** {summary['source_paths']} source paths, {summary['classes']} classes,",
        f"{summary['structs']} structs.",
        "",
        "## The build roots",
        "",
        "Every path is an absolute build path, so the tree the exe was compiled from is in the file:",
        "",
    ]
    lines += [f"- `{root}`" for root in summary["build_roots"]]
    lines += ["", "## The class families", "", "| family | classes |", "|---|---|"]
    lines += [f"| `{name}` | {count} |" for name, count in summary["families"]]
    lines += [
        "",
        "## The netplay family, in full",
        "",
        "This is the anchor the questions registry has been missing: the packets that make a lockstep match name",
        "themselves in the binary.",
        "",
    ]
    lines += [f"- `{name}`" for name in summary["netplay"]]
    lines += [
        "",
        "## The simulation family, in full",
        "",
    ]
    lines += [f"- `{name}`" for name in summary["simulation"]]
    lines += ["", "## Every source path", ""]
    lines += [f"- `{path}`" for path in found.paths]
    lines += [""]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--exe", type=Path, default=DEFAULT_EXE, help="the install's executable")
    parser.add_argument("--json", action="store_true", help="print the report as JSON")
    parser.add_argument("--write", action="store_true", help=f"write {OUT.relative_to(ROOT)}")
    args = parser.parse_args(argv)

    try:
        found = read(args.exe)
    except FileNotFoundError:
        print(f"no executable at {args.exe}: the map needs the install (D8)", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(report(found, args.exe), indent=2))
        return 0

    summary = report(found, args.exe)
    print(
        f"{summary['exe']}: {summary['source_paths']} source paths, {summary['classes']} classes, "
        f"{summary['structs']} structs"
    )
    for root in summary["build_roots"]:
        print(f"  root: {root}")
    print("  families: " + ", ".join(f"{name} {count}" for name, count in summary["families"][:10]))
    print(f"  netplay: {len(summary['netplay'])} Comm* classes")

    if args.write:
        OUT.write_text(markdown(found, args.exe))
        print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
