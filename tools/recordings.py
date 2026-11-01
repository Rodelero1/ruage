"""Walk a recorded game's operation stream.

A recording is `[u32 section_len][u32 0]` + a **deflated header** (a `VER 9.4` payload, the same shape a save has),
then an **uncompressed** op stream: a 28-byte meta block and then `[u32 op][payload]`. The payload grammar is the one
mgz documents for its body (MIT, `mgz/body/__init__.py`): action = `u32 length` + `length+4` bytes; sync = `u32`
increment plus, when the next word is zero, a 28-byte checksum record (+332 when its sequence is non-zero); viewlock =
two floats and a player id; chat = 4 pad, `u32 length`, text.

The validation is the EOF check: a walk that does not consume the stream exactly is refused rather than truncated
silently. All four recordings in the install walk to exact EOF, and the elapsed time the walk accumulates on
`142146` is **2 681 138 ms**, which is the same number the save series' clock gives for its last save - two artifacts,
one timeline (`docs/save-format.md`).

    python3 tools/recordings.py Age2HD/SaveGame/*.aoe2record
"""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from dataclasses import dataclass
from typing import Iterator

META_LEN = 28
SYNC, ACTION, VIEWLOCK, CHAT = 2, 1, 3, 4
KNOWN_OPS = (SYNC, ACTION, VIEWLOCK, CHAT)


class BadStream(Exception):
    """The bytes are not an op stream this walker understands. Never a partial read."""


def split(raw: bytes) -> tuple[bytes, bytes]:
    """`(header, ops)`: the inflated header section and the op stream that follows it."""
    if len(raw) < 8:
        raise BadStream(f"only {len(raw)} bytes: too short for a recording container")
    section, zero = struct.unpack_from("<II", raw, 0)
    if zero != 0:
        raise BadStream(f"the word after the section length is {zero}, not 0")
    if not 8 <= section <= len(raw):
        raise BadStream(f"section length {section} does not fit a {len(raw)} byte file")
    header = zlib.decompressobj(-15).decompress(raw[8:section])
    if not header.startswith(b"VER "):
        raise BadStream(f"the header section does not inflate to a version string: {header[:8]!r}")
    return header, raw[section:]


@dataclass(frozen=True)
class Op:
    """One operation: its time (ms since the match started), its kind, and its payload where it has a readable one."""

    time_ms: int
    kind: str
    value: object = None


def iter_ops(stream: bytes, start: int = META_LEN) -> Iterator[Op]:
    """Every operation from `start` (just past the meta block) to the end of the stream."""
    pos = start
    elapsed = 0
    while pos < len(stream):
        if pos + 4 > len(stream):
            raise BadStream(f"{len(stream) - pos} trailing bytes at {pos:#x}: not an operation")
        op = struct.unpack_from("<I", stream, pos)[0]
        pos += 4
        if op == ACTION:
            if pos + 4 > len(stream):
                raise BadStream(f"action header cut short at {pos:#x}")
            length = struct.unpack_from("<I", stream, pos)[0]
            pos += 4
            if pos + length + 4 > len(stream):
                raise BadStream(f"action body of {length} bytes runs past the end at {pos:#x}")
            yield Op(elapsed, "action", stream[pos : pos + length + 4])
            pos += length + 4
        elif op == SYNC:
            if pos + 4 > len(stream):
                raise BadStream(f"sync cut short at {pos:#x}")
            elapsed += struct.unpack_from("<I", stream, pos)[0]
            pos += 4
            if pos + 4 <= len(stream) and struct.unpack_from("<I", stream, pos)[0] == 0:
                if pos + 28 > len(stream):
                    raise BadStream(f"checksum record cut short at {pos:#x}")
                sequence = struct.unpack_from("<I", stream, pos + 16)[0]
                pos += 28
                if sequence:
                    if pos + 332 > len(stream):
                        raise BadStream(f"sequence state runs past the end at {pos:#x}")
                    pos += 332
            yield Op(elapsed, "sync")
        elif op == VIEWLOCK:
            if pos + 12 > len(stream):
                raise BadStream(f"viewlock cut short at {pos:#x}")
            x, y, player = struct.unpack_from("<ffI", stream, pos)
            pos += 12
            yield Op(elapsed, "viewlock", (x, y, player))
        elif op == CHAT:
            if pos + 8 > len(stream):
                raise BadStream(f"chat header cut short at {pos:#x}")
            length = struct.unpack_from("<I", stream, pos + 4)[0]
            pos += 8
            if pos + length > len(stream):
                raise BadStream(f"chat text of {length} bytes runs past the end at {pos:#x}")
            yield Op(elapsed, "chat", stream[pos : pos + length].decode("latin-1"))
            pos += length
        else:
            raise BadStream(f"unknown operation {op} at {pos - 4:#x}")


def describe(path: str) -> None:
    with open(path, "rb") as handle:
        header, stream = split(handle.read())
    ops = list(iter_ops(stream))
    chats = [op for op in ops if op.kind == "chat"]
    actions = sum(1 for op in ops if op.kind == "action")
    print(
        f"{path}: {len(header)} byte header, {len(ops)} ops over {ops[-1].time_ms / 1000:.1f} s "
        f"({actions} actions, {len(chats)} chats)"
    )
    for chat in chats[-3:]:
        print(f"  {chat.time_ms / 1000:9.1f} s  {chat.value}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Walk a recorded game's operation stream.")
    parser.add_argument("files", nargs="+", help="one or more .aoe2record files")
    args = parser.parse_args(argv)
    failed = 0
    for path in args.files:
        try:
            describe(path)
        except (OSError, BadStream) as error:
            print(f"{path}: {error}", file=sys.stderr)
            failed = 1
    return failed


if __name__ == "__main__":
    sys.exit(main())
