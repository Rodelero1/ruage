"""The exe map: what the tests are for (D140).

The tool reads one file and classifies strings, so the things that can go wrong are the three classifications and the
missing-file case. A fixture binary is not needed: `read()` takes a `Path`, so the tests write a small file of the exact
strings the classifications care about, with NUL separators the way a PE stores them.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import exe_map

BUILD = (
    r"c:\users\jakes_workstation\documents\projects\age_of_empires_2_de\phoenix"
    r"\age2hd_archive_5.7_with_hotfix\aoc\source"
)


def fixture(extra: bytes = b"") -> Path:
    """A NUL-separated blob with four paths (one a duplicate in another case), three classes, one struct, and noise."""
    strings = [
        BUILD + r"\path.cpp",
        BUILD + r"\pathsys.cpp",
        BUILD + r"\motion.cpp",
        BUILD.upper() + r"\PATH.CPP",  # the same file, spelled differently: one path, not two
        r".?AVCommPacketSyncTurn@@",
        r".?AVCommPacketSyncCmdCount@@",
        r".?AVTRIBE_Unit@@",
        r".?AUTribeUnitData@@",
        r"path.cpp",  # a bare file name is prose, not a build path
        r"c:\windows\system32\drivers\etc\hosts.h",  # not under \source\
    ]
    blob = b"\x00".join(s.encode() for s in strings) + b"\x00" + extra
    handle = tempfile.NamedTemporaryFile(delete=False, suffix=".exe")
    handle.write(blob)
    handle.close()
    return Path(handle.name)


class StringTests(unittest.TestCase):
    def test_a_run_of_printable_bytes_is_one_string(self):
        self.assertEqual(exe_map.strings_of(b"ab\x00hello world\x00"), ["hello world"])

    def test_a_run_shorter_than_the_minimum_is_noise(self):
        self.assertEqual(exe_map.strings_of(b"abc\x00abcdef\x00"), ["abcdef"])


class PathTests(unittest.TestCase):
    def test_only_a_build_path_under_source_counts(self):
        found = exe_map.read(fixture())
        names = [path.rsplit("\\", 1)[-1].lower() for path in found.paths]
        self.assertEqual(names, ["motion.cpp", "path.cpp", "pathsys.cpp"])

    def test_the_same_file_in_another_case_is_one_path(self):
        found = exe_map.read(fixture())
        self.assertEqual(len(found.paths), 3, "PATH.CPP is path.cpp: one module, not two")

    def test_the_build_root_is_reported_because_it_is_provenance(self):
        found = exe_map.read(fixture())
        self.assertEqual(len(found.roots), 1)
        self.assertIn("phoenix", found.roots[0].lower())


class ClassTests(unittest.TestCase):
    def test_a_classifier_separates_classes_from_structs(self):
        found = exe_map.read(fixture())
        self.assertIn("CommPacketSyncTurn", found.classes)
        self.assertIn("TRIBE_Unit", found.classes)
        self.assertEqual(found.structs, ["TribeUnitData"])

    def test_the_same_class_twice_is_one_class(self):
        found = exe_map.read(fixture(extra=b"\x00.?AVTRIBE_Unit@@\x00"))
        self.assertEqual(found.classes.count("TRIBE_Unit"), 1)

    def test_a_family_is_the_classes_that_share_their_leading_word(self):
        found = exe_map.read(fixture())
        self.assertEqual(len(found.of_family("Comm")), 2)
        self.assertEqual(len(found.of_family("TRIBE_")), 1)

    def test_a_family_does_not_swallow_a_word_that_merely_starts_like_it(self):
        """`Comm` and `CommandLineHandler` share four letters and nothing else. The real exe has both, and the first
        version of this tool counted the parser as a netplay packet."""
        found = exe_map.read(fixture(extra=b"\x00.?AVCommandLineHandler@@\x00"))
        self.assertNotIn("CommandLineHandler", found.of_family("Comm"))
        self.assertEqual(len(found.of_family("Comm")), 2)


class FileTests(unittest.TestCase):
    def test_a_missing_binary_is_an_error_not_an_empty_map(self):
        with self.assertRaises(FileNotFoundError):
            exe_map.read(Path("/nonexistent/AoK HD.exe"))

    def test_the_cli_reports_a_missing_binary_as_its_own_exit_code(self):
        code = exe_map.main(["--exe", "/nonexistent/AoK HD.exe"])
        self.assertEqual(code, 2)

    def test_the_human_report_prints_and_succeeds(self):
        """The default path is what a person runs, and it is the one a JSON test cannot cover: it reads the families
        twice, once for the summary and once for the printed line."""
        import contextlib
        import io

        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            code = exe_map.main(["--exe", str(fixture())])
        self.assertEqual(code, 0)
        self.assertIn("source paths", buffer.getvalue())
        self.assertIn("netplay:", buffer.getvalue())

    def test_the_report_is_json_with_the_counts_the_docs_quote(self):
        exe = fixture()
        handle = tempfile.NamedTemporaryFile(delete=False, suffix=".json")
        handle.close()
        import contextlib
        import io

        buffer = io.StringIO()
        with contextlib.redirect_stdout(buffer):
            exe_map.main(["--exe", str(exe), "--json"])
        report = json.loads(buffer.getvalue())
        self.assertEqual(report["source_paths"], 3)
        self.assertEqual(report["classes"], 3)
        self.assertEqual(report["netplay"], ["CommPacketSyncCmdCount", "CommPacketSyncTurn"])
        Path(handle.name).unlink()


if __name__ == "__main__":
    unittest.main()
