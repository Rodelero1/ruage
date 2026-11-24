"""The live migration: what `tools/live_migrate.py` must guarantee (docs cleanup).

The bank (`docs/archive/decisions-2026-11-24.md`) is sharded into `docs/live/<area>.md` by the authored
`docs/decision-areas.toml` — mechanically, never by rewording. Every test here is about
a guarantee the migration makes, not a formatting detail: if a decision is lost,
doubled, or silently bucketed, the live files are worse than the bank.

Run: `python3 -m unittest discover -s tools -p 'test_live_migrate.py' -v`
"""

from __future__ import annotations

import unittest
from pathlib import Path

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


class CollisionLiveIds(unittest.TestCase):
    def test_two_sections_sharing_one_id_get_distinct_live_ids(self):
        text = (
            "## D88 - First meaning\n\nBody one. Measured.\n\n"
            "## D88 - Second meaning\n\nBody two. The dat says so.\n\n"
        )
        corpus = corpus_of(text)
        titles = live_migrate.section_titles(text)
        out = live_migrate.render_decision(
            corpus.decisions["D88"], corpus, "Alpha", titles
        )
        self.assertIn("D88a", out)
        self.assertIn("D88b", out)
        self.assertIn("First meaning", out)
        self.assertIn("Second meaning", out)


class OwnerVerdictNormalized(unittest.TestCase):
    def test_an_owner_tag_is_kept_and_counted_as_ours(self):
        text = (
            "## D354 - The rule he remembers\n\n"
            "**Conformance**: OWNER, 2026-11-07: the owner said so.\n\nBody.\n\n"
        )
        corpus = corpus_of(text)
        out = live_migrate.render_decision(
            corpus.decisions["D354"], corpus, "Alpha"
        )
        self.assertIn("**Conformance**: OWNER", out)
        self.assertIn("OURS", out)


class BankLivesInArchive(unittest.TestCase):
    def test_every_bank_reader_points_at_the_archive_file(self):
        import conformance_audit
        import decision_index

        for module in (decisions, decision_index, conformance_audit, live_migrate):
            self.assertEqual(
                module.DECISIONS,
                Path(module.__file__).resolve().parent.parent / "docs" / "archive"
                / "decisions-2026-11-24.md",
                module.__name__,
            )

    def test_the_archived_bank_still_parses_whole(self):
        archive = (
            Path(decisions.__file__).resolve().parent.parent
            / "docs" / "archive" / "decisions-2026-11-24.md"
        )
        self.assertTrue(archive.exists())
        corpus = decisions.parse_decisions(archive.read_text())
        placed, _ = live_migrate.build_plan(corpus, ["A"], {d: "A" for d in corpus.decisions})
        self.assertEqual(sum(len(v) for v in placed.values()), len(corpus.decisions))


class AmendmentSubsectionIsNotACollision(unittest.TestCase):
    def test_an_addendum_subsection_keeps_one_id(self):
        text = (
            "## D5 - Title\n\nBody one.\n\n"
            "### D5 addendum: fix\n\nMore body.\n\n"
        )
        titles = live_migrate.section_titles(text)
        self.assertEqual(titles.get("D5"), ["Title"])
        corpus = corpus_of(text)
        out = live_migrate.render_decision(
            corpus.decisions["D5"], corpus, "Alpha", titles
        )
        self.assertNotIn("D5a", out)
        self.assertIn("More body", out)


class GeneratedFilesOutOfCitationScope(unittest.TestCase):
    def test_live_mirror_is_not_citation_checked(self):
        self.assertFalse(decisions._in_scope("docs/live/combat.md"))

    def test_archived_bank_stays_in_scope(self):
        self.assertTrue(
            decisions._in_scope("docs/archive/decisions-2026-11-24.md")
        )


class RefinementTables(unittest.TestCase):
    def test_a_status_override_names_the_chain_link(self):
        text = "## D59 - Old rule\n\nBody. Measured.\n\n"
        corpus = corpus_of(text)
        out = live_migrate.render_decision(
            corpus.decisions["D59"],
            corpus,
            "Alpha",
            None,
            status_overrides={"D59": ("superseded", "D59b")},
        )
        self.assertIn("superseded by D59b", out)

    def test_a_conformance_addition_promotes_a_flagged_row(self):
        text = "## D78 - Victory is conquest\n\nThe game ends so in the game.\n\n"
        corpus = corpus_of(text)
        out = live_migrate.render_decision(
            corpus.decisions["D78"],
            corpus,
            "Alpha",
            None,
            conformance_additions={"D78": "GAME - the manual's rule, quoted in D59"},
        )
        self.assertIn("**Conformance**: GAME - the manual's rule, quoted in D59", out)
        self.assertNotIn("Conformance needed", out)


if __name__ == "__main__":
    unittest.main()
