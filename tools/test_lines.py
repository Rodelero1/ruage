"""Lines: the unit families an AI names instead of units (D95).

A line is an ordered list of roles, weakest first - `knight -> cavalier -> paladin` - so "the best one I can field"
is the last member the civ can actually train. The table is authored because D95 says the AI names lines and a civ
override decides what answers it, but it must not drift from the pack: the upgrade edges are in the effect table, and
every authored pair is checked against them here.

Run: `python3 -m unittest tools.test_lines -v`
"""

from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

import lines

HERE = Path(__file__).resolve().parent
# Every unit id the synthetic pack "carries": the roles above, plus the units the edges name.
UNITS = {100, 101, 102, 200, 201, 999}
CONFIG = HERE.parent / "config"

# A synthetic effect table: kind 3 is "upgrade unit", and an upgrade edge is (effect name, from, to).
EFFECTS = {
    "effect": [
        {"id": 1, "name": "Cavalier", "commands": [[3, 100, 101, -1, -1.0]]},
        {"id": 2, "name": "Paladin", "commands": [[3, 101, 102, -1, -1.0]]},
        {"id": 3, "name": "Chemistry", "commands": [[3, 200, 201, -1, -1.0]]},
        {"id": 4, "name": "Non-Research", "commands": [[3, 100, 999, -1, -1.0]]},
        {"id": 5, "name": "Something else", "commands": [[4, 100, 500, -1, -1.0]]},
    ]
}


class EdgeTests(unittest.TestCase):
    def test_an_upgrade_command_is_an_edge_and_nothing_else_is(self):
        edges = lines.upgrade_edges(EFFECTS)
        self.assertEqual([(e.from_unit, e.to_unit) for e in edges], [(100, 101), (101, 102), (200, 201)])

    def test_non_research_edges_are_dropped(self):
        # The effect table carries "Non-Research" upgrades (a free unit swap, not a tech a civ can buy). A line that
        # contained one would claim a tier nobody can research.
        edges = lines.upgrade_edges(EFFECTS)
        self.assertNotIn(999, [e.to_unit for e in edges])

    def test_a_chain_follows_edges_from_its_root(self):
        chain = lines.chain_from(100, lines.upgrade_edges(EFFECTS))
        self.assertEqual(chain, [100, 101, 102])

    def test_a_branch_picks_the_longest_path(self):
        branched = {
            "effect": [
                {"id": 1, "name": "A", "commands": [[3, 1, 2, -1, -1.0]]},
                {"id": 2, "name": "B", "commands": [[3, 1, 3, -1, -1.0]]},
                {"id": 3, "name": "C", "commands": [[3, 3, 4, -1, -1.0]]},
            ]
        }
        self.assertEqual(lines.chain_from(1, lines.upgrade_edges(branched)), [1, 3, 4])


class TieBreakTests(unittest.TestCase):
    """A branch with two equal-length paths must resolve the same way every time (M2 survived without these)."""

    # Two paths of the same length whose *ends* differ: the first-explored branch ends at 9, the second at 4. A fixture
    # whose ends are already in ascending order cannot tell the tie-break from the traversal order (the first version of
    # this test did exactly that, and the mutant survived it).
    TWIN = {
        "effect": [
            {"id": 1, "name": "A", "commands": [[3, 1, 2, -1, -1.0], [3, 2, 9, -1, -1.0]]},
            {"id": 2, "name": "B", "commands": [[3, 1, 3, -1, -1.0], [3, 3, 4, -1, -1.0]]},
        ]
    }

    def test_equal_lengths_pick_the_lower_ending_unit(self):
        self.assertEqual(lines.chain_from(1, lines.upgrade_edges(self.TWIN)), [1, 3, 4])

    def test_the_answer_does_not_depend_on_table_order(self):
        forwards = lines.chain_from(1, lines.upgrade_edges(self.TWIN))
        backwards = lines.chain_from(1, lines.upgrade_edges({"effect": list(reversed(self.TWIN["effect"]))}))
        self.assertEqual(forwards, backwards, "the table's order is not the line's order")


class VerifyTests(unittest.TestCase):
    ROLES = {"knight": 100, "cavalier": 101, "paladin": 102, "spearman": 200, "pikeman": 201}
    LINES_OK = {"knight": ["knight", "cavalier", "paladin"], "spear": ["spearman", "pikeman"]}

    def test_a_line_that_matches_the_pack_is_accepted(self):
        self.assertEqual(lines.verify(self.LINES_OK, self.ROLES, lines.upgrade_edges(EFFECTS), UNITS), [])

    def test_a_line_that_skips_a_tier_is_refused(self):
        broken = {"knight": ["knight", "paladin"]}
        problems = lines.verify(broken, self.ROLES, lines.upgrade_edges(EFFECTS), UNITS)
        self.assertEqual(len(problems), 1)
        self.assertIn("knight", problems[0])
        self.assertIn("paladin", problems[0])

    def test_a_line_in_the_wrong_order_is_refused(self):
        backwards = {"knight": ["paladin", "cavalier"]}
        problems = lines.verify(backwards, self.ROLES, lines.upgrade_edges(EFFECTS), UNITS)
        self.assertEqual(len(problems), 1)
        # The message names both sides, so a reader can fix the file without counting ids.
        self.assertIn("paladin", problems[0])
        self.assertIn("cavalier", problems[0])

    def test_a_role_the_economy_does_not_have_is_refused_by_name(self):
        unknown = {"knight": ["knight", "unicorn"]}
        problems = lines.verify(unknown, self.ROLES, lines.upgrade_edges(EFFECTS), UNITS)
        self.assertEqual(len(problems), 1)
        self.assertIn("unicorn", problems[0])

    def test_a_line_of_one_unit_is_accepted(self):
        # A single-tier line is legal: a civ with nothing beyond the base unit still has a line for it.
        self.assertEqual(lines.verify({"knight": ["knight"]}, self.ROLES, lines.upgrade_edges(EFFECTS), UNITS), [])

    def test_a_role_naming_a_unit_the_pack_lacks_is_refused(self):
        # The hole the review found: a one-tier line skipped every upgrade check, so a role could name an id the pack
        # does not carry and `verify` still said "agrees with the pack".
        ghost = dict(self.ROLES, ghost=99999)
        problems = lines.verify({"knight": ["ghost"]}, ghost, lines.upgrade_edges(EFFECTS), UNITS)
        self.assertEqual(len(problems), 1)
        self.assertIn("99999", problems[0])
        self.assertIn("does not carry", problems[0])


class CliTests(unittest.TestCase):
    def test_a_missing_table_is_a_sentence_not_a_traceback(self):
        import tempfile

        with tempfile.TemporaryDirectory() as empty:
            original = lines.LINES
            try:
                lines.LINES = Path(empty) / "lines.toml"
                self.assertEqual(lines.main([]), 2, "a missing table is the author's to fix, and says so")
            finally:
                lines.LINES = original


class ProjectLinesTests(unittest.TestCase):
    """The repository's own table, checked against the pack that is installed. Skipped without the install."""

    def setUp(self):
        effects = CONFIG / "gamedata" / "effects.toml"
        if not effects.exists():
            self.skipTest("no generated game data: run the generator first")
        self.effects = tomllib.loads(effects.read_text())
        self.roles = tomllib.loads((CONFIG / "economy.toml").read_text())["role"]
        self.lines = tomllib.loads((CONFIG / "lines.toml").read_text())["line"]

    def test_every_line_is_a_real_upgrade_chain(self):
        units = {unit["id"] for unit in tomllib.loads((CONFIG / "gamedata" / "units.toml").read_text())["unit"]}
        problems = lines.verify(self.lines, self.roles, lines.upgrade_edges(self.effects), units)
        self.assertEqual(problems, [], "config/lines.toml disagrees with the pack:\n" + "\n".join(problems))

    def test_the_knight_line_is_what_the_effect_table_says_it_is(self):
        ids = [self.roles[role] for role in self.lines["knight"]]
        self.assertEqual(ids, lines.chain_from(ids[0], lines.upgrade_edges(self.effects)))

    def test_the_spear_line_ends_at_the_halberdier(self):
        self.assertEqual(self.lines["spear"][-1], "halberdier")


if __name__ == "__main__":
    unittest.main()
