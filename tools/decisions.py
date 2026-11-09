"""The decision index: what was decided, how, and what is still open (D165).

`docs/decisions.md` is 4,000 lines of prose with two different shapes (a table for D1-D16, `## D<n>` sections after
that), and nothing in it says which rules rest on the dat, which on the owner's word, which on a guess, or which
were rewritten a week later. This tool reads the corpus and *says* those things — and it says them with the phrase it
read them from, so a wrong label is visible rather than authoritative.

Two rules it follows, both borrowed from the project's own:

* **Refuse, do not bucket.** A decision heading it cannot place is an error naming the heading, not a row called
  "other" (D6, D45: a surface that silently ignores is worse than one that refuses).
* **An absence is a finding.** A decision with no citation at all is the reliable signal, so it is a stated category
  rather than a gap in the classifier.

    python3 tools/decisions.py --check      # parse and report coverage; nonzero if anything is unplaced
    python3 tools/decisions.py              # write docs/site/index.html
"""

from __future__ import annotations

import argparse
import html
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
DECISIONS = ROOT / "docs" / "decisions.md"
UNKNOWNS = ROOT / "docs" / "mechanics" / "README.md"
OUTPUT = ROOT / "docs" / "site" / "index.html"

# Where a rule came from. Each entry is a list of patterns; the matched sentence becomes the evidence shown next to the
# label, so the reader can disagree with the classifier without re-reading 4,000 lines.
ORIGINS: dict[str, list[str]] = {
    "owner": [
        r"owner'?s\s+(?:decision|words|ask|answer|rule|question|suggestion|correction|requirement|statement|choice|"
        r"long-term plan|intuition)",
        r"\bthe owner\s+(?:said|chose|asks?|asked|wants?|wanted|decided|answered|corrected|confirmed|reversed|"
        r"accepted|refused|rejected|pointed)",
        r"\bowner chose\b",
        r"\bper the owner\b",
        r"\bthe owner\s*:",          # `The owner: "..."` is the corpus's usual way of quoting him
        r"\bhis answers?\b",
        r"\bthe owner'?s answers?\b",
    ],
    "dat": [
        r"\bthe dat\b",
        r"\bdat'?s\b",
        r"\bdat field",
        r"\bin the (?:unit|tech|effect) table\b",
    ],
    "binary": [
        r"\bthe (?:original|game)'?s? (?:binary|exe|executable)\b",
        r"\bexe\b",
        r"\bdecompil",
        r"\bdisassembl",
        r"\bx64dbg\b",
        r"\bghidra\b",
        r"\bbreakpoint\b",
        r"\breverse[- ]engineer",
        r"\bmachine code\b",
    ],
    "measured": [
        r"\bmeasured\b",
        r"\b\d[\d,]* of the [\d,]+ (?:units|techs|effects|buildings|commands|tiles)\b",  # a census
        r"\bmeasurement\b",
        r"\bcensus\b",
        r"\bprobe\b",
        r"\btimed\b",
        r"\bbenchmark",
    ],
    "published": [
        r"\bwiki\b",
        r"\bthe manual\b",
        r"\bpublished\b",
        r"\bfandom\b",
        r"\baoe2database\b",
        r"\bforums?\b",
        r"\bofficial (?:strategy )?guide\b",
    ],
    "prior-art": [
        r"\bopenage\b",
        r"\bopenaok\b",
        r"\blibrematch\b",
        r"\bgenie-rs\b",
        r"\bgenie-core\b",
        r"\bprior[- ]art\b",
        r"\bgenie'?s\b",
        r"\bgenie/dat\b",
    ],
    "decision": [
        r"\bunder (?:the )?(?:\*\*)?D\d{1,4}\b",
        r"\bper (?:the )?(?:\*\*)?D\d{1,4}\b",
        r"\bD\d{1,4}(?:'s)?\s+(?:rule|schema|identity|boundary|domain|reasoning|commitment)",
        r"\bD\d{1,4}\s+(?:already|promised|requires|required|says|said|set|sets|chose|buys|forbids|puts)\b",
        r"\bdecided in (?:the )?(?:\*\*)?D\d{1,4}\b",
    ],
    "review": [
        r"\badversarial review\b",
        r"\breview finding\b",
        r"\bmutation\b",
        r"\ba review of\b",
        r"\bthe review\b",
    ],
}

# Language that admits the rule is not settled. It is a reading list, not a verdict: the snippet is shown so the reader
# can see whether the doubt is about this rule or about a neighbour.
UNRESOLVED = [
    r"hypothes",
    r"\bguess(?:ing|ed)?\b",
    r"\bnot established\b",
    r"\bunverified\b",
    r"\bplausibl[ey]\b",
    r"\bunread\b",
    r"\bopen question\b",
    r"\bno evidence\b",
    r"\bnothing .{0,40}corroborates\b",
    r"\bnot interpreted\b",
    r"\bnamed rather than\b",
    r"\bwould be inventing\b",
    r"\bcannot be decided\b",
]
# A doubt that has already been settled is not a doubt. "The construction hypothesis was tested and is falsified" is a
# conclusion, and flagging it as unsettled would make the whole column noise.
SETTLED = re.compile(r"falsif|disprov|refut|\bwrong\b|is not a hypothesis", re.IGNORECASE)

GAP_MARKERS = re.compile(
    r"\bNot modelled\b|\bNot done\b|\bNot settled\b|\bNamed limits\b|\bKnown simplifications?\b|"
    r"\bnot implemented\b|\bWhat is not\b|\bStill not (?:modelled|reported|done)\b|\bNot yet (?:done|modelled)\b",
    re.IGNORECASE,
)

D_IN_TEXT = re.compile(r"\bD(\d{1,4})([a-z]?)(?!\d)")
CORRECTS = [
    re.compile(r"correct(?:ing|s|ed)\s+(?:the\s+)?(?:\*\*)?D(\d{1,4})([a-z]?)"),
    re.compile(r"\bD(\d{1,4})([a-z]?)(?:'s)?\s+(?:refusal|premise|clause|claim|rule|reading|version)\b[^.]{0,80}?\b(?:wrong|corrected)"),
    re.compile(r"\bD(\d{1,4})([a-z]?)(?:\*\*)?\s+(?:was|is|had been)\s+(?:simply\s+)?wrong\b"),
    re.compile(r"\bD(\d{1,4})([a-z]?)(?:\*\*)?\s+is corrected\b"),
    re.compile(r"(?:review|re-reading|re-read) of (?:\*\*)?D(\d{1,4})([a-z]?)[^.]{0,160}?\bwrong\b"),
    re.compile(r"correcting\s+(?:the\s+)?earlier\s+(?:\*\*)?D(\d{1,4})([a-z]?)"),
]
SUPERSEDES = [
    re.compile(r"[Ss]upersedes?\s+(?:the\s+)?(?:\*\*)?D(\d{1,4})([a-z]?)"),
    re.compile(r"\bD(\d{1,4})([a-z]?)(?:'s)?\s+(?:clause\s+)?is superseded\b"),
    re.compile(r"[Ss]uperseded by\s+(?:\*\*)?D(\d{1,4})([a-z]?)"),
]

@dataclass
class Decision:
    """One rule, as the corpus states it."""

    id: str
    title: str = ""
    date: str | None = None
    text: str = ""
    origins: dict[str, list[str]] = field(default_factory=dict)
    unresolved: list[str] = field(default_factory=list)
    gaps: list[str] = field(default_factory=list)
    corrected_by: list[str] = field(default_factory=list)
    superseded_by: list[str] = field(default_factory=list)

    @property
    def uncited(self) -> bool:
        """No basis language anywhere: the reliable signal, because it needs no pattern to be right."""
        return not self.origins

    @property
    def number(self) -> tuple[int, str]:
        match = re.match(r"D(\d+)([a-z]?)", self.id)
        assert match
        return int(match.group(1)), match.group(2)


@dataclass
class Unknown:
    """A row of the U-register: a question, and whether it has been answered."""

    number: str
    question: str = ""
    closed: bool = False


@dataclass
class Ref:
    """A mention of a decision id in a file that is not the corpus."""

    id: str
    path: str
    line: int
    snippet: str = ""


@dataclass
class Corpus:
    decisions: dict[str, Decision] = field(default_factory=dict)
    amendments: dict[str, list[str]] = field(default_factory=dict)
    versions: dict[str, int] = field(default_factory=dict)
    notes: list[tuple[str, str]] = field(default_factory=list)
    structural: list[str] = field(default_factory=list)
    unclassified: list[str] = field(default_factory=list)


SNIPPET_WINDOW = 400


def _snippet(text: str, match: re.Match, limit: int = 240) -> str:
    """The sentence a pattern matched in, so the label can be checked.

    The search is windowed, and that is not a micro-optimisation: the first version scanned from the start of the
    section to the match for every match, which is quadratic in section length - a 300 KB section took **6.4 s** and a
    1 MB one **67 s**. The sentence around a match is never further away than a paragraph.
    """
    left_bound = max(0, match.start() - SNIPPET_WINDOW)
    right_bound = min(len(text), match.end() + SNIPPET_WINDOW)
    left = max(text.rfind(". ", left_bound, match.start()), text.rfind("\n\n", left_bound, match.start()))
    start = left + 1 if left != -1 else left_bound
    right = [index for index in (text.find(". ", match.end(), right_bound), text.find("\n\n", match.end(), right_bound)) if index != -1]
    end = min(right) if right else right_bound
    sentence = text[start : end + 1].strip() or text[match.start() : min(len(text), match.end() + 80)].strip()
    return re.sub(r"[*`]", "", " ".join(sentence.split()))[:limit]


def _compile(patterns: dict[str, list[str]] | list[str]):
    """Patterns are compiled once at import: `re.finditer` with a string re-enters the cache on every call."""
    if isinstance(patterns, dict):
        return {name: [re.compile(p, re.IGNORECASE) for p in group] for name, group in patterns.items()}
    return [re.compile(p, re.IGNORECASE) for p in patterns]


ORIGINS = _compile(ORIGINS)
UNRESOLVED = _compile(UNRESOLVED)


def classify_basis(text: str) -> tuple[dict[str, list[str]], list[str]]:
    """(origins with evidence, unresolved phrases) for one decision's own text."""
    origins: dict[str, list[str]] = {}
    for origin, patterns in ORIGINS.items():
        found: list[str] = []
        for pattern in patterns:
            if len(found) >= 3:
                break
            for match in pattern.finditer(text):
                snippet = _snippet(text, match)
                if snippet and snippet not in found:
                    found.append(snippet)
                if len(found) >= 3:
                    break
        if found:
            origins[origin] = found[:3]

    # A sentence that names three or more origins is *about* provenance - a taxonomy, a summary, this tool's own
    # explanation - and counting it makes a decision cite everything. Found on D165, which lists the tag vocabulary
    # and was consequently tagged with eight of them.
    counts: dict[str, set[str]] = {}
    for origin, snippets in origins.items():
        for snippet in snippets:
            counts.setdefault(snippet, set()).add(origin)
    origins = {
        origin: [snippet for snippet in snippets if len(counts[snippet]) < 3]
        for origin, snippets in origins.items()
    }
    # Inside a sentence that quotes the owner, the *categories* are his vocabulary, not our method: D165 quotes him
    # listing "research, reverse engineered, assumed" and was tagged `binary` by his own word.
    for origin, snippets in list(origins.items()):
        if origin == "owner":
            continue
        origins[origin] = [
            snippet for snippet in snippets
            if not ('"' in snippet and "owner" in counts.get(snippet, set()))
        ]
    origins = {origin: snippets for origin, snippets in origins.items() if snippets}

    unresolved: list[str] = []
    for pattern in UNRESOLVED:
        if len(unresolved) >= 4:
            break
        for match in pattern.finditer(text):
            snippet = _snippet(text, match)
            if SETTLED.search(snippet):
                continue
            if snippet and snippet not in unresolved:
                unresolved.append(snippet)
            if len(unresolved) >= 4:
                break
    return origins, unresolved[:4]


def gaps_in(text: str) -> list[str]:
    """Every paragraph that names what this decision does not do."""
    out: list[str] = []
    for paragraph in re.split(r"\n\s*\n", text):
        # A heading that happens to say "not modelled" is a title; the gap is the prose under it.
        if paragraph.lstrip().startswith("#"):
            continue
        if not GAP_MARKERS.search(paragraph):
            continue
        flat = " ".join(paragraph.split())
        flat = re.sub(r"[*`]", "", flat)
        if flat and flat not in out:
            out.append(flat[:400])
    return out


def _ids(patterns, text: str) -> list[str]:
    out: list[str] = []
    for pattern in patterns:
        for match in pattern.finditer(text):
            found = f"D{match.group(1)}{match.group(2) or ''}"
            if found not in out:
                out.append(found)
    return out


def _table_decisions(text: str, corpus: Corpus) -> None:
    for line in text.splitlines():
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 3:
            continue
        match = re.fullmatch(r"\*\*(D\d{1,4}[a-z]?)\*\*", cells[0])
        if not match:
            continue
        identifier = match.group(1)
        body = " ".join(cell for cell in cells[2:] if cell)
        date = cells[1] if re.fullmatch(r"20\d\d-\d\d-\d\d", cells[1]) else None
        corpus.decisions[identifier] = _decision(identifier, _title_from_table(body), date, body)
        corpus.versions[identifier] = corpus.versions.get(identifier, 0) + 1


def _title_from_table(body: str) -> str:
    flat = re.sub(r"[*`]", "", body)
    first = flat.split(".")[0]
    # A period inside a quotation leaves the closing quote behind: `... "attainable.` reads as a typo in the page.
    if flat[len(first) :].startswith('.\"'):
        first += '.\"'
    return first.strip()[:120]


def _decision(identifier: str, title: str, date: str | None, body: str) -> Decision:
    origins, unresolved = classify_basis(f"{title}\n\n{body}")
    return Decision(
        id=identifier,
        title=" ".join(title.split()),
        date=date,
        text=body,
        origins=origins,
        unresolved=unresolved,
        gaps=gaps_in(body),
    )


def _heading_kind(rest: str) -> str:
    """What the text after the id makes this heading: a decision, an amendment, an essay, or nothing legible."""
    if re.match(r"^\s*[-–—]\s*\S", rest):
        return "decision"
    if re.match(r"^\s*\((?:amended|resolved)\b", rest) or re.match(r"^\s*amendment\b", rest):
        return "amendment"
    if re.match(r"^\s*(?:applied|applies)\b", rest):
        return "note"
    return "unclassified"


def parse_decisions(text: str) -> Corpus:
    """Every decision in the corpus, every amendment, and every heading that could not be placed."""
    corpus = Corpus()
    _table_decisions(text, corpus)

    lines = text.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        # `##` is the corpus's normal level, but three decisions (D24a, D94a, D98a) were written under their parent at
        # `###`. Both are decisions; a `###` that does not start with an id is prose inside the section and must not cut
        # it short, which is why the terminator test looks at the id and not only at the level.
        if not re.match(r"^#{2,3} ", line):
            index += 1
            continue
        heading = line.split(" ", 1)[1].strip() if " " in line else ""
        end = index + 1
        while end < len(lines):
            following = lines[end]
            if following.startswith("## ") or (
                following.startswith("### ") and re.match(r"### +D\d", following)
            ):
                break
            end += 1
        body = "\n".join(lines[index + 1 : end]).strip()

        match = re.match(r"^(D\d{1,4}[a-z]?)\b(.*)$", heading)
        if not match:
            corpus.structural.append(heading)
            index = end
            continue

        identifier, rest = match.group(1), match.group(2)
        kind = _heading_kind(rest)
        if kind == "unclassified":
            corpus.unclassified.append(f"## {heading}")
        elif kind == "note":
            corpus.notes.append((identifier, heading))
        elif kind == "amendment":
            corpus.amendments.setdefault(identifier, []).append(body)
            corpus.versions[identifier] = corpus.versions.get(identifier, 0) + 1
        else:
            title = re.sub(r"^\s*[-–—]\s*", "", rest).strip()
            date_match = re.search(r"\b(20\d\d-\d\d-\d\d)\b", body)
            existing = corpus.decisions.get(identifier)
            if existing is None:
                corpus.decisions[identifier] = _decision(
                    identifier, title, date_match.group(1) if date_match else None, body
                )
            else:
                # A second decision section for the same id: the later one is the amendment's substance.
                corpus.amendments.setdefault(identifier, []).append(body)
            corpus.versions[identifier] = corpus.versions.get(identifier, 0) + 1
        index = end

    # A correction or suspension is a fact about the *earlier* decision, so it is recorded on that one.
    for decision in corpus.decisions.values():
        for target in _ids(CORRECTS, f"{decision.title}\n{decision.text}"):
            if target != decision.id and target in corpus.decisions:
                corpus.decisions[target].corrected_by.append(decision.id)
        for target in _ids(SUPERSEDES, f"{decision.title}\n{decision.text}"):
            if target != decision.id and target in corpus.decisions:
                corpus.decisions[target].superseded_by.append(decision.id)
    return corpus


def parse_unknowns(text: str) -> dict[str, Unknown]:
    """The U-register: rows that say `U<n>`, closed when struck through or when the row says so."""
    unknowns: dict[str, Unknown] = {}
    row = re.compile(
        r"^\|\s*(?P<strike_a>~~)?\s*(?P<bold_a>\*\*)?\s*U(?P<number>\d+)(?:-original)?"
        r"\s*(?P=bold_a)?\s*(?P=strike_a)?\s*\|(?P<question>[^|]*)\|"
    )
    closed_words = re.compile(r"RESOLVED|ANSWERED|CLOSED|DECIDED|VALUE FIXED", re.IGNORECASE)
    for line in text.splitlines():
        match = row.match(line)
        if not match:
            continue
        struck = bool(match.group("strike_a"))
        identifier = f"U{match.group('number')}"
        question = " ".join(re.sub(r"[*`~]", "", match.group("question")).split())
        unknowns[identifier] = Unknown(
            number=identifier,
            question=question[:400],
            closed=struck or bool(closed_words.search(line)),
        )
    return unknowns


def dangling_refs(defined, texts: dict[str, str]) -> list[Ref]:
    """Mentions of a decision id that the corpus never defines, with the file and line to look at."""
    refs: list[Ref] = []
    for path, text in texts.items():
        for number, line in enumerate(text.splitlines(), start=1):
            for match in D_IN_TEXT.finditer(line):
                identifier = f"D{match.group(1)}{match.group(2) or ''}"
                if identifier in defined:
                    continue
                refs.append(Ref(id=identifier, path=path, line=number, snippet=line.strip()[:160]))
    return refs


GROUP_SIZE = 25


def groups(corpus: Corpus, size: int = GROUP_SIZE) -> list[tuple[str, str, int]]:
    """The register cut into bands, in id order: `(first id, last id, count)` per band.

    Bands are by **position**, not by number range, because the numbering has holes and letter suffixes (D44a) - a band
    of numbers would claim ids that do not exist. The label therefore names what is actually in the band.
    """
    ordered = sorted(corpus.decisions.values(), key=lambda decision: decision.number)
    return [
        (band[0].id, band[-1].id, len(band))
        for band in (ordered[start : start + size] for start in range(0, len(ordered), size))
        if band
    ]


def status(decision: Decision, corpus: Corpus) -> str:
    """What this rule is now: still the rule, amended, corrected, or replaced."""
    if decision.corrected_by:
        return "corrected"
    if decision.superseded_by:
        return "superseded"
    if corpus.versions.get(decision.id, 1) > 1:
        return "amended"
    return "decided"


def _esc(text: str) -> str:
    return html.escape(text, quote=True)


def render(corpus: Corpus, unknowns: dict[str, Unknown], refs: list[Ref], generated: str) -> str:
    """One self-contained page: no fonts, no scripts from anywhere, works from `file://` and from a static host."""
    decisions = sorted(corpus.decisions.values(), key=lambda decision: decision.number)
    open_unknowns = [unknown for unknown in unknowns.values() if not unknown.closed]
    gap_total = sum(len(decision.gaps) for decision in decisions)
    uncited = [decision for decision in decisions if decision.uncited]
    counts = {
        "decisions": len(decisions),
        "amended": sum(1 for decision in decisions if status(decision, corpus) == "amended"),
        "corrected": sum(1 for decision in decisions if decision.corrected_by),
        "superseded": sum(1 for decision in decisions if decision.superseded_by),
        "uncited": len(uncited),
        "gaps": gap_total,
        "undecided": len(open_unknowns),
        "dangling": len({ref.id for ref in refs}),
        "unclassified": len(corpus.unclassified),
        "structural": len(corpus.structural),
        "notes": len(corpus.notes),
    }

    parts: list[str] = []
    add = parts.append

    add(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">"
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
        "<title>Decisions - Ruage</title><style>" + STYLE + "</style></head><body>"
    )
    add("<header><h1>Decisions</h1>")
    add(
        "<p class=\"lede\">Every rule the corpus states, what it rests on, and what it has not decided yet. "
        "Generated from <code>docs/decisions.md</code> and the U-register; nothing on this page is typed by hand, "
        "so a wrong label means the classifier is wrong and can be seen to be wrong.</p>"
    )
    add("<ul class=\"coverage\" id=\"coverage\">")
    for label, key in [
        ("decisions", "decisions"),
        ("amended", "amended"),
        ("corrected", "corrected"),
        ("superseded", "superseded"),
        ("no stated basis", "uncited"),
        ("named gaps", "gaps"),
        ("undecided questions", "undecided"),
        ("dangling references", "dangling"),
        ("unclassified", "unclassified"),
    ]:
        add(f"<li data-k=\"{key}\"><b>{counts[key]}</b> {label}</li>")
    add("</ul>")
    add(
        "<nav><a href=\"#undecided\">Not decided yet</a> <a href=\"#incomplete\">Decided, but not finished</a> "
        "<a href=\"#review\">Review queue</a> <a href=\"#decisions\">The register</a> "
        "<a href=\"#dangling\">Broken references</a> <a href=\"#health\">How this page knows</a></nav></header><main>"
    )

    # ---- the two registers of "not done", kept apart because they need different actions -------------------------
    add("<section id=\"undecided\"><h2>Not decided yet</h2>")
    add(
        "<p class=\"note\">Questions with no rule behind them. Each one is a place where the engine is either "
        "guessing or absent. From the U-register in <code>docs/mechanics/README.md</code>.</p>"
    )
    if open_unknowns:
        add("<ul class=\"unknowns\">")
        for unknown in sorted(open_unknowns, key=lambda item: int(item.number[1:])):
            add(f"<li><span class=\"id\">{_esc(unknown.number)}</span> {_esc(unknown.question)}</li>")
        add("</ul>")
    else:
        add("<p class=\"empty\">Nothing open, which would be surprising.</p>")
    add("</section>")

    add("<section id=\"incomplete\"><h2>Decided, but not finished</h2>")
    add(
        "<p class=\"note\">Rules that are settled and say themselves what they do not cover. These are not "
        "mistakes; they are the named limits of a decision, and the list is the honest answer to \"what is missing\".</p>"
    )
    add("<ul class=\"gaps\">")
    for decision in decisions:
        for gap in decision.gaps:
            add(
                f"<li><a class=\"id\" href=\"#d-{_esc(decision.id)}\">{_esc(decision.id)}</a> {_esc(gap)}</li>"
            )
    add("</ul></section>")

    # ---- the review queue -----------------------------------------------------------------------------------------
    # Batches and a resume point, not a filter: 166 cards in one scroll is not something anybody finishes. The
    # verdicts live in this browser and leave it only by the export button - a static page cannot write the repository.
    add("<section id=\"review\"><h2>Review queue</h2>")
    add(
        "<p class=\"note\">Work through the register in batches; where you stopped is remembered in this browser. "
        "The chips in the register decide <em>what</em> is in the queue - <b>no stated basis</b>, <b>unresolved</b> "
        "and <b>corrected</b> are the three shortest passes worth making. A verdict and a note are stored in this "
        "browser and leave it only through <b>export</b>.</p>"
    )
    add("<div class=\"reviewbar\">")
    add("<label>batch <select id=\"batch-size\">"
        "<option>10</option><option selected>20</option><option>50</option></select></label>")
    add("<button id=\"review-start\">start</button>")
    add("<button id=\"review-prev\">back</button>")
    add("<button id=\"review-next\">next</button>")
    add("<button id=\"review-flagged\">only my flags</button>")
    add("<span id=\"review-progress\" class=\"count\"></span>")
    add("</div><div class=\"reviewbar\">")
    add("<button id=\"export\">export review</button>")
    add("<label class=\"import\">import <input type=\"file\" id=\"import\" accept=\"application/json\"></label>")
    add("<button id=\"reset\">clear verdicts</button>")
    add("</div></section>")

    # ---- the register ---------------------------------------------------------------------------------------------
    add("<section id=\"decisions\"><h2>The register</h2>")
    add("<div class=\"controls\">")
    add("<input id=\"q\" type=\"search\" placeholder=\"filter by id, title or text\" aria-label=\"filter\">")
    add("<span class=\"chips\" id=\"origins\">")
    for origin in ORIGINS:
        add(f"<button class=\"chip\" data-origin=\"{origin}\">{origin}</button>")
    add("<button class=\"chip\" data-origin=\"none\">no basis</button>")
    add("<button class=\"chip\" data-origin=\"unresolved\">unresolved</button>")
    add("</span>")
    add("<button id=\"expand-all\" class=\"chip\">expand all</button>")
    add("<button id=\"collapse-all\" class=\"chip\">collapse all</button>")
    add("<span class=\"chips\" id=\"statuses\">")
    for name in ("decided", "amended", "corrected", "superseded"):
        add(f"<button class=\"chip\" data-status=\"{name}\">{name}</button>")
    add("</span><span id=\"shown\" class=\"count\"></span></div>")

    for index, decision in enumerate(decisions):
        # Bands are collapsible because the register is longer than anyone scrolls; the header carries what is inside
        # so a collapsed band still says whether it is worth opening.
        if index % GROUP_SIZE == 0:
            if index:
                add("</details>")
            band = decisions[index : index + GROUP_SIZE]
            corrected = sum(1 for item in band if item.corrected_by)
            superseded = sum(1 for item in band if item.superseded_by)
            uncited = sum(1 for item in band if item.uncited)
            facts = [f"{len(band)} decisions"]
            if corrected:
                facts.append(f"{corrected} corrected")
            if superseded:
                facts.append(f"{superseded} superseded")
            if uncited:
                facts.append(f"{uncited} with no stated basis")
            add(
                f"<details class=\"group\" id=\"g-{index // GROUP_SIZE}\" "
                f"data-group=\"{index // GROUP_SIZE}\" open>"
            )
            add(
                f"<summary><span class=\"range\">{_esc(band[0].id)} - {_esc(band[-1].id)}</span> "
                f"<span class=\"gcount\">{' \u00b7 '.join(facts)}</span></summary>"
            )
        state = status(decision, corpus)
        origin_keys = " ".join(decision.origins) if decision.origins else "none"
        add(
            f"<article class=\"decision s-{state}\" id=\"d-{_esc(decision.id)}\" "
            f"data-id=\"{_esc(decision.id)}\" data-origins=\"{origin_keys}\" "
            f"data-unresolved=\"{1 if decision.unresolved else 0}\" data-status=\"{state}\""
        )
        add("<div class=\"head\">")
        add(f"<span class=\"id\">{_esc(decision.id)}</span>")
        add(f"<span class=\"status {state}\">{state}</span>")
        if decision.date:
            add(f"<span class=\"date\">{_esc(decision.date)}</span>")
        add(f"<h3>{_esc(decision.title)}</h3></div>")

        if decision.origins:
            add("<div class=\"basis\">")
            for origin, snippets in decision.origins.items():
                # Prefixed on purpose: an origin called `decision` must not collide with the card's own `.decision`
                # class, which the filter script counts (found in the browser: 180 cards where there are 164).
                # The tag *is* the disclosure, so the row reads as labels rather than labels plus stray arrows.
                add(f"<details class=\"tagbox\"><summary><span class=\"tag tag-{origin}\">{origin}</span></summary><ul>")
                for snippet in snippets:
                    add(f"<li>{_esc(snippet)}</li>")
                add("</ul></details>")
            add("</div>")
        else:
            add("<div class=\"basis\"><span class=\"tag tag-none\">no stated basis</span></div>")

        if decision.unresolved:
            add("<details class=\"unresolved\"><summary>unresolved language</summary><ul>")
            for phrase in decision.unresolved:
                add(f"<li>{_esc(phrase)}</li>")
            add("</ul></details>")

        if decision.gaps:
            add("<details class=\"own-gaps\"><summary>"
                f"{len(decision.gaps)} named gap{'s' if len(decision.gaps) != 1 else ''}</summary><ul>")
            for gap in decision.gaps:
                add(f"<li>{_esc(gap)}</li>")
            add("</ul></details>")

        links = []
        if decision.corrected_by:
            links.append("corrected by " + ", ".join(_esc(other) for other in decision.corrected_by))
        if decision.superseded_by:
            links.append("superseded by " + ", ".join(_esc(other) for other in decision.superseded_by))
        if corpus.amendments.get(decision.id):
            links.append(f"amended {len(corpus.amendments[decision.id])}x")
        if links:
            add(f"<p class=\"links\">{' - '.join(links)}</p>")

        # The reviewer's own reading. A verdict and a note, per decision, kept in the browser and exported as a file -
        # see `docs/reviews/2026-11-09-decision-index.md` for why a static page cannot store them in the repository.
        add(f"<div class=\"verdicts\" data-for=\"{_esc(decision.id)}\">")
        for verdict in ("agree", "question", "wrong"):
            add(
                f"<button class=\"verdict\" data-verdict=\"{verdict}\" data-id=\"{_esc(decision.id)}\">"
                f"{verdict}</button>"
            )
        add(
            f"<input class=\"note\" data-note=\"{_esc(decision.id)}\" "
            f"placeholder=\"note (saved with the verdict)\">"
        )
        add("</div>")
        add("</article>")

    add("</details>")
    add("</section>")

    # ---- dangling -------------------------------------------------------------------------------------------------
    add("<section id=\"dangling\"><h2>Referenced but never written</h2>")
    add(
        "<p class=\"note\">Ids named in the docs or the code that no decision section defines. Each is either a "
        "missing decision or a typo in a citation; both are cheap to fix and neither is visible in a prose read.</p>"
    )
    if refs:
        add("<ul class=\"refs\">")
        for ref in sorted(refs, key=lambda item: (item.id, item.path, item.line)):
            add(
                f"<li><span class=\"id\">{_esc(ref.id)}</span> "
                f"<code>{_esc(ref.path)}:{ref.line}</code> {_esc(ref.snippet)}</li>"
            )
        add("</ul>")
    else:
        add("<p class=\"empty\">Every id cited in the docs and the code is defined.</p>")
    add("</section>")

    # ---- how it knows ---------------------------------------------------------------------------------------------
    add("<section id=\"health\"><h2>How this page knows</h2>")
    add(
        f"<p>Parsed {counts['decisions']} decisions and {len(corpus.amendments)} amended ids from "
        f"<code>docs/decisions.md</code>; {counts['notes']} <code>D&lt;n&gt; applied</code> essays were recognised as "
        f"notes rather than decisions, and {counts['structural']} headings are document structure, not decisions.</p>"
    )
    add(
        "<p>A basis is recorded only from a phrase in the decision itself, and the phrase is shown beside the label. "
        "<b>No stated basis</b> is the reliable category: it needs no pattern to be right. <b>Unresolved language</b> "
        "is a reading list, not a verdict - the quoted sentence is there to be judged.</p>"
    )
    add("<p>The scan for broken references reads <code>docs/</code>, <code>crates/</code>, <code>config/</code>, "
        "<code>SPEC.md</code> and <code>README.md</code>. <code>tools/</code> is excluded: its fixtures name ids that "
        "are meant not to exist.</p>")
    if corpus.unclassified:
        add("<h3>Headings this page could not place</h3><ul class=\"unclassified\">")
        for heading in corpus.unclassified:
            add(f"<li>{_esc(heading)}</li>")
        add("</ul>")
    else:
        add("<p class=\"empty\">Every decision heading was placed.</p>")
    add(f"<p class=\"stamp\">Corpus as of {_esc(generated)}. Rebuild with <code>make decisions</code>.</p>")
    add("</section></main>")
    add("<script>" + SCRIPT + "</script></body></html>")
    return "\n".join(parts)


STYLE = """
:root{--bg:#14161a;--panel:#1b1e24;--ink:#e6e6e6;--dim:#9aa2ad;--line:#2c313a;--acc:#7cc4ff;
--owner:#8fd694;--dat:#7cc4ff;--binary:#c9a0ff;--measured:#ffd479;--published:#ff9f7a;
--prior:#79d2c7;--review:#ff8fb1;--decision:#a8b3c4;--none:#ff6b6b}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.55 ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,sans-serif}
header{padding:28px 22px 12px;border-bottom:1px solid var(--line)}
h1{margin:0 0 6px;font-size:26px;letter-spacing:.01em}
.lede{max-width:70ch;color:var(--dim);margin:0 0 14px}
.coverage{list-style:none;display:flex;flex-wrap:wrap;gap:8px;padding:0;margin:0 0 14px}
.coverage li{background:var(--panel);border:1px solid var(--line);border-radius:6px;padding:4px 9px;color:var(--dim);font-size:13px}
.coverage b{color:var(--ink)}
nav{display:flex;flex-wrap:wrap;gap:14px;padding-bottom:10px;font-size:14px}
a{color:var(--acc);text-decoration:none}
a:hover{text-decoration:underline}
main{padding:10px 22px 60px;max-width:1100px}
section{margin:34px 0}
h2{font-size:19px;margin:0 0 6px;border-bottom:1px solid var(--line);padding-bottom:6px}
.note{color:var(--dim);max-width:80ch;margin:6px 0 14px}
.empty{color:var(--dim);font-style:italic}
.controls{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin:12px 0 18px}
input[type=search]{background:var(--panel);border:1px solid var(--line);color:var(--ink);padding:7px 10px;border-radius:6px;min-width:260px}
.chips{display:flex;flex-wrap:wrap;gap:6px}
.chip{background:var(--panel);border:1px solid var(--line);color:var(--dim);border-radius:999px;padding:4px 10px;font-size:12px;cursor:pointer}
.chip.on{background:#24303f;border-color:var(--acc);color:var(--ink)}
.count{color:var(--dim);font-size:13px;margin-left:auto}
article.decision{background:var(--panel);border:1px solid var(--line);border-left:3px solid var(--line);border-radius:8px;padding:12px 14px;margin:0 0 10px}
article.decision.s-corrected{border-left-color:#ff8fb1}
article.decision.s-superseded{border-left-color:#c9a0ff}
article.decision.s-amended{border-left-color:#ffd479}
.head{display:flex;flex-wrap:wrap;gap:8px;align-items:baseline}
.head h3{margin:0;font-size:15px;font-weight:600;flex:1 1 100%}
.id{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:700;color:var(--acc)}
.status{font-size:11px;text-transform:uppercase;letter-spacing:.06em;padding:1px 7px;border-radius:999px;border:1px solid var(--line);color:var(--dim)}
.status.corrected{color:#ff8fb1;border-color:#5c2b3a}
.status.superseded{color:#c9a0ff;border-color:#41325c}
.status.amended{color:#ffd479;border-color:#5c4a22}
.date{color:var(--dim);font-size:12px}
.basis{display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin-top:8px}
.tag{font-size:11px;padding:1px 7px;border-radius:4px;background:#232830;border:1px solid var(--line);cursor:pointer}
.tagbox{display:inline}
.tagbox>summary{list-style:none;display:inline-block}
.tagbox>summary::-webkit-details-marker{display:none}
.tagbox[open]>summary .tag{outline:1px solid var(--acc)}
.tag-owner{color:var(--owner)} .tag-dat{color:var(--dat)} .tag-binary{color:var(--binary)}
.tag-measured{color:var(--measured)} .tag-published{color:var(--published)}
.tag-prior-art{color:var(--prior)} .tag-review{color:var(--review)} .tag-decision{color:var(--decision)} .tag-none{color:var(--none)}
details{display:inline}
details summary{cursor:pointer;color:var(--dim);font-size:12px}
details ul{margin:6px 0 0;padding-left:20px;color:var(--dim);font-size:13px}
details.unresolved,details.own-gaps{display:block;margin-top:8px}
.links{color:var(--dim);font-size:12px;margin:8px 0 0}
.gaps li,.unknowns li,.refs li{margin:0 0 8px;color:var(--ink)}
.gaps .id,.unknowns .id,.refs .id{margin-right:6px}
code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:12.5px;color:var(--dim)}
.refs code{color:var(--acc)}
.stamp{color:var(--dim);font-size:12px;margin-top:18px}
.hidden{display:none}
details.group{border:1px solid var(--line);border-radius:8px;margin:0 0 12px;padding:0 10px 4px}
details.group>summary{cursor:pointer;padding:9px 2px;display:flex;flex-wrap:wrap;gap:10px;align-items:baseline;list-style:none}
details.group>summary::-webkit-details-marker{display:none}
/* "+" / "-" rather than a triangle glyph: the container's fonts render ▸ as a tofu box, and a 10px
   CSS triangle reads as an up-arrow at this size. A sign is unambiguous in every font. */
details.group>summary::before{content:"+";color:var(--acc);font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:700;font-size:15px;line-height:1;width:14px;display:inline-block;text-align:center;margin-right:7px}
details.group[open]>summary::before{content:"-"}
details.group .range{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:700;color:var(--acc);font-size:14px}
details.group .gcount{color:var(--dim);font-size:12px;margin-left:auto;text-align:right}
.reviewbar{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin:10px 0}
.reviewbar select,.reviewbar button{background:var(--panel);border:1px solid var(--line);color:var(--ink);border-radius:6px;padding:5px 10px;font-size:13px;cursor:pointer}
.reviewbar button.on{background:#24303f;border-color:var(--acc)}
.reviewbar label{color:var(--dim);font-size:13px}
.verdicts{display:flex;flex-wrap:wrap;gap:6px;align-items:center;margin-top:10px;border-top:1px dashed var(--line);padding-top:8px}
.verdict{background:transparent;border:1px solid var(--line);color:var(--dim);border-radius:999px;padding:2px 10px;font-size:12px;cursor:pointer}
.verdict:hover{color:var(--ink)}
.verdict[data-verdict=agree].on{color:#8fd694;border-color:#2f5c3a;background:#1d2a22}
.verdict[data-verdict=question].on{color:#ffd479;border-color:#5c4a22;background:#2a2519}
.verdict[data-verdict=wrong].on{color:#ff8fb1;border-color:#5c2b3a;background:#2a1c22}
/* the native file input cannot be themed; it is made invisible and the label is the button */
.reviewbar .import{position:relative;overflow:hidden;background:var(--panel);border:1px solid var(--line);color:var(--ink);border-radius:6px;padding:5px 10px;font-size:13px;cursor:pointer;display:inline-block}
#import{position:absolute;inset:0;opacity:0;cursor:pointer}
.note{flex:1 1 220px;background:transparent;border:1px solid var(--line);color:var(--ink);border-radius:6px;padding:3px 8px;font-size:12px}
article.decision.v-agree{border-left-color:#8fd694}
article.decision.v-question{border-left-color:#ffd479}
article.decision.v-wrong{border-left-color:#ff8fb1}

/* Light mode overrides the variables, not the rules: the earlier version set a light body and left the cards dark,
   and its selector had a trailing comma so it applied to nothing at all. */
@media (prefers-color-scheme: light){
:root{--bg:#f7f7f5;--panel:#fff;--ink:#16181c;--dim:#5b6470;--line:#d9dad4;--acc:#0b62a4}
.tag{background:#eef0ee}
.chip.on{background:#e4eef8}
}
"""

SCRIPT = r"""(function(){
  var KEY='ruage-decisions-review';
  var q=document.getElementById('q'),origins=new Set(),statuses=new Set(),
      cards=[].slice.call(document.querySelectorAll('article.decision')),
      shown=document.getElementById('shown'),
      progress=document.getElementById('review-progress'),
      batchSel=document.getElementById('batch-size'),
      groups=[].slice.call(document.querySelectorAll('details.group')),
      flaggedOnly=false,reviewing=false,cursor=0,
      store=load();

  function load(){
    try{var s=JSON.parse(localStorage.getItem(KEY));if(s&&s.verdicts)return s;}catch(e){}
    return {version:1,verdicts:{}};
  }
  function save(){try{localStorage.setItem(KEY,JSON.stringify(store));}catch(e){}}
  function verdictOf(id){return (store.verdicts[id]||{}).verdict||'';}
  function batch(){return parseInt(batchSel.value,10)||20;}

  function paint(card){
    var id=card.dataset.id,v=store.verdicts[id]||{};
    card.classList.remove('v-agree','v-question','v-wrong');
    if(v.verdict)card.classList.add('v-'+v.verdict);
    card.querySelectorAll('[data-verdict]').forEach(function(b){
      b.classList.toggle('on',b.dataset.verdict===v.verdict);});
    var note=card.querySelector('[data-note]');
    if(note&&note.value!==(v.note||''))note.value=v.note||'';
  }
  function paintAll(){cards.forEach(paint);}

  function matches(card){
    var term=q.value.toLowerCase();
    if(term&&(card.dataset.id+' '+card.textContent).toLowerCase().indexOf(term)<0)return false;
    if(origins.size){
      var keys=card.dataset.origins.split(' '),hit=false;
      origins.forEach(function(o){
        if(o==='none'&&card.dataset.origins==='none')hit=true;
        else if(o==='unresolved'&&card.dataset.unresolved==='1')hit=true;
        else if(keys.indexOf(o)>=0)hit=true;});
      if(!hit)return false;
    }
    if(statuses.size&&!statuses.has(card.dataset.status))return false;
    if(flaggedOnly){
      var v=verdictOf(card.dataset.id);
      if(v!=='question'&&v!=='wrong')return false;
    }
    return true;
  }

  function apply(){
    var list=cards.filter(matches);
    cards.forEach(function(c){c.classList.add('hidden');});
    var slice=reviewing?list.slice(cursor,cursor+batch()):list;
    slice.forEach(function(c){c.classList.remove('hidden');});
    shown.textContent=slice.length+' of '+cards.length+' shown';
    groups.forEach(function(g){
      var visible=g.querySelectorAll('article.decision:not(.hidden)').length;
      g.classList.toggle('hidden',visible===0);
      // Open whatever holds a match: a narrowed list behind seven collapsed headers looks empty.
      if((reviewing||list.length<cards.length)&&visible)g.open=true;
    });
    var reviewed=cards.filter(function(c){return verdictOf(c.dataset.id);}).length;
    progress.textContent=reviewed+' of '+cards.length+' reviewed'+
      (reviewing&&list.length?' \u00b7 batch '+(Math.floor(cursor/batch())+1)+' of '+Math.ceil(list.length/batch()):'');
  }

  q.addEventListener('input',function(){cursor=0;apply();});
  document.querySelectorAll('.chip').forEach(function(chip){
    chip.addEventListener('click',function(){
      // `expand all` / `collapse all` are styled as chips but are not filters: without this guard they land in the
      // statuses set as `undefined` and hide every card (found by clicking collapse-all and watching the list empty).
      if(!chip.dataset.origin&&!chip.dataset.status)return;
      var set=chip.dataset.origin?origins:statuses,key=chip.dataset.origin||chip.dataset.status;
      if(set.has(key))set.delete(key);else set.add(key);
      chip.classList.toggle('on',set.has(key));
      cursor=0;apply();
    });
  });

  document.querySelectorAll('[data-verdict]').forEach(function(btn){
    btn.addEventListener('click',function(){
      var id=btn.dataset.id,v=btn.dataset.verdict;
      if(verdictOf(id)===v){delete store.verdicts[id];}
      else{
        store.verdicts[id]={verdict:v,note:(store.verdicts[id]||{}).note||'',at:new Date().toISOString()};
      }
      save();paint(btn.closest('article'));apply();
    });
  });
  document.querySelectorAll('[data-note]').forEach(function(input){
    input.addEventListener('change',function(){
      var id=input.dataset.note,entry=store.verdicts[id]||{verdict:'',at:''};
      entry.note=input.value;entry.at=entry.at||new Date().toISOString();
      store.verdicts[id]=entry;save();
    });
  });

  document.getElementById('expand-all').addEventListener('click',function(){
    groups.forEach(function(g){g.open=true;});
  });
  document.getElementById('collapse-all').addEventListener('click',function(){
    groups.forEach(function(g){if(!g.classList.contains('hidden'))g.open=false;});
  });
  batchSel.addEventListener('change',function(){cursor=0;apply();});
  document.getElementById('review-start').addEventListener('click',function(){
    reviewing=true;cursor=0;apply();
    document.getElementById('decisions').scrollIntoView();
  });
  document.getElementById('review-next').addEventListener('click',function(){
    var total=cards.filter(matches).length;
    cursor=Math.min(cursor+batch(),Math.max(0,total-1));reviewing=true;apply();
    document.getElementById('decisions').scrollIntoView();
  });
  document.getElementById('review-prev').addEventListener('click',function(){
    cursor=Math.max(0,cursor-batch());reviewing=true;apply();
    document.getElementById('decisions').scrollIntoView();
  });
  document.getElementById('review-flagged').addEventListener('click',function(){
    flaggedOnly=!flaggedOnly;this.classList.toggle('on',flaggedOnly);cursor=0;apply();
  });

  function exportPayload(){
    return JSON.stringify({tool:'ruage-decisions',version:1,
      exported:new Date().toISOString(),verdicts:store.verdicts},null,2);
  }
  window.__exportReview=exportPayload;
  document.getElementById('export').addEventListener('click',function(){
    var blob=new Blob([exportPayload()],{type:'application/json'}),a=document.createElement('a');
    a.href=URL.createObjectURL(blob);a.download='decisions-review.json';a.click();
    setTimeout(function(){URL.revokeObjectURL(a.href);},1000);
  });
  document.getElementById('import').addEventListener('change',function(e){
    var file=e.target.files[0];if(!file)return;
    var reader=new FileReader();
    reader.onload=function(){
      try{
        var data=JSON.parse(reader.result);
        if(data&&data.verdicts){store={version:1,verdicts:data.verdicts};save();paintAll();apply();}
      }catch(err){alert('that is not a review file');}
    };
    reader.readAsText(file);
  });
  document.getElementById('reset').addEventListener('click',function(){
    if(!confirm('clear every verdict in this browser?'))return;
    store={version:1,verdicts:{}};save();paintAll();apply();
  });

  paintAll();apply();
})();"""


SCAN = [
    ("docs", "*.md"),
    ("crates", "*.rs"),
    ("config", "*.toml"),
]
SCAN_FILES = ["SPEC.md", "README.md"]


def _in_scope(relative: str) -> bool:
    """Whether a path is one whose citations the page checks."""
    if relative in SCAN_FILES:
        return True
    parts = Path(relative).parts
    return (
        bool(parts)
        and parts[0] in [folder for folder, _ in SCAN]
        and relative.endswith(tuple(pattern.replace("*", "") for _, pattern in SCAN))
        and "site" not in parts
    )


def tracked_paths() -> list[str] | None:
    """The repository's own files, or None when this is not a git checkout."""
    try:
        out = subprocess.run(
            ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, text=True, timeout=15
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return [path for path in out.stdout.split("\0") if path]


def scan_sources(paths: list[str] | None = None) -> dict[str, str]:
    """Every file whose citations are checked, keyed by repo-relative path.

    The scope is the **repository**, not the working tree: a page committed from this checkout must not quote an
    untracked note another session is still writing. `git ls-files` answers that, and a plain walk is the fallback for
    a checkout that has no git (an export, a tarball).
    """
    if paths is None:
        paths = tracked_paths()
        if paths is None:
            paths = [
                str(path.relative_to(ROOT))
                for folder, pattern in SCAN
                if (ROOT / folder).exists()
                for path in (ROOT / folder).rglob(pattern)
            ] + SCAN_FILES
    texts: dict[str, str] = {}
    for relative in sorted(paths):
        if not _in_scope(relative):
            continue
        path = ROOT / relative
        if path.exists():
            texts[relative] = path.read_text(errors="replace")
    return texts


def build(decisions_path: Path = DECISIONS) -> tuple[Corpus, dict[str, Unknown], list[Ref]]:
    corpus = parse_decisions(decisions_path.read_text())
    unknowns = parse_unknowns(UNKNOWNS.read_text()) if UNKNOWNS.exists() else {}
    refs = dangling_refs(corpus.decisions, scan_sources())
    return corpus, unknowns, refs


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Index the decision corpus into one browsable page.")
    parser.add_argument("--check", action="store_true", help="parse and report coverage; write nothing")
    parser.add_argument("--out", default=str(OUTPUT), help="where to write the page")
    parser.add_argument("--decisions", default=str(DECISIONS), help="the corpus (defaults to docs/decisions.md)")
    args = parser.parse_args(argv)

    try:
        corpus, unknowns, refs = build(Path(args.decisions))
    except FileNotFoundError as error:
        print(f"cannot read the corpus: {error.filename} is not there", file=sys.stderr)
        return 2
    decisions = sorted(corpus.decisions.values(), key=lambda decision: decision.number)
    uncited = [decision for decision in decisions if decision.uncited]
    open_unknowns = [unknown for unknown in unknowns.values() if not unknown.closed]
    dangling = sorted({ref.id for ref in refs})

    print(
        f"{len(decisions)} decisions, {sum(len(v) for v in corpus.amendments.values())} amendment sections, "
        f"{len(uncited)} with no stated basis, "
        f"{sum(len(d.gaps) for d in decisions)} named gaps, {len(open_unknowns)} open questions, "
        f"{len(dangling)} dangling ids, {len(corpus.unclassified)} unclassified headings"
    )
    if dangling:
        print("dangling: " + ", ".join(dangling))
    if uncited:
        print("no stated basis: " + ", ".join(decision.id for decision in uncited))
    if corpus.unclassified:
        for heading in corpus.unclassified:
            print(f"UNPLACED: {heading}", file=sys.stderr)
        return 1
    if args.check:
        return 0

    newest = max((decision.date for decision in decisions if decision.date), default="unknown")
    target = Path(args.out)
    target.parent.mkdir(parents=True, exist_ok=True)
    # Write beside the target and rename: a browser reloading while `make decisions` runs would otherwise be served a
    # half-written page, and the commit would be a truncated file if the box rebooted mid-write.
    temporary = target.with_name(target.name + ".tmp")
    temporary.write_text(render(corpus, unknowns, refs, newest))
    os.replace(temporary, target)
    print(f"wrote {target.relative_to(ROOT)} ({target.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BrokenPipeError:
        # `python3 tools/decisions.py | head` closed the pipe and the traceback read like a defect in the tool.
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        raise SystemExit(0)
