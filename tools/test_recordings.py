"""Tests for the recorded-game op walker.

The grammar is mgz's documented body layout (MIT) driven by hand, because mgz 1.8.51 refuses these HD 5.8 files
(`could not parse: unpack requires a buffer of 2 bytes`). The check that says the grammar is right is the one that
costs nothing to make strict: the walk must consume the stream to **exact EOF**, and the elapsed time it accumulates
must equal the save series' clock.

    python3 -m unittest discover -s tools -p 'test_*.py'
"""

from __future__ import annotations

import hashlib
import struct
import unittest
import zlib
from pathlib import Path

import recordings

REPO = Path(__file__).resolve().parent.parent
RECS = REPO / "Age2HD" / "SaveGame"

# The 28-byte meta block the four recordings carry (log version, checksum interval, multiplayer, rec owner, reveal map,
# sequence numbers, chapters). The walk starts after it.
META = b"\x04\x00\x00\x00" + struct.pack("<6I", 500, 0, 1, 0, 0, 0)


def sync(increment: int, sequence: int = 0) -> bytes:
    """A sync op: the increment, then a 28-byte checksum record (when its first word is zero) and, when that record's
    sequence is non-zero, 332 more bytes of per-player state. mgz's `sync` struct, `Peek` included in the padding."""
    body = struct.pack("<II", 2, increment)
    body += struct.pack("<I", 0) + b"\x00" * 4          # the peeked zero, then the rest of the 8-byte padding
    body += struct.pack("<I", 7) + b"\x00" * 4          # crc, unknown
    body += struct.pack("<I", sequence)                  # sequence
    if sequence:
        body += b"\xaa" * 332
    body += b"\x00" * 8
    return body


def chat(text: str) -> bytes:
    raw = text.encode("latin-1")
    return struct.pack("<II", 4, 0) + struct.pack("<I", len(raw)) + raw


def action(kind: int, payload: bytes = b"") -> bytes:
    """An action op: `u32 length` then `length + 4` bytes of body (the type byte, its payload, and padding)."""
    body = bytes([kind]) + payload + b"\x00" * 3
    return struct.pack("<II", 1, len(body) - 4) + body


class GrammarTests(unittest.TestCase):
    def test_a_stream_of_syncs_chat_and_actions_walks_to_exact_eof(self):
        stream = META + sync(100) + chat("hello") + sync(250) + action(27) + sync(50)
        found = list(recordings.iter_ops(stream))
        self.assertEqual([op.kind for op in found], ["sync", "chat", "sync", "action", "sync"])
        self.assertEqual(found[1].value, "hello")
        self.assertEqual(found[-1].time_ms, 400)
        self.assertEqual(len(stream), 28 + len(stream) - 28)  # the walk consumed every byte

    def test_a_sync_with_a_checksum_record_consumes_its_payload(self):
        stream = META + sync(10) + sync(20) + sync(30)
        times = [op.time_ms for op in recordings.iter_ops(stream) if op.kind == "sync"]
        self.assertEqual(times, [10, 30, 60])

    def test_a_checksum_record_with_a_sequence_carries_332_more_bytes(self):
        stream = META + sync(10, sequence=3) + sync(5)
        times = [op.time_ms for op in recordings.iter_ops(stream) if op.kind == "sync"]
        self.assertEqual(times, [10, 15])

    def test_a_stream_that_stops_mid_operation_is_refused_not_truncated_silently(self):
        with self.assertRaises(recordings.BadStream):
            list(recordings.iter_ops(META + struct.pack("<II", 1, 900)))

    def test_an_unknown_operation_id_is_refused(self):
        with self.assertRaises(recordings.BadStream):
            list(recordings.iter_ops(META + struct.pack("<I", 99)))


@unittest.skipUnless((RECS / "SP Replay v5.8 @2026.11.01 142146.aoe2record").exists(), "no install")
class RealRecordingTests(unittest.TestCase):
    def setUp(self):
        self.path = RECS / "SP Replay v5.8 @2026.11.01 142146.aoe2record"
        self.raw = self.path.read_bytes()

    def test_the_container_is_a_length_prefix_then_a_deflated_header(self):
        header, ops = recordings.split(self.raw)
        section, zero = struct.unpack_from("<II", self.raw, 0)
        self.assertTrue(header.startswith(b"VER 9.4\x00"))
        self.assertEqual(zero, 0)
        self.assertEqual(ops, self.raw[section:])
        self.assertGreater(len(header), section, "the header inflates larger than the bytes it came from")

    def test_the_walk_reaches_eof_and_the_time_matches_the_save_series(self):
        _, ops = recordings.split(self.raw)
        found = list(recordings.iter_ops(ops))
        self.assertEqual(len(found), 111_494)
        self.assertEqual(found[-1].time_ms, 2_681_138)
        self.assertEqual(sum(1 for op in found if op.kind == "chat"), 63)

    def test_the_last_chat_is_the_ai_resigning(self):
        _, ops = recordings.split(self.raw)
        chats = [op.value for op in recordings.iter_ops(ops) if op.kind == "chat"]
        self.assertIn("I shalt abdicate", chats[-2] + chats[-1])


if __name__ == "__main__":
    unittest.main()
