"""The vocabulary gate has to be provable: a tool that decides whether a pack can go name-keyed, and that nothing tested, is a
tool that can silently measure the wrong column - which is exactly what it did once (it read the dat's five-letter codes and
reported "56 unnamed, 28 duplicated" while `names.toml` held a unique key for every unit).

Each test builds a **tiny pack** in a temp directory and runs the tool against it with `--root`, so the real config is never
touched and a change to the tool cannot be "verified" by the pack it was written for.

Run: `pytest tools/test_vocabulary.py`
"""

from __future__ import annotations

import subprocess
import sys
import textwrap
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL = HERE / "vocabulary.py"


def pack(root: Path, *, names: dict[int, str], units: list[tuple[int, int]], roles: dict[str, int],
         commands: list[list[int]] | None = None) -> Path:
    """A pack with `units` as (id, name_str_id) pairs, `roles` as role -> unit id, and one effect carrying `commands`."""
    (root / "config" / "gamedata").mkdir(parents=True)
    names_toml = "\n".join(f'{uid} = "{name}"' for uid, name in names.items())
    (root / "config" / "names.toml").write_text(f"[unit]\n{names_toml}\n")
    body = "\n".join(
        f'[[unit]]\nid = {uid}\nname = "CODE{uid}"\n' for uid, _ in units
    )
    (root / "config" / "gamedata" / "units.toml").write_text(body)
    effects = ""
    if commands:
        effects = "[[effect]]\nid = 1\nname = \"E\"\ncommands = [" + ", ".join(
            f"[{a}, {b}, {c}]" for a, b, c in commands
        ) + "]\n"
    (root / "config" / "gamedata" / "effects.toml").write_text(effects)
    role_lines = "\n".join(f'{name} = {uid}' for name, uid in roles.items())
    (root / "config" / "economy.toml").write_text(f"[role]\n{role_lines}\n")
    return root


def run(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(TOOL), "--root", str(root)],
        capture_output=True,
        text=True,
        check=False,
    )


def test_a_named_pack_is_ready(tmp_path: Path) -> None:
    pack(tmp_path, names={1: "villager", 2: "farm"}, units=[(1, 5121), (2, 5149)],
         roles={"villager": 1, "farm": 2})
    result = run(tmp_path)
    assert result.returncode == 0, result.stdout + result.stderr
    assert "units with no key: 0" in result.stdout
    assert "verdict: ready" in result.stdout


def test_a_unit_with_no_key_blocks_the_pack(tmp_path: Path) -> None:
    pack(tmp_path, names={1: "villager"}, units=[(1, 5121), (2, 5149)], roles={"villager": 1})
    result = run(tmp_path)
    assert result.returncode == 1
    assert "units with no key: 1" in result.stdout


def test_two_units_sharing_a_key_block_the_pack(tmp_path: Path) -> None:
    # The real pack has four units whose string is "BARRICADE"; a key-keyed schema needs one key per unit.
    pack(tmp_path, names={1: "barricade", 2: "barricade"}, units=[(1, 1), (2, 2)], roles={})
    result = run(tmp_path)
    assert result.returncode == 1
    assert "duplicate keys: 1" in result.stdout


def test_a_role_naming_a_missing_unit_blocks_the_pack(tmp_path: Path) -> None:
    pack(tmp_path, names={1: "villager"}, units=[(1, 5121)], roles={"villager": 1, "ghost": 99})
    result = run(tmp_path)
    assert result.returncode == 1
    assert "naming a unit the table lacks: 1" in result.stdout


def test_a_cycle_in_the_upgrade_graph_blocks_the_pack(tmp_path: Path) -> None:
    # Two units upgrading into each other, as the fishing ships really do in the dat.
    pack(tmp_path, names={1: "a", 2: "b"}, units=[(1, 1), (2, 2)], roles={},
         commands=[[3, 1, 2], [3, 2, 1]])
    result = run(tmp_path)
    assert result.returncode == 1
    assert "units on a cycle: 2" in result.stdout


def test_the_tool_reads_names_not_the_dat_codes(tmp_path: Path) -> None:
    """The mistake that made the first inventory wrong: a unit whose code is unreadable but whose key is fine must pass."""
    pack(tmp_path, names={1: "villager"}, units=[(1, 5121)], roles={})
    (tmp_path / "config" / "gamedata" / "units.toml").write_text(
        textwrap.dedent(
            """\
            [[unit]]
            id = 1
            name = "VMBAS"
            """
        )
    )
    result = run(tmp_path)
    assert result.returncode == 0, result.stdout
    assert "units with no key: 0" in result.stdout
