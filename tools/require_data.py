#!/usr/bin/env python3
"""Refuse to call a test run meaningful when the generated data is absent.

Every suite in `crates/` returns early, quietly, when `config/gamedata/` is missing. That politeness is a decision (D8: a
stranger's clone must build and run without the game's data), and it has a price that nobody priced until it bit: `cargo
test` on a worktree without the generated files is **green while proving nothing**. Forty real failures hid behind that for
a day, and the way they were found was a fresh worktree with the data copied in.

So the gate runs this first. Absent data is a loud failure; a machine that genuinely has no install opts out, on purpose,
with `RUAGE_ALLOW_NO_DATA=1`, and is told what the run is worth.

    python3 tools/require_data.py            # exit 1 when the generated data is missing
    RUAGE_ALLOW_NO_DATA=1 python3 tools/require_data.py   # exit 0, with a warning
"""

from __future__ import annotations

import os
import sys
from collections.abc import Mapping
from pathlib import Path

# What the importer writes and every suite reads. All four, because a suite that needs only one of them still skips when
# that one is absent, and "half the data" is the state that produces a green run with holes in it.
REQUIRED = ("units.toml", "techs.toml", "effects.toml", "civs.toml")

OPT_OUT = "RUAGE_ALLOW_NO_DATA"


def missing(root: Path) -> list[str]:
    """The generated files that are absent or empty under `root/config/gamedata`."""
    folder = root / "config" / "gamedata"
    return [
        name
        for name in REQUIRED
        if not (folder / name).is_file() or (folder / name).stat().st_size == 0
    ]


def check(root: Path, env: Mapping[str, str] | None = None) -> tuple[int, str]:
    """The exit code and the line to say. Pure, so the test can assert the wording as well as the code."""
    source: Mapping[str, str] = os.environ if env is None else env
    absent = missing(root)
    folder = root / "config" / "gamedata"
    if not absent:
        return 0, f"require-data: {len(REQUIRED)} generated files present in {folder}"
    named = ", ".join(absent)
    if source.get(OPT_OUT) == "1":
        return 0, (
            f"require-data: WARNING - {named} absent, and {OPT_OUT}=1 is set. The suite will run and every "
            "data-driven test will skip itself: a green result here proves nothing about the sim."
        )
    return 1, (
        f"require-data: FAILED - {named} absent from {folder}.\n"
        "  The suite would skip every data-driven test and report success. Generate the data first:\n"
        "    python3 tools/importer.py   (or copy config/gamedata/ from a machine that has the install)\n"
        f"  On a machine with no game install, say so out loud: {OPT_OUT}=1 make test"
    )


def main(argv: list[str]) -> int:
    root = Path(argv[1]).resolve() if len(argv) > 1 else Path(__file__).resolve().parent.parent
    code, line = check(root)
    print(line, file=sys.stderr if code else sys.stdout)
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv))
