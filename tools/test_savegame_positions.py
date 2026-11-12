#!/usr/bin/env python3
"""Tests for `savegame_positions.py`, written before the fix they guard.

Two failures it pins, both found on the owner's own saves (2026-11-13):

1. **A window whose band holds exactly one body crashed.** `ring_fit` took the nearest gap over
   "the others" and `min()` of an empty sequence raises `ValueError`, so a save with one worker on
   the ring produced a traceback instead of a number.
2. **The default band was the artefact band.** It defaulted to 1.05-1.20 - the ring D186 believed
   in - and the owner's bush save shows its men at 0.70 (edge) and 0.94 (corner), with the 1.00
   readings being tile centres. A default that reports the wrong row is worse than no default.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import savegame_positions as sp  # noqa: E402


class RingFit(unittest.TestCase):
    def test_one_body_on_the_ring_is_a_number_and_not_a_crash(self) -> None:
        """A single body in the band: count 1, its radius, and no neighbour to be close to."""
        points = [(0.70, 0.00), (3.10, 0.10)]
        count, mean, closest, nearest = sp.ring_fit(points, (0.0, 0.0), 0.60, 1.00)
        self.assertEqual(count, 1)
        self.assertAlmostEqual(mean, 0.70, places=3)
        self.assertEqual((closest, nearest), (0.0, 0.0))

    def test_no_body_in_the_band_is_zeroes(self) -> None:
        count, mean, closest, nearest = sp.ring_fit([(3.0, 0.0)], (0.0, 0.0), 0.60, 1.00)
        self.assertEqual((count, mean, closest, nearest), (0, 0.0, 0.0, 0.0))

    def test_two_bodies_report_their_true_gap(self) -> None:
        points = [(0.70, 0.0), (0.0, 0.70)]
        count, mean, closest, nearest = sp.ring_fit(points, (0.0, 0.0), 0.60, 1.00)
        self.assertEqual(count, 2)
        self.assertAlmostEqual(mean, 0.70, places=3)
        self.assertAlmostEqual(closest, math.hypot(0.70, 0.70), places=3)

    def test_the_default_band_is_the_touching_band(self) -> None:
        """The owner's saves put workers at 0.70 and 0.94; 1.00 is a tile centre, not a worker."""
        band = sp.DEFAULT_BAND
        for radius, expected in ((0.70, True), (0.76, True), (0.94, True), (1.00, False), (1.156, False)):
            self.assertEqual(band[0] <= radius <= band[1], expected, (radius, band))

    def test_a_body_at_a_corner_is_outside_the_edge_but_inside_the_band(self) -> None:
        """0.5*sqrt(2) + 0.2 = 0.907, and the bush save measures 0.936 there."""
        count, mean, _, _ = sp.ring_fit([(0.662, 0.662)], (0.0, 0.0), 0.60, 1.00)
        self.assertEqual(count, 1)
        self.assertAlmostEqual(mean, math.hypot(0.662, 0.662), places=3)


if __name__ == "__main__":
    unittest.main()
