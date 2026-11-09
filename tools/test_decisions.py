"""The decision index: what `tools/decisions.py` must be able to say about the corpus (D165).

The index exists because 160-odd decisions live in 4,000 lines of prose and nothing says which of them rest on the
dat, which on the owner's word, which on a guess, and which are referenced but were never written down. Every test
here is about *a claim the page makes*, not about a parsing detail: if the parser stops recognising a basis, the page
silently stops making the claim, so the claim is what is pinned.

Run: `python3 -m unittest tools.test_decisions -v`
"""

from __future__ import annotations

import unittest
from pathlib import Path

import decisions

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


def parse(text: str) -> decisions.Corpus:
    return decisions.parse_decisions(text)


class TableForm(unittest.TestCase):
    """D1-D16 are one row each, with the date in its own column."""

    def test_a_table_row_is_a_decision_with_its_date_and_text(self):
        text = (
            "| # | Date | Decision | Rationale | Commits us to |\n"
            "|---|---|---|---|---|\n"
            "| **D1** | 2026-10-23 | **Answered: G1 — generally similar.** Own engine. | "
            "Owner's words: \"sensibly attainable\". | Bands in §6.1. |\n"
        )
        corpus = parse(text)
        self.assertIn("D1", corpus.decisions)
        decision = corpus.decisions["D1"]
        self.assertEqual("2026-10-23", decision.date)
        self.assertIn("generally similar", decision.text)
        self.assertIn("Owner's words", decision.text)

    def test_a_quoted_first_sentence_keeps_its_closing_quote(self):
        """Splitting a title at the first period cut inside a quotation, leaving `...attainable` unclosed."""
        text = '| **D1** | 2026-10-23 | **Answered: G1 — "generally similar."** Own engine. | r | c |\n'
        self.assertTrue(parse(text).decisions["D1"].title.endswith('."'), parse(text).decisions["D1"].title)

    def test_a_table_row_does_not_become_a_heading_decision(self):
        text = "| **D2** | 2026-10-23 | A file oracle. | 13 saves. | None. |\n"
        corpus = parse(text)
        self.assertEqual(["D2"], list(corpus.decisions))
        self.assertEqual([], corpus.unclassified)


class ProseForm(unittest.TestCase):
    """D17 onward are `## D<n> - Title` sections."""

    def test_a_prose_section_carries_its_title_and_the_date_in_its_body(self):
        text = (
            "## D17 - Four layers, and only one is about Age of Empires\n"
            "\n"
            "**Decided 2026-10-25.** The engine must ship a game that is not AoE2.\n"
            "\n"
            "## D18 - Target shape\n"
            "\n"
            "*2026-11-08. Something else.*\n"
        )
        corpus = parse(text)
        self.assertEqual("Four layers, and only one is about Age of Empires", corpus.decisions["D17"].title)
        self.assertEqual("2026-10-25", corpus.decisions["D17"].date)
        self.assertEqual("2026-11-08", corpus.decisions["D18"].date)

    def test_an_applied_note_is_not_a_decision(self):
        """`## D7 applied: ...` is an essay about D7, not a decision with id D7."""
        text = (
            "## D7 applied: the generator rewrite\n"
            "\n"
            "The script was deleted and rewritten.\n"
            "\n"
            "## D8 - No game data is committed\n"
            "\n"
            "**Decided 2026-10-23.**\n"
        )
        corpus = parse(text)
        self.assertEqual(["D8"], list(corpus.decisions))
        self.assertEqual([], corpus.unclassified)
        self.assertEqual("D7", corpus.notes[0][0])

    def test_an_amendment_is_recorded_against_its_decision_not_as_a_second_one(self):
        text = (
            "## D14 - Keep Ruage on HD\n"
            "\n"
            "**Decided 2026-10-23.**\n"
            "\n"
            "## D14 (amended 2026-10-25) - openage is the primary prior-art source\n"
            "\n"
            "The watch now leads with openage.\n"
        )
        corpus = parse(text)
        self.assertEqual(2, corpus.versions["D14"])
        self.assertIn("openage", " ".join(corpus.amendments["D14"]))
        self.assertEqual(1, len(corpus.decisions))

    def test_a_decision_written_one_level_down_is_still_a_decision(self):
        """D24a, D94a and D98a live at `###` under their parent. Missing them makes their ids look dangling."""
        text = "## D24 - X\n\n**Decided.**\n\n### D24a - The zero flag\n\n**Decided.**\n"
        corpus = parse(text)
        self.assertIn("D24a", corpus.decisions)
        self.assertEqual("The zero flag", corpus.decisions["D24a"].title)

    def test_a_prose_subheading_does_not_cut_its_decision_in_short(self):
        text = "## D24 - X\n\n### What is not modelled\n\n**Not modelled:** the door.\n"
        corpus = parse(text)
        self.assertEqual(1, len(corpus.decisions["D24"].gaps))
        self.assertEqual([], corpus.structural)

    def test_a_heading_with_no_decision_id_is_document_structure_and_is_counted(self):
        text = "## D99 - Fine\n\n## Prior art: another project\n"
        corpus = parse(text)
        self.assertEqual(["Prior art: another project"], corpus.structural)
        self.assertEqual([], corpus.unclassified)

    def test_a_malformed_decision_heading_is_reported_not_swallowed(self):
        """`## D99 Some title` with no separator is a heading the parser cannot place, so the build must refuse it."""
        text = "## D99 - Fine\n\n## D100 Some title with no separator\n"
        corpus = parse(text)
        self.assertEqual(["## D100 Some title with no separator"], corpus.unclassified)


class Basis(unittest.TestCase):
    """Where a decision came from, recorded with the phrase that says so."""

    def test_the_owner_asking_is_recorded_with_the_phrase(self):
        text = "## D3 - One civ\n\n**Decided 2026-10-23.** Answered: start with British. Owner's decision.\n"
        decision = parse(text).decisions["D3"]
        self.assertIn("owner", decision.origins)
        self.assertTrue(
            any("Owner's decision" in snippet for snippet in decision.origins["owner"]),
            decision.origins["owner"],
        )

    def test_a_dat_value_is_a_basis_of_its_own(self):
        text = "## D5 - The dat wins\n\nWhere the dat and the wiki disagree, the dat wins.\n"
        decision = parse(text).decisions["D5"]
        self.assertIn("dat", decision.origins)

    def test_a_measurement_is_a_basis_of_its_own(self):
        text = "## D10 - Two grains\n\nMeasured: `dat_value_decimals = 4`, `per_tick_decimals = 3`.\n"
        self.assertIn("measured", parse(text).decisions["D10"].origins)

    def test_reading_another_project_is_named_rather_than_called_research(self):
        text = "## D13 - Format crates\n\nSurveyed: `librematch/genie` is AGPL-3.0.\n"
        self.assertIn("prior-art", parse(text).decisions["D13"].origins)

    def test_a_decision_with_no_basis_language_anywhere_is_named_as_such(self):
        """The reliable signal is the *absence* of a citation, so it must be explicit."""
        text = "## D99 - A rule with no reason given\n\nIt was done this way.\n"
        decision = parse(text).decisions["D99"]
        self.assertEqual({}, decision.origins)
        self.assertTrue(decision.uncited)

    def test_categories_named_inside_a_quote_from_the_owner_are_his_words_not_our_method(self):
        """D165 quotes the owner listing "reverse engineered" as a category; that must not tag the decision `binary`."""
        text = ('## D99 - X\n\n*2026-11-01. The owner: "research, reverse engineered, assumed - all of it."*\n\n'
                "It was implemented.\n")
        origins = parse(text).decisions["D99"].origins
        self.assertIn("owner", origins)
        self.assertNotIn("binary", origins)

    def test_a_sentence_that_lists_the_origin_vocabulary_is_not_evidence(self):
        """D165 explains what the tags mean; the explanation must not become the decision's own basis."""
        text = ("## D99 - How the index works\n\nA basis is one of owner, dat, binary, measured, published, "
                "prior-art, decision, review.\n")
        self.assertEqual({}, parse(text).decisions["D99"].origins)

    def test_a_sentence_naming_two_sources_still_counts(self):
        text = "## D99 - X\n\nMeasured on the dat: speed is 0.737.\n"
        origins = parse(text).decisions["D99"].origins
        self.assertIn("measured", origins)
        self.assertIn("dat", origins)

    def test_a_guess_is_flagged_with_the_sentence_that_admits_it(self):
        text = (
            "## D99 - A rate\n\n"
            "The farm's 0.400 is plausibly a throughput limit, but the interpretation stays a hypothesis.\n"
        )
        decision = parse(text).decisions["D99"]
        self.assertTrue(
            any("hypothesis" in phrase for phrase in decision.unresolved),
            decision.unresolved,
        )


class Status(unittest.TestCase):
    """A rule that was rewritten must not read as the current rule."""

    def test_a_decision_that_corrects_an_earlier_one_marks_the_earlier_as_corrected(self):
        text = (
            "## D31 - The ceiling refuses the order\n\n"
            "**Decided 2026-10-25.**\n"
            "\n"
            "## D34 - The ceiling pauses; it never refuses (correcting D31's refusal)\n"
            "\n"
            "**Decided 2026-10-25.** That was wrong.\n"
        )
        corpus = parse(text)
        self.assertEqual(["D34"], corpus.decisions["D31"].corrected_by)
        self.assertEqual("corrected", decisions.status(corpus.decisions["D31"], corpus))

    def test_a_superseded_rule_names_its_replacement(self):
        text = (
            "## D9 - The scale divides a power of ten\n\n**Decided 2026-10-23.**\n"
            "\n"
            "## D10 - Two derived grains\n\n**Decided 2026-10-23.** Supersedes D9's clause.\n"
        )
        corpus = parse(text)
        self.assertEqual(["D10"], corpus.decisions["D9"].superseded_by)
        self.assertEqual("superseded", decisions.status(corpus.decisions["D9"], corpus))

    def test_a_current_decision_is_just_decided(self):
        text = "## D50 - Vision\n\n**Decided 2026-10-29.**\n"
        corpus = parse(text)
        self.assertEqual("decided", decisions.status(corpus.decisions["D50"], corpus))


class Gaps(unittest.TestCase):
    """Every decision names what it does not do; those names are the incomplete register."""

    def test_the_not_modelled_paragraph_becomes_a_gap_on_its_decision(self):
        text = (
            "## D32 - Reach\n\n**Decided 2026-10-26.**\n\n"
            "**Not modelled:** projectiles are instant, and there is no line of sight.\n"
            "\n"
            "## D33 - A queue\n\n**Decided 2026-10-26.**\n"
        )
        decision = parse(text).decisions["D32"]
        self.assertEqual(1, len(decision.gaps))
        self.assertIn("line of sight", decision.gaps[0])

    def test_not_done_is_a_gap_too(self):
        text = "## D99 - X\n\n7. **Not done**: the villager build card.\n"
        self.assertIn("build card", " ".join(parse(text).decisions["D99"].gaps))


class Unknowns(unittest.TestCase):
    """The U-register is the 'not decided yet' half, and it lives in one table."""

    TABLE = (
        "| # | Question | Why it matters | How to settle |\n"
        "|---|---|---|---|\n"
        "| ~~U1~~ | ~~Gather rates~~ **RESOLVED** - effect 195. | — | Done. |\n"
        "| U5 | Are speeds tiles/second? | Every path needs it. | Oracle. |\n"
        "| **U17** | **Negative packed values.** | A Korean tech lands wrong. | A recording. |\n"
        "| U6 | Reload semantics **MOSTLY RESOLVED** - interval. | — | Tick split out. |\n"
    )

    def test_a_struck_unknown_is_closed(self):
        unknowns = decisions.parse_unknowns(self.TABLE)
        self.assertTrue(unknowns["U1"].closed)

    def test_an_answer_word_is_enough_to_close_one_that_is_not_struck(self):
        self.assertTrue(decisions.parse_unknowns(self.TABLE)["U6"].closed)

    def test_a_plain_row_is_still_open(self):
        unknowns = decisions.parse_unknowns(self.TABLE)
        self.assertFalse(unknowns["U5"].closed)

    def test_an_open_question_keeps_its_text(self):
        self.assertIn("tiles/second", decisions.parse_unknowns(self.TABLE)["U5"].question)


class References(unittest.TestCase):
    def test_a_reference_to_a_decision_that_was_never_written_is_dangling(self):
        defined = {"D1": None, "D2": None}
        texts = {
            "docs/a.md": "see D1, D2 and D3.\nsecond line mentions D3 again\n",
            "crates/x.rs": "// D4\n",
        }
        refs = decisions.dangling_refs(defined, texts)
        self.assertEqual({"D3", "D4"}, {ref.id for ref in refs})
        self.assertTrue(all(ref.id in {"D3", "D4"} for ref in refs))
        where = {(ref.id, ref.path, ref.line) for ref in refs}
        self.assertIn(("D3", "docs/a.md", 1), where)
        self.assertIn(("D3", "docs/a.md", 2), where)

    def test_a_defined_id_is_never_dangling(self):
        self.assertEqual([], decisions.dangling_refs({"D7": None}, {"x.md": "D7\n"}))

    def test_a_version_string_is_not_a_decision_reference(self):
        self.assertEqual([], decisions.dangling_refs({}, {"x.md": "D-v0.58 and v0.58.1\n"}))


class Page(unittest.TestCase):
    """The rendered page, not the model: it has to stand on its own."""

    def corpus(self):
        return parse(
            "## D1 - A\n\nMeasured. Not modelled: the door.\n"
            "\n## D2 - B (correcting D1)\n\n**Decided 2026-10-25.** The owner asked.\n"
        )

    def test_the_page_loads_nothing_from_outside_itself(self):
        """Quoted evidence may contain a URL; the page must not *fetch* anything."""
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        self.assertNotIn("<script src=", html)
        self.assertNotIn("<link rel=", html)
        self.assertNotIn('src="http', html)
        self.assertNotIn('href="http', html)

    def test_a_decision_that_contains_markup_is_escaped_not_executed(self):
        """The corpus is prose written by agents; a `<script>` in a title must not become one in the page."""
        text = '## D1 - A <script>alert("x")</script> title\n\n**Not modelled:** <img src=x onerror=1>.\n'
        html = decisions.render(parse(text), unknowns={}, refs=[], generated="2026-11-09")
        self.assertNotIn("<script>alert", html)
        self.assertIn("&lt;script&gt;", html)
        self.assertNotIn("<img src=x", html)

    def test_an_empty_corpus_still_renders(self):
        html = decisions.render(decisions.Corpus(), unknowns={}, refs=[], generated="unknown")
        self.assertIn(">0</b> decisions", html)
        self.assertIn("Every id cited", html)

    def test_origin_tags_are_prefixed_so_they_cannot_be_counted_as_cards(self):
        """An origin named `decision` collided with the card class and the counter said 180 of 180 where there were 164."""
        corpus = parse("## D1 - A\n\nMeasured.\n")
        html = decisions.render(corpus, unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn('class="tag tag-measured"', html)
        self.assertNotIn('class="tag measured"', html)

    def test_the_page_states_its_own_coverage(self):
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn(">2</b> decisions", html)
        self.assertIn(">0</b> unclassified", html)

    def test_an_unclassified_heading_appears_on_the_page_by_name(self):
        corpus = self.corpus()
        corpus.unclassified.append("## A heading nobody could place")
        html = decisions.render(corpus, unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn("A heading nobody could place", html)

    def test_the_two_registers_are_separate(self):
        """'not decided yet' and 'decided but incomplete' are different actions."""
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn("id=\"undecided\"", html)
        self.assertIn("id=\"incomplete\"", html)


class Cost(unittest.TestCase):
    """A build that hangs on a long section is a build nobody runs twice."""

    def test_one_long_decision_section_does_not_take_minutes(self):
        import time

        text = "## D1 - A long one\n\n" + ("The owner asked. " * 20000) + "\n"
        started = time.perf_counter()
        decisions.parse_decisions(text)
        elapsed = time.perf_counter() - started
        self.assertLess(elapsed, 2.0, f"a 300 KB section took {elapsed:.1f} s")


class ReviewQueue(unittest.TestCase):
    """Reviewing 166 decisions needs a resume point, not a scrollbar."""

    def corpus(self):
        return parse("## D1 - A\n\n`crates/sim/src/combat.rs`\n\n## D2 - B\n\nA villager gathers wood.\n")

    def test_the_page_has_a_review_queue_with_a_batch_size(self):
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn('id="review"', html)
        self.assertIn('id="batch-size"', html)

    def test_every_card_can_carry_a_verdict(self):
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        for verdict in ("agree", "question", "wrong"):
            self.assertIn(f'data-verdict="{verdict}"', html)

    def test_the_page_can_export_the_review_so_it_leaves_the_browser(self):
        html = decisions.render(self.corpus(), unknowns={}, refs=[], generated="2026-11-09")
        self.assertIn('id="export"', html)
        self.assertIn('ruage-decisions-review', html)


class TheScan(unittest.TestCase):
    """A committed page must quote the repository, not whatever is lying in the working tree."""

    def test_the_scan_reads_the_paths_it_is_given_and_nothing_else(self):
        texts = decisions.scan_sources(["docs/decisions.md"])
        self.assertEqual(["docs/decisions.md"], list(texts))

    def test_a_path_outside_the_scanned_folders_is_not_read(self):
        self.assertEqual({}, decisions.scan_sources(["tools/decisions.py"]))


class TheCommandLine(unittest.TestCase):
    """The build has to fail politely: a traceback in a make target teaches nobody anything."""

    def test_a_missing_corpus_is_a_message_and_a_status_not_a_traceback(self):
        code = decisions.main(["--check", "--decisions", "/nonexistent/decisions.md"])
        self.assertEqual(2, code)


class TheRealCorpus(unittest.TestCase):
    """The strict one: the corpus as it actually stands must parse with nothing unplaced."""

    def test_every_decision_in_the_repository_is_parsed_and_placed(self):
        corpus = decisions.parse_decisions((ROOT / "docs/decisions.md").read_text())
        self.assertGreater(len(corpus.decisions), 150)
        self.assertEqual([], corpus.unclassified)

    def test_every_decision_without_a_basis_is_listed_so_it_can_be_triaged(self):
        corpus = decisions.parse_decisions((ROOT / "docs/decisions.md").read_text())
        uncited = [d for d in corpus.decisions.values() if d.uncited]
        # Not zero: the corpus has rules whose reasoning is prose and nothing else. The page has to name them,
        # which is the whole point - but if *every* decision were uncited the classifier would be broken.
        self.assertLess(len(uncited), len(corpus.decisions))

    def test_the_repository_s_own_unknowns_parse(self):
        unknowns = decisions.parse_unknowns((ROOT / "docs/mechanics/README.md").read_text())
        self.assertGreaterEqual(len(unknowns), 15)
        self.assertTrue(any(not unknown.closed for unknown in unknowns.values()))


if __name__ == "__main__":
    unittest.main()
