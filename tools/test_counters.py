"""Tests for the counter report.

The tool exists so a `[[rule]]` that answers knights with pikemen can be *written* from numbers instead of memory
(D91). Its arithmetic has to be the engine's, not a second opinion, so the first test pins the formula against
`damage_between`'s own rules: each attack class minus the target's armour in that class, negatives count zero, a class
the target does not list contributes nothing, and the total floors at `minimum`.

    python3 -m unittest discover -s tools -p 'test_*.py'
"""

from __future__ import annotations

import unittest
from pathlib import Path

import counters

REPO = Path(__file__).resolve().parent.parent
UNITS = REPO / "config" / "gamedata" / "units.toml"
ECONOMY = REPO / "config" / "economy.toml"
COMBAT = REPO / "config" / "combat.toml"

KNIGHT = counters.Unit(id=38, name="KNGHT", hp=100, reload=1.8, costs=[(0, 60), (3, 75)],
                       attacks=[(4, 10), (15, 0), (11, 0), (21, 0)], armours=[(4, 2), (8, 0), (3, 2)])
PIKEMAN = counters.Unit(id=93, name="PKEMN", hp=45, reload=3.0, costs=[(0, 35), (1, 25)],
                        attacks=[(29, 1), (21, 1), (5, 15), (4, 3), (8, 15), (16, 9), (30, 12), (35, 4), (34, 9)],
                        armours=[(27, 0), (1, 0), (4, 0), (3, 0)])


class FormulaTests(unittest.TestCase):
    def test_a_bonus_class_the_target_lists_lands_in_full(self):
        # Pikeman: 3 military vs the knight's 2 melee armour => 1, plus 15 bonus vs the cavalry class the knight lists
        # at 0 => 16. Every other class the knight does not list contributes nothing.
        self.assertEqual(counters.damage(PIKEMAN, KNIGHT, minimum=1, missing=10_000), 16)

    def test_the_return_blow_is_computed_the_same_way(self):
        self.assertEqual(counters.damage(KNIGHT, PIKEMAN, minimum=1, missing=10_000), 10)

    def test_a_class_the_target_does_not_list_contributes_nothing(self):
        # The engine assumes a huge armour value for an unlisted class, so `amount - 10000` floors at zero. Getting
        # this backwards would make every unit a counter to every other one.
        attacker = counters.Unit(id=1, name="X", hp=1, reload=1.0, costs=[], attacks=[(99, 50)], armours=[])
        self.assertEqual(counters.damage(attacker, KNIGHT, minimum=1, missing=10_000), 1)

    def test_the_total_floors_at_the_configured_minimum(self):
        weak = counters.Unit(id=2, name="W", hp=1, reload=1.0, costs=[], attacks=[(4, 2)], armours=[])
        self.assertEqual(counters.damage(weak, KNIGHT, minimum=1, missing=10_000), 1)

    def test_damage_per_second_uses_the_attacker_reload(self):
        self.assertAlmostEqual(counters.dps(PIKEMAN, KNIGHT, minimum=1, missing=10_000), 16 / 3.0, places=6)

    def test_cost_is_the_sum_of_the_resources_it_takes(self):
        self.assertEqual(counters.resource_cost(PIKEMAN), 60)
        self.assertEqual(counters.resource_cost(KNIGHT), 135)


@unittest.skipUnless(UNITS.exists() and ECONOMY.exists() and COMBAT.exists(),
                     "no content pack: the generated units table is not committed (D8)")
class RealPackTests(unittest.TestCase):
    def setUp(self):
        self.pack = counters.load(UNITS, ECONOMY, COMBAT)

    def test_the_formula_agrees_with_the_pack(self):
        # The pack's `pikeman` role is unit **358**, the upgraded pikeman, whose cavalry bonus is **22** - not the 15 a
        # spearman (93) carries, which is what the synthetic fixture above uses. The first version of this test asserted
        # 16 from the spearman's row and was wrong; the tool was right.
        knight = self.pack.unit("knight")
        pikeman = self.pack.unit("pikeman")
        self.assertEqual(counters.damage(pikeman, knight, self.pack.minimum, self.pack.missing), 24)
        self.assertEqual(counters.damage(knight, pikeman, self.pack.minimum, self.pack.missing), 10)

    def test_the_counter_to_a_knight_is_a_pikeman_line_unit(self):
        # The check that the tool says something true about the game: the best infantry answer to a knight is the
        # spearman line, by damage per resource - not the militia line, which has the same role in the pack.
        ranked = self.pack.rank_against("knight", ["militia", "spearman", "pikeman", "halberdier"])
        self.assertIn(ranked[0][0], {"pikeman", "halberdier", "spearman"})
        self.assertNotEqual(ranked[0][0], "militia")

    def test_a_missing_role_is_named_rather_than_skipped(self):
        with self.assertRaises(counters.UnknownRole):
            self.pack.unit("cataphract_of_doom")


if __name__ == "__main__":
    unittest.main()
