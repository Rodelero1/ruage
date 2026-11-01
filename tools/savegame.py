"""The save-game container: inflate it, read its header, and walk its named string records.

Scope, stated plainly: this reads what the format has been *verified* to hold - the compression, the twelve-byte header
and the string records, whose framing is enforced by the game's own reader. It does not yet read the world: no clock, no
stockpile, no entity list. `docs/save-format.md` carries the register of what is verified, what is inferred and what is
still unknown, with the address of the code that settled each one.

Nothing here is derived data at rest: it runs against the user's own install at runtime and writes nothing (D8/D22).

    python3 tools/savegame.py Age2HD/SaveGame/brit1.aoe2spgame
"""

from __future__ import annotations

import argparse
import math
import struct
import sys
import zlib
from dataclasses import dataclass
from typing import Iterator

MAGIC_PREFIX = b"VER "
# The save line, not just `VER `: a `.dat` is raw DEFLATE too and its body starts `VER 5.7`, so the prefix alone made
# this reader call a database a save. 9.3 and 9.4 are the two strings the binary carries for this line.
SAVE_MAGICS = ("VER 9.3", "VER 9.4")
HEADER_LEN = 16

# A save of a busy 500-population match inflates to a few tens of megabytes; the largest file in this install is 20 MB.
# The cap exists because 200 MB from a 204 KB stream is a *measured* ratio of this format, so a truncated or hostile file
# could otherwise ask for all the memory the box has.
DEFAULT_MAX_BODY = 64 * 1024 * 1024

# The game's own gate on the value at offset 8, in `FUN_005e6730`: below zero or above 12.5 is
# `Invalid save game version %f`, and the fields that follow are read in three branches cut at 12.35, 12.37 and 12.38.
# So the range is not a guess - it is the original's, and a file outside it is one this reader should not pretend
# to understand.
VERSION_MIN = 0.0
VERSION_MAX = 12.5

# A string record's first four bytes are `tag<<16 | length` with tag 0x0A60: FUN_004340d0 checks
# `(word & 0xffff0000) == 0x0a600000` and takes the low half as the length before reading that many bytes.
STRING_TAG_BYTES = b"\x60\x0a"
MAX_STRING = 255


class NotASave(Exception):
    """The bytes are not a save game this reader understands. Never a partial read."""


def inflate(raw: bytes, max_bytes: int = DEFAULT_MAX_BODY) -> bytes:
    """The inflated body of a `.aoe2spgame`, which is raw DEFLATE from offset 0 with no container header.

    A stream that does not end exactly at EOF is refused: DEFLATE's own `eof` plus `unused_data` is the only check that
    distinguishes "this whole file decoded" from "the first few megabytes decoded and the rest was silently dropped".
    A body larger than `max_bytes` is refused before it is materialised, because the ratio is unbounded.
    """
    if not raw:
        raise NotASave("empty file")
    stream = zlib.decompressobj(-15)
    try:
        body = stream.decompress(raw, max_bytes + 1)
        body += stream.flush()
    except zlib.error as error:
        raise NotASave(f"not raw DEFLATE: {error}") from error
    if len(body) > max_bytes:
        raise NotASave(f"the body inflates past the {max_bytes} byte cap")
    if not stream.eof:
        raise NotASave("the DEFLATE stream ends before the file does")
    if stream.unused_data:
        raise NotASave(f"{len(stream.unused_data)} bytes after the end of the stream")
    return body


@dataclass(frozen=True)
class Header:
    """The first sixteen bytes of every save and recording.

    Twelve bytes are understood: `VER 9.4\\0` and a float save-game version. The fourth word is a second float - 1000.0,
    1005.0 or 1006.0 across this install's files - and **its meaning is not known**; it is exposed under a name that
    says so rather than given a plausible one (`docs/save-format.md`, "what is still unknown").
    """

    magic: str
    version: float
    unidentified: float

    @classmethod
    def parse(cls, body: bytes) -> "Header":
        if len(body) < HEADER_LEN:
            raise NotASave(f"only {len(body)} bytes: too short for a header")
        if body[:4] != MAGIC_PREFIX or body[7] != 0x00:
            raise NotASave(f"no version string at offset 0: {body[:8]!r}")
        magic = body[:7].decode("ascii", "replace")
        if magic not in SAVE_MAGICS:
            raise NotASave(f"{magic!r} is not a save-line version ({', '.join(SAVE_MAGICS)})")
        version, unidentified = struct.unpack_from("<ff", body, 8)
        if not math.isfinite(version) or not VERSION_MIN <= version <= VERSION_MAX:
            raise NotASave(f"save version {version!r} is outside the range the game itself accepts")
        return cls(magic=magic, version=version, unidentified=unidentified)


def iter_strings(body: bytes, start: int = 0) -> Iterator[tuple[int, str]]:
    """Every string record from `start` on, as `(offset, text)`.

    The walk is a byte scan for the tag rather than a structural parse, and that is deliberate: the sections around the
    records are not understood yet, so anything that claimed to know where "the next record" begins would be inventing
    the layout. A candidate is accepted only if its length lands inside the buffer and its bytes are text, which is what
    keeps binary that happens to contain the tag out of the result.
    """
    cursor = start
    while True:
        tag = body.find(STRING_TAG_BYTES, cursor)
        if tag < 0:
            return
        cursor = tag + 1
        record = tag - 2
        if record < 0:
            continue
        length = int.from_bytes(body[record:tag], "little")
        if not 0 < length <= MAX_STRING:
            continue
        end = tag + 2 + length
        if end > len(body):
            continue
        raw = body[tag + 2 : end]
        if raw.endswith(b"\x00"):
            raw = raw[:-1]
        if not raw or not all(0x20 <= byte < 0x7F for byte in raw):
            continue
        yield record, raw.decode("ascii")


# A save's clock is a match, not a day: the field the game writes is whole seconds times 1000, so a value past
# 24 hours at a fixed offset is some other counter that happens to increase.
CLOCK_MAX_MS = 24 * 60 * 60 * 1000
CLOCK_MIN_MS = 1000


def find_game_clock(bodies: list[bytes], min_ms: int = CLOCK_MIN_MS, max_ms: int = CLOCK_MAX_MS):
    """Every offset whose `u32` advances across `bodies` in whole seconds, as `(offset, values)`.

    This is how the clock was found and how it is re-found for a new match: the offset is **not** portable between
    matches (a second match in the set has unrelated bytes there), so it is discovered from a series rather than
    written down as a constant. The rules are the format's own - the value is game time in whole seconds, so every
    value is a multiple of 1000 - plus the order the caller supplies, which has to be chronological.

    Cost is linear in the first body and cheap afterwards: the second body is only consulted where the first one
    already looks like a clock.
    """
    if len(bodies) < 2:
        return []
    limit = min(len(body) for body in bodies) - 4
    found = []
    for offset in range(0, limit + 1):
        value = struct.unpack_from("<I", bodies[0], offset)[0]
        if not min_ms <= value <= max_ms or value % 1000:
            continue
        values = [value]
        for body in bodies[1:]:
            value = struct.unpack_from("<I", body, offset)[0]
            if not min_ms <= value <= max_ms or value % 1000 or value <= values[-1]:
                break
            values.append(value)
        else:
            found.append((offset, values))
    return _without_byte_shifts(found)


def _without_byte_shifts(found):
    """Drop the byte-shifted aliases of a field: reading the same bytes one or two positions early sees `v << 8` or
    `v << 16`, which is also "increasing" and also a multiple of 1000 whenever `v` is a multiple of 125. They are the
    same field, and reporting both would let a caller count one clock as two.
    """
    def shifted(small, big):
        return len(small) == len(big) and all(b == s * 256 or b == s * 65536 for s, b in zip(small, big))

    return [
        (offset, values)
        for offset, values in found
        if not any(other_values != values and other_values[0] < values[0] and shifted(other_values, values)
                   for _, other_values in found)
    ]


def gaps(values: list[int]) -> list[int]:
    """The differences between consecutive values - the interval between two saves, once the clock is known."""
    return [later - earlier for earlier, later in zip(values, values[1:])]


def show_series(paths: list[str]) -> int:
    """Report the clock and the interval between saves, for one match's saves given in chronological order."""
    bodies = []
    for path in paths:
        with open(path, "rb") as handle:
            bodies.append(inflate(handle.read()))
    found = find_game_clock(bodies)
    if len(found) != 1:
        print(f"{len(paths)} files: {len(found)} offsets advance in whole seconds, expected exactly 1", file=sys.stderr)
        for offset, values in found:
            print(f"  {offset:#08x}: {values}", file=sys.stderr)
        return 1
    offset, values = found[0]
    print(f"{len(paths)} files, clock at {offset:#08x}")
    for index, (path, value) in enumerate(zip(paths, values)):
        step = "" if index == 0 else f"  +{(value - values[index - 1]) / 1000:8.1f} s"
        print(f"  {value / 1000:9.1f} s  {path}{step}")
    print(f"  span {values[-1] / 1000:.1f} s")
    return 0


def describe(path: str) -> None:
    with open(path, "rb") as handle:
        body = inflate(handle.read())
    header = Header.parse(body)
    print(
        f"{path}: {len(body)} bytes inflated, {header.magic}, save version {header.version:g}, "
        f"word at 12 = {header.unidentified:g} (unidentified)"
    )
    for offset, text in list(iter_strings(body))[:20]:
        print(f"  {offset:#08x}  {text}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Read the container and named records of a save game.")
    parser.add_argument("files", nargs="+", help="one or more .aoe2spgame files")
    parser.add_argument(
        "--series",
        action="store_true",
        help="the files are one match's saves in chronological order: report the clock and the interval between them",
    )
    args = parser.parse_args(argv)
    if args.series:
        try:
            return show_series(args.files)
        except (OSError, NotASave) as error:
            print(error, file=sys.stderr)
            return 1
    failed = 0
    for path in args.files:
        try:
            describe(path)
        except (OSError, NotASave) as error:
            print(f"{path}: {error}", file=sys.stderr)
            failed = 1
    return failed


if __name__ == "__main__":
    sys.exit(main())
