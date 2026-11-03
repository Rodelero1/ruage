"""Tests for the data gate that stands in front of a test run.

    python3 -m unittest discover -s tools -p 'test_*.py'

The point of the gate is that **absent data must not look like success**, so every case here is about what the run says
rather than about the files themselves.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import require_data

REPO = Path(__file__).resolve().parent.parent


def write(root: Path, name: str, text: str = "x = 1\n") -> None:
    folder = root / "config" / "gamedata"
    folder.mkdir(parents=True, exist_ok=True)
    (folder / name).write_text(text, encoding="utf-8")


class TheGate(unittest.TestCase):
    def test_an_empty_clone_lists_every_required_file(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            absent = require_data.missing(Path(raw))
        self.assertEqual(sorted(absent), sorted(require_data.REQUIRED))

    def test_a_zero_byte_file_counts_as_missing(self) -> None:
        # A failed import leaves the file behind and empty; treating that as "present" is how a green run hides a hole.
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for name in require_data.REQUIRED:
                write(root, name)
            write(root, "units.toml", "")
            absent = require_data.missing(root)
        self.assertEqual(absent, ["units.toml"])

    def test_present_data_passes_and_names_the_folder(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for name in require_data.REQUIRED:
                write(root, name)
            code, line = require_data.check(root, env={})
        self.assertEqual(code, 0)
        self.assertIn("gamedata", line)

    def test_absent_data_fails_loudly_and_says_how_to_opt_out(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            code, line = require_data.check(Path(raw), env={})
        self.assertEqual(code, 1)
        self.assertIn("FAILED", line)
        self.assertIn("units.toml", line)
        self.assertIn(require_data.OPT_OUT, line)

    def test_the_opt_out_passes_but_says_the_run_proves_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            code, line = require_data.check(Path(raw), env={require_data.OPT_OUT: "1"})
        self.assertEqual(code, 0)
        self.assertIn("WARNING", line)
        self.assertIn("proves nothing", line)

    def test_only_one_missing_file_still_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for name in require_data.REQUIRED:
                write(root, name)
            (root / "config" / "gamedata" / "civs.toml").unlink()
            code, line = require_data.check(root, env={})
        self.assertEqual(code, 1)
        self.assertIn("civs.toml", line)
        self.assertNotIn("units.toml", line)


class ThisCheckout(unittest.TestCase):
    def test_this_checkout_has_its_data(self) -> None:
        if require_data.missing(REPO):
            self.skipTest("no generated data here: this machine is not the one the gate guards")
        code, line = require_data.check(REPO, env={})
        self.assertEqual(code, 0, line)


if __name__ == "__main__":
    unittest.main()
