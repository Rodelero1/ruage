"""**The bank is 200k tokens; nobody can load it. This makes it a page.**

The owner, 2026-11-22: *"Is too many decisions causing problems with context being filled and starting to forget?"* - yes, and
the failure is not memory but *discovery*: `docs/archive/decisions-2026-11-24.md` is ~833 KB (~210k tokens, larger than a context window), so a
session can only grep it, and a ruling it does not already suspect exists is a ruling it will not apply. The same session then
re-decides what was decided in November, or implements a rule at one of four call sites.

So this generates `docs/decision-index.md`: **one line per row** - id, claim, verdict, source, fixture - grouped by area, plus
the lists that matter (rows with a claims-the-game verdict and no source, rows naming no fixture, rows superseded). A session
reads that page instead of the bank, and greps the bank for the ids it needs.

Areas are a **heuristic** over each row's title and body, not a taxonomy; the point is a page you can scan, not a filing system.

Usage:
    python3 tools/decision_index.py            # write docs/decision-index.md
    python3 tools/decision_index.py --print    # print it instead
    python3 tools/decision_index.py --summary  # counts only

Advisory, never a gate (D209's lesson: a gate over prose teaches a fake line).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DECISIONS = ROOT / "docs" / "archive" / "decisions-2026-11-24.md"
OUT = ROOT / "docs" / "decision-index.md"

# One parser, not two: the conformance verdict, the claim and the source pointer are `conformance_audit`'s definitions.
sys.path.insert(0, str(HERE))
import conformance_audit as audit  # noqa: E402  (a sibling script, deliberately shared)

ROW = re.compile(r"^(#{2,3}) (D\w+) - (.+)$", re.MULTILINE)
HEADING = re.compile(r"^#{1,3} ", re.MULTILINE)
DATE = re.compile(r"(20\d\d-\d\d-\d\d)")
DECLARED = re.compile(r"fixture:([^\n]{4,160})", re.IGNORECASE)

FIXTURE = re.compile(
    r"(?:tests/)?([a-z0-9_]{4,})\.rs\b"       # `foo.rs`, with or without `tests/`
    r"|\btests/([a-z0-9_]{4,})\b"             # `tests/foo`, a file named without its extension
    r"|\bfixture:\s*([a-z0-9_,\s]{4,})"      # the convention this tool documents: `Fixture: a, b`
)
# `Superseded by D200`, `Superseded in part by D239 and D241`: the row says a *part* of it is gone, which is the case that
# matters most - a ruling that reads as live while only half of it is.
SUPERSEDED = re.compile(r"supersed\w*(?:\s+in\s+part)?\s+by\s+(D\w+)", re.IGNORECASE)
SUPERSEDES = re.compile(r"\bsupersed\w*\s+(?!by\b)(D\d+)", re.IGNORECASE)

# Title/body keywords -> the page's groups. **Scored, not first-match**, and a tie is broken by this order - which is why
# `Economy` is listed after `Buildings`: "a drop site" is an economy phrase that contains the word "site".: a title hit weighs 5 and a body hit 1, because
# "body", "place" and "step" appear in every other row's prose and first-match filing put 261 of 340 rows under one area.
AREAS: list[tuple[str, str]] = [
    ("Combat & units", r"\battack|damage|armou?r|pierce|melee|projectile|arrow|siege|militar\w*|\bunit|hit point|"
                       r"line of sight|conversion|monk|heal|garrison|stance|patrol|ballistic|accuracy"),
    ("Economy & gathering", r"gather|resource|wood|gold|\bfood|stone|\bfarm|drop site|drop-off|carry|bank|"
                            r"villager|hunt|forage|fishing|economy|stockpile|trade|market|pile|berry|tree\b"),
    ("Buildings & construction", r"\bbuild|construction|foundation|\bsite|house|town cent|footprint|repair|\bwall|"
                                 r"\bgate|pack|unpack|placement"),
    ("Tech, ages & content", r"\btech|research|\bage\b|upgrade|civilisation|civilization|bonus|\beffect|\bpolicy|"
                             r"content pack|unit line|\bdat\b|generator"),
    ("Movement & pathfinding", r"pathfind|path finding|\bpath\b|collision|waypoint|\broute|movement|\bmotion|"
                               r"standoff|\bbump|crowd|overlap|\bnudge|formation|collider|avoidance|"
                               r"smooth|refine"),
    ("Simulation core", r"\btick|fixed point|determin\w*|\bhash|\bseed|snapshot|save game|replay|\bclock|"
                        r"\border|command|queue|schedul|random|budget|outcome|victory|defeat"),
    ("AI & scenario", r"\bAI\b|scenario|\brule|objective|\bteam|quota|acceptance|skirmish|difficulty"),
    ("Rendering & UI", r"render|sprite|graphic|\bUI\b|button|panel|theme|colour|color|sound|audio|camera|\bfog"),
    ("Process & tooling", r"review|\btest|fixture|\btool|audit|document|decision|process|hook|commit|branch|"
                          r"skill|probe|bench|clippy|\bfmt\b|verify|ledger"),
]

CLAIM_CHARS = 150


def area_of(title: str, body: str) -> str:
    """The area with the most keyword evidence: a title hit is worth five body hits."""
    heading = title.lower()
    prose = body[:800].lower()
    best, best_score = "Other", 0
    for name, pattern in AREAS:
        score = 5 * len(re.findall(pattern, heading, re.IGNORECASE)) + len(re.findall(pattern, prose, re.IGNORECASE))
        if score > best_score:
            best, best_score = name, score
    return best


def first_sentence(body: str) -> str:
    """The row's own first claim, in its own words - the line a scanning reader needs."""
    for raw in body.splitlines():
        line = raw.strip()
        if not line or line.startswith(("**Conformance**", "*20", ">", "|", "#", "-", "*")):
            continue
        # Markdown emphasis and code ticks read as noise in a one-line index.
        line = re.sub(r"[`*_]", "", line)
        line = re.sub(r"\((?:[^()]*)\)", "", line)
        line = re.sub(r"\s+", " ", line).strip()
        if len(line) < 20:
            continue
        if len(line) > CLAIM_CHARS:
            cut = line[:CLAIM_CHARS]
            line = cut[: cut.rfind(" ")] + " ..."
        return line
    return "(no prose - heading only)"


def rows(text: str):
    marks = list(ROW.finditer(text))
    for i, mark in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(text)
        body = text[mark.end() : end]
        # Cut the body at the next heading of any level, so a row does not swallow a section header's prose.
        cut = HEADING.search(body)
        if cut:
            body = body[: cut.start()]
        yield mark.group(2), mark.group(3).strip(), body


def parse(text: str) -> tuple[list[dict], dict[str, str], dict[str, list[str]]]:
    """Every row of the bank as one dict: id, title, claim, verdict, source, fixtures, area, date.

    Pure over the text, so `tools/test_decision_index.py` can pin the parsing without a 832k-character corpus.
    """
    entries = []
    superseded_by: dict[str, str] = {}
    supersedes: dict[str, list[str]] = {}
    for ident, title, body in rows(text):
        verdict = audit.TAG.search(body)
        kind = verdict.group(1).upper() if verdict else None
        where = (verdict.group(2) or "").strip() if verdict else ""
        claims_game = bool(audit.CLAIM.search(title + body[:1200]))
        sourced = bool(audit.SOURCE.search(body))
        # Markdown emphasis is stripped first, so a `**Fixture**: <name>` line reads the same as `Fixture: <name>`.
        probe = body.replace("*", "").replace("`", "")
        # **A declared fixture wins over a path.** `Fixture: a_nearer_wall_wins_over_a_nearer_anchor (crates/sim/tests/gather.rs)`
        # names the test; the path only names the file it lives in, and an index that answers "gather" for every row in that
        # file answers nothing.
        # A test's name, not any word on the line: fixture functions are snake_case, so the underscore is the filter that
        # keeps "outside" and "pending" out of a column that is meant to name tests.
        declared = [
            word
            for line in DECLARED.findall(probe)
            for word in re.findall(r"\b[a-z0-9]+_[a-z0-9_]{4,}\b", line)
        ]
        fixtures: list[str] = list(declared)
        for found in ([] if declared else FIXTURE.findall(probe)):
            name = next((part for part in found if part), "")
            for name in name.replace(",", " ").split():
                if name not in {"lib", "mod", "main"} and name not in fixtures:
                    fixtures.append(name)
        fixtures.sort()
        date = DATE.search(body)
        for target in SUPERSEDED.findall(body):
            superseded_by[ident] = target
        for target in SUPERSEDES.findall(title + body):
            supersedes.setdefault(target, []).append(ident)
        entries.append(
            {
                "id": ident,
                "title": title,
                # **The row's own title is the line**, not its first sentence: a body often opens mid-argument
                # ("bar can be removed."), and a scanning reader needs the heading's claim.
                "claim": title if len(title) >= 12 else f"{title} - {first_sentence(body)}",
                "kind": kind,
                "where": where,
                "sourced": sourced,
                "fixtures": fixtures,
                "area": area_of(title, body),
                "date": date.group(1) if date else "",
                "claims_game": claims_game,
            }
        )

    return entries, superseded_by, supersedes


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print", action="store_true", dest="show")
    parser.add_argument("--summary", action="store_true")
    args = parser.parse_args()

    text = DECISIONS.read_text(encoding="utf-8")
    entries, superseded_by, supersedes = parse(text)

    groups: dict[str, list[dict]] = {}
    for entry in entries:
        groups.setdefault(entry["area"], []).append(entry)
    for group in groups.values():
        group.sort(key=lambda e: (len(e["id"]), e["id"]))

    silent = [e for e in entries if e["kind"] is None]
    game_without_source = [e for e in entries if e["claims_game"] and not e["sourced"]]
    no_fixture = [e for e in entries if not e["fixtures"]]
    marked_superseded = sorted(superseded_by.items())

    out: list[str] = []
    out.append("<!-- Generated by tools/decision_index.py. Do not edit: run `make decision-index`. -->")
    out.append("")
    out.append("# Decision index")
    out.append("")
    out.append(
        f"**{len(entries)} rows.** `docs/archive/decisions-2026-11-24.md` is ~{len(text) // 1000}k characters and cannot be loaded whole; this page "
        "is what a session reads instead, then greps the bank for the ids it needs."
    )
    out.append("")
    out.append(
        f"- **{len(silent)}** rows carry no `Conformance` verdict (they neither claim the game nor say the choice is ours)."
    )
    out.append(f"- **{len(game_without_source)}** rows read as a claim about the game with **no source pointer**.")
    out.append(f"- **{len(no_fixture)}** rows name no fixture - the \"believed implemented\" list, where D150's pile rule lived.")
    out.append(f"- **{len(marked_superseded)}** rows say they are superseded.")
    out.append("")
    out.append("Areas are a keyword heuristic over each row; the point is a page you can scan, not a taxonomy.")
    out.append("")

    for area in [name for name, _ in AREAS] + ["Other"]:
        group = groups.get(area)
        if not group:
            continue
        out.append(f"## {area} ({len(group)})")
        out.append("")
        for entry in group:
            verdict = entry["kind"] or "—"
            if entry["kind"] == "GAME":
                verdict = "GAME" if entry["sourced"] else "GAME **no source**"
            bits = [f"**{entry['id']}** — {entry['claim']}", verdict]
            if entry["fixtures"]:
                bits.append("`" + "`, `".join(entry["fixtures"][:2]) + "`")
            if entry["id"] in superseded_by:
                bits.append(f"**superseded by {superseded_by[entry['id']]}**")
            elif entry["id"] in supersedes:
                bits.append(f"supersedes {', '.join(supersedes[entry['id']])}")
            out.append("- " + " · ".join(bits))
        out.append("")

    if game_without_source:
        out.append("## Rows that claim the game with no source (burn-down)")
        out.append("")
        for entry in game_without_source:
            out.append(f"- **{entry['id']}** — {entry['title']}")
        out.append("")

    page = "\n".join(out).rstrip() + "\n"
    if args.summary:
        print(f"rows={len(entries)} silent={len(silent)} game_no_source={len(game_without_source)} "
              f"no_fixture={len(no_fixture)} superseded={len(marked_superseded)}")
    elif args.show:
        print(page)
    else:
        OUT.write_text(page, encoding="utf-8")
        print(f"wrote {OUT.relative_to(ROOT)}: {len(entries)} rows, {len(page) // 1000}k characters, "
              f"{len(groups)} areas, no_fixture={len(no_fixture)}, game_no_source={len(game_without_source)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
