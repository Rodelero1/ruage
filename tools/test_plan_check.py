"""What `tools/plan_check.py` must be able to refuse, and the real document it must accept.

The dependency tree is a status table whose whole value is that a row can be checked (`docs/dependency-tree.md`, §1 and
§11). A table nobody checks rots in the two ways that cost this project sessions: an id that is referred to and never
defined, and an open item that is defined and never scheduled. Both are silent, so both are tests.

Every case here is a **finding** the tool must produce on a small document written for it, plus one run over the real
document, which must be clean.

Run: `python3 -m unittest tools.test_plan_check -v`
"""

from __future__ import annotations

import io
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

import plan_check

# A document that must pass every check: one item of each kind, referenced everywhere it has to be, and a real file.
GOOD = """# A tree

## 1. How to read it

`DONE`, `PART`, `MISSING` are the vocabulary.

## 2. Foundations

| id | Item | Status | Evidence |
|---|---|---|---|
| F1 | fixed point | **DONE** | `README.md` |

## 3. What runs

| id | Item | Status | Needs | Evidence |
|---|---|---|---|---|
| S1 | movement | **DONE** | F1 | `README.md:1` |
| S2 | gather | **DONE-ish** | S1 | `README.md`. **Hole: T1** |

## 4. The debts

| id | Debt | Why | Evidence | Unblocks |
|---|---|---|---|---|
| T1 | a wrong number | it lands in the wrong field | `README.md:1` (U1) | nothing |

## 5. Product

| id | Item | Status | Needs |
|---|---|---|---|
| P1 | save load | **MISSING** | F1 |

## 6. The sequence

### Phase 0

1. **T1** — the number.
2. **P1** — the loader.

## 7. Probes

```sh
grep -n x README.md
```
"""


def findings(document: str, name: str = "doc.md") -> list[str]:
    """Run the tool over `document` written into a scratch tree, and return its findings.

    The scratch tree also carries the two corpora the checks read, because a citation of `U1` or `D1` is only a
    reference if the register and the decision corpus have it — the real ones must not be able to make a fixture pass.
    """
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "README.md").write_text("# one line\n", encoding="utf-8")
        (root / "docs" / "mechanics").mkdir(parents=True)
        (root / "docs" / "mechanics" / "README.md").write_text("| U1 | a question |\n", encoding="utf-8")
        (root / "docs" / "decisions.md").write_text("## D1 - a decision\n", encoding="utf-8")
        path = root / name
        path.write_text(document, encoding="utf-8")
        out = io.StringIO()
        with redirect_stdout(out):
            code = plan_check.main(["--doc", str(path), "--root", str(root)])
        printed = out.getvalue()
    return [] if code == 0 else [line for line in printed.splitlines() if line.startswith("  ")]


class TheVocabulary(unittest.TestCase):
    def test_a_status_outside_the_vocabulary_is_refused(self) -> None:
        bad = GOOD.replace("**DONE-ish**", "**MOSTLY**")
        self.assertTrue(
            any("MOSTLY" in finding for finding in findings(bad)),
            "a status tag the legend does not define would make the table unreadable",
        )


class TheIds(unittest.TestCase):
    def test_a_defined_twice_id_is_refused(self) -> None:
        bad = GOOD.replace("| T1 | a wrong number", "| T1 | a wrong number |\n| T1 | the same id again")
        self.assertTrue(any("T1" in finding for finding in findings(bad)))

    def test_a_reference_to_an_undefined_id_is_refused(self) -> None:
        bad = GOOD.replace("**Hole: T1**", "**Hole: T9**")
        self.assertTrue(any("T9" in finding for finding in findings(bad)))

    def test_an_open_item_that_no_phase_names_is_refused(self) -> None:
        bad = GOOD.replace("2. **P1** — the loader.", "")
        self.assertTrue(
            any("P1" in finding for finding in findings(bad)),
            "an item nobody scheduled is the miss this document exists to prevent",
        )

    def test_a_hollow_row_with_no_debt_to_blame_is_refused(self) -> None:
        bad = GOOD.replace("**Hole: T1**", "the hole is not named")
        self.assertTrue(any("S2" in finding for finding in findings(bad)))


class TheCitations(unittest.TestCase):
    def test_a_path_that_does_not_exist_is_refused(self) -> None:
        bad = GOOD.replace("`README.md:1` (U1)", "`crates/nope.rs:1` (U1)")
        self.assertTrue(any("nope.rs" in finding for finding in findings(bad)))

    def test_a_line_past_the_end_of_the_file_is_refused(self) -> None:
        bad = GOOD.replace("`README.md:1` (U1)", "`README.md:99` (U1)")
        self.assertTrue(any("README.md" in finding for finding in findings(bad)))

    def test_a_u_row_or_d_row_is_only_a_reference_if_the_corpus_has_it(self) -> None:
        bad = GOOD.replace("`README.md:1` (U1)", "`README.md:1` (U99)")
        bad = bad.replace("## 7. Probes", "See D99.\n\n## 7. Probes")
        found = findings(bad)
        self.assertTrue(any("U99" in finding for finding in found))
        self.assertTrue(any("D99" in finding for finding in found))


class TheRealDocument(unittest.TestCase):
    def test_the_shipped_dependency_tree_is_clean(self) -> None:
        out = io.StringIO()
        with redirect_stdout(out):
            code = plan_check.main([])
        self.assertEqual(code, 0, out.getvalue())
        self.assertIn("plan ok", out.getvalue())


if __name__ == "__main__":
    unittest.main()
