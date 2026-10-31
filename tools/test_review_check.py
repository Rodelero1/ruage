# Tests for the review gate. Written before the implementation, because the gate is the thing that is supposed to be
# impossible to forget - and a gate nobody tested is a gate that passes everything.
#
# Run with: python3 -m unittest discover -s tools -p 'test_*.py'

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import review_check


GOOD = """
# Review: the tile count

## Performance

26.4 ns/entity/tick against 25.5 on the previous commit: neutral.

## Stability

Two occupants on one tile used to free each other's ground; counted now.
"""


class NeedsReview(unittest.TestCase):
    def test_a_rust_change_needs_a_review(self):
        self.assertTrue(review_check.needs_review(["crates/sim/src/path.rs"]))

    def test_a_content_change_needs_a_review(self):
        self.assertTrue(review_check.needs_review(["config/economy.toml"]))

    def test_a_docs_only_change_does_not(self):
        self.assertFalse(review_check.needs_review(["docs/next-step.md", "SPEC.md", "AGENTS.md"]))

    def test_the_review_itself_does_not_need_a_review(self):
        self.assertFalse(review_check.needs_review(["docs/reviews/2026-10-31-x.md"]))


class Check(unittest.TestCase):
    def test_a_code_change_without_a_review_file_is_refused(self):
        problems = review_check.check(["crates/sim/src/path.rs"], {})
        self.assertTrue(problems, "a code change with no review passed")
        self.assertIn("no review", " ".join(problems).lower())

    def test_a_review_missing_stability_is_refused(self):
        text = GOOD.replace("## Stability", "## Something else")
        problems = review_check.check(
            ["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": text}
        )
        self.assertTrue(any("stability" in p.lower() for p in problems), problems)

    def test_a_review_missing_performance_is_refused(self):
        text = GOOD.replace("## Performance", "## Notes")
        problems = review_check.check(
            ["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": text}
        )
        self.assertTrue(any("performance" in p.lower() for p in problems), problems)

    def test_performance_without_a_number_is_refused(self):
        text = GOOD.replace("26.4 ns/entity/tick against 25.5 on the previous commit: neutral.", "It felt fine.")
        problems = review_check.check(
            ["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": text}
        )
        self.assertTrue(any("measure" in p.lower() or "number" in p.lower() for p in problems), problems)

    def test_a_review_with_both_sections_and_a_number_passes(self):
        problems = review_check.check(
            ["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": GOOD}
        )
        self.assertEqual(problems, [])

    def test_a_deleted_review_file_does_not_count_as_a_review(self):
        # `git diff --cached --name-only` lists a deletion the same way it lists an addition, and `git show :path` then
        # returns nothing: the change must be refused rather than pass on the filename.
        problems = review_check.check(["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": ""})
        self.assertTrue(problems)

    def test_an_empty_review_file_is_refused(self):
        problems = review_check.check(["crates/sim/src/path.rs", "docs/reviews/r.md"], {"docs/reviews/r.md": ""})
        self.assertTrue(problems)


if __name__ == "__main__":
    unittest.main()
