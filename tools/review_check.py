#!/usr/bin/env python3
"""The review gate: a commit that changes engine, content or tooling must carry its adversarial review.

Why a script and not a paragraph in `AGENTS.md`: the rule "compact, implement, review for performance and stability, push"
has been stated three times and skipped twice. A rule that lives only in someone's memory is a rule that is followed when
it is remembered. This one is checked by `.githooks/pre-commit`, so the commit fails instead.

What it enforces, and deliberately nothing more:

* a staged change under `crates/`, `config/`, `tools/`, `Age2HD/` or `build/` needs a staged review note under
  `docs/reviews/`;
* the note has a `## Performance` section that contains a **measured** value (a number with a time unit or a percentage)
  and a `## Stability` section with something in it.

It cannot judge whether the review is honest - only a reader can. It can only refuse the commit that has no review at all,
which is the failure that actually keeps happening.

    python3 tools/review_check.py          # check the staged set (what the hook runs)
"""

from __future__ import annotations

import re
import subprocess
import sys

# A path under one of these is "code" for the purposes of the gate. `docs/`, `SPEC.md` and the READMEs are not: a typo fix
# does not need a performance measurement, and demanding one would teach everyone to write a fake review.
CODE_PREFIXES = ("crates/", "config/", "tools/", "Age2HD/", "build/")
REVIEW_PREFIX = "docs/reviews/"
REQUIRED_SECTIONS = ("## performance", "## stability")
MEASURED = re.compile(r"\d+(?:\.\d+)?\s*(?:ns|us|µs|ms|s|%|x)\b", re.IGNORECASE)


def needs_review(paths) -> bool:
    """Whether this set of paths changes something that owes a review."""
    return any(path.startswith(CODE_PREFIXES) for path in paths)


def review_paths(paths):
    """The staged review notes."""
    return [
        path
        for path in paths
        if path.startswith(REVIEW_PREFIX) and path.endswith(".md")
    ]


def _section(text: str, heading: str) -> str:
    """The body under one heading, up to the next heading of the same or a higher level."""
    lowered = text.lower()
    at = lowered.find(heading)
    if at < 0:
        return ""
    body = text[at + len(heading) :]
    end = len(body)
    for index, line in enumerate(body.splitlines(keepends=True)):
        if line.startswith("## ") and index > 0:
            end = sum(len(part) for part in body.splitlines(keepends=True)[:index])
            break
    return body[:end]


def check(paths, texts) -> list[str]:
    """The problems with this staged set. Empty means it may be committed."""
    paths = list(paths)
    if not needs_review(paths):
        return []
    notes = review_paths(paths)
    if not notes:
        return [
            "no review: this change touches code, so it needs a note under docs/reviews/ staged with it "
            "(compact, implement, review for performance and stability, then push)"
        ]

    problems = []
    for note in notes:
        text = texts.get(note, "")
        if not text.strip():
            problems.append(f"{note}: the review is empty")
            continue
        for heading in REQUIRED_SECTIONS:
            body = _section(text, heading).strip()
            if not body:
                problems.append(f"{note}: no {heading.strip('# ').strip()} section with content")
            elif heading == "## performance" and not MEASURED.search(body):
                problems.append(
                    f"{note}: the performance section has no measured number (ns, us, ms, s or %)"
                )
    return problems


def staged_paths() -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--cached", "--name-only"], capture_output=True, text=True, check=True
    )
    return [line for line in out.stdout.splitlines() if line.strip()]


def staged_text(path: str) -> str:
    out = subprocess.run(
        ["git", "show", f":{path}"], capture_output=True, text=True
    )
    return out.stdout if out.returncode == 0 else ""


def main(argv) -> int:
    paths = staged_paths()
    texts = {path: staged_text(path) for path in review_paths(paths)}
    problems = check(paths, texts)
    if not problems:
        return 0
    print("REFUSED: the review gate.", file=sys.stderr)
    for problem in problems:
        print(f"  {problem}", file=sys.stderr)
    print("", file=sys.stderr)
    print("Write docs/reviews/<date>-<topic>.md with `## Performance` (a measured number) and", file=sys.stderr)
    print("`## Stability`, stage it with the change, and commit again. Rule: AGENTS.md.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
