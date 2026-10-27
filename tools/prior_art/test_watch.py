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
        watch.WATCHLIST = [("owner/repo", "a project")]
        watch.STATE_PATH = self.path

    def run_gate(self, head_sha, committed_at, argv=None):
        watch.head_commit = lambda repo: (head_sha, committed_at)
        out = io.StringIO()
        old = sys.argv
        sys.argv = ["watch.py"] + (argv or [])
        try:
            with contextlib.redirect_stdout(out):
                code = watch.main()
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
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        watch.WATCHLIST = [("owner/gone", "a project")]
        watch.STATE_PATH = self.path

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
        self.addCleanup(setattr, watch, "STATE_PATH", watch.STATE_PATH)
        watch.WATCHLIST = [("owner/works", "a project"), ("owner/gone", "another")]
        watch.STATE_PATH = Path(self.tmp.name) / "build" / "prior-art-state.json"

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
