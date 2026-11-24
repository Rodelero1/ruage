"""The decision index's own tests (D355).

The page exists so a session stops re-deciding what the bank already holds, so the things worth pinning are the ones that would
make it *lie*: a row filed in the wrong area, a claim taken from the middle of an argument, a verdict read off a sentence that
is not a verdict, a fixture column quoting a filename instead of a test.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import decision_index as index  # noqa: E402

SAMPLE = """
## D1 - The first ruling

**Conformance**: GAME - `FUN_004aaea0`, measured.

The body explains what was decided and why it took a slice.

## D2 - A choice of ours

**Conformance**: OURS - the UI issues orders.

## D3 - The third ruling

**Fixture**: a_nearer_wall_wins_over_a_nearer_anchor (`crates/sim/tests/gather.rs`), written first.

Superseded by D4.

## D4 - The fourth ruling
"""


class ParseTests(unittest.TestCase):
    def setUp(self) -> None:
        self.entries, self.superseded_by, self.supersedes = index.parse(SAMPLE)
        self.by_id = {entry["id"]: entry for entry in self.entries}

    def test_every_row_is_read(self) -> None:
        self.assertEqual(sorted(self.by_id), ["D1", "D2", "D3", "D4"])

    def test_the_title_is_the_line(self) -> None:
        # A body often opens mid-argument; the heading is the claim a scanning reader needs.
        self.assertEqual(self.by_id["D1"]["claim"], "The first ruling")

    def test_the_verdict_is_read_from_the_conformance_line_only(self) -> None:
        self.assertEqual(self.by_id["D1"]["kind"], "GAME")
        self.assertEqual(self.by_id["D2"]["kind"], "OURS")
        self.assertIsNone(self.by_id["D3"]["kind"])

    def test_a_declared_fixture_wins_over_a_filename(self) -> None:
        # The path says `gather`; the test is `a_nearer_wall_wins_over_a_nearer_anchor`.
        self.assertEqual(self.by_id["D3"]["fixtures"], ["a_nearer_wall_wins_over_a_nearer_anchor"])

    def test_a_supersession_is_read_both_ways(self) -> None:
        self.assertEqual(self.superseded_by, {"D3": "D4"})

    def test_area_is_scored_not_first_match(self) -> None:
        # "body" and "step" are common prose words: a row about gathering must not be filed under movement because of them,
        # which is what first-match filing did to 261 of 340 rows.
        title = "Gathering wood at a drop site"
        body = "the body of the man takes a step"
        self.assertEqual(index.area_of(title, body), "Economy & gathering")


if __name__ == "__main__":
    unittest.main()
