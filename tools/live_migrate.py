"""Shard the decision bank into per-area live files — mechanically, never by rewording.

`docs/archive/decisions-2026-11-24.md` holds every ruling in one ~10k-line file; no session can load it.
This reads the corpus with `tools/decisions.py` (the same parser the site index uses),
groups by the authored `docs/decision-areas.toml`, and writes `docs/live/<area>.md` —
one file per area, each decision verbatim plus scaffolding (area, status, conformance
flag). A decision with no area is refused, never bucketed; two sections sharing one id
are both emitted, never merged.

Usage:
    python3 tools/live_migrate.py --check    # plan only; nonzero on refusal
    python3 tools/live_migrate.py            # write docs/live/<slug>.md
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DECISIONS = ROOT / "docs" / "archive" / "decisions-2026-11-24.md"
AREAS_FILE = ROOT / "docs" / "decision-areas.toml"
UNKNOWNS = ROOT / "docs" / "mechanics" / "README.md"
OUT_DIR = ROOT / "docs" / "live"

sys.path.insert(0, str(HERE))
import conformance_audit as audit  # noqa: E402 (sibling script, deliberately shared)
import decisions  # noqa: E402


def build_plan(
    corpus: decisions.Corpus, order: list[str], mapping: dict[str, str]
) -> tuple[dict[str, list[decisions.Decision]], list[str]]:
    """Group decisions by authored area. Refusals, never buckets: anything unfiled
    is a problem naming the id, and unfiled decisions are not emitted."""
    problems: list[str] = []
    for identifier in sorted(
        corpus.decisions, key=lambda i: corpus.decisions[i].number
    ):
        if identifier not in mapping:
            problems.append(f"{identifier} has no area in docs/decision-areas.toml")
    for identifier in sorted(mapping):
        if identifier not in corpus.decisions:
            problems.append(
                f"docs/decision-areas.toml names {identifier}, "
                "which no decision defines"
            )
    grouped: dict[str, list[decisions.Decision]] = {name: [] for name in order}
    for decision in sorted(corpus.decisions.values(), key=lambda d: d.number):
        area = mapping.get(decision.id)
        if area is None:
            continue
        grouped.setdefault(area, []).append(decision)
    plan = {name: grouped[name] for name in order if grouped.get(name)}
    for name in grouped:
        if name not in plan and grouped[name]:
            plan[name] = grouped[name]
    return plan, problems


def render_decision(
    decision: decisions.Decision,
    corpus: decisions.Corpus,
    area: str,
    bank_titles: dict[str, list[str]] | None = None,
) -> str:
    """One decision, verbatim, plus scaffolding. Amendments (later substance under the
    same id) follow the body; a bank numbering collision (two sections, two titles,
    one id) renders as distinct live IDs `D<na>`/`D<nb>` instead. A game claim with
    no source pointer is flagged."""
    titles = (bank_titles or {}).get(decision.id, [decision.title])
    if len(titles) > 1:
        return _render_collision(decision, corpus, area, titles)
    lines = [f"### {decision.id} - {decision.title}", ""]
    lines = [f"### {decision.id} - {decision.title}", ""]
    if decision.date:
        lines += [f"*Date:* {decision.date}", ""]
    lines += [f"*Area:* {area}", f"*Status:* {decisions.status(decision, corpus)}", ""]
    lines += [decision.text, ""]
    for pos, amendment in enumerate(corpus.amendments.get(decision.id, []), 1):
        lines += [
            f"#### Amendment {pos} (bank held two sections under one id)",
            "",
            amendment,
            "",
        ]
    owner_tag = re.search(
        r"^\*\*Conformance\*\*:\s*OWNER\b",
        decision.text,
        re.MULTILINE | re.IGNORECASE,
    )
    if owner_tag:
        lines += [
            "*Live verdict:* OURS — owner ruling, normalized from the bank's "
            "nonstandard OWNER tag (C15). Schema allows GAME/OURS/UNVERIFIED only.",
            "",
        ]
    elif not audit.TAG.search(decision.text):
        haystack = f"{decision.title}\n{decision.text}"
        if audit.CLAIM.search(haystack) and not audit.SOURCE.search(haystack):
            lines += [
                "**Conformance needed**: claims the game with no source pointer "
                "— UNVERIFIED until one is named.",
                "",
            ]
    return "\n".join(lines).rstrip() + "\n"


def section_titles(text: str) -> dict[str, list[str]]:
    """Every decision-kind `## D<n> - <title>` section title in bank order. Two
    distinct titles under one id is a numbering collision (C13), not an amendment:
    amendment/addendum/note subsections are the parser's, not second decisions."""
    found: dict[str, list[str]] = {}
    for match in re.finditer(r"(?m)^#{2,3} +(D\d{1,4}[a-z]?)\b\s*(.*)$", text):
        if decisions._heading_kind(match.group(2)) != "decision":
            continue
        found.setdefault(match.group(1), []).append(match.group(2).strip(" -–—"))
    return found


def _render_collision(
    decision: decisions.Decision,
    corpus: decisions.Corpus,
    area: str,
    titles: list[str],
) -> str:
    """Two bank sections, two titles, one id: distinct live IDs, both bodies kept."""
    bodies = [decision.text] + corpus.amendments.get(decision.id, [])
    parts = []
    for pos, title in enumerate(titles):
        live_id = f"{decision.id}{chr(ord('a') + pos)}"
        body = bodies[pos] if pos < len(bodies) else "*body missing from parse*"
        parts += [
            f"### {live_id} - {title} (bank id {decision.id})",
            "",
            f"*Area:* {area}",
            f"*Status:* live (bank numbering collision — see C13; "
            f"bank status was {decisions.status(decision, corpus)})",
            "",
            body,
            "",
        ]
    return "\n".join(parts).rstrip() + "\n"


def render_area(
    area: str,
    items: list[decisions.Decision],
    corpus: decisions.Corpus,
    open_questions: dict[str, str],
    bank_titles: dict[str, list[str]] | None = None,
) -> str:
    lines = [
        f"# Live — {area}",
        "",
        "<!-- Generated by tools/live_migrate.py. Do not hand-edit: "
        "fix the bank row and re-run. -->",
        "",
        f"**{len(items)} rulings.** Status and conformance scaffolding are generated; "
        "bodies are the bank's words verbatim.",
        "",
    ]
    for decision in items:
        lines.append(render_decision(decision, corpus, area, bank_titles))
        lines.append("")
    if open_questions:
        lines += ["## Open questions (U-register, this area)", ""]
        for number in sorted(open_questions, key=lambda n: int(n[1:])):
            lines += [f"- **{number}** — {open_questions[number]}", ""]
    return "\n".join(lines)


def main(check: bool = False) -> int:
    text = DECISIONS.read_text()
    corpus = decisions.parse_decisions(text)
    titles = section_titles(text)
    order, mapping, questions = decisions.load_areas(AREAS_FILE.read_text())
    unknowns = decisions.parse_unknowns(UNKNOWNS.read_text())
    open_by_area: dict[str, dict[str, str]] = {}
    for number, area in questions.items():
        item = unknowns.get(number)
        if item is not None and not item.closed:
            open_by_area.setdefault(area, {})[number] = item.question
    plan, problems = build_plan(corpus, order, mapping)
    if check:
        for problem in problems:
            print(problem)
        total = sum(len(v) for v in plan.values())
        print(f"{total} decisions placed in {len(plan)} areas")
        return 1 if problems else 0
    OUT_DIR.mkdir(exist_ok=True)
    for area, items in plan.items():
        (OUT_DIR / f"{decisions.slug(area)}.md").write_text(
            render_area(area, items, corpus, open_by_area.get(area, {}), titles)
        )
    for problem in problems:
        print(f"REFUSED: {problem}", file=sys.stderr)
    total = sum(len(v) for v in plan.values())
    print(f"wrote {len(plan)} area files, {total} decisions")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    sys.exit(main(check=args.check))
