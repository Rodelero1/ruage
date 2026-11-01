"""Tests for the save-game container reader.

Test-first shape: every assertion here was written against a *measured* fact from the user's own install (kept out of
git, D8/D22) or a synthetic fixture built to the framing the binary's own reader enforces. The real-fixture classes are
skipped when the install is absent, because a checkout without a game has to stay runnable.

    python3 -m unittest discover -s tools -p 'test_*.py'
"""

from __future__ import annotations

import hashlib
import zlib
import struct
import unittest
from pathlib import Path

import savegame

REPO = Path(__file__).resolve().parent.parent
SAVES = REPO / "Age2HD" / "SaveGame"

# SHA-256 of the fixtures the assertions were measured on. Committed (the files are not): a later session can prove it is
# looking at the same bytes, and a different save fails loudly instead of quietly asserting a different world.
BRIT1_SHA256 = "6d467847755ade735293aa0b897467dbd1f2536f6e998d4453e99b38c8668624"   # 2018 build, save version 12.5
OLD_SHA256 = "43824ead063be2879eea7bf12990a4a88b1e3ba825c1274d2a0697895d6471d0"     # `1.aoe2spgame`, save version 12.36


def framed(text: str, length: int | None = None) -> bytes:
    """One string record as the binary writes it: u32 tag|len, then the bytes."""
    raw = text.encode("latin-1")
    return struct.pack("<I", 0x0A60_0000 | (len(raw) if length is None else length)) + raw


def header(version: float, tail: bytes = b"") -> bytes:
    return b"VER 9.4\x00" + struct.pack("<f", version) + tail


class FramingTests(unittest.TestCase):
    def test_a_string_record_is_a_tagged_length_then_the_bytes(self):
        body = b"\x00" * 7 + framed("Promisory")
        found = list(savegame.iter_strings(body))
        self.assertEqual(found, [(7, "Promisory")])

    def test_a_length_that_runs_past_the_end_is_not_a_string(self):
        # The last four bytes claim nine bytes of text and hold one: a reader that trusts the length reads off the end.
        body = b"\x00" * 4 + struct.pack("<I", 0x0A60_0000 | 9) + b"P"
        self.assertEqual(list(savegame.iter_strings(body)), [])

    def test_a_length_outside_the_plausible_range_is_rejected(self):
        # 300 bytes and 0 bytes are both live values in this format's other records; neither is a name.
        body = struct.pack("<I", 0x0A60_0000 | 0) + struct.pack("<I", 0x0A60_0000 | 300)
        self.assertEqual(list(savegame.iter_strings(body)), [])

    def test_binary_that_is_not_text_is_not_a_string(self):
        body = struct.pack("<I", 0x0A60_0000 | 4) + b"\x00\x01\x02\x03"
        self.assertEqual(list(savegame.iter_strings(body)), [])


class ContainerTests(unittest.TestCase):
    def test_a_dat_file_that_inflates_is_not_mistaken_for_a_save(self):
        # `.dat` files are raw DEFLATE too, and theirs starts `VER 5.7`. Checking only the `VER ` prefix made the reader
        # call a database a save - it is the *save line* (9.3/9.4) that distinguishes them.
        body = b"VER 5.7\x00" + struct.pack("<f", 12.5) + b"\x00" * 8
        packed = zlib.compressobj(6, zlib.DEFLATED, -15)
        raw = packed.compress(body) + packed.flush()
        with self.assertRaises(savegame.NotASave):
            savegame.Header.parse(savegame.inflate(raw))

    def test_an_inflated_body_past_the_cap_is_refused_rather_than_filling_memory(self):
        # A 4 KB DEFLATE stream can ask for gigabytes. The cap is what keeps a corrupt or hostile file from taking the
        # box down; 200 MB from 204 KB is a measured ratio of this format, so the guard is not theoretical.
        packed = zlib.compressobj(1, zlib.DEFLATED, -15)
        raw = packed.compress(b"\x00" * 4096) + packed.flush()
        with self.assertRaises(savegame.NotASave):
            savegame.inflate(raw, max_bytes=1024)

    def test_the_shipped_cap_is_the_one_that_was_chosen(self):
        self.assertEqual(savegame.DEFAULT_MAX_BODY, 64 * 1024 * 1024)


class HeaderTests(unittest.TestCase):
    def test_the_header_is_the_version_string_then_a_float_save_version(self):
        # 0x41480000 *is* the string `HA` when read as two chars - which is how this reader first misread the format.
        parsed = savegame.Header.parse(header(12.5) + b"\x00" * 8)
        self.assertEqual(parsed.magic, "VER 9.4")
        self.assertAlmostEqual(parsed.version, 12.5, places=4)

    def test_the_older_save_line_is_a_version_of_its_own_and_not_a_different_format(self):
        parsed = savegame.Header.parse(header(12.36) + b"\x00" * 8)
        self.assertAlmostEqual(parsed.version, 12.36, places=4)

    def test_a_version_the_game_itself_would_refuse_is_refused_here(self):
        # FUN_005e6730 rejects a save version below 0 or above 12.5 (`Invalid save game version %f`); a reader that
        # accepted 13.0 would claim to understand a format the game does not.
        with self.assertRaises(savegame.NotASave):
            savegame.Header.parse(header(13.0) + b"\x00" * 8)

    def test_a_file_that_is_not_a_save_is_refused(self):
        with self.assertRaises(savegame.NotASave):
            savegame.Header.parse(b"VER 9.4\x00" + struct.pack("<f", 12.5)[:2])

    def test_a_truncated_file_is_refused_rather_than_indexed_past(self):
        with self.assertRaises(savegame.NotASave):
            savegame.Header.parse(b"VER 9.4\x00\x00\x00")

    def test_a_world_of_empires_file_is_not_an_age_of_empires_save(self):
        with self.assertRaises(savegame.NotASave):
            savegame.Header.parse(b"VER 9.4\x00" + b"\xff" * 12)


@unittest.skipUnless((SAVES / "brit1.aoe2spgame").exists(), "no install: the fixture lives in the user's save folder")
class NewSaveFixtureTests(unittest.TestCase):
    def setUp(self):
        self.path = SAVES / "brit1.aoe2spgame"
        digest = hashlib.sha256(self.path.read_bytes()).hexdigest()
        if digest != BRIT1_SHA256:
            self.skipTest(f"brit1.aoe2spgame is not the fixture the numbers were measured on ({digest})")
        self.body = savegame.inflate(self.path.read_bytes())

    def test_the_container_is_raw_deflate_that_ends_exactly_at_eof(self):
        self.assertEqual(len(self.body), 3_963_296)

    def test_the_header_says_version_12_5_and_a_second_number_of_1006(self):
        parsed = savegame.Header.parse(self.body)
        self.assertEqual(parsed.magic, "VER 9.4")
        self.assertAlmostEqual(parsed.version, 12.5, places=4)
        self.assertAlmostEqual(parsed.unidentified, 1006.0, places=4)

    def test_the_named_records_are_where_the_walk_finds_them(self):
        strings = dict(savegame.iter_strings(self.body))
        self.assertEqual(strings[0xA4], "Promisory")
        self.assertEqual(strings[0xB6], "Alfred the Great")
        self.assertEqual(strings[0x2C7], "Black_Forest")
        self.assertEqual(strings[0x2FB], "Standard Game")

    def test_the_walk_does_not_depend_on_what_it_has_already_seen(self):
        # Two passes over the same bytes must agree: a walker that mutates its cursor on a rejected candidate would not.
        self.assertEqual(list(savegame.iter_strings(self.body)), list(savegame.iter_strings(self.body)))


@unittest.skipUnless((SAVES / "1.aoe2spgame").exists(), "no install: the fixture lives in the user's save folder")
class OldSaveFixtureTests(unittest.TestCase):
    def setUp(self):
        self.path = SAVES / "1.aoe2spgame"
        digest = hashlib.sha256(self.path.read_bytes()).hexdigest()
        if digest != OLD_SHA256:
            self.skipTest(f"1.aoe2spgame is not the fixture the numbers were measured on ({digest})")

    def test_an_older_save_reads_with_its_own_version_and_not_as_a_corrupt_file(self):
        # This file's bytes 10..11 are `A\\0`, not `HA`: the first version of this reader refused eight of the user's
        # own saves because it had read a float as text. Refusing a real save is the defect this test pins.
        parsed = savegame.Header.parse(savegame.inflate(self.path.read_bytes()))
        self.assertEqual(parsed.magic, "VER 9.4")
        self.assertAlmostEqual(parsed.version, 12.36, places=4)


if __name__ == "__main__":
    unittest.main()
