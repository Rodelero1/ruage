# SPEC — AoE2 HD → AI-assisted recreation

**Read this file first, every session.** It is the entry point: current state, verified
ground truth, corrections to older notes, and the agreed procedure. Everything else in
this tree is either source data or supporting documentation.

| | |
|---|---|
| Spec version | 0.14 |
| Last updated | 2026-10-23 |
| Decision status | **D2 answered (file-based oracle); D1 recommendation made — see §6** |
| Historical handoff | [`docs/age2hd.md`](docs/age2hd.md) — read it *through* the errata in §4 |
| Ground truth | `Age2HD/` (the game install) — read-only, treat as immutable |

---

## 1. What this project is

Rebuild Age of Empires II HD as an AI-assisted reimplementation that loads the **user's
own legally purchased copy** of the game files. Assets are never copied, converted for
redistribution, or committed. The engine reads the install in place.

Two goals were conflated in the previous session's notes. They are not the same project
and they cost wildly different amounts:

- **G1 — a deterministic engine of our own.** Fixed-point sim, seeded RNG, stable
  iteration order, headless. Needs no reverse engineering at all.
- **G2 — behavioral identity with HD 5.7.** Requires an *oracle*: something that says
  "HD produces X here." There is no such oracle in this container (§5).

G1 is achievable and verifiable. G2 is only meaningful if an oracle exists. §6 asks which
one v0 is; nothing downstream is safe until that is answered.

---

## 2. Layout (authoritative)

```
workspace/Age2HD/
├── SPEC.md                  ← you are here (agent-writable)
├── docs/                    ← documentation we write
│   ├── age2hd.md            historical handoff, 2026-10-23 (has known errors, §4)
│   ├── hermes.container     podman quadlet describing the host container
│   ├── mechanics/           one file per subsystem, §7
│   ├── config.md            config layer (D6)
│   ├── decisions.md         D1–D7
│   └── sources.md           every outside source, with trust level
├── config/                  ← the engine's only input (D6)
│   ├── gamedata/*.toml      GENERATED from the dat — never hand-edited
│   ├── MANIFEST.toml        GENERATED provenance + per-file sha256
│   ├── overrides.toml       hand-edited, survives regeneration
│   └── sim|combat|economy|movement.toml   hand-written behaviour constants
├── Cargo.toml               Rust workspace (D9)
├── crates/
│   ├── fixed/               ruage-fixed — exact decimal → i64 fixed point, checked ops
│   └── data/                ruage-data — strict config loading, -1.0 sentinel
├── tools/                   ← shipped tooling, TEST-FIRST (D7, §11)
│   ├── genconfig/           dat → config/gamedata generator
│   └── tests/               pytest suite
├── build/                   derived artifacts: inflated dats, converted JSON, probe
│                            scripts (exempt from D7), Ghidra projects. Never committed.
└── Age2HD/                  the Steam install — READ ONLY, do not modify
    ├── AoK HD.exe           main binary
    ├── resources/_common/dat/       the two .dat files (the crown jewels)
    ├── resources/_common/drs/       already-extracted loose sprites/sounds
    ├── resources/_common/slp/       loose sprites
    ├── resources/_common/ai/        .per / .per2 AI scripts
    ├── resources/_common/random-map-scripts/   .rms
    └── resources/en, _packages, Docs, SaveGame, …
```

Rules: write only to `SPEC.md`, `docs/`, `config/`, `tools/`, `build/`. Never write into
`Age2HD/`. Anything generated goes in `build/` or `config/gamedata/` and must be
reproducible from a recipe in this file.

---

## 3. Verified ground truth

Everything in this section was checked against the files in this container on 2026-10-23.
**Do not re-derive it.** Re-run §9 to confirm nothing has drifted.

### 3.1 Hashes (provenance anchors)

| File | SHA-256 | Size |
|---|---|---|
| `Age2HD/AoK HD.exe` | `cbd10d81b93601ffb26773d250b7478969da92d70792a40ae423294202e14650` | 7,521,280 |
| `resources/_common/dat/empires2_x1_p1.dat` | `28b50786d62cc956679bf56b42b272ec04eb93642268e997484dbc63bb24e6cb` | 926,991 |
| `resources/_common/dat/empires2_x2_p1.dat` | `21591ac67251d8674635d5634f2d8e9ff80ad90f3c792f9503d60dd658d61058` | 2,260,159 |

### 3.2 The `.dat` files are raw-DEFLATE compressed

This is the single most important operational fact in this document, and it is absent
from `docs/age2hd.md`.

Raw bytes have entropy 7.99 bits/byte and begin `ec bd 05 78` — no `VER` header. They are
**raw DEFLATE with no zlib/gzip framing**. Inflate at offset 0 with `wbits=-15`:

```python
import zlib
raw = open(path, "rb").read()
o = zlib.decompressobj(-15)
out = o.decompress(raw) + o.flush()   # o.eof is True, o.unused_data is empty
```

| File | compressed | inflated | header |
|---|---|---|---|
| `empires2_x1_p1.dat` | 926,991 | 5,634,431 | `VER 5.7` |
| `empires2_x2_p1.dat` | 2,260,159 | 13,723,696 | `VER 5.7` |

Both inflate cleanly with zero trailing bytes and ratio 0.165. Every other asset in the
install is plaintext (SLP files start `2.0N ArtDesk 1.00 SLP Writer`), so this is an
HD-specific quirk — corroborated by the exe, which contains an all-zero section literally
named `Inf32Dat`.

Consequence: **any dat parser must be fed the inflated stream.** Handing a parser the
file on disk produces garbage, not an error message.

### 3.3 Binary facts

| Property | Value |
|---|---|
| Format | PE32, x86 (`machine 0x14c`), image base `0x400000`, `SizeOfImage 0xa44000` |
| Sections | 6, file ends exactly at last section (no overlay → **not packed**) |
| `.text` | 5,943,296 bytes, entropy 6.43 (normal compiler output) |
| `.rdata` | 1,042,944 · `.data` 144,896 raw / 3,375,172 virtual |
| `Inf32Dat` | 44,544 bytes, all zero in file — runtime inflation buffer |
| Build timestamp | 1,534,962,379 → **2018-08-22 18:26 UTC** |
| Version strings | `VER 5.7`, `VER 9.3`, `VER 9.4` (one binary, three format versions) |
| Function prologues | ≥13,543 `55 8B EC` (lower bound; tail-calls and `fastcall` are missed) |

**Build-path leak** (`.rdata`) — the most useful single string in the binary:

```
C:\Users\Jakes_Workstation\Documents\Projects\Age_Of_Empires_2_DE\phoenix\
    Age2HD_Archive_5.7_with_Hotfix\AoC\Source\Builds\Final\AoK HD.pdb
```

HD 5.7 was built from the AoE2 **DE** `phoenix` tree. Consequences: `grst/freeage` (the
DE reimplementation) is primary prior art, not a footnote; DE modding/reversing knowledge
transfers; and one binary dispatches between three dat format versions (§3.4).

### 3.4 Static-analysis entry points

Dat filename strings and their VAs — cross-reference these instead of analysing the whole
binary:

| String | Section | VA |
|---|---|---|
| `empires2_x1_p1.dat` | `.rdata` | `0x5c9a64` |
| `empires2_x2_p1.dat` | `.rdata` | `0x5c9afc` |
| `empires2_x1_p1.dat` | `.data` | `0x6af538` |
| `empires2_x1_p2.dat` | `.data` | `0x6af5bc` |

### 3.5 Install inventory (fresh counts, `find -iname` from `Age2HD/Age2HD/`)

| Kind | Count | Note |
|---|---|---|
| `.slp` sprites | 3,623 | plaintext, `2.0N` header |
| `.rms` random-map scripts | 122 | their own scripting language |
| `.per` / `.per2` AI scripts | 74 / 30 | flat fact/action rules, not a general language |
| `.scx` scenarios | 49 | |
| `.wav` / `.mp3` | 2,246 / 1,910 | |
| `.drs` containers | **0** | already extracted to loose files |
| Civilizations in x2 dat | **31** | all 31 HD civs confirmed by name |

### 3.6 Dat extraction — complete

Both dats parse end to end, units included, into `build/converted/*.json`. Two independent
implementations agree on every overlapping count, and units-per-civ equals task lists
exactly (866/866, 1,402/1,402) — an internal consistency check that a field-size error
would break.

| | x1 | x2 |
|---|---|---|
| civs (incl. Gaia) | 19 | 32 |
| **units per civ** | 866 slots / **672 populated** | **1,402 slots / 1,166 populated** |
| techs | 460 | 801 |
| effects | 514 | 798 |
| sprites | 7,367 | 11,175 |
| task lists | 866 slots (271 populated) | 1,402 slots (412 populated) |
| terrains | 42 | 100 |
| JSON | 54.3 MB | 137.3 MB |

**Data-model finding: units are stored per civilisation**, not in one global table
(866 × 19, 1,402 × 32). The engine should load one civ as canonical and apply per-civ
overrides. Civ 0 is Gaia; names are internal (`British`, not Britons) and display names
live in the language tables, not the dat.

Method, caveats, spot checks and licences: `docs/parse-notes.md`. Raw parse output:
`build/parse-2026-10-23.txt`.

---

## 4. Errata for `docs/age2hd.md`

The historical handoff is useful for narrative and has correct hashes, but eight claims
are wrong or stale. Read it, then apply these:

| # | Claim in `docs/age2hd.md` | Reality |
|---|---|---|
| 1 | Phase 1 = "parse `empires2_x1_p1.dat` → JSON" | Files are raw-DEFLATE; inflate first (§3.2) |
| 2 | "DRS archives → loose files" is a task | 0 `.drs` files exist; already loose (§3.5) |
| 3 | "27 civs w/ expansions" | 31 (§3.5) |
| 4 | Link timestamp 2018-08-27 | 2018-08-22 18:26 UTC (§3.3) |
| 5 | "1,564 mp3" | 1,910 (§3.5) |
| 6 | REA "~278k stars" | 369.9k stars / 92.6k forks as of 2026-10-23 — and see §5.3 |
| 7 | "REA compresses the RE bottleneck to weeks" | Unverified assumption, not a finding (§5.3) |
| 8 | "Phase 1 reference: openage `convert/`, reimplement in Python" | Wrong reference — use §5.2 |

Also: the doc's environment section is **correct** and should be trusted over what `free`
and `nproc` report (§8). Its ownership note (`nobody:nogroup`) is real and still applies
to `docs/`.

---

## 5. What the plan was getting wrong

### 5.1 No oracle, so "bit-exact" is unfalsifiable

`docs/age2hd.md` targets "deterministic lockstep sim (bit-exact multiplayer/replays)" but
never names an oracle. The exe can be read here but never run: no Wine, no Windows, no
Steam. HD's own replays are not an oracle either — they desync across patch versions and
there is no stepping debugger.

Without an oracle, Phase 2 ("write a behavior spec with evidence citations") cannot be
wrong, which means it cannot be trusted. Every claim would be pseudocode plus community
folklore.

### 5.2 Phase 1 is already written, and in Rust

The doc points at openage's `convert/` and proposes reimplementing it in Python. The
actual state of the art:

| Tool | Language | Use |
|---|---|---|
| `SiegeEngineers/genie-rs` | Rust | dat / drs / slp read+write — same language as our engine |
| `Tapsa/genieutils` | C++ | canonical reference implementation |
| `hszemi/genieutils-py`, `Alians713/GenieDatParser`, `HSZemi/aoe2dat` | Python/C#/Go | dat parsing/editing |
| `KSneijders/AoE2ScenarioParser` | Python | `.scx` scenarios |
| `rehan-remade/universal-modder` | Python | documents `.sld` + dat structure |
| `SiegeEngineers/aoc-dev-resources` | — | index of all of the above |

Rebuilding this is weeks of duplicate work producing a worse artifact.

### 5.3 REA is a tool, not an accelerator

REA is an MCP front-end that lets an agent drive Ghidra/Hopper/IDA. It inherits their
decompilation quality; it does not improve it. Its star count (369.9k, up from 278k in
the old notes) is not evidence that the approach works, and the repo is six months old
(initial commit 2026-04-14, then `betterBinaryMCP`). Budget for it as "a convenient way
to query Ghidra from a session", not as a phase-shrinking breakthrough.

Hard constraint that follows: `.text` is 5.94 MB with ≥13.5k functions on 2 cores and
4 GiB. Full auto-analysis is hours; decompiling the binary is not something any session
does. Method must be **question-driven and xref-anchored** (§3.4).

### 5.4 No vertical slice

The old plan was data → unbounded spec → engine. Nothing runs until the end. That is
openage's shape, and the doc cites openage as its own cautionary tale.

---

## 6. Open decisions (blocking)

| # | Question | Why it blocks |
|---|---|---|
| **D1** | ~~G1 or G2?~~ **Answered 2026-10-23 — G1.** "Generally similar behaviour, sensibly attainable." G2 was priced and rejected: no per-tick oracle exists here, the original's sim is x87 float, and a bit-exact copy would be a derivative work rather than an engine. G1+ (a per-subsystem exactness budget) was considered and **not** adopted | Sets every acceptance criterion, and whether the RE phase is bounded |
| **D2** | ~~Is there a machine that can run HD, usable as an oracle?~~ **Answered 2026-10-23 — a driven game is not needed.** The install already holds 13 `.aoe2spgame` saves; they are raw DEFLATE (`VER 9.4`) and inflate to 4.5–11 MB of world state. Snapshots answer U1/U2/U4/U5/U6 and partly U8/U10; they cannot answer U7 (pathing). Details: `docs/oracle.md` |
| **D3** | ~~Content scope~~ **Answered 2026-10-23 — one civ (British) first, others later.** British is dat civ index 1 (0 is Gaia), not a preference. A second civ costs a type-102 disable-list join, not a per-civ unit delta layer (U13: 5 differing unit rows across all 31 civs) | Sets the data model's generality requirement |
| **D4** | ~~Private / publishable engine / publishable both~~ **Answered 2026-10-23 — B: publish engine + tooling, keep spec and mechanics internal.** Condition: a stranger with their own install must be able to clone, build and run it. Checklist and blocking gap in §6.2 | Whether the RE-derived spec is a deliverable, and how strict clean-room hygiene must be |

**D1 recommendation, on the evidence in `docs/oracle.md`: G1.** A snapshot oracle can hold a
reimplementation to "the numbers agree"; it cannot hold one to "the trajectory is identical"
without per-tick instrumentation we do not have. G1 with snapshot validation is measurable;
G2 is not.

**D5 (2026-10-23): the dat outranks community documentation for every numeric value.**
Applied first to the farming rate (U11): effective 0.400, the farm building's own rate capping
the villager's 0.530. Full reasoning and the cheap re-measurement: `docs/decisions.md`.

Unanswered decisions are recorded here, not silently assumed. If a session needs to move
before D1/D3/D4 are answered, it may do only work that is identical under every branch:
steps 1, 2 and the save-game reader.

### 6.1 G1 acceptance thresholds (step 3 exit criterion)

Written down so "close enough" is a number someone can check, per D1. Two classes:

**Exact — zero tolerance.** These are data or our own bookkeeping, not behaviour:

| Measure | Threshold |
|---|---|
| Unit stats, costs, train times, tech effects | equality with `config/gamedata/` (which is checked against the dat) |
| Damage per hit | the published formula, exactly (`docs/mechanics/combat.md`) |
| Per-tick displacement | `speed / tick_rate` exact, enforced by `Fixed::div_exact` |
| Replay determinism | identical state hash for the same command log, across runs and across insertion orders |
| Scenario start | identical map for the same seed and `.rms` |

**Tolerance-banded — behaviour only observable through play.** These are the numbers to hit in
step 5; they are deliberately loose, because G1 does not claim the original's trajectory:

| Measure | Threshold | Why this number |
|---|---|---|
| Villager gather rate over 60 s | within 5% of the dat rate | travel and turn-around vary; the rate itself is dat-exact |
| Building completion time | within 1 tick = **50 ms at 20 Hz** of `creation_info` | build time is data; the tick phase is not |
| Time to kill, equal-supply mirror fights | within 10% of the hand-computed value from dat stats + damage formula | attack/reload phase offsets are unpublished |
| Unit arrival time over ground | within 2 ticks = **100 ms at 20 Hz** | path smoothing and waypoint snapping are unpublished |
| Population / resource accounting | exact at every tick | bookkeeping, not behaviour |

**Why the ms column exists.** U12 is now *supported but unproven*: 20 Hz is corroborated by the dat's 50 ms reload grain and by a 10-syncs/second cadence in three real HD recordings against 59 in a DE one (see `docs/oracle.md`). If the original turns out to
run at 60 Hz, every "1 tick" band above silently becomes 16.7 ms and the tolerances tighten by 3×
without anyone deciding to tighten them. Bands are therefore stated in both units, and a session
that changes `tick_rate_hz` must restate the ms column rather than inherit it.

A scenario that misses a tolerance is a **bug with a recorded reproduction**, not a licence to
widen the band. Widening one requires a row here with the evidence.

### 6.2 Publishability checklist (D4 = B)

The owner's condition is concrete: **clone → build → run, with only the user's own game install.**
Measured against the tree on 2026-10-23:

| Requirement | State |
|---|---|
| `cargo build` from a clean clone | **works** — building needs no game data at all |
| Run a world without game data | **works** — `ruage-headless --config config/sim.toml --seed 7 --ticks 100` prints tick, state hash and exact position; the config it needs is tracked |
| Determinism visible from outside | **works** — the same invocation twice is byte-identical (tested as a subprocess, not just as a library call) |
| Produce `config/gamedata/` from the user's own install | **Open — a packaging question, not a licence one.** The engine never imports the parser: `crates/` has zero references to it (verified), and at runtime the engine reads TOML only. The dependency edge is `tools/genconfig → GenieDatParser` (Divy1211's library, GPL-3.0, a build-time dat miner — *not* openage's converter). So the GPL reaches the **generator**, never the game, and the engine's MIT/Apache status is unaffected. What is missing is simply that a public clone has no generator to run: ship `tools/` as a separately-licensed GPL-3.0 component, keep it private, or write our own reader. |
| Licence split | **decided (D12)**: the generator and any parser fork move to their own repository, GPL-3.0; the engine repo stays MIT/Apache and pure Rust, so this repo never contains an import of GPL code. Not urgent — a publish-time step |
| No engine-code references to internal docs | todo — code comments cite `docs/mechanics/`, which will not ship |
| Content check in CI | todo — the pre-commit hook is path-based and contributors will not have it installed |
| README / quickstart | todo |

**The blocking item is the dat reader.** Option B is therefore not "publish today" but "publish once
the reader exists", unless the owner prefers the GPL-dependency shortcut.

---

## 7. Procedure

From step 4 onward every step is executed **test-first** — see §11. No production code exists
before a test that was watched failing.

| # | Step | Produces | Exit criterion |
|---|---|---|---|
| 0 | Evidence base | This file | Hashes, counts, recipes recorded |
| 1 | Data layer: inflate + parse | **done** — `build/converted/*.json` for both dats, units included (§3.6) | 19 (x1) / 32 (x2) civs; units 672 of 866 slots (x1) and 1,166 of 1,402 (x2); techs/effects/sprites/tasks all parse; two implementations agree |
| 2 | Knowledge harvest | **done** — `docs/mechanics/` (6 files + index), every fact tagged `known` / `inferred` / `unknown` | 12 unknowns registered (U1–U12): 5 resolved, U11 open, U12 now *supported by two independent measurements* (50 ms dat grain; 10 vs 59 syncs/s in HD vs DE recordings) but not proven |
| 2b | Oracle harness: `VER 9.4` save reader | **partial, stopped** — `build/tools/save.py` inflates all 13 saves and reads player names where framing is clean (5/13) | Not met and deprioritised: no public spec, no second implementation to validate against, resources/units/tick not located. Rationale in `docs/oracle.md` |
| 3 | Decision gate | **done** — D1 = G1 (2026-10-23), D2 = file-based oracle, thresholds written into §6.1 | "Done" is a number someone can check |
| 4 | Rust workspace skeleton | **done** — `crates/fixed` (11), `crates/data` (15 + corpus + 13 economy + 6 map + 6 units), `crates/sim` (10 library + 7 seeding + 7 subprocess). All four loaders strict; headless runner seeds a real match | `cargo test` = **82 green**, clippy `-D warnings` clean, fmt clean. A seeded 2000-tick run prints `tick=2000 entities=8 players=2 hash=d91a1a7e79ea6b08` |
| 5 | Vertical slice | **in progress** — a villager earns wood end to end and **walks around terrain**: A* over the tile grid routes it to the node, home to the nearest drop site its variant accepts, and back again; a walled-off node is refused at order time. **Next:** build (house → barracks) spending the stockpile, then combat and the scripted opponent |
| 6 | Gap closure | Backlog in §7.1, ordered by `unknown × gameplay impact`; reverse-engineer **one** subsystem | Pathfinding **or** combat resolution lands with evidence — not both half-done |
| 7 | RE tooling, only now | JDK 21 + Ghidra in user space, `npx rea-agents@6.7.0`, headless `-Xmx2g` | One step-6 question answered by walking xrefs from §3.4 |
| 8 | Persist | Updated SPEC.md, `docs/`, commit-worthy `build/` recipes | A new session resumes from files alone |

### 7.1 Step-6 backlog, from the game's own subsystem split

Taken from `librematch/genie`'s `genie-core` module list (a port of DE build 158041, so the split
is the engine's, not a guess). Their names are kept so a reader can map back. Their list continues
past the names visible here.

| Subsystem | What it is here | State / blocker |
|---|---|---|
| `gather`, `gather_search` | villager task targeting, resource search radius, drop-off routing | data now available (task lists, drop sites, D11); sim work outstanding — **blocks step 5** |
| `build` | construction progress | blocked on `hp_per_second_per_villager`, still `-1.0` (economy.toml) |
| `farm_queue` | farm reseeding queue | food economy; after build |
| `attack`, `damage`, `best_target` | hit resolution and target selection | formula known (`docs/mechanics/combat.md`); **ordering and simultaneity are not** |
| `effect` | tech-effects interpreter | types 4/5/3 decoded; 0/2/6/101–103 open (U2) |
| `group` | formations and group movement | arrival times depend on it; tolerance band in §6.1 |
| `facing`, `fly` | orientation, projectiles | ties to SLP frame rates; no renderer yet |
| `garrison` | units inside buildings and rams | after combat |
| `artifact` | relics | after garrison |
| `abilities` | unit special abilities | DE-heavy; the 5.7 subset is unmeasured — candidate U15 |
| `explore` | scouting / fog | probably irrelevant headless |
| `diplomacy` | stances | trivial |
| `delete` | entity removal (deaths) | needed as soon as combat lands |
| `game_status` | win/lose | end of the slice |
| `checkpoint` | state snapshot | we already have `state_hash` |
| `ai_kinds` | AI behaviour kinds | our slice uses scripted commands, not AI |
| `fmath` | their f32 emulation | **deliberately not adopted** — fixed point replaces it (D9), which is only valid under G1 |

Steps 1–2 are hours and retire most of the old Phase 1 and a chunk of Phase 2: gather
rates, tech costs, unit stats and civ bonuses are either in the dat or in published
community tables. Reverse-engineering those spends a 2-core/4 GiB decompiler budget on
facts that are already written down.

Step 5 before step 6 is deliberate — a running bad game says which unknowns matter.

---

## 8. Environment

| | |
|---|---|
| Container | rootless podman quadlet, `docs/hermes.container`, image `nousresearch/hermes-agent:stable` |
| cgroup limits | **4 GiB RAM (`memory.max=4294967296`), 2.0 CPUs (`cpu.max=200000/100000`)** |
| User | `hermes` uid/gid 10000 — **no sudo, no root**, `apt` present but unusable |
| Toolchain | Node 26.7.0, npm 12.0.2, Python 3.14.7, `python3 -m pip` 26.2.1, git, objdump, gcc/g++/make/cmake |
| Installed by us (user-space) | **uv 0.13.5 + uvx** at `~/.local/bin` (was *not* in the image despite expectation); **Rust 1.99.1 + cargo** at `/opt/data/tools/{cargo,rustup}` |
| Missing | java, Ghidra, Boost, LZ4 headers, `file`, `7z`, `unzip`, `jq`, Wine, Steam |
| Disk | 865 GB free on `/opt/data` |
| Scratch | `/opt/data/cache/scratch` — pruned after 24 h idle, never keep anything there |

`uv`, `cargo` and `rustc` are **not on the default `PATH`** (`HOME` is `/opt/data/home`).
Every session that needs them must start with:

```bash
export PATH="$HOME/.local/bin:/opt/data/tools/cargo/bin:$PATH"
```

**Do not trust `free` or `nproc`.** They report the *host* (15 GiB / 6 cores) because the
cgroup limits are applied by podman, not visible to those tools. The 4 GiB / 2 CPU limits
are real and are why Ghidra runs at `-Xmx2g`.

**Permission gotcha — currently blocking `docs/`:** directories created on the host and
dropped through the bind mount arrive as `nobody:nogroup 755`, so the agent can read but
not create files in them. `docs/` is in this state. One-time fix from the host:

```bash
chown -R 10000:10000 ~/apps/hermes/workspace/Age2HD/docs   # or chmod -R g+w
```

`Age2HD/Age2HD/` is deliberately left read-only — that is a feature, not a bug: it stops
a session from mutating source data.

Nothing may be installed into this container as a side effect of analysis; user-space
portables go under `/opt/data/tools`.

---

## 9. Sanity check

Run this at session start. Any mismatch means the tree changed and §3 is stale.

```bash
cd /opt/data/workspace/Age2HD
sha256sum Age2HD/"AoK HD.exe" \
          Age2HD/resources/_common/dat/empires2_x1_p1.dat \
          Age2HD/resources/_common/dat/empires2_x2_p1.dat
python3 -c "import zlib;d=open('Age2HD/resources/_common/dat/empires2_x1_p1.dat','rb').read();o=zlib.decompressobj(-15);print(o.decompress(d)[:8],len(d))"
find Age2HD -iname '*.slp' | wc -l   # expect 3623
```

Expected: the three hashes in §3.1, `b'VER 5.7' 926991`, and `3623`.

---

## 10. Guardrails

- The user owns the game. Analysis and a personal-use engine are fine.
- Never copy, commit, publish, or convert-for-distribution any game asset — no `.dat`,
  `.slp`, `.drs`, sound, or `.exe`. Ship engine + converters + instructions that point at
  the user's own install.
- Derived data lives in `build/` and is regenerated by recipe, never checked in.
- The RE-derived spec (`docs/mechanics/`) stays strictly separate from engine code
  regardless of D4, so the clean-room boundary is cheap to enforce later if the answer
  changes.
- Batch heavy work; persist intermediates; 4 GiB and 2 cores are the real budget.

---

## 11. Test-first development (non-negotiable)

Every line of shipped code starts with a failing test. No exceptions without the project
owner saying so in writing.

### The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Code written before its test is deleted, not kept "as reference" and not "adapted". If it
exists without a test, it cannot be trusted, and unverified code is worse than missing code.

### Scope — D7

| Must be test-first | Exempt |
|---|---|
| Rust engine crates (`sim`, `data`, `config`) | Throwaway analysis probes in `build/tools/` (`save-explore.py`, `introspect.py`, one-off dat queries) |
| Shipped tooling that produces artefacts (`gen_config.py`) | Generated output (`config/gamedata/*.toml`) — verified by tests, not written by hand |
| Anything that touches the dat loader, the config loader, or the sim loop | Documentation and `docs/**` |

The exemption is for *exploration*, not for shipping: when a probe's logic graduates into the
engine, it is deleted and re-implemented test-first.

### The cycle

1. **RED** — write one minimal test for one behaviour. Name it after the behaviour, not the
   function (`pike_vs_knight_deals_bonus_damage`, not `test_damage_2`).
2. **Verify RED — mandatory.** Run it and watch it fail *for the right reason* (feature
   missing, not a typo or an import error). The failing run is recorded in the session:
   command, test name, failure message. No recorded failure, no GREEN step.
3. **GREEN** — the simplest code that passes. Hardcoding a return value is allowed here;
   scope creep is not. No extra features, no unrelated refactors.
4. **Verify GREEN — mandatory.** The specific test passes, then the whole suite passes with
   clean output.
5. **REFACTOR** — remove duplication, improve names, keep tests green the entire time. No new
   behaviour.
6. Repeat. One behaviour per cycle.

**Vertical slices only.** Never write a pile of tests and then a pile of implementation
(horizontal slicing). Each test is written knowing what the previous cycle just taught us
about the interface.

### Coverage — D7

Behaviour-based, with no percentage target. The gate is "every behaviour has a test that was
seen failing", not "90% of lines executed". A line-coverage number can be raised without
testing a single behaviour, so it is not the gate.

### Project-specific tests that must exist before the corresponding code

| Behaviour | Test |
|---|---|
| Config loading is strict | unknown key → startup error; missing required key → error; `-1.0` sentinel → refuses if used |
| Dat values reach the engine | knight loads 100 HP / 2/2 armour / 60F 75G from `config/gamedata/units.toml` |
| Override precedence | `overrides.toml` beats `gamedata/`; regeneration does not clobber it |
| Manifest staleness | dat hash differs from `MANIFEST.toml` → startup refuses with a stale-config error |
| Fixed-point exactness | all 56 distinct dat speeds produce integer displacement per tick at scale 20000 |
| Determinism | two runs, same seed, same commands → identical state hash at tick N |
| Damage formula | pike vs knight matches the published calculation, including the floor at 1 |
| Effects interpreter | Wheelbarrow's type-5 commands multiply speed by 1.1 and carry by 1.25 |

### Red flags — if any of these happen, stop and start the cycle over

- Code before test; test written after the implementation
- Test passed the first time it ran (it is testing existing behaviour — fix the test)
- Cannot explain why the test failed
- "I'll add the test later", "it's too simple to test", "I already checked it manually"
- Keeping generated-then-abandoned code "for reference"

### First application (2026-10-23)

The untested generator (`build/tools/gen_config.py`, written before D7 existed) was
**deleted** and rewritten from scratch as `tools/genconfig/` — 17 tests, each written first
and watched failing. Proof that the rewrite preserved behaviour: stripping comment lines,
all four generated files are **byte-identical in data** to what the old script produced
(units 22,158 / techs 5,609 / effects 3,194 / civs 194 data lines).

### Second application (2026-10-23) — adversarial review

The review attacked the claims rather than re-reading them, and found four defects the suite
could not see. All were fixed test-first; the failing run is quoted for each.

| Behaviour | Test | The RED run showed |
|---|---|---|
| The scale must give exact *per-tick* displacement, not merely representable decimals | `scale_must_give_exact_per_tick_displacement`, `tick_rate_and_scale_must_agree` | `assertion left == right failed: scale 10000 loses 0.737: expected Invalid, got Ok(10000)` and `60 Hz with scale 20000 must be refused` |
| Both decimal grains are required, bounded, and ordered | `decimal_grains_are_required_and_bounded` | `unknown field 'per_tick_decimals'` after the field was added to the fixture |
| Contradictory config is refused, not accepted | `schema_version_must_be_supported`, `determinism_invariants_are_rules_not_switches`, `zero_valued_sizes_are_refused` | `floats_in_sim_state = true` loaded as `Ok`, `schema = 99` loaded as `Ok` |
| A 40-digit literal is an error, not a panic (and not a silent wrap in release) | `overflowing_literal_is_an_error_not_a_panic` | `panicked at crates/fixed/src/lib.rs:114` |

The corpus sweep (`crates/data/tests/corpus.rs`) is the one test that would have caught all of
the above at once, and it earned its keep immediately: its first run **falsified the review's
own hypothesis** — `unit 110 work_rate = 0.0111 is 11.1 scale units per tick, not a whole
number`. The dat is not uniformly 3 dp. That produced the two-grain split (D10) rather than
one number that was quietly wrong for two buildings.

`cargo clippy -- -D warnings` was also red before this session (two `should_implement_trait`
errors on `add`/`sub`); §12 claimed "zero warnings", which was true of `cargo test` only. The
gate now passes with `--all-targets`.

### Standing process after every change (owner, 2026-10-23)

Two steps are now mandatory at the end of every change, before the next one starts:

1. **Compact.** Update `SPEC.md` (step status, changelog row, version), `docs/` and any index so
   that a new session can resume from the files alone — no reliance on this conversation.
2. **Adversarial review.** Attack the change as if it were someone else's: verify the claims
   against real output, look for the failure the tests do not cover, and report what is wrong
   even when it is our own number. Then fix what it finds, test-first, and record the finding.

Both exist because this project's only durable memory is its files, and because a claim that has
not been attacked is a claim that has not been checked.

### Reporting

Each cycle's report to the owner names the test, states that it was seen failing, and shows
the command that proved it. A session that cannot show a failing run has not done TDD, and
should say so rather than claim otherwise.

---

## 12. Document index

| Path | Status | Purpose |
|---|---|---|
| `SPEC.md` | current, v0.43 | Entry point — this file |
| `docs/age2hd.md` | historical, **superseded by §4** | 2026-10-23 handoff from the previous session |
| `docs/hermes.container` | reference | Host container definition |
| `docs/parse-notes.md` | current | Step 1 record: toolchain, reproduce commands, parse counts, cross-validation, per-civ unit model, caveats, licences |
| `tools/` | current | **D7** — test-first tooling: `genconfig/` (dat → config) + 17 pytest tests |
| `config/` | current | **D6** — gamedata TOML (generated) + behaviour configs + `overrides.toml`; see `docs/config.md` |
| `docs/config.md` | current | Config layer: precedence, strict loading, provenance tags, regeneration, staleness check, D4 caveat |
| `crates/fixed`, `crates/data` | current | Engine skeleton: fixed-point arithmetic + strict config loader, 26 tests |
| `crates/sim` | current | Deterministic tick loop: entity-id iteration, exact per-tick movement via `Fixed::div_exact`, PCG32 with separate sim/map streams, FNV-1a state hash, command queue honouring `command_apply`, 10 tests |
| `crates/data/tests/corpus.rs` | current | Whole-corpus sweep: every value in `config/gamedata/` is exactly representable, and every speed is a whole number of scale units per tick. Skips when the generated data is absent |
| `docs/licensing.md` | current | **D8** — repo hygiene: what is never committed, legal basis, enforcement |
| `.githooks/pre-commit` | current | Refuses commits containing game data or extracted values **at known paths**. It matches paths and extensions, not content: a CSV of unit stats under `config/` commits cleanly (verified 2026-10-23) |
| `docs/sources.md` | current | Every outside source used, what for, trust level, and the version caveat (wiki = DE, dat = 5.7) |
| `docs/oracle.md` | current | The oracle: 13 saves found, raw DEFLATE, `VER 9.4`; what snapshots can/can't answer; D1 recommendation |
| `docs/mechanics/*.md` | current | 6 subsystem files + `README.md` (tag legend, index, unknown registry U1–U12) |
| `docs/formats/*.md` | planned | dat / slp / dld / rms / per — what our parser assumes |
| `docs/decisions.md` | current | D1–D8 with rationale: D5 dat authoritative, D6 config layer, D7 mandatory TDD, D8 no game data in git |
| `build/` | throwaway | Inflated dats, converted JSON, Ghidra projects |

---

## 13. Changelog

| Date | Change |
|---|---|
| 2026-10-23 | v0.1 — created. Absorbs and corrects `docs/age2hd.md`; records the DEFLATE discovery, the 31-civ count, the phoenix/DE build-path leak, and the dat-loader xrefs |
| 2026-10-23 | v0.2 — step 1 partial: both dats parse to EOF via `genie-dat` (§3.6). Unit table is not covered by that crate; unit/building reader is the remaining step-1 work. `genie-rs` is GPL-3.0 (D4 impact) |
| 2026-10-23 | v0.3 — **step 1 complete.** `GenieDatParser` covers units: 866/1,402 units per civ, cross-validated against `genie-dat`. Data-model finding: units are per-civisation, not global. `build/converted/*.json` written. Environment: uv was **not** present and was installed; Rust toolchain added |
| 2026-10-23 | v0.4 — **step 2 complete.** `docs/mechanics/` written: units, combat, economy, techs/effects, world/rms, determinism, plus an index with 10 registered unknowns (U1–U10). Decoded by known-value cross-check: attack/armour class 3/4, effect types 4 (attribute), 5 (multiplier), 3 (unit upgrade). Reported Double-Bit Axe's +20% wood multiplier as *missing* from effect 193 — **this was wrong, see v0.7** (tech 200 is Bodkin Arrow; DBA is tech 202) |
| 2026-10-23 | v0.5 — **D2 answered, without needing a driven game.** 13 `.aoe2spgame` saves already in the install inflate (raw DEFLATE, `VER 9.4`, 4.5–11 MB) → snapshot oracle. Added step 2b (save reader), `docs/oracle.md`, and the recommendation that D1 = G1, since snapshots validate numbers but not trajectories |
| 2026-10-23 | v0.6 — step 2b attempted and **stopped deliberately.** All 13 saves inflate and the version string + name-record convention are proven (5/13 yield names), but the layout is not uniform and there is no public spec or second implementation to validate against. Save parsing is not on the G1 critical path; U1/U4/U6 should come from published community tables instead |
| 2026-10-23 | v0.7 — **numbers harvested.** U4 (damage formula) and U6 (reload semantics) resolved from published sources; **U1 resolved from the dat**: gather rates are `work_rate` per villager task variant, tech multipliers are `c=13` type-5 commands (DBA = tech 202, effect 195, ×1.2). Corrected the earlier "DBA multiplier missing" error (tech 200 is Bodkin Arrow). New unknowns: U11 farming-rate discrepancy, U12 tick rate. Added `docs/sources.md`. |
| 2026-10-23 | v0.8 — **D5 recorded: dat is authoritative for all numbers.** U11 farming resolved by decision as 0.400 (farm building rate caps the villager's 0.530; `work_value1=1.0` everywhere, so no hidden scaling). Rule added: the farm rate loads from `FARM.task_info.work_rate` as one named constant. New `docs/decisions.md` (D1–D5). |
| 2026-10-23 | v0.9 — U11 value fixed at **0.400** (farm building `work_rate`) under D5; the "farm work_rate = construction speed" reading was **tested and falsified** (`HOUS` has `work_rate=0.0`, build time is a separate `creation_info` field, farms build faster than houses). Mechanism still open: `FTRAP` carries 1.0, so containers are not uniform. |
| 2026-10-23 | v0.10 — **D6: nothing numeric is hardcoded.** `config/gamedata/*.toml` generated from the dat (1,166 units / 801 techs / 798 effects / 32 civs, 574 KB total) + hand-written `sim/combat/economy/movement.toml` + path-keyed `overrides.toml`. Strict loading. All files parse; 0 mismatches vs the JSON pipeline. Also: found that unit arrays have **1,402 slots but 1,166 populated ids**; and settled the fixed-point scale at **20,000** (all 56 dat speeds exact at 20 Hz). |
| 2026-10-23 | v0.11 — **D7: TDD is mandatory** (§11). Iron Law, recorded-RED requirement, scope = engine crates + shipped tooling (probes exempt), behaviour-based coverage with no % gate, plus the eight project-specific tests that must precede their code (config strictness, override precedence, manifest staleness, fixed-point exactness, determinism hash, damage formula, effects interpreter). |
| 2026-10-23 | v0.12 — `gen_config.py` **deleted and rewritten test-first** as `tools/genconfig/` (17 tests, each seen failing first: float formatting, sub-precision guard, string escaping, name decoding, unit/tech/effect/civ records, all four renderers, manifest, CLI, and a real-dat integration test). Output verified byte-identical in data to the deleted script. SPEC §2 layout now includes `config/` and `tools/`. |
| 2026-10-23 | v0.13 — **D8: no game data in version control** (EU compliance). `git init` + `.gitignore` + tested pre-commit hook; 35 files / 328 KB committed, zero game data staged. Added `docs/licensing.md` (legal basis: Software Directive Art. 6 interoperability, Database Directive Art. 7 substantial extraction) and the project skill `age2hd-engine-conventions` so future sessions get the invariants and dat traps without re-discovery. |
| 2026-10-25 | v0.43 — **production: pay, wait out the create time, get a unit.** `Command::Train` refuses in a fixed order — not a building, not finished, already producing, not allowed to make this unit, cannot afford it — then charges and starts a tick timer resolved to `create_time x tick_rate`, so a 25 s villager at 20 Hz appears exactly 500 ticks later. The building-to-unit edge is *not* in the data we parse, so `[train]` in `config/economy.toml` carries it as authored content (TC 109 → 83, barracks 12 → 74) and the loader refuses empty lists, zero ids and duplicate buildings. Two bugs found by review and fixed test-first: the production phase ran later in the same tick as the command that started it, taking a free tick (499 instead of 500) — now `Training::fresh`; and a second order silently overwrote the running job *and* charged for it, so a paid-for unit vanished — now `AlreadyTraining`, with the test deliberately banking 100 food so it cannot pass for the wrong reason. New `docs/mechanics/training.md`. **116 tests** (109 before this step; the difference is the three economy-table tests and four training tests). |
| 2026-10-25 | v0.42 — **build works: pay, place, work, finish.** `Command::Build` checks five things before any money moves — the unit does not move and has a `create_time`, the player can pay every cost entry, the tile is open ground, nothing stands there, and a route exists — then deducts the price, inserts a one-tile site and sends the builder. Progress is counted in **builder-ticks** and HP is derived from it (`max_hp x progress / total`), so a mill's 17.14 HP/s cannot drift and a house lands exactly 500 ticks after the builder arrives at 20 Hz. Review found one real hole, fixed test-first: the route was planned *after* payment, which left a paid-for site nobody could reach; a forest-walled fixture map now proves the refusal (it fails with `expected NoPath, got Ok(())` without the check). Also added a guard that the two resource-id mappings cannot be folded into one — stockpile slot 0 is wood, dat id 0 is food. New `docs/mechanics/build.md` records the rule and its limits (a building does not block movement, one tile per building, population cost ignored, no repair or refund). **109 tests.** |
| 2026-10-25 | v0.41 — **U15 was a false alarm: build time was never missing.** I searched for a field *named* `build_time`, found none in either parser, and declared it absent — but the dat calls it **`create_time`**, which our generator has been emitting as `train_time` since step 1. For a unit that is training time; for a building it is build time, and the values prove it: house 25 s, barracks 50 s, farm 15 s, mill 35 s, castle 200 s, town centre 100 s, all matching published figures. The raw-dat probe that preceded this (three approaches, none converging) is discarded; no new dependency is needed, so `genie-dat` stays a future option rather than a present one. Pinned by test in `crates/data/tests/units.rs` and documented on the field. The general lesson, recorded with U12's: a field's *name* in one tool is not evidence about the data — and "the parser doesn't model it" is not "the dat doesn't have it". |
| 2026-10-25 | v0.40 — **build time is missing from the data, and that is a fork in the road.** Checked the parser (`creation_info` has 34 attributes, none a build time), the generated `units.toml`, and the older `build/converted/*.json` dumps: none of them carry it. The dat must contain it, but nothing in our pipeline exposes it, so the build mechanic cannot land without either a bounded RE probe to find the field's offset or published values accepted as provisional. Registered as **U15**; asked the owner which way to go rather than inventing a number (D5, D6). |
| 2026-10-25 | v0.39 — **A\* on the tile grid; villagers walk around forests.** `crates/sim/src/path.rs`: 8-connected A\* with integer costs (10 cardinal, 14 diagonal), ordered by `(f, g, y, x)` so expansion is deterministic and no hash iteration order reaches the result. Costs are algorithm constants, not game data — `movement.toml`'s weights are still `-1.0` sentinels (U5/U7) and will replace them here when measured. Routes are planned at order time and re-planned for the trip home and the return to work; the waypoint list is simulation state and is hashed. A walled-off node is refused at order time (`NoPath`) rather than walked into. Review found two defects the tests had not: **after depositing there was no route back**, so the villager walked from the town centre straight through the wall it had gone around (caught by a test that samples every tick), and **A\* refused to start from a blocked tile** — a villager standing on a tree could therefore never get a route home at all. Both fixed; 104 tests green. |
| 2026-10-25 | v0.38 — **wood reaches the stockpile.** `Task::Return { node, drop, kind, … }`: a full load turns the worker around, the destination is the nearest building the *variant's own* `drop_sites` accepts (so a lumber camp beats the town centre when one is closer), and whole resources are banked per player with the fraction kept in hand. `ResourceKind::index()` gives an exhaustive stockpile index, so a new resource kind cannot silently miss its slot. Ten tests written first; one existing test was wrong rather than the code — it asserted a villager *stays* full, which stopped being true once it could deliver, so it now asserts the cap is never exceeded and that deliveries keep coming. Known limit: after depositing, a villager whose node emptied goes idle instead of seeking another of the same kind. 100 tests green. |
| 2026-10-25 | v0.37 — **a villager gathers.** `Command::Gather { villager, node }` resolves the variant, rate and capacity from the tables once, at order time, and stores them on the entity, so nothing in the tick loop reads a table and a replay cannot name its own numbers. `Task::Gather` walks the unit toward the node in a straight line (no obstacle test yet) and, once standing still on it, takes `min(rate, room under the cap, what the node holds)` per tick. `Entity.amount` became fixed-point so a take can be fractional and the node can never go negative. Eight tests written first, then a review that found two things: **two villagers on a one-wood tree each took one wood** (each take was bounded by an amount read before the loop; now granted sequentially against what remains), and `target` was dead state that still fed the state hash. Known and deliberate: a villager that fills up keeps standing on the node, and an exhausted node leaves it idle — returning and depositing is the next increment. 98 tests green. |
| 2026-10-25 | v0.36 — **resource nodes are simulation entities.** `EntityKind::Resource(ResourceKind)` plus `amount`, seeded from `config/map.toml`: 118 nodes (90 wood at 100, 12 food at 150, 8 gold at 400, 8 stone at 300), ids after the players, position on their map tile. `[gather.resource_task]` joins a map resource kind to a gather task (`food = "forage"`, because the map's food is berries) and the loader refuses a name that is not a variant. Adversarial pass found two holes: a resource could be placed **under a town centre** (both are on grass, nothing else noticed — now refused by the map loader), and nodes carried `unit: 0`, which is a *real* dat unit, so absence was indistinguishable from an academy — `unit` is now `Option<u32>` and the hash covers the distinction. 89 tests green. |
| 2026-10-25 | v0.35 — **a match seeds itself; the CLI no longer invents unit stats.** `economy.toml` gains `[start]` (town centre 109 RTWC, villager 83 VMBAS, three villagers per player) because those ids were the last numbers still in danger of being hardcoded (D6). `World::seed_from` places one town centre on each map town-centre tile and the villagers on the nearest free walkable tiles in ring order, taking hp and speed from `gamedata/units.toml` and resolving the speed at the configured scale. `Entity` gained `player`, `kind` and `unit`, and the state hash covers them. The headless runner now takes `--map`, `--economy` and `--units` instead of `--speed`/`--hp`, and refuses the old flags: `tick=2000 entities=8 players=2 hash=d91a1a7e79ea6b08`. 13 data tests and 7 seeding tests written first. 82 tests green, step 4 closes. |
| 2026-10-25 | v0.34 — **real HD recordings obtained; U12 gains its second line of evidence.** `aoc-mgz` ships `hd-5.7.aoe2record` as a test fixture, so a recording of the exact version family was downloadable without a Windows machine. Decoded the container by hand (the library's own header parser fails on these files): `u32 section_len`, raw-DEFLATE header inflating to the same **`VER 9.4`** magic as the save games, then an **uncompressed** operation stream. hd-5.7 yields 68,011 ops — 31,104 SYNC, 31,104 VIEWLOCK, 5,792 ACTION, 11 CHAT — over 51 minutes. Sync cadence: HD 10.1/s (5.7), 10.9 (5.3), 8.6 (5.8) against **DE 59.2/s**, a 6:1 ratio. DE at 60 turns/s syncing once per turn, HD at 20 turns/s syncing every second turn, is the reading that fits; combined with the 50 ms reload grain (where 0.25 s is 5 turns at 20 Hz but 2.5 at 10 Hz) the 20 Hz assumption is corroborated from two directions and remains, honestly, *not proven*. The recordings also restore the step-2b oracle: actions carry millisecond timestamps, so build completions and arrival times can be compared against real games. |
| 2026-10-23 | v0.33 — **adversarial pass on the unit loader.** One real hole: a `drop_sites` entry pointing at a unit that is not in the table loaded fine, which is a villager who carries wood forever and never deposits it — visible ten minutes into a game, not at startup. Now refused, naming the id, with the real file proving all 46 carriers resolve (test written first). Three other invariants were probed and already hold in the generated data, so they are asserted rather than enforced: radius never negative (0.0–4.0), no unit names the same cost resource twice, and no drop site is dangling. 65 tests green. |
| 2026-10-23 | v0.32 — **the unit table loads, and the data disagreed with the types twice.** `crates/data/src/units.rs`: 1,166 units keyed by id, `MaybeFixed` everywhere a dat float enters state, strict fields, duplicate ids refused. Six tests written first; the real-file test then failed twice on genuine data facts: `train_time = -1` on four units (meaning "not trainable", not "instant") and `armours` carrying class id `-1` (meaning "no armour"). Both fixed by following the data — `class` is `i16` (236 units are -1), `train_time` is `i32`, class ids in attack/armour pairs are signed. Also recorded in `docs/mechanics/units.md`: `costs` can carry `[4, n]`, and resource 4 is **population**, which the knight has. 64 tests green. |
| 2026-10-23 | v0.31 — **the slice map exists and loads.** `config/map.toml`, generated deterministically by `build/tools/make_map.py`: 64×64, mirrored point-for-point, 90 wood tiles, 12 food, 8 gold, 8 stone, one town centre per player (16,16) and (47,47). The loader (`crates/data/src/map.rs`, 6 tests written first) refuses a wrong-width row, an unknown terrain character, a resource off the map, a side without a town centre, and — the useful one — **wood that is not on a forest tile**, i.e. a tree you could walk through. Terrain is `G`/`F` only for now; water is deliberately absent. 58 tests green. |
| 2026-10-23 | v0.30 — **economy loader landed, and a possible oracle found.** `crates/data/src/economy.rs`: strict TOML, `-1.0` sentinel, `farm.rate` refuses negatives (minus one is a typo there, not a sentinel), the D11 variant table must be non-empty and must not name the base villager, population/depletion must be positive. 7 tests written first, all seen failing. 50 tests green. Separately: `librematch/AoE-Kaitai` defines the **DE recorded-game body including "syncs"** — the closest thing to a per-tick oracle in a file — but it is DE-only and this install has zero recordings, so it is recorded as a lead, not a tool. The `.ksy`-as-spec approach is noted for our own future readers (`.aoe2spgame`, `.scx`). |
| 2026-10-23 | v0.29 — **D15: step-5 scope fixed.** Hand-authored 64×64 map in `config/` (not `.rms`, not `.scx`); a scripted opponent command list, which doubles as the replay format; grid A* pathing with the weights already in `movement.toml`. The three content pipelines — 122 `.rms` scripts, 49 `.scx` scenarios, 74 `.per` AI scripts — are future work and are explicitly not slice features: each is a language or format implementation, and none of them makes a villager gather wood. Prior-art review moved from monthly to **weekly** (Mondays 09:00 UTC). |
| 2026-10-23 | v0.28 — **D14: stay on HD/G1, borrow narrowly, watch the field.** Adopt only the format crates (D13), never their engine core; rebuild the step-6 backlog from the game's own subsystem split (§7.1, 16 subsystems taken from `genie-core`'s module list, each with its current blocker — including `fmath`, whose f32 emulation we deliberately do not adopt because fixed point replaces it under G1). Offer HD-specific findings upstream. Prior art reviewed periodically and recorded. |
| 2026-10-23 | v0.27 — **prior art: `librematch/genie` is doing G2, and it works.** Survived the survey rather than dismissing it: the project is nine days old, pushed the day of the survey, and `genie-core`'s own module docs state it "reproduces build 158041 **bit for bit**", keeping the game's f32 arithmetic and instruction-level conversions, with a module list taken from the game's subsystems (`gather, gather_search, farm_queue, attack, damage, best_target, garrison, ghost, group …`). So G2 is demonstrated as feasible by a peer with an oracle — and it contradicts D9: under G2 you keep f32, which is exactly why fixed-point is right only under G1. Their evidence is unaudited (7 stars, one visible author). **D13: adopt their GPL-3.0 format crates in the tooling repo** (`genie-dat` already cross-validated our step 1; `genie-save` could revive the abandoned 13-save oracle; `genie-rec` for recordings) while the engine stays MIT/Apache on the TOML boundary. Their module list also becomes a step-6 roadmap. |
| 2026-10-23 | v0.26 — **U12: nobody has published the tick rate, and openage deliberately doesn't have one.** Verified: openage's time subsystem is a millisecond clock on its own thread driving an event queue (50 ms cap, 1 ms floor), so they sidestep fixed ticks entirely; `librematch/genie` is the closest analogue — deterministic Rust, aimed at exact DE replay, ships a `genie-oracle` — and is the best place to ask, since it has the same problem. The Ensemble "1500 Archers" paper documents the AoE1 turn model but its turn length wasn't extractable here. §6.1 bands are now stated in **milliseconds as well as ticks**, because if the true rate is 60 Hz every "1 tick" band silently tightens 3×. Also recorded: a wrong-but-consistent tick rate cannot desync two clients of our engine — desync needs *disagreement* between them, or accumulated drift from a scale/tick mismatch, which `div_exact` refuses. |
| 2026-10-23 | v0.25 — **correction: the per-task data was never missing.** `dat.unit_data.tasks` exposes 412 populated per-unit task lists at `VER 5.7.2`; the `VersionError` came from `unit.task_info.tasks`, which is gated to AoK and DE only. Each villager variant declares its gathering `resource_in` (1 wood, 2 stone, 3 gold, 0 food, 15/16/17 pseudo — U14), so the task → variant mapping is **derivable**, and `economy.toml`'s hand-written table becomes a cross-check (D11 amended). Separately, `unique_unit_effect_ids` and `name2` are **DE-5.9-only fields**, absent from 5.7.2 — the parser was right to refuse, and no fork would help, so D12's "fork is likely" is withdrawn while the separate repo stands. Lesson recorded: a version-gated accessor is not evidence of absent data. |
| 2026-10-23 | v0.24 — **D12: the generator moves to its own repository**, with a fork of the GPL-3.0 parser if we need to patch it — and we probably will, since `VER 5.7.2` already raises `VersionError` on the per-task list and cannot read `unique_unit_effect_ids` or `name2` (the gaps U13 and D11 hit). Forking GPL code is permitted and stays GPL, so the licence boundary becomes a repo boundary. This repo stays MIT/Apache pure Rust. Publish-time step, not a change today. |
| 2026-10-23 | v0.23 — **correction: the GPL does not reach the engine.** §6.2 framed the missing generator as a licence blocker; it is a packaging question. Verified: `crates/` contains zero references to the parser, the engine reads TOML at runtime only, and the parser is `Divy1211/GenieDatParser` (GPL-3.0, a build-time dat miner) — **not** openage's converter, which SPEC §5.2 rejects. The dependency edge is `tools/genconfig → parser`, so the engine's MIT/Apache status is unaffected and the generated TOML is not GPL-encumbered (it is derived game data, governed by D8). `docs/licensing.md` now records the edge, the provenance and the three packaging options. |
| 2026-10-23 | v0.22 — **standing process: compact, then adversarial review after every change** (§11). Adversarial review of v0.21 found four things, all fixed test-first: (1) the runner invented game values — a default `--speed 1.35` and a hardcoded `hp = 100`, both D6 violations; speed and hp are now required, with 3 new subprocess tests seen failing first (`--speed is required`, `--ticks is repeated`, `--help` exits 0). (2) Unknown, valueless and repeated flags were silently ignored — a run of 10 ticks could be reported as 10,000; all are errors now. (3) `test_config_consistency` built a name→id dict while 180 units are called RESERVED and 56 are unnamed; it now asserts the name is unique. (4) The test total was misreported in v0.21's session (claimed 38, actual 40; now 43) — recorded because a self-report that is off by two is a self-report that cannot be trusted unchecked. Verified alongside: 46 units carry drop sites, with no dangling ids and no duplicates. Accepted and documented: `--ticks` is unbounded. |
| 2026-10-23 | v0.21 — **D4 answered: B (publish engine + tooling, keep the spec internal).** Condition from the owner: a stranger with their own install must be able to clone, build and run. `ruage-headless` now exists and is tested **as a subprocess** (3 tests): `--config config/sim.toml --seed 7 --ticks 100 --speed 1.35` prints `tick=100 hash=bdd1d648344fcb6d x=135000`; the same command twice is byte-identical; a sub-precision speed is refused (`1.41421356 needs more precision than scale 20000`) rather than rounded. Building and running need no game data. §6.2 records the checklist and the one blocking gap: `tools/genconfig` imports **GPL-3.0** `GenieDatParser`, so a public clone cannot produce `config/gamedata/` — the owner's condition is met for build+run, not yet for a complete game, until either our own dat reader exists or `tools/` is licensed GPL-3.0. |
| 2026-10-23 | v0.20 — **D11 landed on the data side: drop sites are emitted and the task→variant table exists.** `genconfig` now emits `drop_sites` per unit (test-first: two new pytest tests, both seen failing). Measured from the real dat: lumberjack 123 → 109 + **562** (lumber camp), miner 124 and gold miner 579 → 109 + **584** (mining camp), forage/hunt/fish/herd → 109 + 68 (mill), base villager 83 → 109 + mill only (the trap). `economy.toml` gained the 10-row task→variant table with rates left to `gamedata/`, and `tools/tests/test_config_consistency.py` guards the join (id exists, is a `VM*` variant, plausible rate, has a drop site, base villager absent). 21 pytest tests green. Side finding: the repair variant VMREP (156) carries `work_rate = 12.5` — a dat candidate for the unresolved `repair_hp_per_second`, recorded as a candidate only, sentinel kept. |
| 2026-10-23 | v0.19 — **D3 answered: one civ (British) first.** British is dat civ index 1 (0 is Gaia), so the name is incidental; the loader must not hardcode it. Step 5's scope is British vs British. With U13 measured the same day, adding the next civ later costs a type-102 disable-list join, not a data-model change. |
| 2026-10-23 | v0.18 — **measured: civ differentiation is not in the unit table.** All 31 non-Gaia civs match civ 1 in 5 rows out of ~36,000 (Khmer zeroes `carry`/sets `work_rate` 1.0 on houses and drop sites; Celts have one extra unit). Civ identity is `tech_tree_effect_id` — a **type-102 disable list** (British 31 entries, Goths 38) — plus `team_bonus_effect_id`, the unreadable `unique_unit_effect_ids`, and the U8 resource array. New U13. The earlier assumption that a second civ needs a per-civ unit delta layer was therefore wrong, and is corrected in `docs/mechanics/units.md`. D11 records the gathering-data shape: `genconfig` emits drop sites, `economy.toml` carries the task→variant table. |
| 2026-10-23 | v0.17 — **step 4: the tick loop and state hash exist.** `crates/sim`, 10 tests, each seen failing first (compile-RED: the crate had only tests, so every name was unresolved). Landed: `World::step` applies queued commands then integrates in entity-id order; movement uses `Fixed::div_exact` so an inexact `speed / tick_rate` is an error rather than a rounded step; PCG32 with separate `sim`/`map` streams whose ids are the ASCII of their config names; FNV-1a 64 over explicit little-endian bytes (no `usize`, no layout assumptions); `command_apply` honoured from config, both `next_tick` and `immediate`; `max_entities` enforced. Proof of the determinism claim: two worlds built with the same seed and command log produce equal state hashes for 100 ticks, and spawn order does not change the hash. 37 tests green, clippy clean. |
| 2026-10-23 | v0.16 — **D1 answered: G1.** "Generally similar behaviour, sensibly attainable." G2 was priced first (it needs a per-tick oracle that does not exist here; the original's sim is x87 float; a bit-exact copy is a derivative work) and rejected; G1+ was considered and not adopted. §6.1 now states the acceptance thresholds: exact where the value is data or our own bookkeeping, tolerance-banded (5% gather, 1 tick build, 10% time-to-kill, 2 ticks arrival) where behaviour is only observable through play. Step 3 (decision gate) is closed. |
| 2026-10-23 | v0.15 — **adversarial review, four defects fixed test-first.** The scale guard was arithmetically inverted (`10^k % scale == 0` accepted 1, 2, 4, 5, 8 and rejected 300); it is replaced by two derived rules (D10). `Fixed::parse` accumulated digits unchecked, so a long literal panicked in debug and would have wrapped in release — now `OutOfRange`. The loader accepted contradictory config (`schema = 99`, `floats_in_sim_state = true`, `max_entities = 0`, one RNG stream, 60 Hz with a 20 kHz scale); each is now a hard error. `cargo clippy -D warnings` was red and is now clean with `--all-targets`. New `crates/data/tests/corpus.rs` sweeps the whole generated corpus and immediately falsified the review's own three-decimal hypothesis (`work_rate = 0.0111` on two buildings) — hence two grains, not one. 26 tests green. Unfixed and recorded: gather/drop-site data is not in the generated config (step 5 blocker), the pre-commit hook is path-based only, and `docs/config.md` described a manifest staleness check that does not exist. |
| 2026-10-23 | v0.14 — **step 4 started, test-first.** `crates/fixed` (9 tests) and `crates/data` (9 tests), 18 green. Every cycle showed a real RED. Two domain rules fell out and are now enforced: **D9** — the fixed-point domain is simulation state only (load-time constants such as pathing weights stay f64), and the scale must divide a power of ten, so 20000 loads and 3 is rejected. *(The second clause was wrong and is superseded by D10 — see v0.15.)* |
