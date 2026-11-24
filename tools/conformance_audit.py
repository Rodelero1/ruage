"""**Which rulings still claim the game without a source?** A burn-down list, not a gate.

The owner, 2026-11-22: *"purge our rulings which are not conformant with original game, decomp or online sources once and for
all"*. The failure this measures is specific: a decision row that reads as a fact about AoE2 ("the original does X", "the game's
own answer is Y") while the row carries no pointer to the thing that settles it - a binary address, a `FUN_` name, a decomp
file, a quoted URL, or a test that pins an observed behaviour.

A row that is plainly **our choice** ("teams are content", "the UI issues orders") is not a defect and needs no source; what it
needs is to *say* it is ours. So each row may carry one tag line:

    **Conformance**: GAME - <the pointer that settles it>
    **Conformance**: OURS - <why we diverge, in one clause>
    **Conformance**: UNVERIFIED - <what would settle it>

Usage:
    python3 tools/conformance_audit.py            # the rows that need a verdict, by area
    python3 tools/conformance_audit.py --all      # every row and its tag
    python3 tools/conformance_audit.py --check    # nonzero while any game claim lacks a source (burn-down)
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DECISIONS = ROOT / "docs" / "archive" / "decisions-2026-11-24.md"

ROW = re.compile(r"^## (D\w+) - (.+)$", re.MULTILINE)
TAG = re.compile(r"^\*\*Conformance\*\*:\s*(GAME|OURS|UNVERIFIED)\b(.*)$", re.MULTILINE | re.IGNORECASE)

# A phrase that turns a sentence into a claim about the original game rather than about this engine.
CLAIM = re.compile(
    r"the original|the game'?s? own|in the game\b|the binary|the decomp|decompilation|"
    r"as (?:the|in the) game|AoE2|Age of Empires|\bHD\b|the real game|the actual game",
    re.IGNORECASE,
)

# **Evidence about the game**, as opposed to a pointer into our own tree. A row may cite `crates/sim/src/lib.rs` for where the
# code lives and still be asserting a fact about AoE2 with nothing behind it - that is the failure this list exists to find.
SOURCE = re.compile(
    r"0x[0-9a-f]{3,}"                       # a binary address, object offset or field
    r"|FUN_[0-9a-f]{8}"                     # a function in the decompilation corpus
    r"|func_[0-9A-F]{6,}"                   # a function in samuelcaldas' decomp
    r"|https?://"                           # a published source
    r"|\.(?:cpp|h)\b"                      # a decomp or install source file
    r"|docs/(?:prior-art|sources)\.md"  # entry files that stayed at root
    r"|docs/archive/research/(?:movement-in-the-binary|gather-seating-evidence|pathfinding-[a-z-]+|oracle|"
    r"parse-notes)\.md"  # evidence files moved to the archive (Phase A)
    r"|\bmeasured\b"                       # a number this tree took
    r"|`[A-Za-z_]+`\s*\(\d"                # a call quoted with its arguments
    r"|build-order guide|wiki|forum|talk|slide"  # a published source named in prose
    r"|\bmanual\b|Liquipedia|Fandom|the dat|the pack|genie|openage|openaok|aoe2mc|replay|"
    r"screenshot|string table|task_info|search_radius|hit points"
    ,
    re.IGNORECASE,
)

# A row whose basis is the owner's word is **ours by choice**, and says so - that is not a fake fact about the game.
OWNER = re.compile(r"\bthe owner\b|owner(?:'s)? (?:rule|ruling|answer|decision|instruction|words)|\*\*owner", re.IGNORECASE)


def rows(text: str) -> list[tuple[str, str, str]]:
    """Every decision row as `(id, title, body)`."""
    starts = [(m.start(), m.group(1), m.group(2).strip()) for m in ROW.finditer(text)]
    out = []
    for index, (start, did, title) in enumerate(starts):
        end = starts[index + 1][0] if index + 1 < len(starts) else len(text)
        out.append((did, title, text[start:end]))
    return out


def audit(text: str) -> list[dict]:
    findings = []
    for did, title, body in rows(text):
        tag = TAG.search(body)
        claim = CLAIM.search(body)
        sourced = SOURCE.search(body)
        findings.append(
            {
                "id": did,
                "title": title,
                "tag": tag.group(1).upper() if tag else None,
                "tag_note": (tag.group(2).strip(" -") if tag else ""),
                "claims": bool(claim),
                "claim": claim.group(0) if claim else "",
                "sourced": bool(sourced),
            }
        )
    return findings


def needs_verdict(finding: dict) -> bool:
    """A row that talks about the game, carries no tag, and points at nothing."""
    return finding["claims"] and not finding["sourced"] and finding["tag"] is None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="every row, with its tag")
    parser.add_argument("--check", action="store_true", help="nonzero while any game claim lacks a source")
    arguments = parser.parse_args()

    findings = audit(DECISIONS.read_text(encoding="utf-8"))
    pending = [f for f in findings if needs_verdict(f)]
    tagged = {
        "GAME": sum(1 for f in findings if f["tag"] == "GAME"),
        "OURS": sum(1 for f in findings if f["tag"] == "OURS"),
        "UNVERIFIED": sum(1 for f in findings if f["tag"] == "UNVERIFIED"),
    }
    if arguments.all:
        for finding in findings:
            state = finding["tag"] or ("CITED" if finding["sourced"] else ("CLAIM-ONLY" if finding["claims"] else "-"))
            print(f"{finding['id']:>6}  {state:<10} {finding['title'][:96]}")
        return 0

    print(
        f"{len(findings)} rows: GAME {tagged['GAME']}, OURS {tagged['OURS']}, UNVERIFIED {tagged['UNVERIFIED']}, "
        f"cited-or-silent {len(findings) - len(pending) - sum(tagged.values())}, "
        f"**pending {len(pending)}**"
    )
    for finding in pending:
        print(f"{finding['id']:>6}  ({finding['claim']})  {finding['title'][:88]}")
    if arguments.check:
        return 1 if pending else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
