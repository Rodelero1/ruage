"""Behavioural tests for the prior-art gate.

The gate's failure mode that matters is silence: a watch that reports nothing because it is
broken looks exactly like a quiet day. So each test pins what "due" means, and the last one
pins that a broken state file makes projects due rather than cold.

Run: python3 -m unittest discover -s tools/prior_art -v   (or pytest)
"""

import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("watch", HERE / "watch.py")
watch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(watch)

NOW = datetime(2026, 10, 27, 9, 0, tzinfo=timezone.utc)


class IntervalTests(unittest.TestCase):
    def test_active_project_is_looked_at_every_two_days(self):
        self.assertEqual(watch.interval_days(NOW - timedelta(days=1), NOW), 2.0)

    def test_warm_project_weekly(self):
        self.assertEqual(watch.interval_days(NOW - timedelta(days=5), NOW), 7.0)

    def test_cold_project_fortnightly(self):
        self.assertEqual(watch.interval_days(NOW - timedelta(days=400), NOW), 14.0)


class StateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "build" / "prior-art-state.json"

    def test_missing_state_is_empty_not_an_error(self):
        self.assertEqual(watch.load_state_at(self.path), {})

    def test_corrupt_state_is_empty_not_an_error(self):
        self.path.parent.mkdir(parents=True)
        self.path.write_text("{not json")
        self.assertEqual(watch.load_state_at(self.path), {})

    def test_save_leaves_no_temp_file(self):
        watch.save_state_at(self.path, {"a/b": {"sha": "x"}})
        self.assertEqual(json.loads(self.path.read_text()), {"a/b": {"sha": "x"}})
        self.assertEqual([p.name for p in self.path.parent.iterdir()], ["prior-art-state.json"])


class GateTests(unittest.TestCase):
    """One repo, one fetch, inspected through main()'s stdout — the thing cron actually reads."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "build" / "prior-art-state.json"
        self.addCleanup(setattr, watch, "WATCHLIST", watch.WATCHLIST)
        self.addCleanup(setattr, watch, "head_commit", watch.head_commit)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        watch.WATCHLIST = [("owner/repo", "a project")]
        watch.STATE_PATH = self.path
        watch.DISCOVERY_STATE_PATH = Path(self.tmp.name) / "build" / "prior-art-discovery.json"
        watch.search_repositories = lambda query, per_page=30: []

    def run_gate(self, head_sha, committed_at, argv=None):
        watch.head_commit = lambda repo: (head_sha, committed_at)
        out = io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"] + (argv or [])
        try:
            with contextlib.redirect_stdout(out):
                code = watch.main(now=NOW)
        finally:
            sys.argv = old
        return out.getvalue(), code

    def seed(self, sha, looked, interval=2.0):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps({"owner/repo": {
            "sha": sha, "last_checked_at": looked.isoformat(), "next_due_in_days": interval,
            "last_commit": "2026-10-01T00:00:00Z"}}))

    def test_never_looked_is_due(self):
        out, _ = self.run_gate("a" * 40, NOW - timedelta(days=1))
        self.assertIn("owner/repo", out)
        self.assertIn("never looked", out)

    def test_unchanged_within_interval_is_silent(self):
        self.seed("a" * 40, NOW - timedelta(hours=6))
        out, _ = self.run_gate("a" * 40, NOW - timedelta(days=1))
        self.assertEqual(out, "")

    def test_head_moving_overrides_a_quiet_interval(self):
        self.seed("a" * 40, NOW - timedelta(hours=6))
        out, _ = self.run_gate("b" * 40, NOW - timedelta(hours=2))
        self.assertIn("head moved", out)
        self.assertIn("was aaaaaaaa", out)

    def test_expired_interval_is_due_even_without_a_move(self):
        self.seed("a" * 40, NOW - timedelta(days=3))
        out, _ = self.run_gate("a" * 40, NOW - timedelta(days=3))
        self.assertIn("interval expired", out)

    def test_run_records_the_head_so_tomorrow_is_quiet(self):
        self.seed("a" * 40, NOW - timedelta(days=3))
        self.run_gate("c" * 40, NOW - timedelta(days=1))
        state = json.loads(self.path.read_text())
        self.assertEqual(state["owner/repo"]["sha"], "c" * 40)

    def test_broken_state_makes_the_project_due_rather_than_cold(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps({"owner/repo": {"sha": "a" * 40, "last_checked_at": "garbage"}}))
        out, _ = self.run_gate("a" * 40, NOW - timedelta(days=1))
        self.assertIn("interval expired", out)


if __name__ == "__main__":
    unittest.main()


class FailureTests(unittest.TestCase):
    """A fetch that fails must be loud, and must leave that project's state alone."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "build" / "prior-art-state.json"
        self.addCleanup(setattr, watch, "WATCHLIST", watch.WATCHLIST)
        self.addCleanup(setattr, watch, "head_commit", watch.head_commit)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        watch.WATCHLIST = [("owner/gone", "a project")]
        watch.STATE_PATH = self.path
        watch.DISCOVERY_STATE_PATH = Path(self.tmp.name) / "build" / "prior-art-discovery.json"
        watch.search_repositories = lambda query, per_page=30: []

    def test_a_failed_fetch_fails_the_run_and_names_the_repo(self):
        def boom(repo):
            raise RuntimeError("owner/gone: HTTP Error 403: rate limited")

        watch.head_commit = boom
        out, err = io.StringIO(), io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"]
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = watch.main()
        finally:
            sys.argv = old
        self.assertEqual(code, 1)
        self.assertIn("WATCH FAILURE", err.getvalue())
        self.assertIn("owner/gone", err.getvalue())

    def test_a_failed_fetch_does_not_advance_that_projects_state(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps({"owner/gone": {
            "sha": "a" * 40, "last_checked_at": "2026-10-20T00:00:00+00:00",
            "next_due_in_days": 2.0, "last_commit": "2026-10-01T00:00:00Z"}}))
        watch.head_commit = lambda repo: (_ for _ in ()).throw(RuntimeError("nope"))
        old = sys.argv
        sys.argv = ["watch.py"]
        try:
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                watch.main()
        finally:
            sys.argv = old
        state = json.loads(self.path.read_text())
        self.assertEqual(state["owner/gone"]["last_checked_at"], "2026-10-20T00:00:00+00:00")

    def test_a_naive_timestamp_is_due_rather_than_a_crash(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps({"owner/gone": {
            "sha": "a" * 40, "last_checked_at": "2026-10-25T09:00:00", "next_due_in_days": 2.0}}))
        watch.head_commit = lambda repo: ("a" * 40, NOW - timedelta(days=1))
        out = io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"]
        try:
            with contextlib.redirect_stdout(out):
                code = watch.main()
        finally:
            sys.argv = old
        self.assertEqual(code, 0)
        self.assertIn("interval expired", out.getvalue())


class MixedRunTests(unittest.TestCase):
    """A partial failure must not be hidden by the projects that worked."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.addCleanup(setattr, watch, "WATCHLIST", watch.WATCHLIST)
        self.addCleanup(setattr, watch, "head_commit", watch.head_commit)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        watch.WATCHLIST = [("owner/works", "a project"), ("owner/gone", "another")]
        watch.STATE_PATH = Path(self.tmp.name) / "build" / "prior-art-state.json"
        watch.DISCOVERY_STATE_PATH = Path(self.tmp.name) / "build" / "prior-art-discovery.json"
        watch.search_repositories = lambda query, per_page=30: []

    def test_one_due_project_does_not_mask_one_failed_fetch(self):
        def head(repo):
            if repo == "owner/gone":
                raise RuntimeError("owner/gone: HTTP Error 403: rate limited")
            return ("b" * 40, NOW - timedelta(days=1))

        watch.head_commit = head
        out, err = io.StringIO(), io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"]
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = watch.main()
        finally:
            sys.argv = old
        self.assertIn("owner/works", out.getvalue())
        self.assertEqual(code, 1, "a failed fetch must fail the run even when something else is due")
        self.assertIn("owner/gone", err.getvalue())


class WatchlistTests(unittest.TestCase):
    """The list itself is behaviour: a project the owner asked for must be on it, with its role."""

    def test_the_original_art_replica_is_watched_with_its_engine_named(self):
        roles = dict(watch.WATCHLIST)
        self.assertIn("yanfengliu/aoe2", roles)
        self.assertIn("civ-engine", roles["yanfengliu/aoe2"])


class DiscoveryCadenceTests(unittest.TestCase):
    """The sweep is monthly, and a sweep that cannot be dated is due — never quietly skipped."""

    def test_never_scanned_is_due(self):
        self.assertTrue(watch.discovery_due({}, NOW))

    def test_scanned_three_days_ago_is_not_due(self):
        state = {"last_scan_at": (NOW - timedelta(days=3)).replace(microsecond=0).isoformat()}
        self.assertFalse(watch.discovery_due(state, NOW))

    def test_scanned_31_days_ago_is_due(self):
        state = {"last_scan_at": (NOW - timedelta(days=31)).replace(microsecond=0).isoformat()}
        self.assertTrue(watch.discovery_due(state, NOW))

    def test_unreadable_scan_time_is_due_rather_than_cold(self):
        self.assertTrue(watch.discovery_due({"last_scan_at": "garbage"}, NOW))


def _repo(full_name, *, fork=False, archived=False, desc="a project", stars=1, lang="Rust",
          pushed_days_ago=1, lic="MIT"):
    return {
        "full_name": full_name, "fork": fork, "archived": archived, "description": desc,
        "stargazers_count": stars, "language": lang, "license": {"spdx_id": lic},
        "pushed_at": (NOW - timedelta(days=pushed_days_ago)).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "created_at": (NOW - timedelta(days=200)).strftime("%Y-%m-%dT%H:%M:%SZ"),
    }


class DiscoveryFilterTests(unittest.TestCase):
    """What the sweep may show the agent: things it cannot already know, and nothing unjudgeable."""

    def sweep(self, items_by_query, known=()):
        calls = []

        def search(query):
            calls.append(query)
            return items_by_query.get(query, [])

        candidates, failures = watch.discover(known=set(known), search=search, now=NOW)
        return candidates, failures, calls

    def test_a_new_project_with_a_description_is_a_candidate(self):
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: [_repo("someone/new-engine")]})
        self.assertEqual([c["full_name"] for c in candidates], ["someone/new-engine"])

    def test_an_already_watched_project_is_not_a_candidate(self):
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: [_repo("SFTtech/openage")]},
                                      known={"SFTtech/openage"})
        self.assertEqual(candidates, [])

    def test_forks_and_archived_repos_are_dropped(self):
        items = [_repo("a/fork", fork=True), _repo("a/archived", archived=True), _repo("a/live")]
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: items})
        self.assertEqual([c["full_name"] for c in candidates], ["a/live"])

    def test_a_project_without_a_description_cannot_be_judged_and_is_dropped(self):
        items = [_repo("a/mystery", desc=""), _repo("a/described")]
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: items})
        self.assertEqual([c["full_name"] for c in candidates], ["a/described"])

    def test_a_project_untouched_for_over_three_months_is_dropped(self):
        items = [_repo("a/stale", pushed_days_ago=watch.DISCOVERY_STALE_DAYS + 1),
                 _repo("a/warm", pushed_days_ago=watch.DISCOVERY_STALE_DAYS - 1)]
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: items})
        self.assertEqual([c["full_name"] for c in candidates], ["a/warm"])

    def test_a_project_found_by_two_queries_is_reported_once(self):
        items = [_repo("a/engine", stars=5)]
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: items,
                                       watch.DISCOVERY_QUERIES[1]: items})
        self.assertEqual(len(candidates), 1)

    def test_the_newest_push_comes_first(self):
        items = [_repo("a/old", pushed_days_ago=40), _repo("a/new", pushed_days_ago=1),
                 _repo("a/mid", pushed_days_ago=10)]
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: items})
        self.assertEqual([c["full_name"] for c in candidates], ["a/new", "a/mid", "a/old"])

    def test_a_candidate_carries_what_a_relevance_judgement_needs(self):
        candidates, _, _ = self.sweep({watch.DISCOVERY_QUERIES[0]: [
            _repo("a/engine", stars=7, lang="C++", lic="MIT", desc="an AoE2 engine")]})
        candidate = candidates[0]
        for field in ("full_name", "desc", "stars", "lang", "licence", "pushed"):
            self.assertIn(field, candidate)
        self.assertEqual(candidate["stars"], 7)
        self.assertEqual(candidate["licence"], "MIT")

    def test_every_query_is_asked(self):
        _, _, calls = self.sweep({})
        self.assertEqual(calls, watch.DISCOVERY_QUERIES)

    def test_one_failed_query_is_named_and_the_others_still_run(self):
        def search(query):
            raise RuntimeError("HTTP Error 403: rate limit exceeded")

        candidates, failures = watch.discover(known=set(), search=search, now=NOW)
        self.assertEqual(candidates, [])
        self.assertEqual(len(failures), len(watch.DISCOVERY_QUERIES))
        self.assertIn("rate limit", failures[0])


class DiscoveryInTheGateTests(unittest.TestCase):
    """The sweep as cron sees it: what reaches stdout, what reaches stderr, and what wakes a quiet day."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.build = Path(self.tmp.name) / "build"
        self.addCleanup(setattr, watch, "WATCHLIST", watch.WATCHLIST)
        self.addCleanup(setattr, watch, "head_commit", watch.head_commit)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        watch.WATCHLIST = [("owner/repo", "a project")]
        watch.STATE_PATH = self.build / "prior-art-state.json"
        watch.DISCOVERY_STATE_PATH = self.build / "prior-art-discovery.json"
        watch.head_commit = lambda repo: ("a" * 40, NOW - timedelta(days=1))
        self.searches = []

    def search_returning(self, items):
        def search(query):
            self.searches.append(query)
            return items if query == watch.DISCOVERY_QUERIES[0] else []
        watch.search_repositories = search

    def run_gate(self, argv=None, discovery_state=None):
        if discovery_state is not None:
            self.build.mkdir(parents=True, exist_ok=True)
            watch.DISCOVERY_STATE_PATH.write_text(json.dumps(discovery_state))
        out, err = io.StringIO(), io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"] + (argv or [])
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = watch.main(now=NOW)
        finally:
            sys.argv = old
        return out.getvalue(), err.getvalue(), code

    def discovery_lines(self, out):
        return [line for line in out.splitlines() if line.startswith("DISCOVERY\t")]

    def test_a_due_sweep_prints_candidates_the_agent_can_judge(self):
        self.search_returning([_repo("someone/new-engine", stars=12, lang="Rust")])
        out, _, code = self.run_gate()
        lines = self.discovery_lines(out)
        self.assertEqual(code, 0)
        self.assertTrue(any("someone/new-engine" in line for line in lines), out)
        self.assertTrue(any("12" in line for line in lines), out)

    def test_a_due_sweep_wakes_a_quiet_day(self):
        self.search_returning([_repo("someone/new-engine")])
        out, _, _ = self.run_gate()  # no state at all: the watched repo is never looked at, so due
        self.assertNotEqual(out, "", "a sweep with candidates must not be suppressed as an empty tick")

    def test_nothing_is_asked_within_the_month(self):
        self.search_returning([_repo("someone/new-engine")])
        self.run_gate(discovery_state={"last_scan_at": (NOW - timedelta(days=3)).isoformat(), "seen": {}})
        self.assertEqual(self.searches, [])

    def test_dry_run_does_not_sweep(self):
        self.search_returning([_repo("someone/new-engine")])
        self.run_gate(argv=["--dry-run"])
        self.assertEqual(self.searches, [])

    def test_a_complete_sweep_stamps_the_scan_and_remembers_what_it_saw(self):
        self.search_returning([_repo("someone/new-engine")])
        self.run_gate()
        state = json.loads(watch.DISCOVERY_STATE_PATH.read_text())
        self.assertEqual(state["last_scan_at"], NOW.replace(microsecond=0).isoformat())
        self.assertIn("someone/new-engine", state["seen"])

    def test_a_failed_sweep_is_loud_and_does_not_stamp_the_scan(self):
        def boom(query):
            raise RuntimeError("HTTP Error 403: rate limit exceeded")

        watch.search_repositories = boom
        out, err, code = self.run_gate()
        self.assertEqual(code, 1, "a sweep that could not run must fail the run, not pass as quiet")
        self.assertIn("DISCOVERY FAILURE", err)
        self.assertFalse(watch.DISCOVERY_STATE_PATH.exists())
        self.assertEqual(self.discovery_lines(out), [])

    def test_a_candidate_already_seen_is_not_reported_again(self):
        self.search_returning([_repo("someone/known")])
        self.run_gate(discovery_state={
            "last_scan_at": (NOW - timedelta(days=40)).isoformat(),
            "seen": {"someone/known": "2026-09-01T00:00:00+00:00"}})
        out, _, _ = self.run_gate(discovery_state={
            "last_scan_at": (NOW - timedelta(days=40)).isoformat(),
            "seen": {"someone/known": "2026-09-01T00:00:00+00:00"}})
        self.assertEqual(self.discovery_lines(out), [])

    def test_the_report_is_capped_and_says_how_many_were_found(self):
        many = [_repo(f"someone/repo{n}", pushed_days_ago=n + 1) for n in range(watch.DISCOVERY_MAX_REPORTED + 4)]
        self.search_returning(many)
        out, _, _ = self.run_gate()
        printed = [line for line in self.discovery_lines(out) if "someone/repo" in line]
        self.assertEqual(len(printed), watch.DISCOVERY_MAX_REPORTED)
        self.assertTrue(any("more" in line for line in self.discovery_lines(out)), out)


class DiscoveryReviewTests(unittest.TestCase):
    """Defects found reviewing the sweep before shipping it: overflow, unescaped text, unstable order."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.build = Path(self.tmp.name) / "build"
        self.addCleanup(setattr, watch, "WATCHLIST", watch.WATCHLIST)
        self.addCleanup(setattr, watch, "head_commit", watch.head_commit)
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        self.addCleanup(setattr, watch, "DISCOVERY_STATE_PATH", watch.DISCOVERY_STATE_PATH)
        self.addCleanup(setattr, watch, "search_repositories", watch.search_repositories)
        watch.WATCHLIST = [("owner/repo", "a project")]
        watch.STATE_PATH = self.build / "prior-art-state.json"
        watch.DISCOVERY_STATE_PATH = self.build / "prior-art-discovery.json"
        watch.head_commit = lambda repo: ("a" * 40, NOW - timedelta(days=1))

    def gate(self, items):
        watch.search_repositories = lambda query, per_page=30: items if query == watch.DISCOVERY_QUERIES[0] else []
        out = io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"]
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
                watch.main(now=NOW)
        finally:
            sys.argv = old
        return out.getvalue()

    def test_overflow_is_not_marked_seen_so_it_can_surface_next_month(self):
        items = [_repo(f"someone/repo{n}", pushed_days_ago=n + 1)
                 for n in range(watch.DISCOVERY_MAX_REPORTED + 4)]
        self.gate(items)
        state = json.loads(watch.DISCOVERY_STATE_PATH.read_text())
        self.assertEqual(len(state["seen"]), watch.DISCOVERY_MAX_REPORTED,
                         "only what was actually reported may be marked seen, or the overflow is lost")

    def test_a_description_with_a_tab_or_newline_cannot_break_the_line(self):
        items = [_repo("someone/tabbed", desc="an engine\twith\na newline")]
        out = self.gate(items)
        matching = [l for l in out.splitlines() if "someone/tabbed" in l]
        self.assertEqual(len(matching), 1, "the description must not split the line: " + repr(matching))
        # one DISCOVERY prefix column + five fields
        self.assertEqual(matching[0].count("\t"), 6, matching[0])

    def test_equal_push_times_are_ordered_by_name_so_the_report_is_stable(self):
        items = [_repo("someone/zed", pushed_days_ago=2), _repo("someone/alpha", pushed_days_ago=2)]
        out = self.gate(items)
        names = [l.split("\t")[1] for l in out.splitlines() if l.startswith("DISCOVERY\t") and "/" in l]
        self.assertEqual(names, ["someone/alpha", "someone/zed"])


class DiscoveryNoiseTests(unittest.TestCase):
    """Measured against live data: the readme qualifier is the only thing that finds a project whose
    description says "Age of Empires 2" and whose name says nothing, so recall stays and the noise
    it drags in (curated lists, SEO drill sites, our own repos) is filtered instead."""

    def sweep(self, items):
        return watch.discover(known=set(), search=lambda q: items, now=NOW)[0]

    def test_curated_lists_are_not_candidates(self):
        items = [_repo("someone/awesome-aoe", desc="A curated list of AoE2 resources"),
                 _repo("someone/directory", desc="A directory of open-source game engines"),
                 _repo("someone/engine", desc="An AoE2 engine in Rust")]
        self.assertEqual([c["full_name"] for c in self.sweep(items)], ["someone/engine"])

    def test_an_seo_drill_site_is_not_a_candidate(self):
        items = [_repo("someone/drills", desc="Best Browser-Based RTS Hotkey Trainer 2026"),
                 _repo("someone/engine", desc="A from-scratch Age of Empires II engine")]
        self.assertEqual([c["full_name"] for c in self.sweep(items)], ["someone/engine"])

    def test_our_own_repositories_are_not_prior_art(self):
        items = [_repo("Rodelero1/ruage-tools", desc="Importer for ruage: AoE2 HD .dat files"),
                 _repo("someone/engine", desc="An AoE2 engine")]
        self.assertEqual([c["full_name"] for c in self.sweep(items)], ["someone/engine"])
