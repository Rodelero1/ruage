"""The dependency tree, checked rather than believed (2026-11-14).

`docs/dependency-tree.md` says what is built, what is hollow and what order the rest goes in, and **nothing keeps it
honest**: a row can name an id that is not defined, an open item can be missing from the sequence, a `DONE-ish` row can
lose the debt that explains it, and a `file:line` can point at a file that does not exist. Every one of those has already
happened in a document in this tree, so every one of them is a check.

What it refuses, and why each is a *miss* rather than a style point:

* **A status outside the vocabulary** — the legend is five tags; a sixth makes the table unreadable, not richer.
* **An id defined twice, or referred to and never defined** — the tree is read as a graph, so a dangling edge is a hole.
* **An open item no phase names** — the whole purpose of the order is that nothing important is left out, and an item
  that exists in a table and nowhere in §8 is exactly that.
* **A `DONE-ish`/`PART` row with no `T` debt named** — "it runs, but something it feeds is a placeholder" without the
  something is the note that cost this project a session.
* **A path that does not exist, or a line past the end of the file** — a citation is evidence; a wrong one is worse than
  none, and one was made while writing the document this checks.
* **A `U<n>` the register does not have, or a `D<n>` that is neither written nor listed as dangling** — the corpora are
  the authority, and `docs/dangling.toml` exists so an unwritten id has a home.

Two limits, stated so a green run is not read as more than it is: a bare filename (`lib.rs:3435`) is resolved against
every file of that name in the tree and its line accepted if **any** of them is long enough, and a path into a *generated*
or *installed* directory (`config/gamedata/`, `build/`, `Age2HD/`, `target/`) is not checked, because a fresh clone does
not have it (D8).

    python3 tools/plan_check.py                  # check docs/dependency-tree.md
    python3 tools/plan_check.py --doc path.md    # check another
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

DOC = ROOT / "docs" / "dependency-tree.md"
#: The legend in §1. A tag that is not here is refused by name, never bucketed.
STATUS = {"DONE", "DONE-ish", "PART", "OPEN", "MISSING"}

#: Directories a fresh clone does not have, so a citation into them is not a finding (D8).
UNCHECKED = ("config/gamedata/", "build/", "Age2HD/", "target/", "docs/site/")

#: Groups whose items must each appear in the sequence: a debt, a missing mechanic, a product step, a seam.
SCHEDULED = set("TMPR")

ID = re.compile(r"\b([FSTMPR])(\d{1,2})([a-z]?)\b")
ROW = re.compile(r"^\|\s*\*{0,2}([FSTMPR])(\d{1,2})\*{0,2}\s*\|")
CELL = re.compile(r"^\*\*([A-Za-z][A-Za-z -]{2,12})\*\*$")
CITATION = re.compile(r"([A-Za-z0-9_][A-Za-z0-9_./-]*\.(?:rs|toml|md|py|yml|sh|inc|json))(?::(\d+)(?:-(\d+))?)?")


class Findings:
    """A list that says how many checks ran, so a green run can report what it covered."""

    def __init__(self) -> None:
        self.lines: list[str] = []
        self.checks = 0
        self.ids = 0
        self.citations = 0

    def add(self, where: str, detail: str) -> None:
        self.lines.append(f"  {where}: {detail}")


def _sections(text: str) -> list[tuple[str, str]]:
    """The document split on its `## ` headings, as `(heading, body)` pairs.

    By **title**, not by number: the document may be renumbered as slices land, and a check that silently stopped
    finding its section would pass while checking nothing.
    """
    out: list[tuple[str, str]] = []
    parts = re.split(r"(?m)^##\s+(.*)$", text)
    for index in range(1, len(parts), 2):
        out.append((parts[index].strip(), parts[index + 1]))
    return out


def _section(sections: list[tuple[str, str]], wanted: str) -> str:
    """The body of the first heading matching `wanted` (a regex), or empty."""
    for heading, body in sections:
        if re.search(wanted, heading, re.IGNORECASE):
            return body
    return ""


def _defined(text: str) -> tuple[dict[str, str], Findings]:
    """Every id that starts a table row, with the row it starts."""
    found: dict[str, str] = {}
    findings = Findings()
    for line in text.splitlines():
        match = ROW.match(line)
        if not match:
            continue
        ident = f"{match.group(1)}{match.group(2)}"
        findings.ids += 1
        if ident in found:
            findings.add(ident, "defined twice (the tree is read as a graph, so one row per id)")
            continue
        found[ident] = line
    return found, findings


def _vocabulary(text: str, findings: Findings) -> None:
    """The status column of a row that starts with an id, where it is a single bold word."""
    for line in text.splitlines():
        if not ROW.match(line):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        row_id = cells[0].strip("*")
        for cell in cells[1:]:
            match = CELL.match(cell)
            if match and match.group(1) not in STATUS and match.group(1) not in {"DONE (external)"}:
                findings.add(row_id, f"status {match.group(1)!r} is not one of {sorted(STATUS)}")


def _references(text: str, defined: dict[str, str], findings: Findings) -> set[str]:
    """Every id the document names, refusing the ones no row defines. Returns what it saw."""
    seen: set[str] = set()
    for match in ID.finditer(text):
        base = f"{match.group(1)}{match.group(2)}"
        seen.add(base)
        if base not in defined:
            findings.add(base, f"referred to at {_line_of(text, match.start())} and never defined")
    return seen


def _scheduling(sections: list[tuple[str, str]], defined: dict[str, str], findings: Findings) -> None:
    """An open item that no phase names is the miss this document exists to prevent."""
    sequence = _section(sections, r"sequence")
    if not sequence:
        findings.add("the sequence", "no section whose title says sequence, so nothing could be checked")
        return
    for ident in sorted(defined):
        if ident[0] in SCHEDULED and ident not in sequence and not _scheduled_by_suffix(ident, sequence):
            findings.add(ident, "defined and named by no phase of §8 (an item nobody scheduled)")


def _scheduled_by_suffix(ident: str, sequence: str) -> bool:
    """`T1a`/`T1b` are the two halves of `T1`: a sub-id schedules its parent."""
    return bool(re.search(rf"\b{ident}[a-z]\b", sequence))


def _hollow(sections: list[tuple[str, str]], findings: Findings) -> None:
    """A `DONE-ish` row owes the thing that explains it: a debt (`T`) or a missing mechanic (`M`)."""
    body = _section(sections, r"what runs|match today")
    if not body:
        findings.add("the status table", "no section whose title says what runs, so no hollow row was checked")
        return
    for line in body.splitlines():
        if not ROW.match(line):
            continue
        if "DONE-ish" not in line and "**PART**" not in line:
            continue
        if not re.search(r"\b[TM]\d{1,2}\b", line):
            findings.add(line.split("|")[1].strip(), "is DONE-ish/PART and names no T or M (the hole is unnamed)")


def _line_of(text: str, offset: int) -> str:
    return f"line {text.count(chr(10), 0, offset) + 1}"


class Index:
    """Every source file, so a partial citation (`world/gather.rs`) can be resolved without guessing.

    The document cites what a reader needs: a full path where a path is the point, and the tail of one where the row
    already says which crate it is about. Both resolve here; a citation whose tail matches nothing does not.
    """

    def __init__(self, root: Path) -> None:
        self.root = root
        self.by_name: dict[str, list[Path]] = {}
        self.relative: list[tuple[str, Path]] = []
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            text = str(path.relative_to(root))
            if any(text.startswith(skip) for skip in UNCHECKED):
                continue
            if ".git/" in text or ".venv/" in text or "node_modules/" in text:
                continue
            self.by_name.setdefault(path.name, []).append(path)
            self.relative.append((text, path))

    def resolve(self, cited: str) -> list[Path]:
        direct = self.root / cited
        if direct.exists():
            return [direct]
        sibling = self.root.parent / cited  # `ruage-tools/...` is the importer's own repo (D12)
        if sibling.exists():
            return [sibling]
        if "/" not in cited:
            return self.by_name.get(cited, [])
        tail = "/" + cited
        return [path for text, path in self.relative if text.endswith(tail)]


def _citations(text: str, index: Index, findings: Findings) -> None:
    """A citation is evidence: the file must exist and the line must be inside it.

    Deduplicated on the **path and the line**, not on the path: `README.md` in one row and `README.md:99` in another are
    two claims, and dropping the second because the first exists is how a wrong line survives a green run.
    """
    seen: set[tuple[str, str | None]] = set()
    lengths: dict[str, int] = {}
    for span in re.findall(r"`([^`\n]+)`", text):
        # A span is prose, a shell command or a citation, and the citation pattern decides: an extension is in the
        # pattern, so a command's words and flags cannot match it. Only a span that is *itself* a command is skipped -
        # the earlier version skipped every span holding a space, which silently excluded `file:line description`.
        if span.startswith(("grep", "python", "make", "cargo", "git", "./", "ls ", "wc ")):
            continue
        for match in CITATION.finditer(span):
            cited, first, last = match.group(1), match.group(2), match.group(3)
            if (cited, first) in seen:
                continue
            seen.add((cited, first))
            findings.citations += 1
            if any(cited.startswith(skip) or cited.startswith(skip.rstrip("/")) for skip in UNCHECKED):
                continue
            paths = index.resolve(cited)
            if not paths:
                findings.add(cited, "cited and no such file")
                continue
            if first is None:
                continue
            wanted = max(int(first), int(last or first))
            for path in paths:
                key = str(path)
                if key not in lengths:
                    lengths[key] = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
            longest = max(lengths[str(path)] for path in paths)
            if longest < wanted:
                findings.add(cited, f"cited at line {wanted} but the file{'s' if len(paths) > 1 else ''} end at {longest}")


def _corpora(root: Path) -> tuple[set[str], set[str]]:
    """The two corpora the document cites by id, read from the tree it describes."""
    register = root / "docs" / "mechanics" / "README.md"
    u_rows = {f"U{number}" for number in re.findall(r"\bU(\d+)\b", register.read_text(encoding="utf-8"))} if register.exists() else set()
    decisions = root / "docs" / "decisions.md"
    dangling = root / "docs" / "dangling.toml"
    corpus = ""
    for path in (decisions, dangling):
        if path.exists():
            corpus += path.read_text(encoding="utf-8")
    known = {f"D{number}{suffix}" for number, suffix in re.findall(r"\bD(\d+)([a-z]?)\b", corpus)}
    return u_rows, known


def _unknowns_and_decisions(text: str, root: Path, findings: Findings) -> None:
    u_rows, decisions = _corpora(root)
    for number in set(re.findall(r"\bU(\d+)\b", text)):
        if f"U{number}" not in u_rows:
            findings.add(f"U{number}", "named and not in the U-register (docs/mechanics/README.md)")
    for number, suffix in set(re.findall(r"\bD(\d+)([a-z]?)\b", text)):
        if f"D{number}{suffix}" not in decisions:
            findings.add(f"D{number}{suffix}", "named and neither written in decisions.md nor listed in dangling.toml")


def check(doc: Path, root: Path) -> Findings:
    """Every check, over one document. The findings carry the line's own words so they can be found."""
    text = doc.read_text(encoding="utf-8")
    sections = _sections(text)
    defined, findings = _defined(text)
    _vocabulary(text, findings)
    _references(text, defined, findings)
    _scheduling(sections, defined, findings)
    _hollow(sections, findings)
    _citations(text, Index(root), findings)
    _unknowns_and_decisions(text, root, findings)
    findings.checks = 8
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check the dependency tree against itself and the tree it describes.")
    parser.add_argument("--doc", default=str(DOC), help="the document to check")
    parser.add_argument("--root", default=str(ROOT), help="the repository it describes")
    args = parser.parse_args(argv)

    doc = Path(args.doc)
    if not doc.exists():
        print(f"plan: no such document: {doc}", file=sys.stderr)
        return 2
    findings = check(doc, Path(args.root))
    if findings.lines:
        print(f"plan: {len(findings.lines)} finding(s) in {doc}")
        for line in findings.lines:
            print(line)
        return 1
    print(
        f"plan ok: {findings.ids} items, {findings.citations} citations, {findings.checks} checks in "
        f"{doc.relative_to(Path(args.root)) if str(doc).startswith(str(args.root)) else doc}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
