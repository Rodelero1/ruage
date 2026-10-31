# The RE toolchain: REA on Linux, the Windows box as a target (D86)

Written 2026-10-31. This file records the answer to "run Hermes with REA on a Windows box to reverse engineer
AoE2 HD", the measurements that decided it, and what the first questions to the binary are. A session with
no conversation history can resume from here.

## What REA is, read from its own documentation

[morluto/rea](https://github.com/morluto/rea) (MIT, TypeScript, npm package `rea-agents`, installed here at
**6.8.0`) is **an MCP tool server**, not an agent: Hermes registers it, then calls its tools. It wraps
Hopper (macOS only), Ghidra, IDA or Kuna, analyses locally, and returns pseudocode, assembly, strings, calls
and references with the evidence and the limitations attached.

So "Hermes with REA on Windows" decomposes into two independent questions, and they do not have the same
answer: *where does the analyser run* and *where does the target run*.

## The measurements that decided the shape

`docs/windows-ghidra-p0.md` in the REA repository, and `rea doctor` on this box (2026-10-31):

| Fact | Value | Consequence |
|---|---|---|
| Windows support | **experimental read-only P0**: Windows 10+ x64, operator-installed Ghidra 12.x, 27 read-only Ghidra inventory/memory/function-analysis operations on native PE | no debugging, no writes, no parity with the Linux side |
| Windows 32-bit PE | admission on **repository main** only; npm releases through 5.0.0 admit x86-64 PE targets on Windows | the weakest cell for this target |
| Windows transport | "authenticated IPv4 loopback with a private native-owned bearer descriptor and Job Object process ownership" | deliberately **not reachable from another machine** |
| Hopper | macOS only | unavailable here, and unavailable on Windows |
| Ghidra line accepted by REA 6.8.0 | 12.1.x, verified 12.1.4 (12.2 is rejected as `config_drift`) | pin 12.1.4, not `latest` |
| JDK | 64-bit full JDK; 25.0.5 accepted, 21.0.13 was refused by the version check | pin JDK 25 |

And the target itself, read from its PE header:

| Field | Value |
|---|---|
| path | `Age2HD/AoK HD.exe` (the install is read-only; nothing here writes to it) |
| size | 7,521,280 bytes |
| DOS/PE | `MZ`, `e_lfanew` 0x140, `PE\0\0` |
| machine | `0x14c` — **i386 (32-bit x86)** |
| PE timestamp | 1534962379 = **2018-08-22T18:26:19Z** |

**A PE is a file.** Ghidra or IDA on Linux decompiles this binary exactly as well as they do on Windows, and
the Linux side of REA is the non-experimental side with the full operation set. The Windows side would add a
read-only, loopback-bound, x64-first analyser to read a 32-bit binary that Linux already reads better.

## D86 — REA runs on this container; the Windows box is a target, not an analyst

1. **REA is installed here**, on the same machine as the engine, the dat, the importer and the commits. Static
   findings land in the agent that writes the Rust, with no coordination layer, no second memory, no second
   SPEC.
2. **The Windows box is the oracle farm**, which is what `docs/oracle.md` (D2) already says it is for: it
   runs the game, presses Save, and drops `.aoe2spgame` files where this box can read them. Its unique value
   is *execution and observation*, not analysis.
3. **If a Windows agent is ever added** — Hermes installs natively on Windows, and A2A is the supported
   Hermes-to-Hermes transport with per-peer bearer tokens, an agent card and push notifications — its job is
   to drive the game and the debugger and to ship artifacts with provenance. It never asserts a mechanic the
   Linux side cannot re-derive from an artifact. One brain, one SPEC.
4. **Findings enter the repository as data plus provenance** — measured numbers, file offsets, function
   addresses, the fixture that produced them — never as prose conclusions from a remote node.

Rejected, with the price stated: putting the analyser on Windows costs a second Hermes config, a second set
of credentials, an experimental read-only boundary, and a 32-bit-PE admission path that lives on `main` — to
buy static output this box can produce today.

## What is installed, and how it is invoked

| Component | Path | Version |
|---|---|---|
| REA | `/opt/data/tools/npm/bin/rea` | 6.8.0 |
| Ghidra | `/opt/data/tools/ghidra_12.1.4_PUBLIC` | 12.1.4 (2026-09-21) |
| JDK | `/opt/data/tools/jdk-25.0.5+8` | Temurin 25.0.5+8 |
| Node | `/opt/data/tools/node-v26.13.0-linux-x64` | 26.13.0 (REA refuses the system 26.7.0) |
| Project cache | `/opt/data/tools/rea/ghidra-project` | `REA_GHIDRA_PROJECT_PATH` |

Environment for every invocation: `/opt/data/tools/rea/env.sh`. Two install traps, both paid for once:

- **`python -m zipfile` drops the Unix mode bits**, so Ghidra's `support/analyzeHeadless` and
  `Ghidra/Features/Decompiler/os/linux_x86_64/decompile` land 0644 and `rea doctor` reports them missing. Use
  `/opt/data/tools/bin/unzip-mode.py`, which restores modes from `ZipInfo.external_attr >> 16`. There is no
  `unzip` on this box.
- **`npm -g` fails on the read-only toolchain prefix**; REA is installed with
  `npm install -g --prefix /opt/data/tools/npm`. Frida's install script is deliberately blocked there, which
  only disables dynamic instrumentation.

## The 2018 timestamp, and the open question it touches

`oracle.md` asks whether the community `mgz-fast` parser, which supports HD 5.8, applies to this install
("whether 5.7 differs is **unknown**"). The executable's PE timestamp is 2018-08-22, which fits the 5.8 line
and not a 2013-era 5.7 build. **This is a lead, not a finding**: the dat header still says VER 5.7, and the
authority is the exe's version resource plus a real `.aoe2record` from this install. Do not treat the
timestamp as resolving it.

## The first questions for the binary

These are the rows `oracle.md` marks as *not* answerable from save files, so they are exactly what the
disassembly is for. Each one must come back as an address plus a number that a fixture can test.

1. **Damage:** where armour is subtracted, where bonus classes are multiplied, in what order, and what the
   minimum-1 clamp is applied to. The engine already applies effect additions before multiplications
   (v0.56); the binary decides whether that is the game's order or ours.
2. **The per-tick unit update:** the order of speed integration, attack reload, and the gather tick within one
   tick, which is the U6/U7 gap that snapshots cannot see.
3. **Age-up timing:** what a kind-102 gating effect does at the tick boundary — the U10 "aftermath, not the
   transition" problem.
