#!/usr/bin/env python3
"""**Read unit positions out of an AoE2 HD save, and measure a gathering ring.**

The save's order queue carries destinations, not standing places (`docs/save-format.md`), so a ring measurement needs the
objects themselves. This finds them: the world section holds records whose first eight bytes are the object's **x and y as
float32 in tiles** (the bush save chains them at a **229-byte stride**; other saves lay them out differently), and it prints every
position in a window plus, for a named square, how many bodies stand on a ring around it and at what distance.

Usage:

    python3 tools/savegame_positions.py <save> --window 62 76 68 82 [--centre 64.5 78.5]

`--window` is `minx miny maxx maxy` around the cluster of interest. The answer that matters is the radii column: men working one
square stand at 1.10-1.17 tiles from its centre, spaced 0.40 apart, which is D186's ring.

Measured 2026-11-12 from `bush.aoe2spgame`: ten foragers, radii 1.100-1.174 (mean **1.143**), closest pair **0.400** - the same
ring and the same spacing the tree save shows (`owner-test2`: 1.105-1.189), so bushes and trees share it.
"""

from __future__ import annotations

import argparse
import math
import struct
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import savegame  # noqa: E402  (sibling module, same directory)

STRIDE = 229


def collect(body: bytes, window: tuple[float, float, float, float],
            near: float) -> tuple[list[tuple[float, float]], str]:
    """Every object position inside `window`, duplicates collapsed, plus a note on the stride that was seen.

    **The stride is reported, not required.** A save can lay its records out with a different gap from one save to the next (the
    bush save chains at 229 bytes; the mine save's window shows gaps of 68, 30 and 192), and a measurement only needs the
    positions. Reading every float pair and collapsing copies nearer than `near` is both simpler and save-independent.
    """
    lo_x, lo_y, hi_x, hi_y = window
    offsets: list[int] = []
    points: list[tuple[float, float]] = []
    for offset in range(0, len(body) - 8):
        x, y = struct.unpack_from("<ff", body, offset)
        if (x, y) == (0.0, 0.0):
            continue
        if not (lo_x <= x <= hi_x and lo_y <= y <= hi_y):
            continue
        offsets.append(offset)
        if all(math.hypot(x - kept[0], y - kept[1]) > near for kept in points):
            points.append((x, y))
    gaps = Counter(b - a for a, b in zip(offsets, offsets[1:]))
    common = gaps.most_common(1)[0] if gaps else (0, 0)
    note = f"stride {common[0]} x{common[1]}" if common[0] else "no stride"
    return points, note


def ring_fit(points: list[tuple[float, float]], centre: tuple[float, float],
             low: float, high: float) -> tuple[int, float, float, float]:
    """How many points sit in `[low, high]` of `centre`, their mean radius, the closest pair, and the mean nearest gap."""
    ring = [p for p in points if low <= math.hypot(p[0] - centre[0], p[1] - centre[1]) <= high]
    if not ring:
        return 0, 0.0, 0.0, 0.0
    radii = [math.hypot(p[0] - centre[0], p[1] - centre[1]) for p in ring]
    gaps = [
        min(math.hypot(p[0] - q[0], p[1] - q[1]) for j, q in enumerate(ring) if j != i)
        for i, p in enumerate(ring)
    ]
    return len(ring), sum(radii) / len(radii), min(gaps), sum(gaps) / len(gaps)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("save", type=Path)
    parser.add_argument("--window", nargs=4, type=float, required=True, metavar=("MINX", "MINY", "MAXX", "MAXY"))
    parser.add_argument("--centre", nargs=2, type=float, metavar=("X", "Y"),
                        help="the square's centre in tiles; default is the middle of the window")
    parser.add_argument("--band", nargs=2, type=float, default=(1.05, 1.20), metavar=("LOW", "HIGH"),
                        help="the ring band to count in (default 1.05-1.20)")
    parser.add_argument("--near", type=float, default=0.20,
                        help="collapse points closer than this: one object stores its position more than once "
                             "(0.20 by default; real men stand 0.40 apart)")
    arguments = parser.parse_args()

    body = savegame.inflate(arguments.save.read_bytes())
    window = tuple(arguments.window)
    points, stride_note = collect(body, window, arguments.near)  # type: ignore[arg-type]
    if len(points) < 3:
        print("no object positions in that window", file=sys.stderr)
        return 1
    centre = (tuple(arguments.centre) if arguments.centre
              else ((window[0] + window[2]) / 2, (window[1] + window[3]) / 2))

    print(f"{len(points)} object positions ({stride_note})")
    radii = sorted(math.hypot(p[0] - centre[0], p[1] - centre[1]) for p in points)
    print(f"radii from ({centre[0]:.2f}, {centre[1]:.2f}): " + " ".join(f"{r:.3f}" for r in radii))
    count, mean, closest, nearest = ring_fit(points, centre, arguments.band[0], arguments.band[1])
    print(f"on the ring {arguments.band[0]}-{arguments.band[1]}: {count} at mean {mean:.3f}, "
          f"closest pair {closest:.3f}, mean nearest {nearest:.3f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
