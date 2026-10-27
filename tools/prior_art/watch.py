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

Usage:
  watch.py              # gate: print due projects only
  watch.py --status     # print the whole watchlist with due dates
  watch.py --dry-run    # do not write state
Exit code is 0 whether or not anything is due; a non-zero exit means the check itself failed.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
STATE_PATH = REPO_ROOT / "build" / "prior-art-state.json"
API = "https://api.github.com/repos/{repo}/commits?per_page=1"

# The watchlist, grouped exactly as docs/prior-art.md describes it. Order is the report order:
# the reference engine first, then the active deterministic implementation, then the rest.
WATCHLIST: list[tuple[str, str]] = [
    ("SFTtech/openage", "reference engine: asset pipeline, engine/content split, time model"),
    ("librematch/genie", "active deterministic reimplementation and format readers"),
    ("Zaydme/openaok", "from-scratch AoK engine: fixed tick, pather, dat 5.7 reader"),
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


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--status", action="store_true", help="print the whole watchlist, not just what is due")
    ap.add_argument("--dry-run", action="store_true", help="do not write state")
    args = ap.parse_args()

    now = datetime.now(timezone.utc)
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
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
