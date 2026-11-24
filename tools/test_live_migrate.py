"""The live migration: what `tools/live_migrate.py` must guarantee (docs cleanup).

The bank (`docs/decisions.md`) is sharded into `docs/live/<area>.md` by the authored
`docs/decision-areas.toml` — mechanically, never by rewording. Every test here is about
a guarantee the migration makes, not a formatting detail: if a decision is lost,
doubled, or silently bucketed, the live files are worse than the bank.

Run: `python3 -m unittest discover -s tools -p 'test_live_migrate.py' -v`
"""

from __future__ import annotations

import unittest

import decisions
import live_migrate


def corpus_of(text: str) -> decisions.Corpus:
    return decisions.parse_decisions(text)


class NoLossNoDoubling(unittest.TestCase):
    def test_every_filed_decision_lands_in_exactly_one_area(self):
        text = (
            "## D1 - First\n\nDecided 2026-10-23. Owner's decision.\n\n"
            "## D2 - Second\n\nDecided 2026-10-24. Measured 5 of the 10 units.\n\n"
            "## D3 - Third\n\nDecided 2026-10-25. The dat says so.\n\n"
        )
        order = ["Alpha", "Beta"]
        mapping = {"D1": "Alpha", "D2": "Beta", "D3": "Alpha"}
        plan, problems = live_migrate.build_plan(
            corpus_of(text), order, mapping
        )
        self.assertEqual([], problems)
        landed = [d.id for ids in plan.values() for d in ids]
        self.assertEqual(sorted(["D1", "D2", "D3"]), sorted(landed))

    def test_an_unfiled_decision_is_refused_not_bucketed(self):
        text = "## D9 - Nowhere\n\nDecided 2026-10-23. Something.\n\n"
        plan, problems = live_migrate.build_plan(
            corpus_of(text), ["Alpha"], {"D1": "Alpha"}
        )
        self.assertTrue(any("D9" in p for p in problems))
        landed = [d.id for ids in plan.values() for d in ids]
        self.assertNotIn("D9", landed)


class AmendmentsSurvive(unittest.TestCase):
    def test_two_sections_with_one_id_are_both_emitted(self):
        text = (
            "## D88 - First meaning\n\nBody one. Measured.\n\n"
            "## D88 - Second meaning\n\nBody two. The dat says so.\n\n"
        )
        out = live_migrate.render_decision(
            corpus_of(text).decisions["D88"],
            corpus_of(text),
            "Alpha",
        )
        self.assertIn("Body one", out)
        self.assertIn("Body two", out)


class ConformancePreserved(unittest.TestCase):
    def test_a_game_pointer_survives_the_move_verbatim(self):
        text = (
            "## D99 - A game fact\n\n**Conformance**: GAME - `FUN_007ed280`.\n\n"
            "Body. Measured.\n\n"
        )
        out = live_migrate.render_decision(
            corpus_of(text).decisions["D99"],
            corpus_of(text),
            "Alpha",
        )
        self.assertIn("FUN_007ed280", out)
        self.assertIn("**Conformance**: GAME", out)

    def test_a_game_claim_without_a_pointer_is_flagged_not_silenced(self):
        text = "## D98 - The game does X\n\nThe original does X. No pointer.\n\n"
        out = live_migrate.render_decision(
            corpus_of(text).decisions["D98"],
            corpus_of(text),
            "Alpha",
        )
        self.assertIn("Conformance needed", out)


if __name__ == "__main__":
    unittest.main()
