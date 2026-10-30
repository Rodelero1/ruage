#!/usr/bin/env python3
"""Prior-art watch for Ruage.

Prints only the projects that are due under D14's amendment, one line each. Empty output is the
normal case and means "nothing to look at today".

Due means either:
  * the head commit sha differs from the one recorded last time (the override that matters), or
  * the backoff interval recorded at the last look has expired.

The interval is derived from upstream activity, never from our own calendar: a project touched
within 2 days is looked at every 2 days, within a week weekly, otherwise fortnightly.

State lives in build/prior-art-state.json (gitignored). It is written on every run, so a report
and the advance of the state happen in the same invocation; a project reported today is not
reported again tomorrow unless it moves.

A closed list can only answer "did a project we know about move?", so once a month the gate also
sweeps GitHub's search API for projects the list has never heard of and prints them as DISCOVERY
lines: candidates for the owner to judge, not report material. That scan's own state lives in
build/prior-art-discovery.json, advances only on a complete sweep, and remembers only what was
actually printed — an overflow comes back next month instead of vanishing.

Usage:
  watch.py              # gate: print due projects only
  watch.py --status     # print the whole watchlist with due dates
  watch.py --dry-run    # do not write state, and do not sweep
Exit code is 0 whether or not anything is due; a non-zero exit means the check itself failed.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
STATE_PATH = REPO_ROOT / "build" / "prior-art-state.json"
API = "https://api.github.com/repos/{repo}/commits?per_page=1"

# The watchlist is a closed list, so by construction it can only ever answer "did a project we
# already know about move?". A monthly sweep of the search API is what can answer the other
# question — is there a project we have never heard of? — and it is a separate rate-limit bucket
# from the per-repo commit reads, so it cannot starve the gate.
DISCOVERY_INTERVAL_DAYS = 30
DISCOVERY_STATE_PATH = REPO_ROOT / "build" / "prior-art-discovery.json"
DISCOVERY_MAX_REPORTED = 8
DISCOVERY_STALE_DAYS = 90
# Measured against live data before adopting (2026-10-30), searching names, descriptions and topics
# only: 35 candidates, and the project that motivated the sweep was found. Adding `readme` raised it
# to 57 and the extra 22 were a "point Claude at any game" mod pack, decompilation directories, a
# starred-repos mirror and an emulator — recall did not improve, precision collapsed. The noise the
# queries still drag in (curated lists, SEO drill pages, our own account) is filtered below.
DISCOVERY_IGNORED_OWNERS = {"rodelero1"}  # our own account: prior art is other people's work
DISCOVERY_NOISE = re.compile(
    r"\bawesome\b|curated list|list of|directory of|my starred|hotkey (trainer|drill)|"
    r"\bdrills\b|cheat[- ]?sheet",
    re.IGNORECASE,
)
DISCOVERY_QUERIES: list[str] = [
    "aoe2 engine in:name,description",
    '"age of empires" engine in:description',
    "age of empires ii reimplementation in:name,description",
    "topic:aoe2",
    "aoe2 deterministic simulation in:name,description",
]

# The watchlist, grouped exactly as docs/prior-art.md describes it. Order is the report order:
# the reference engine first, then the active deterministic implementation, then the rest.
WATCHLIST: list[tuple[str, str]] = [
    ("SFTtech/openage", "reference engine: asset pipeline, engine/content split, time model"),
    ("librematch/genie", "active deterministic reimplementation and format readers"),
    ("Zaydme/openaok", "from-scratch AoK engine: fixed tick, pather, dat 5.7 reader"),
    ("yanfengliu/aoe2", "TypeScript replica on its own civ-engine: original art, LLM playtest loop, DE-shaped spec"),
    ("SiegeEngineers/genie-rs", "crate-level model of the dat format"),
    ("SiegeEngineers/genieutils-py", "independent reading of the same format"),
    ("SiegeEngineers/sld-extractor", "DE sprite extraction"),
    ("SiegeEngineers/WololoKingdoms", "HD-era file conversion, HD vs DE deltas"),
    ("SiegeEngineers/aoc-reference-data", "published-value cross-checks"),
    ("SiegeEngineers/halfon", "unit/building reference"),
    ("SiegeEngineers/aoe2techtree", "tech tree reference"),
    ("SiegeEngineers/rms-check", "random map script language"),
    ("SiegeEngineers/rms-doc", "random map script language reference"),
]

UA = "ruage-prior-art-watch (+https://github.com/; one request per repo per day)"


def http_json(url: str, timeout: float = 20.0) -> object:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def head_commit(repo: str) -> tuple[str, datetime]:
    """Return (sha, committed-at) for the default branch head. Retries transient failures."""
    last_err: Exception | None = None
    for attempt in range(3):
        try:
            data = http_json(API.format(repo=repo))
            if not isinstance(data, list) or not data:
                raise ValueError(f"unexpected payload for {repo}")
            commit = data[0]
            when = commit["commit"]["committer"]["date"]
            return commit["sha"], datetime.strptime(when, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
        except (urllib.error.URLError, urllib.error.HTTPError, ValueError, KeyError, TimeoutError) as err:
            last_err = err
            if isinstance(err, urllib.error.HTTPError) and err.code in (403, 404):
                break  # rate limit or gone: retrying now will not help
            time.sleep(2 * (attempt + 1))
    raise RuntimeError(f"{repo}: {last_err}") from last_err


def interval_days(committed_at: datetime, now: datetime) -> float:
    """D14's backoff table, applied to how recently upstream itself moved."""
    age = now - committed_at
    if age <= timedelta(days=2):
        return 2.0
    if age <= timedelta(days=7):
        return 7.0
    return 14.0


def search_repositories(query: str, per_page: int = 30) -> list[dict]:
    """One search-API call. A separate rate-limit bucket from the per-repo commit reads."""
    url = "https://api.github.com/search/repositories?" + urllib.parse.urlencode(
        {"q": query, "sort": "updated", "order": "desc", "per_page": per_page}
    )
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=25) as resp:
        return json.loads(resp.read().decode("utf-8")).get("items", [])


def _candidate(item: dict) -> dict:
    """The five facts a relevance judgement needs, and the flags that disqualify without one."""
    return {
        "full_name": item.get("full_name", ""),
        "desc": (item.get("description") or "").strip(),
        "stars": item.get("stargazers_count") or 0,
        "lang": item.get("language") or "-",
        "licence": (item.get("license") or {}).get("spdx_id") or "none",
        "pushed": (item.get("pushed_at") or "")[:10],
        "created": (item.get("created_at") or "")[:10],
        "fork": bool(item.get("fork")),
        "archived": bool(item.get("archived")),
    }


def _pushed_datetime(candidate: dict) -> datetime | None:
    """None means the payload carried no usable date; such a candidate is kept, not silently lost."""
    try:
        return datetime.strptime(candidate["pushed"], "%Y-%m-%d").replace(tzinfo=timezone.utc)
    except (ValueError, TypeError):
        return None


def _newest_first(candidates: list[dict]) -> list[dict]:
    """Newest push first, ties broken by name so a report does not reorder itself between runs."""
    ordered = sorted(candidates, key=lambda c: c["full_name"].lower())
    ordered.sort(key=lambda c: _pushed_datetime(c) or datetime(1970, 1, 1, tzinfo=timezone.utc), reverse=True)
    return ordered


def format_candidate(candidate: dict) -> str:
    """One line, tab-separated like the gate's own lines, carrying everything a judgement needs.

    The description comes from a stranger's repository, so its whitespace is flattened: a tab or a
    newline in it would otherwise forge a column boundary or a second line in a report.
    """
    description = " ".join(candidate["desc"].split())[:140]
    return (
        f'{candidate["full_name"]}\t{candidate["stars"]}*\t{candidate["lang"]}\t'
        f'licence {candidate["licence"]}\tpushed {candidate["pushed"]}\t{description}'
    )


def discover(known: set[str], search=search_repositories, now: datetime | None = None) -> tuple[list[dict], list[str]]:
    """Projects the watchlist has never heard of, newest push first.

    A candidate has to be judgeable without opening it (a description) and alive (touched inside the
    staleness window); forks, archived repos, curated lists, SEO drill sites and our own account are
    noise. A query that fails is named and the rest still run — one dead query must not silently
    shrink the sweep.
    """
    now = now or datetime.now(timezone.utc)
    watched = {repo.lower() for repo in known}
    found: dict[str, dict] = {}
    failures: list[str] = []
    for query in DISCOVERY_QUERIES:
        try:
            items = search(query)
        except Exception as err:  # noqa: BLE001 - one query's failure must not end the sweep
            failures.append(f"{query}: {err}")
            continue
        for item in items:
            candidate = _candidate(item)
            key = candidate["full_name"].lower()
            if not key or key in watched or key in found:
                continue
            if candidate["fork"] or candidate["archived"] or not candidate["desc"]:
                continue
            if key.split("/")[0] in DISCOVERY_IGNORED_OWNERS:
                continue
            if DISCOVERY_NOISE.search(candidate["desc"]) or DISCOVERY_NOISE.search(candidate["full_name"]):
                continue
            pushed = _pushed_datetime(candidate)
            if pushed is not None and now - pushed > timedelta(days=DISCOVERY_STALE_DAYS):
                continue
            found[key] = candidate
    return _newest_first(list(found.values())), failures


def discovery_due(state: dict, now: datetime) -> bool:
    """Monthly, and an undateable state is due rather than cold.

    The same failure the gate was built against: a scan that cannot be dated must not be the reason
    nothing is ever reported.
    """
    raw = state.get("last_scan_at")
    if not raw:
        return True
    try:
        last = datetime.fromisoformat(raw)
    except (ValueError, TypeError):
        return True
    if last.tzinfo is None:
        return True
    return now - last >= timedelta(days=DISCOVERY_INTERVAL_DAYS)


def load_state_at(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return {}


def save_state_at(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")
    os.replace(tmp, path)


def load_state() -> dict:
    return load_state_at(STATE_PATH)


def save_state(state: dict) -> None:
    save_state_at(STATE_PATH, state)


def main(now: datetime | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--status", action="store_true", help="print the whole watchlist, not just what is due")
    ap.add_argument("--dry-run", action="store_true", help="do not write state")
    args = ap.parse_args()

    now = now or datetime.now(timezone.utc)
    state = load_state()
    due_lines: list[str] = []
    status_lines: list[str] = []
    failures: list[str] = []

    for repo, role in WATCHLIST:
        prev = state.get(repo, {})
        try:
            sha, committed_at = head_commit(repo)
        except RuntimeError as err:
            failures.append(str(err))
            continue

        moved = bool(prev.get("sha")) and prev["sha"] != sha
        last_checked = prev.get("last_checked_at")
        elapsed_ok = False
        if last_checked and not moved:
            try:
                looked = datetime.fromisoformat(last_checked)
                elapsed_ok = now - looked >= timedelta(days=float(prev.get("next_due_in_days", 2.0)))
            except (ValueError, TypeError):
                elapsed_ok = True  # unreadable state: treat as due rather than silently cold
        is_due = moved or not last_checked or elapsed_ok
        nxt = interval_days(committed_at, now)

        if is_due:
            reason = "head moved" if moved else ("never looked" if not last_checked else "interval expired")
            was = str(prev.get("sha", "-"))[:8]
            idle = (now - committed_at).days
            due_lines.append(
                f"{repo}\t{role}\thead {sha[:8]}\twas {was}\tlast commit {committed_at:%Y-%m-%d}"
                f" ({idle}d idle)\tnext interval {nxt:g}d\treason: {reason}"
            )
        status_lines.append(
            f"{repo:<38} {sha[:8]}  last commit {committed_at:%Y-%m-%d}  "
            f"{'DUE' if is_due else f'due after {nxt:g}d'}"
        )

        state[repo] = {
            "previous_sha": prev.get("sha", ""),
            "sha": sha,
            "last_commit": committed_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "last_checked_at": now.replace(microsecond=0).isoformat(),
            "next_due_in_days": nxt,
        }

    if not args.dry_run and state:
        save_state(state)

    if args.status:
        print("STATE: " + str(STATE_PATH))
        print("\n".join(status_lines))
        return 0

    for line in due_lines:
        print(line)
    for failure in failures:
        print(f"WATCH FAILURE\t{failure}", file=sys.stderr)

    discovery_failed = False
    discovery_state = load_state_at(DISCOVERY_STATE_PATH)
    if not args.dry_run and discovery_due(discovery_state, now):
        candidates, dead_queries = discover(
            {repo for repo, _ in WATCHLIST}, search=search_repositories, now=now
        )
        seen = dict(discovery_state.get("seen") or {})
        fresh = [c for c in candidates if c["full_name"].lower() not in seen]
        if fresh:
            print(f"DISCOVERY\t{len(fresh)} project(s) not on the watchlist, newest push first")
            for candidate in fresh[:DISCOVERY_MAX_REPORTED]:
                print("DISCOVERY\t" + format_candidate(candidate))
            if len(fresh) > DISCOVERY_MAX_REPORTED:
                print(f"DISCOVERY\t... and {len(fresh) - DISCOVERY_MAX_REPORTED} more, never reported before")
        for dead in dead_queries:
            print(f"DISCOVERY FAILURE\t{dead}", file=sys.stderr)
        discovery_failed = bool(dead_queries)
        if not dead_queries:
            stamp = now.replace(microsecond=0).isoformat()
            for candidate in fresh[:DISCOVERY_MAX_REPORTED]:
                seen[candidate["full_name"].lower()] = stamp
            save_state_at(DISCOVERY_STATE_PATH, {"last_scan_at": stamp, "seen": seen})
    return 1 if (failures or discovery_failed) else 0


if __name__ == "__main__":
    sys.exit(main())
