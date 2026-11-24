"""The conformance audit's source patterns must follow the archive move (Phase A).

Research files moved `docs/<name>.md` -> `docs/archive/research/<name>.md`.
A bank row citing the new path is sourced; the audit must say so. Pure unit
tests over the pattern, plus one integration pin on the real bank.

Run: `python3 -m unittest discover -s tools -p 'test_conformance_audit.py' -v`
"""

from __future__ import annotations

import unittest
from pathlib import Path

import conformance_audit as audit

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


class MovedPathsStaySourced(unittest.TestCase):
    def test_archive_research_paths_count_as_sources(self):
        self.assertTrue(
            audit.SOURCE.search("docs/archive/research/movement-in-the-binary.md")
        )
        self.assertTrue(
            audit.SOURCE.search("docs/archive/research/pathfinding-conformance.md")
        )
        self.assertTrue(audit.SOURCE.search("docs/archive/research/oracle.md"))

    def test_root_entry_paths_still_count(self):
        self.assertTrue(audit.SOURCE.search("docs/prior-art.md"))
        self.assertTrue(audit.SOURCE.search("docs/sources.md"))


class BurnDownUnchanged(unittest.TestCase):
    def test_the_real_bank_still_has_no_pending_rows(self):
        text = (ROOT / "docs" / "archive" / "decisions-2026-11-24.md").read_text()
        pending = [f for f in audit.audit(text) if audit.needs_verdict(f)]
        self.assertEqual(pending, [])


if __name__ == "__main__":
    unittest.main()
