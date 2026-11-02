"""Names: the readable identifiers, derived from the game's own strings (D97).

The table is generated, so these tests are about the two things generation gets wrong: the sentinel and the corpse.
"""

from __future__ import annotations

import re
import tomllib
import unittest
from pathlib import Path

import names

CONFIG = Path(__file__).resolve().parent.parent / "config"
SNAKE = re.compile(r"^[a-z][a-z0-9_]*$")

# A synthetic pack: id 1 has a name, id 2 is the sentinel (no name), id 3 shares id 1's name, id 9 is id 1's corpse.
STRINGS = {5068: "Knight", 5070: "Cavalier"}
RECORDS = [
    names.Record(id=1, code="KNGHT", name_str_id=5068, hp=100, dead_unit_id=9),
    names.Record(id=2, code="TERRC", name_str_id=5000, hp=1, dead_unit_id=-1),
    names.Record(id=3, code="PALDN", name_str_id=5068, hp=120, dead_unit_id=-1),
    names.Record(id=9, code="KNGHT_D", name_str_id=0, hp=0, dead_unit_id=-1),
]


class SlugTests(unittest.TestCase):
    def test_a_display_name_becomes_snake_case(self):
        self.assertEqual(names.slug("Elite Cataphract"), "elite_cataphract")
        self.assertEqual(names.slug("Man-at-Arms"), "man_at_arms")
        self.assertEqual(names.slug("Two-Handed Swordsman"), "two_handed_swordsman")

    def test_an_ampersand_reads_as_and_rather_than_disappearing(self):
        self.assertEqual(names.slug("Spear & Shield"), "spear_and_shield")

    def test_nothing_survives_as_punctuation_or_a_leading_underscore(self):
        self.assertEqual(names.slug("  (Elite) Skirmisher!  "), "elite_skirmisher")


class TableTests(unittest.TestCase):
    def setUp(self):
        self.table, self.problems = names.name_table(RECORDS, STRINGS)

    def test_a_named_unit_takes_the_games_word_for_it(self):
        self.assertEqual(self.table[1], "knight")
        self.assertEqual(self.table[3], "knight_2", "the second unit with that name is numbered")

    def test_a_unit_the_game_has_no_name_for_falls_back_to_its_code(self):
        self.assertEqual(self.table[2], "terrc", "5000 is the sentinel, not a name")

    def test_a_corpse_is_named_after_what_left_it(self):
        self.assertEqual(self.table[9], "knight_corpse", "not 'knght_d', which is what the dat calls it")

    def test_a_collision_is_reported_rather_than_hidden(self):
        self.assertTrue(self.problems, "the second 'Knight' is worth a line in the report")

    def test_the_base_unit_keeps_the_plain_word_and_its_variant_is_suffixed(self):
        # `Town Center` is 71 (what an age turns the centre into) and 109 (the one a match starts with). By id alone 71
        # would take the name and the unit the engine actually uses would read `town_center_2` - backwards.
        pack = [
            names.Record(id=71, code="RTWC3", name_str_id=6001, hp=2400, dead_unit_id=-1),
            names.Record(id=109, code="RTWC", name_str_id=6001, hp=2400, dead_unit_id=-1),
        ]
        table, _ = names.name_table(pack, {6001: "Town Center"}, targets={71})
        self.assertEqual(table[109], "town_center", "the base unit is not the upgrade's result")
        self.assertEqual(table[71], "town_center_2")

    def test_two_animals_sharing_one_body_name_it_after_the_first(self):
        shared = [
            names.Record(id=5, code="BOARX", name_str_id=6001, hp=75, dead_unit_id=7),
            names.Record(id=6, code="JAVEL", name_str_id=6002, hp=50, dead_unit_id=7),
        ]
        table, _ = names.name_table(shared, {6001: "Wild Boar", 6002: "Javelina"})
        self.assertEqual(table[7], "wild_boar_corpse", "the base animal names the body, not its variant")


class ProjectNamesTests(unittest.TestCase):
    """The repository's own table. Skipped when the pack has not been generated."""

    def setUp(self):
        path = CONFIG / "names.toml"
        if not path.exists():
            self.skipTest("config/names.toml has not been generated")
        self.table = {int(k): v for k, v in tomllib.loads(path.read_text())["unit"].items()}
        units = CONFIG / "gamedata" / "units.toml"
        if not units.exists():
            self.skipTest("no generated units to check against")
        self.units = {unit["id"]: unit for unit in tomllib.loads(units.read_text())["unit"]}

    def test_every_unit_has_a_name(self):
        missing = sorted(set(self.units) - set(self.table))
        self.assertEqual(missing, [], f"units with no name: {missing[:10]}")

    def test_no_unit_has_a_name_that_is_not_an_identifier(self):
        bad = {i: n for i, n in self.table.items() if not SNAKE.match(n)}
        self.assertEqual(bad, {}, f"not snake_case: {list(bad.items())[:5]}")

    def test_names_are_unique_because_they_become_the_vocabulary(self):
        seen: dict[str, int] = {}
        for unit_id, name in sorted(self.table.items()):
            self.assertNotIn(name, seen, f"units {seen.get(name)} and {unit_id} are both {name!r}")
            seen[name] = unit_id

    def test_the_shifted_codes_are_not_repeated(self):
        # The dat says PALDN at 283 (the cavalier) and HEROC at 569 (the paladin). These four assertions are what the
        # whole derivation exists for: the code is not the name.
        self.assertEqual(self.table[283], "cavalier")
        self.assertEqual(self.table[569], "paladin")
        self.assertEqual(self.table[7], "skirmisher")
        self.assertEqual(self.table[359], "halberdier")

    def test_a_corpse_is_named_for_its_living_unit(self):
        self.assertEqual(self.table[449], "scout_cavalry_corpse")
        self.assertEqual(self.table[595], "sheep_corpse")


if __name__ == "__main__":
    unittest.main()
