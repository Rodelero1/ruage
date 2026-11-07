# The binary names its own modules (D140)

*2026-11-07. Step 1 of four adopted from the prior-art watch thread. The finding was: `AoK HD.exe` carries its own source
paths and its RTTI class names, so the module map can be read from a file this project owns instead of inferred from
someone else's decompilation. **Verified, and the numbers are better than the digest's.***

## What the file says

`tools/exe_map.py`, one pass over the 7,521,280-byte exe:

| | count |
|---|---|
| source paths | **84** |
| RTTI classes (`.?AV`) | **541** |
| RTTI structs (`.?AU`) | 4 |
| `Comm*` netplay classes | **31** |
| strings ≥ 6 printable bytes | 16,666 |

The digest said 78 paths and 938 classes; the difference is the string minimum (it counted with a different one) and the
fact that `. ?AV` appears more than once per class in a PE — the tool deduplicates, which is why 1,541 descriptors resolve
to 541 classes.

**The build root is in the file**, which is the provenance argument in one line:

```
c:\users\jakes_workstation\documents\projects\age_of_empires_2_de\phoenix\age2hd_archive_5.7_with_hotfix
```

HD 5.7 was cut from the DE project tree — recorded in `SPEC.md` already, now evidenced by a path list rather than by one
string.

**The module map matches what the corpus gave us** (`docs/movement-in-the-binary.md`), from a different source:
`path.cpp` **and** `pathsys.cpp` separately, `motion.cpp`, `move_obj.cpp`, `game\ai\actions\act_gath.cpp` and the four
`tact_*.cpp`. That is U7's "six pathers" shape, named by the binary rather than inferred from a port.

## What is actually *new*: the class taxonomy

Source paths give modules; RTTI gives **classes**, and the netplay family is the one the registry has been missing
(`U12`):

`CommPacketSyncTurn`, `CommPacketSyncCmdCount`, `CommPacketPlayerCommand`, `CommPacketChecksum`,
`CommPacketSpeedChange`, `CommPacketResign`, `CommPacketPing/Pong`, `CommPacketDropPlayer`, the pre-game state packets,
`CommP2PChannel`, `CommHandler`, `CommState`.

That is a **turn-synchronised command stream**: per turn, a batch of player commands, a count, and a checksum. It also
gives the recording's sync cadence a shape to hang on, and the arithmetic is worth writing down even though it settles
nothing: 31,104 `SYNC` ops over 3,066.04 s is **10.15 a second**, one every ~98.6 ms — every **second** turn at 20 Hz.
Equally consistent with a 10 Hz sim, which the dat's 50 ms authoring granularity rules out. Recorded as a hypothesis in
`docs/oracle.md`, not as a rate.

## The tool

`tools/exe_map.py` — `read()` → `Map { paths, classes, structs }`, with `roots`, `families()` and `of_family()`;
`--json` for a machine-readable report, `--write` for `docs/binary-class-map.md` (generated, 217 lines, do not hand-edit).

Two rules it enforces on itself, both from this project's own history:

1. **A missing binary is an error, not an empty map** (`exit 2`, with the reason). An empty map and "the file is not
   there" must never look the same — the D130 lesson, applied to a tool.
2. **A family is a word, not a prefix.** The first version's `of_family("Comm")` counted 32 netplay classes and one of
   them was `CommandLineHandler`. The real number is 31, and a later reader trusts whichever number the docs carry.

## Test-first, honestly

The tool was written **before** its tests — a D7 violation, and this is where I say so rather than dress it up. What
stands in for the missing RED is a mutation sweep: five deliberate breaks, each run against the tests that claim to pin
it, **five killed, none survived**:

| mutation | the test that caught it |
|---|---|
| drop the `\source\` requirement | `test_only_a_build_path_under_source_counts` |
| stop folding case in the dedupe | `test_the_same_file_in_another_case_is_one_path` |
| count structs as classes | `test_a_classifier_separates_classes_from_structs` |
| return an empty map for a missing file | `test_a_missing_binary_is_an_error_not_an_empty_map` |
| lower the string minimum to 1 | `test_a_run_shorter_than_the_minimum_is_noise` |

Two more tests came **after** a real failure each, so their RED is genuine:

- the printed (non-JSON) report called `summary["families"](10)` on a list — `TypeError: 'list' object is not callable`;
- `--report` was in the docstring and not in the parser, found by running the documented command.

13 tests, 0.004 s.

## Performance

- **409 ms** for the whole exe (7.5 MB, 16,666 strings), measured twice; the first byte-at-a-time loop took **535 ms** and
  a regex over the raw bytes takes **123 ms** for the same set, which is why `strings_of` is now the regex. Output was
  compared byte for byte before and after the swap: identical.
- **Memory**: one `bytes` object for the file (7.5 MB) plus the string list (16,666 entries) — bounded, and the tool exits.
- **The generated doc**: **217 lines / 15,330 bytes** (`stat`), regenerable in the same 409 ms.
- The acceptance match is untouched — no engine code, `f328ebc8d4d34b78` and 726 tests as before.

## Stability

- **Truncated or binary-garbage input cannot crash it**: `strings_of` decodes ASCII only (the regex's own class), so no
  UTF-8 decode error is possible; a file with no strings yields an empty map, which the report prints as zeros.
- **Deterministic output**: paths, classes and structs are sorted; the doc is regenerable and was diffed.
- **No dependence on the corpus, the network or the dat**: the only input is the exe, so the map stops being reproducible
  exactly when the install is missing — which is the one case that should stop it.
- **`docs/binary-class-map.md` is generated and says so** in its first line, so a hand edit is visible in the diff of the
  next regeneration.
