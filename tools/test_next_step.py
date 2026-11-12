"""`docs/next-step.md` is one step, and the cap is enforced (D193).

The owner's rule is "compact first: persist state to files so a fresh session resumes without the conversation". By 2026-11-12 the
file was **350,038 bytes** - history that every session had to read before it could start. These tests pin the three claims the cap
makes: the file is headed by the open step, it is small, and splitting keeps the current block intact while moving the rest.

Run: `python3 -m unittest tools.test_next_step -v`
"""

from __future__ import annotations

import unittest

import next_step

SMALL = """# OPEN - the one thing to do

Do it.

**State.** Small.
"""


class TheFileIsANextStep(unittest.TestCase):
    def test_a_small_file_headed_by_the_open_step_passes(self):
        self.assertEqual([], next_step.check(SMALL))

    def test_a_file_over_the_cap_is_refused_with_its_size(self):
        text = SMALL + ("filler line\n" * 4_000)
        problems = next_step.check(text)
        self.assertEqual(1, len(problems))
        self.assertIn("cap", problems[0])
        self.assertIn("--split", problems[0])

    def test_a_file_that_does_not_start_with_the_open_step_is_refused(self):
        text = "# DONE - something old\n\nProse.\n" + SMALL
        problems = next_step.check(text)
        self.assertEqual(1, len(problems))
        self.assertIn("starts with", problems[0])


class SplittingKeepsTheStepAndMovesTheRest(unittest.TestCase):
    def test_split_keeps_the_first_block_and_moves_every_later_one(self):
        text = SMALL + "\n# OPEN - an old step\n\nOld prose.\n\n# NEXT - older still\n\nMore.\n"
        current, history = next_step.split(text)
        self.assertEqual(SMALL.rstrip("\n"), current.rstrip("\n"))
        self.assertIn("an old step", history)
        self.assertIn("older still", history)
        self.assertNotIn("an old step", current)

    def test_split_of_a_single_block_moves_nothing(self):
        current, history = next_step.split(SMALL)
        self.assertEqual("", history)
        self.assertEqual(SMALL.rstrip("\n"), current.rstrip("\n"))


if __name__ == "__main__":
    unittest.main()
