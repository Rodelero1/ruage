"""The one open step: `docs/next-step.md` is capped, and its history is history.

The owner's rule for every step is "compact first: persist state to files so a fresh session resumes without the conversation". By
2026-11-12 that file was **350,038 bytes** - twenty times the largest source file - and a session that read it (this one did, at
101 KB and growing) spent its context on history instead of on the step. A "next step" that is 350 KB is not a next step.

So: one file, one step, sixteen kilobytes, and everything older moves to `docs/history/`. The history is not deleted - it is already
in git, which is a better archive than a live file that the next session has to read.

    python3 tools/next_step.py --check      # nonzero if over the cap or not headed by the one open step
    python3 tools/next_step.py --split      # move everything after the first block into docs/history/next-step-<date>.md
"""

from __future__ import annotations

import argparse
import datetime
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
NEXT_STEP = ROOT / "docs" / "next-step.md"
HISTORY = ROOT / "docs" / "history"

# Sixteen kilobytes is about four thousand words: a page a session reads in one go and still has context left to work with.
CAP = 16_384

# The first line has to say what the file is, so a stale file is obvious rather than authoritative.
HEADING = "# OPEN"


def check(text: str, cap: int = CAP) -> list[str]:
    """Everything wrong with a `next-step.md`, as sentences. Empty means it is a next step."""
    problems: list[str] = []
    body = text.lstrip("\n")
    if not body.startswith(HEADING):
        first = body.split("\n", 1)[0][:60] if body else "(empty)"
        problems.append(
            f"docs/next-step.md starts with {first!r}, not {HEADING!r}: the one open step has to be the first thing in the file"
        )
    size = len(text.encode("utf-8"))
    if size > cap:
        problems.append(
            f"docs/next-step.md is {size:,} bytes, over the {cap:,} cap by {size - cap:,}: move everything after the first "
            f"block to docs/history/ with `python3 tools/next_step.py --split`"
        )
    return problems


def split(text: str) -> tuple[str, str]:
    """The current block, and the history that follows it.

    A block starts at a top-level `# ` heading. The first block is the open step; everything from the **second** top-level heading on
    is history. A file with one block has no history to move.
    """
    lines = text.split("\n")
    starts = [index for index, line in enumerate(lines) if line.startswith("# ")]
    if len(starts) < 2:
        return text, ""
    cut = starts[1]
    return "\n".join(lines[:cut]).rstrip() + "\n", "\n".join(lines[cut:])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="keep docs/next-step.md to one step, and its history in docs/history/")
    parser.add_argument("--check", action="store_true", help="report problems; nonzero if there are any")
    parser.add_argument("--split", action="store_true", help="move everything after the first block into docs/history/")
    parser.add_argument("--file", default=str(NEXT_STEP), help="the file (defaults to docs/next-step.md)")
    args = parser.parse_args(argv)

    path = Path(args.file)
    try:
        text = path.read_text()
    except OSError as error:
        print(f"cannot read {path}: {error}", file=sys.stderr)
        return 2

    if args.split:
        current, history = split(text)
        if not history.strip():
            print("nothing to move: the file has one block")
            return 0
        HISTORY.mkdir(parents=True, exist_ok=True)
        today = datetime.date.today().isoformat()
        target = HISTORY / f"next-step-{today}.md"
        marker = f"\n\n<!-- moved out of docs/next-step.md on {today} -->\n\n"
        with target.open("a") as sink:
            if target.stat().st_size == 0:
                sink.write(f"# History moved out of docs/next-step.md on {today}\n")
            sink.write(marker)
            sink.write(history)
        path.write_text(current)
        print(f"moved {len(history.encode('utf-8')):,} bytes to {target.relative_to(ROOT)}; {len(current.encode('utf-8')):,} left")
        text = current

    problems = check(text)
    for problem in problems:
        print(f"NEXT-STEP: {problem}", file=sys.stderr)
    if problems:
        return 1
    if args.check:
        print(f"docs/next-step.md is {len(text.encode('utf-8')):,} bytes, under the {CAP:,} cap, headed by the open step")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
