# SPEC — AoE2 HD → AI-assisted recreation

**Read this file first, every session.** It is the entry point: current state, verified
ground truth, corrections to older notes, and the agreed procedure. Everything else in
this tree is either source data or supporting documentation.

| | |
|---|---|
| Spec version | 0.84 |
| Last updated | 2026-10-26 |
| Decision status | **D1 = G1, D2 = file-based oracle, D17 = four layers with the resource enum deleted — see §6 and `docs/decisions.md`** |
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

Rules: write only to `SPEC.md`, `docs/`, `config/`, `build/`. Never write into
`Age2HD/`. Anything generated goes in `build/` or `config/gamedata/` and must be
reproducible from a recipe in this file.

**The importer is a separate repository** (`ruage-tools`, D12, split 2026-10-26): it holds the
`.dat` reader, the semantics that belong to Age of Empires (resource ids, packed attack, effect
kinds) and its pytest suite. It is GPL-3.0-or-later because it imports `GenieDatParser`; this
repository is MIT OR Apache-2.0 and reads only TOML. The split is why this repository's history
contains no import of that parser (checked: `git log --all -- tools/` is empty after the rewrite),
so publishing it publishes nothing borrowed.

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
| Shipped tooling that produces artefacts (now in `ruage-tools`, same rule) | Generated output (`config/gamedata/*.toml`) — verified by tests, not written by hand |
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
| `SPEC.md` | current, v0.84 | Entry point — this file |
| `docs/age2hd.md` | historical, **superseded by §4** | 2026-10-23 handoff from the previous session |
| `docs/hermes.container` | reference | Host container definition |
| `docs/parse-notes.md` | current | Step 1 record: toolchain, reproduce commands, parse counts, cross-validation, per-civ unit model, caveats, licences |
| `ruage-tools` (separate repo) | current | **D7/D12** — test-first importer: `genconfig/` (dat → content pack) + 22 pytest tests, GPL-3.0-or-later |
| `README.md`, `LICENSE-MIT`, `LICENSE-APACHE` | current | Public entry point and the dual licence (D21) |
| `config/` | current | **D6** — gamedata TOML (generated) + behaviour configs + `overrides.toml`; see `docs/config.md` |
| `docs/config.md` | current | Config layer: precedence, strict loading, provenance tags, regeneration, staleness check, D4 caveat |
| `crates/fixed`, `crates/data` | current | Fixed-point arithmetic + strict config loaders (sim, economy, map, units, techs, effects, combat, resources, pack) |
| `crates/sim` | current | Deterministic tick loop: entity-id iteration, exact per-tick movement via `Fixed::div_exact`, PCG32 with separate sim/map streams, FNV-1a state hash, command queue honouring `command_apply`, gather/build/train/combat, scenario runner |
| `crates/data/tests/corpus.rs` | current | Whole-corpus sweep: every value in `config/gamedata/` is exactly representable, and every speed is a whole number of scale units per tick. Skips when the generated data is absent |
| `docs/licensing.md` | current | **D8** — repo hygiene: what is never committed, legal basis, enforcement |
| `.githooks/pre-commit` | current | Refuses commits containing game data or extracted values **at known paths**. It matches paths and extensions, not content: a CSV of unit stats under `config/` commits cleanly (verified 2026-10-23) |
| `docs/sources.md` | current | Every outside source used, what for, trust level, and the version caveat (wiki = DE, dat = 5.7) |
| `docs/oracle.md` | current | The oracle: 13 saves found, raw DEFLATE, `VER 9.4`; what snapshots can/can't answer; D1 recommendation |
| `docs/mechanics/*.md` | current | 6 subsystem files + `README.md` (tag legend, index, unknown registry U1–U12) |
| `docs/formats/*.md` | planned | dat / slp / dld / rms / per — what our parser assumes |
| `docs/decisions.md` | current | D1–D21 with rationale: D5 dat authoritative, D6 config layer, D7 mandatory TDD, D8 no game data in git, D17 four layers, D20 eight sides, D21 licence |
| `docs/next-step.md` | current | The effect interpreter, priced from the data (kind 3 landed v0.55; kinds 4/5 remain), measured from `effects.toml`: 1,667 of 3,306 commands are kinds 4/5, 270 effects are nothing else, and the engine already resolves stats at order time |
| `docs/plan-resource-deletion.md` | **done (2026-10-26)** | The plan D17 was paid down with: what was load-bearing, the order of work, and the test that proves the boundary — `a_content_pack_with_our_own_resources_loads_and_spends` |
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
| 2026-10-27 | v0.84 — **ballistics**, and a shot that can miss. Two halves, because either alone is nothing: an arrow now **lands where it was aimed** and hurts the target only if it is still within **half a tile** of that point (before this, every shot was a hit — a projectile flew to its aim and then damaged the target wherever it had got to), and **attribute 19 (ballistics)**, decoded in v0.83, is consumed: with it the aim is the **intercept**, computed by iterating the time of flight three times along the target's motion, in integer arithmetic including an integer square root. The target's motion is **read, not stored** — `Entity.velocity` is a spawn-time field that movement never updates, so `motion_of` asks what step the target's task would take this tick. Mutation-proved: removing the landing check makes a runner take arrows without Ballistics, and disabling the lead makes the Ballistics case miss; a stationary-target control shows the change is about movement, not a broken check. **Named limitation:** the anchors do not exercise this path — nothing in the scripted scenario fires an arrow. **281 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.83 — **effect kind 0**, the third namespace: `[kind, unit, class, attribute, value]` where kind 0 **sets**, 4 adds and 5 multiplies (only 4 and 5 pack attack/armour, which is why the same attribute arrives as `1025.0` in one and `5.0` in another). All 106 kind-0 commands are now decoded by the importer; attributes are named only where a tech pins them — **21 population** (Slavs' team bonus sets twelve military buildings to 5, the Chinese town centre to 10), **11 accuracy** (Thumb Ring 100), **19 ballistics**, **20 minimum range** (Andean Sling 0), **102 extra projectiles** (Yasama 3) — and two stay `other:<id>` because nothing pins them. **Population is the one the engine consumes**: `population_provided(player, unit)` replaces the role count, so the ceiling sums what each finished building supports, and a **team bonus** now reaches the engine at all — `modifiers_for` reads researched effects *and* a per-player `bonus_effects` list filled from `civ.team_bonus_effect`. Mutation-proved: ignoring `set` fails the Slavs test, never granting the bonus fails it too, with a British control that gains nothing. **Stability**: both anchors unchanged. **279 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.82 — **rallies that gather, and rallies you cannot reach** (correcting D39 on the user's word). An unreachable rally is no longer "stay home": `walk_towards` tries the tile, then the first reachable tile on each ring around it out to ten rings, and walks the unit as close as the map allows. A rally onto a **resource** is a gathering order — the node is chosen by the tile and worked through the ordinary gather path, so a villager rallied onto stone with wood on three sides gathers the **stone** (proven on a purpose-built map, down to the stone's amount falling) and never substitutes the nearest tree. Units that cannot work the node (a spearman on a tree) and nodes that cannot be reached both fall back to walking as close as possible. Mutation-proved: disabling the gather fails the stone test, removing the ring search fails the pocket test. **Named cost:** the ring search is bounded at ten rings (~440 tiles, each with a path attempt) and runs **once per produced unit**, never in a tick. **Stability**: both anchors unchanged. **278 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.81 — **rally points**, which the engine had none of: `Command::Rally { building, point }` with three states. `Some(tile)` walks the finished unit there via a new `Task::Move`; `None` clears it and restores the default (beside the building); and `Some` of the **building's own tile** keeps the unit **inside** — the half of the production-building rule (D38) that needed a rally. `RallyPoint::Inside` is derived from clicking the building rather than needing a second command. State is a side table (`World.rallies`) that is hashed, consulted once per finished unit and never per tick. Off-map rallies are refused (`OffMap`); an **unreachable** one is not — the unit stands where it appeared — recorded in D39 as a named gap rather than assumed. Mutation-proved both ways: ignoring the inside arm fails the "stays inside" test, ignoring the tile arm fails the "walks there" test. **Stability**: both anchors unchanged. **Performance**: one map lookup per produced unit; `Task` grew a variant, which is the kind of change pass thirteen's floor can move for no reason. **276 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.80 — **a production building holds only what it has just trained.** D36 let any building with a capacity take any unit beside it; the barracks, archery range, stable, siege workshop and dock accept only a unit they have just made (by rallying to themselves) and never take one back, while town centres, towers and castles accept outsiders. **The data draws the line itself:** those five carry a garrison capacity of ten and **no attack array**, the three shelters carry both — so the rule is "a building that can fight takes outsiders in", derived rather than enumerated (D17), and the refusal is `CannotGarrisonHere`. **Named, not modelled:** the other half of the rule needs rally points, which this engine has none of — a trained unit always appears on the neighbouring tile — so a barracks holds nobody today, which is right for a production building. Lands with the v0.79 corrections: ejection at 20% instead of death, garrison volleys as dps arithmetic, and the Throwing Axeman test that had been passing off the pierce filter rather than the range check. Mutation-proved four ways across the two commits. **Stability**: anchors unchanged. **271 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.79 — **correcting D36 on two counts, both from published rules.** (1) A garrison is **ejected, not killed**: a building throws it out at **20% of its hit points or below**, unharmed and where it stands, and takes nobody in until repaired above that — content `[combat] garrison_eject_fraction = 0.2`, checked where damage lands rather than in a per-tick scan. (2) **Extra arrows are dps arithmetic**: `floor(sum of garrisoned units' dps / building dps)` where dps is the **pierce** attack over the reload, and the division happens once at the end; the qualifier is the **damage class**, which our own data settles — a Throwing Axeman (281) and a Gbeto (1013) are ranged with *melee* damage and add nothing, while archers, cavalry archers and hand cannoneers add. **Deliberately not modelled, and named:** each extra arrow should transmit the garrisoned unit's attack (only the first is the building's), and a garrisoned villager should count as a fixed 2.5 dps — today every arrow carries the building's damage and villagers contribute nothing. Mutation-proved three ways; the third showed the Throwing Axeman test was passing off the pierce filter rather than the range check, so it now says what it pins. **Stability**: anchors unchanged. **270 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.78 — **garrison, and buildings that fight.** Every unit carries `garrison` (town centre 15, castle 20, barracks 10; 55 buildings have a real capacity) and town centres/towers/castles carry their own `attack` array and `range`; both were parsed and unused. `Command::Garrison`/`Ungarrison` with the **building's own capacity** as the limit and a standing-on-it-or-beside-it rule (`TooFar` otherwise, no teleport); a garrisoned unit still costs population, cannot act (`Garrisoned`), cannot be attacked (`NotATarget` — the building is the target) and is off the map for movement; and **a garrison dies with its shelter** rather than pouring out of a razed town centre. A building fights as an armed entity that cannot walk: reach from `range`, damage from its attack array, shot as a projectile — and `start_attack` skips pathing for a building, guarded by **kind, not speed**, because a stationary unit in a test still wants its route (one existing test caught exactly that). Mutation-proved three ways, and one found a hole: removing the capacity check and letting a garrison survive its shelter were both caught, but **letting garrisoned units be attacked was caught by nothing** until a test was added. **Line of sight stays out with its reason written:** 844 units carry `los`, but nothing asks — no fog, no scouting, no unseen-untargetable rule (the D26 pattern). **Stability**: both anchors unchanged. **268 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.77 — **a shot is an entity.** Ranged combat's instant damage becomes an arrow that flies at `[combat] projectile_speed` tiles per second, carries the damage resolved when the order was given, and lands where it was **aimed** — a position, not the target — so an arrow whose target walks or dies first finds nothing and does not retarget. `EntityKind::Projectile`, not a unit: no population, not attackable (`NotATarget`). Speed zero keeps the old instant behaviour, so existing packs are unaffected. **Two shapes of the tick dictated the code, both found by breaking them:** (1) the entity list is index-stable while positions are applied, so a shot created inside the strike pass shifted every later index and the panic fired in tests that never mentioned projectiles — shots are now recorded and created after movement, like `finished` and `buried`; (2) `step` already had a `spent` vector for consumed waypoints, and the obvious name for landed arrows bound to it, so arrows were never removed — it is `landed_arrows` now. Mutation-proved: forcing shots back to instant fails both projectile tests. **Known:** projectile ids are "one past the highest" (an O(n) scan per shot, as trained units already do) — recorded in performance.md as the first fix if arrow-heavy fights ever profile badly. **Stability**: both anchors unchanged. **261 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.76 — **the ceiling pauses training; it never refuses the order** (correcting v0.73's rule, on the user's word and the game's). A queue may hold more units than the player can support — so `check_train` no longer looks at population at all, `PopulationCapped` is deleted along with the two helpers that existed only to serve it, and the production pass asks **once per player with a building in production** whether they are at their ceiling. A capped player's jobs are skipped, which leaves their countdown untouched; when a slot frees, the same job continues. The price of the wrong rule was its tests: five asserted `PopulationCapped` and all five were rewritten. **A weak test of my own, recorded:** `a_full_player_holds_the_order_and_pauses_the_clock` still passes with the pause removed, because a crowd of villagers around a town centre leaves a finished unit nowhere to stand — "nothing arrived" is true for the wrong reason. The mutation is caught by the queue-drain test and by the house and death tests. **Stability**: both anchors unchanged. **Performance**: one entity-list walk per producing player per tick, skipped entirely when nobody trains — the shape that would matter at scale, where the fix is a maintained count. **259 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.75 — **the training queue, done without widening `Entity`.** One slot per building was a slice, not the game: `[train] queue_length = 5` is content, the head still lives in `Entity.training` (so the tick loop keeps its fast path and `Entity` does not grow — the pass-seven lesson), and the rest waits in `World.queued`, a side table by building like `paths` and `Units::variants`. An order is **paid when it is placed**; a finished unit hands the building to the next, which starts counting from the create time it was priced at; and the population gate counts **committed** units, so a queue cannot slip past housing. **The finding:** with `command_apply = "next_tick"`, `submit_checked` validates against state where earlier orders have not landed — five orders in one tick all saw an empty queue and all passed. The checks now count pending `Train` commands per building, and pending orders per player for the population gate; immediate-apply mode would have hidden this entirely. `AlreadyTraining` is gone, replaced by `QueueFull`. **A weak test of mine fell out of the mutation run:** the cap-versus-queue test placed orders that were still pending, so it passed with the committed count deleted; it steps once now and fails when the count is removed. Mutation-proved three ways: ignoring pending orders fails the queue-length test, ignoring committed units fails the population-queue test, and never handing over fails the arrival test. **Stability**: both anchors unchanged (`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`) — meaningful rather than incidental this time, because the scripted run does train villagers and does enter the training code. **Performance**: `Entity` unchanged in size, tick loop unchanged in shape, one `saturating_sub` per building with a job. **258 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.74 — **ranged combat: `range` was the largest hole and it is attribute 1.** Above zero on 277 of 1,166 units, all of which closed to adjacency and fought like spearmen. A unit with reach now strikes from anywhere inside it and walks only until it is inside; a melee unit keeps the adjacency rule, diagonals included. One comparison on **squared distance in engine units** — no square root, no trigonometry to disagree across platforms — and the reach rides on the task, resolved at order time like damage and reload, so a tech landing mid-fight cannot extend a shot already in the air. Range is the fourth attribute through the existing modifier path and the one that finally makes the archer line visible: Fletching/Bodkin/Bracer are `range +1` on class 0, tested **behaviourally** (an archer five tiles out walks a tile without Fletching, stands still with it). Mutation-proved twice: forcing the melee branch fails all three ranged tests; dropping the range modifiers fails exactly the Fletching one. **Not modelled and now named:** projectiles (a shot is instant damage at range, not an entity), line of sight (a unit shoots anything in reach it can path to, walls included), buildings that shoot. **Stability**: both anchors unchanged (`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`) because neither enters the changed code — recorded as exactly that rather than as "no cost". **Performance**: the first tick-path change in six passes; a handful of integer ops per attacking entity per tick, bounded by attackers, not entities. **256 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.73 — **housing, and the last dead content table.** `[population]` (`cap_default = 200`, `house_population = 5`) had been validated at load for months while the sim never read it; a house was 25 wood and a shape. Population is now one per live unit (the dat carries no per-unit cost), the ceiling is `min(cap_default, finished houses × 5 + finished town centres × 5)`, and a train order past it is refused with `PopulationCapped` **before it is priced**. `town_centre_population = 5` is new content and published fact (house and town centre both five; a castle's twenty is not modelled). **The Huns forced a rule instead of a formula:** their tree bars the house, so a house-counting cap pins them at zero all match; the exemption is read from the same bar the engine already checks (`civ_barred_unit`), giving them `cap_default` — no new field, and a pack with no civs has no exemption. Three mutations, three precise failures: no check fails four tests, counting unfinished houses fails only the construction test, dropping the exemption fails only the Huns test. Two findings from making the tests honest: a trained unit with every neighbouring tile taken **waits inside its building** (a crowd stalls training), and the fixture had to load the effect tables or the Huns exemption does not exist. **Stability**: four-hour probe and scripted run both unchanged (`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`) — the first step in a while where no hash moved, because the feature changes what a player may do, not what the world is. **Performance**: two list scans per order, none per tick. **253 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.72 — **"at least N of these", not "all of them".** The dat's tech carries `min_required_techs` beside `required_tech_ids`, and the importer had been exporting only the ids, so every prerequisite read as "all" — while 120 ("Shadow Node for Age Two") lists five buildings and needs two, 147 needs both of `[101, 667]`, and 659 needs one of `[120, 638]`. Nineteen techs have `min < count`. Both copies of the gate (the order path in `check_research` and the availability resolver) now count; content that says nothing keeps the old meaning, "all", so no pack changes. **Two corrections, both mine.** (1) D29 had claimed that an archery range buildable without a barracks was "wrong against the game": it is not — the archery range has no building prerequisite in AoE2, and the dat spells that as an OR-set holding an always-true marker (638), so the D29 paragraph is rewritten. (2) The first version of the counting test used the Hand Cannon tech and **passed with the "all" rule restored**, because 285 ("C-Bonus, Free Chemistry") is granted free once its own prerequisites are met; the mutation run caught it, and the tests that count now use a pack of our own — one tech per gate copy — because no tech in this dat reaches the order path with a count. Mutation-proved both ways: restoring "all" fails exactly the two counting tests and leaves both refusal tests passing. **Stability**: four-hour probe unchanged (`b30d49d89f8bcf4f`); scripted run `05c91e3bff3f9eee`, because the OR-sets now grant more techs at load. **Performance**: not claimed from the probe — counting runs on a research, and `min_required` rides on an already-loaded table. **248 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.71 — **the tech tree was unreachable, and the dat's habit is why.** Written as tests first against machinery that looked complete: four of six passed at once (a Dark Age barracks builds, a Dark Age archery range does not, a barracks cannot train a spearman before Feudal and can afterwards). The fifth failed for real — **an archery range was unobtainable even in Feudal** — because tech 147 requires 667, which requires 122 "Shadow Barracks -- Age Two", whose effect has **no commands**, plus 638, which has no effect at all. The dat spells "*you have a barracks*" as a tech, and nothing granted it; the same habit is why every earlier test hand-granted 659 before researching an age (the Feudal Age requires a *shadow town centre*). `availability_techs` now includes effects that do nothing, not only effects that enable units, so the Feudal Age is reachable by playing — 500 food at the town centre, no hand-granting — which the sixth test proves end to end. Mutation-proved: turning shadow bookkeeping off fails exactly the two Feudal-dependent tests and leaves the four Dark Age ones passing. **Known simplification (D29):** the shadows are satisfied up front, so an archery range needs no barracks standing; the name that would say so is a display string, not data. **Stability**: four-hour probe unchanged (`b30d49d89f8bcf4f` — that world loads no techs); the scripted run moves to 2,400 ticks, `431d5b8fb2076854`, because the shadow techs are hashed state. **Performance**: not claimed from the probe — the list is walked on a research, never inside a tick. **244 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.70 — **the farm food line, and a wrong reading corrected.** Kind 1 is `[1, 36, 1, -1, 75]`: a change to a **resource**, not to a unit — the field was read as a unit id in earlier notes, which made Horse Collar look like it re-priced a bombard cannon. The numbers settle it: 75/125/175 are the published Horse Collar/Heavy Plow/Crop Rotation bonuses, all additions to resource 36. The importer decodes kind 1 into its own table (`resource_modifiers`, next to kind 4/5's unit attributes — two namespaces, one command list) and content supplies the meaning: `[farm] food_resource = 36`, the third appearance of the D17 shape (a dat namespace with no names in it). A tech that lands adds its food to **every plot the player has that still holds some**, and plots built afterwards start at the new total; a **spent plot stays spent** — its food was eaten, and refilling it would be inventing food. Modes other than 0/1 exist once (`-1`, effect 605) and are carried uninterpreted. Mutation-proved: filtering the resource match to `false` fails the two upgrade tests and correctly leaves the spent-plot test passing. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f`; the scripted anchor moves to 2,400 ticks (`046fd1d8d8d0301c`) because 900 ticks never reached the build order. **Performance**: not claimed from the probe — nothing was added to a tick or to `World`. **238 tests**, clippy and fmt clean; importer `468d861`. |
| 2026-10-27 | v0.69 — **farms, and three defects about standing.** `[farm]` has been read at last: `unit = "farm"` (a role) and `resource = "food"` are the entire extent of the engine's farming knowledge — a farm is a building it would otherwise treat like any other, and it becomes worth gathering from only because content says so. `gather_source` is now the one authority on "what may be worked here" (validator, order and trip home all call it); a plot holds `food_per_farm` from the moment it is marked out and refuses work until finished (`BuildingNotReady`); `use_villager_rate` picks the pace (0.4 building / 0.53 variant — still an open data question). **Three defects fell out, all about where a worker stands:** arrival was `position == node.position`, which is never true for a blocked building tile; the builder walked *onto* its site and finished the job standing inside the building, from which no route home can be planned; and `return_task` read the stockpile column off the node's *kind*, so a farm gave no trip home and a villager filled its load and stood there for the rest of the match. Fixed by `work_position` (neighbouring tile that is free **and reachable** — a free tile inside a wall cost the scenario a step), adjacency as arrival in both ticks, and the column coming from `gather_source`. Because positions changed, the ten-minute scenario's `nth` tree choices became reachability-aware (`nearest_resource` filters on `World::can_reach`). Mutation-proved: setting a farm's food to zero fails both farm tests. **The size hypothesis was wrong and is recorded as such:** `Task`/`Entity` are 32/184 bytes on both builds, so the +7% the idle probe shows cannot be struct growth; it stays unresolved (pass sixteen) and the four-hour probe is unchanged at `b30d49d89f8bcf4f`. **235 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.68 — **teams, and a queue re-ordered on evidence.** `[start] teams = [1, 1]` (entry *n* is player *n+1*, the last entry covers the rest; **empty is a free-for-all**, so the eight existing combat tests keep their meaning and no pack has to say "nobody has allies"). Two players on one team cannot attack each other — `AlliedTarget`, the general case of `FriendlyFire`, which was always a team rule with one player in it — and the resolved teams are hashed **only when content names them**, so a world without teams hashes exactly as before. **The announced next step, team bonuses, was deliberately not taken (D26):** every sampled `team_bonus_effect` targets a mechanic this engine lacks — British and Goths a building's *work rate*, French line of sight, Slavs garrison capacity, Teutons a farm resource — so applying them would produce tests that cannot fail. Kind 0 (attributes 19/20/21: garrison and projectiles) and kind 1 (a *resource* id: the farm food line at 75/125/175) are the same story, and the sim has **zero** mentions of a farm even though `[farm]` has been parsed and unused for months. Mutation-proved: `if false &&` on the ally check fails `players_on_one_team_cannot_attack_each_other`. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f`. **Performance**: not claimed from the probe — no tick path, per pass thirteen's floor. **232 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.67 — **per-player civilisations.** `[start] civ = "British"` becomes `[start] civs = ["British", "Huns"]`: entry *n* is player *n+1*, the last entry covers any further player, so a one-entry list still means "everyone plays this". `World` carries `civ_by_player: [Option<u32>; 8]`, the two bar lookups (`barred_by`, `civ_barred_unit`) take a player, and the civ ids are **hashed per player** — a replay that lost whose civ was which would refuse different techs silently. The project's own pack names two civs, which is what makes the bar rules a live case rather than a fixture: player 1 (British) is refused the Hussar (428) and can build houses, player 2 (Huns) is allowed the tech — its refusal is the prerequisite, not the civ — and cannot build a house at all. `two_players_can_be_different_civilisations` asserts both halves in one world, matching on the error *variant* so the test pins the bar rather than the prerequisite chain. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f` (no civs loaded ⇒ nothing hashed); scripted run with all tables `936b127946fd2ef3`. **Performance**: not claimed from the probe — this is a change with no tick path, and pass thirteen established that the idle probe's between-build floor is ~±8%; it is judged by inspection (`civ_of` runs per order) plus the hashes. **230 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.66 — **a civ can take a unit away, and a measurement finding that outranks it.** `[2, unit, 0, -1, 0]` is "disabled": one row in the whole table, in the Huns' tree, for the house — and the Huns cannot build houses, so `check_build`/`check_train` refuse with `UnitBarred` naming the civ, and no tech lifts it (D24a). The four `b = -1` rows are **not** that: they sit in effects called "Enable sheep/llama/cow/turkey", carried by techs that *cost* 40 food + 25 wood and take 15 s, with an attribute and a value that no enable/disable reading explains — so they stay uninterpreted and named in D24a rather than guessed. The bar is **derived on demand** (one effect's command list, on an order), because the first version cached it in a `BTreeSet` field on `World` and that is exactly the shape pass seven warned about. **The measurement finding matters more than the feature (pass thirteen).** The A/B reported +6% at 126 entities for a change with no tick path, three rounds both orderings, so it was chased: removing the field changed nothing (2.42–2.44); and every A/B since pass seven had run the base worktree with `units.toml` **stripped of `variant_of`** (necessary then, unnecessary now), which made the base's table smaller — fed the current binary the stripped file and it matched base (2.23–2.27), while both builds with the full file split 2.25–2.27 against 2.44–2.51. What remains is between-build layout sensitivity of **roughly ±8% in the idle 126-entity probe**, which is the same order as every "~2% residual" reported since pass seven — those were this sensitivity, not regressions. The probe stays useful for per-tick changes and the entity-scaling curve; a change with no tick path is judged by inspection plus the hashes, and those are unchanged (`b30d49d89f8bcf4f`). **229 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.65 — **the spear line, and the wrong id hiding in it (D25).** The user's description — spearmen free with the Feudal Age, pikemen and halberdiers researched — is how the data works, and checking it found our content naming the **militia** as a spearman: unit 74 is 40 hp, 60 food + 20 gold, 21 s (published militia numbers) and *every* infantry upgrade starts from it (`[3, 74, 75]` Man-at-Arms, `[3, 74, 77]` Long Swordsman, `[3, 74, 473]`, `[3, 74, 567]`), while the dat's name field calls it "SPRMN" — one ahead of what the ids are. The line is **93 → 358 → 359**: tech 87 "Pike" (free, instant, requires Feudal) makes 93 available, 197 upgrades 93 → 358 at the Castle Age, 429 upgrades *both* 93 → 359 and 358 → 359 at the Imperial Age, and civs without them carry 197/429 on their restriction list. Content now says `militia = 74`, `spearman = 93`, `pikeman = 358`, `halberdier = 359`, and the barracks trains militia and spearmen. Two rules made that honest: **an upgrade target is not available, it is an upgrade** (`check_train` refuses a unit whose upgrade tech is un-researched, so a Dark Age barracks cannot train halberdiers just because the entry lists the line), and **the line counts for the entry check** (a pikeman matches a `[train.entry]` naming spearmen, through the family root). Tests: the line walked as far as it is unlocked (Dark Age refuses 93, Feudal allows it, 358 refused until 197), and the upgrade reaching units already on the map 93 → 358 (45 hp → 55, not a reset) → 359 (60 hp). The ten-minute scenario now trains **militia**, because that is what a Dark Age barracks can make — its earlier "spearman" was a militia with a wrong label. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f`; scripted run `470c148c117eae84` at 900 ticks. **Performance**: within the same ~2% band recorded since the seventh pass (126: 2.42 vs 2.37–2.38 µs/tick; 2,126: 36.80–37.48 vs 36.91–38.60). **228 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.64 — **unit availability: a unit exists once a tech "makes it available" (D24).** Kind 2 with a positive flag is that command, and the effect names state it themselves — "Barracks (make avail)", "Watch Tower (make avail)", "Castle -- Age Three", "Trebuchet", "Pike". The census is equally plain: **109 units are gated by 81 techs that do nothing else**, each costs nothing, takes no time, has no home building, and requires an age (59 also require 266, the "a castle exists" marker); exactly one has no requirements at all (332, the outpost). So two content-driven rules: a gated unit cannot be **built or trained** until a tech carrying a positive kind-2 row for it lands (`check_build` and `check_train` both refuse with `UnitUnavailable`, **before** the price, because "you cannot have this yet" outranks "you are poor"), and a tech that is only bookkeeping is **applied by the engine as soon as its prerequisites are met** — which is why aging up makes the barracks, watch tower, pike and castle appear without a click on an invisible tech, and why content needs no per-age table. Ungated units (villager, house, spearman) are untouched, which is what keeps the rule from becoming a blanket lock. Tests: a castle refused in the Dark Age and buildable the tick after the Castle Age (tech 137 auto-applies); a trebuchet refused at a Castle-Age castle and trainable at Imperial (256); a house and the barracks available from the start (220 needs only 104). The four `b = 0/-1` rows (the Huns' house, four "enable animal" rows) are **not** interpreted and are recorded as the known gap, because their direction is established by nothing in the data. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f`; the scripted run with all tables hashes `66179905f8bcffa8`, the difference being the techs the opening now applies for itself. **Performance**: parity (126: 2.24–2.26 vs 2.27–2.28 µs/tick; 2,126: 37.19–37.42 vs 37.08–37.19), with the long-standing 2% residual at 1,126 unchanged. **226 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.63 — **the civ tech tree, read as a restriction list (D23).** The dat gives each civilisation an effect whose commands are all kind 102 with an id and nothing else — no state field, so the file does not say permit or forbid. The data settled it: **no civ's list contains anything every civ has** (Loom, the three ages, Wheelbarrow, Forging appear in 0 of 32 lists, which a permission list could not survive), and **Britain's 31 entries are its real omissions** (428 → Hussar, 384/434 → Eagle line, 320 → Siege Onager, 265 → Paladin). So `civs.toml` gains a loader (`id`, `name`, `tech_tree_effect`, `team_bonus_effect`, plus the 211-number `resources` array carried uninterpreted), `[start] civ = "British"` names the civilisation by the table's own word, and `check_research` refuses a barred tech **before** the prerequisites — a tech you can never have should say so rather than ask for an age — with a refusal that names the civ ("not available to the British"). `World::load_civs` refuses a civ name the table lacks, `--civs` joins the headless runner, and the civ id is **hashed only when set**, so a world without a civ hashes exactly as before while a replay cannot lose the civ silently. Mutation-proved: disabling `barred_by` makes the test fail with the prerequisite message instead of the civ one. **Stability**: four-hour probe unchanged at `b30d49d89f8bcf4f`; the scripted run with `--civs` hashes `692abe64904f0474`, which is the civ being state. **223 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.62 — **the standard opening stockpile, and a scenario that finally demonstrates research.** `[start] stock = { food = 200, wood = 200, gold = 100, stone = 200 }` — resource **names** to amounts, resolved through the pack, applied once per world (`start_stock_applied` is hashed state, because it decides whether a later `load_resources` re-applies and wipes what was gathered). A name the pack has no column for is refused with the name in the message: a player who silently starts with no gold cannot pay for anything and has no way to know why. With it, `config/scenario.toml` researches **Loom at t=0** (50 of its 100 gold) and still affords its spearman, so the ten-minute script demonstrates the effect interpreter end to end instead of a queue. The churn is the interesting part — every test asserting "the player starts broke" was stating a *content* fact as an engine rule: three gather tests now count from their opening balance, the broke-bank tests ask for a pack with `start.stock` cleared, and the two own-pack tests failed informatively with `UnknownResourceName { name: "food" }` because a grain/iron/salt pack cannot open with 200 food. One test lost an assertion that measured wall-clock ticks between "arrival" (inferred from position) and the finish, which moved by one now that the villager no longer walks to a node first; it asserts the rule instead — 500 **builder-ticks**, the last of which finishes the house — and records why the old form was a fixture artefact. **Performance**: parity in the interleaved A/B in both orderings (126: 2.28–2.35 vs 2.34–2.38; 2,126: 36.35–37.57 vs 36.47–36.99 µs/tick), as a seed-time array assignment should be. **Stability**: four-hour probe still 126 entities, hash now `b30d49d89f8bcf4f` and scripted 600-tick `d5fe4af2413a0791` — both moved *because* the opening is state, which is the feature, not a drift. **216 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.61 — **a match opens with its starting techs, because without them research was impossible.** `[start] techs = ["dark_age"]` in `economy.toml` with `[tech_role] dark_age = 104`: every tech in the table lists 104 as a prerequisite, so a world that began with nothing researched could research **nothing at all** — the tests had been granting it by hand, which is the smell of a test doing what the game should. The engine grants them from **both** `seed_from` and `load_research`, because a headless runner and a scenario runner call those in opposite orders and the opening position must not depend on that; granting twice is harmless (a set insert, a zero health delta, and a kind-3 upgrade that finds no unit still carrying the old id). A `[start] techs` role the pack cannot name is refused at load, and the grants are refused as `MissingTech` when the loaded tables lack the tech — content and tables that disagree should say so rather than start a match that silently cannot research. Two purpose-built fixtures gained a 104 entry and failed informatively until they did. **Performance**: nothing in the tick path changed — interleaved A/B is parity at 126 entities (2.23–2.28 vs 2.37–2.53 µs/tick) and 1,126 (19.71–19.99 vs 19.38–20.00), and 2,126 reads 36.7–37.8 vs 34.8–36.1, the **same 2–3% residual v0.60 recorded as unexplained** on a box carrying load 4.8. Hashes: the probes that load no research are unchanged (`936e71d5a641044f`, `0f5eea624d75d87a`), and the scripted run **with** `--techs`/`--effects` hashes `db92f158160e7a5b` — the intended difference, an opening state that contains the Dark Age. **214 tests**, clippy and fmt clean. |
| 2026-10-27 | v0.60 — **tech-tree prerequisites, and the age-up defect they uncovered.** Every id in a tech's `required` is a **tech** id (all 614 entries resolve to techs, even where the number also names a building — tech 50 is Masonry, unit 50 is a farm), so `check_research` refuses a tech whose prerequisite this player has not researched: `MissingPrerequisite`, named with the pack's `[tech_role]` word where one exists. `World::grant_tech` completes a research instantly for a world that begins later in the game, and routes through `finish_research` so a granted tech applies exactly like a paid one. The gate was untestable without confronting what an age-up *is*: the Feudal Age effect carries **14 kind-3 commands that replace building ids** (town centre 109 → 71, barracks 12 → 498, blacksmith 103 → 105, house 70 → 463), and the engine applied them literally, so after an age-up **villagers could not deposit** (the drop list names 109), **Forging was refused at its own blacksmith** (105), and **`[train.entry]` stopped matching a barracks** (498). Three breakages from one mechanism, invisible until a test granted an age. The fix is a **family**: `ruage-tools` derives `variant_of` from the kind-3 pairs (190 units have a parent; the dat's `base_id` points at itself and is useless), `Units::root` walks the chain, and the four places that name a building — drop sites, research location, training, and the scenario's role search — compare families instead of ids. Mutation-proved by restoring `unit == wanted` in `is_family`, which fails `an_age_up_does_not_take_the_blacksmith_away`. **Performance, interleaved A/B against a `HEAD` worktree**: the first shape put `variant_of` on `Unit` and cost **4–6% at 1,126–2,126 entities with nothing new running per tick**; stripping the field from the data restored base numbers, which located the cost in the table's footprint (8 bytes × 1,166 records for a relation 190 have). Moving it to `Units.variants` narrowed the gap to **2–3%, unexplained, recorded as open** — and at 126 entities, a real match, the builds are indistinguishable. Stability: four-hour probe `936e71d5a641044f` and 600-tick scripted run `0f5eea624d75d87a`, both **unchanged**, because the gates fire on orders and every dark-age unit's root is itself. **210 tests** (25 in the importer), clippy and fmt clean. |
| 2026-10-27 | v0.59 — **the test debt from v0.58 is paid, and `[[train.entry]]` stopped quoting ids.** The v0.58 note said the build-site fix had no test because the seeded fixture refuses every build order (`NoPath`) and the scenario runner cannot afford Masonry. The fix was the content, not the claim: a purpose-built pair of tables (**a free, one-second tech that doubles building health**) removes the race against the pack's 50-second Masonry, and the scenario runner finds the legal build site the fixture would not. Two tests: `a_house_ordered_after_a_health_tech_is_quoted_the_new_maximum` (the order-time quote — it passed immediately, which is the missing proof for v0.58's untested fix) and `a_house_finished_after_a_health_tech_reaches_the_new_maximum`, which **failed at 550 first** and found the same defect one layer down: a site already going up when the tech lands stores the maximum it was quoted, and `finish_research` now updates `construction.max_hp`. Separately, `[[train.entry]]` names roles (`building = "town_centre"`, `units = ["villager"]`), resolved at load through `[role]`; an unknown role is refused by name and an integer is refused outright. **Performance measured properly this time**: a built worktree of `HEAD` run **interleaved** with the working tree, so the host's load cancels — 1126 and 2126 entities identical (19.3 vs 19.3, 36.9 vs 36.9 µs/tick), 126 entities inside the noise band (2.18–2.43 µs across four runs), which retires the v0.58 "3–8% slower" as the artifact it was. Hashes confirm it: four-hour probe `936e71d5a641044f` and the 600-tick scripted run `0f5eea624d75d87a` are both byte-identical to the previous pass, because nothing in this slice runs per tick. **205 tests**, clippy and fmt clean. |
| 2026-10-26 | v0.58 — **two defects found by reviewing v0.57 and fixed.** (1) **Speed and gather-rate additions were applied in the wrong unit.** The data states them in tiles/work per second (nine speed additions, two gather-rate), the engine holds them as fixed-point at `scale`, and the code added the raw value — so a +0.35 tiles/s modifier moved a unit by 0.35 scale units, which is indistinguishable from nothing. `scale_fixed` now scales an addition before adding it; `a_speed_addition_arrives_in_tiles_per_second_and_not_in_scale_units` pins it with the free instant tech that gives a 1.2-tile scout +0.35 (31000 at scale 20000, not 24000.35). (2) **A build site was quoted the table's health rather than the player's**, so a house ordered after Masonry finished at 550 instead of 605; `check_build` now derives from `max_hp`. That one is **fixed but untested** — the seeded fixture refuses every build order from where its villagers stand (`NoPath`), and the scenario runner cannot afford Masonry plus a house on the scripted income — and the same defect remains one layer down for a building already going up when the tech lands. Both recorded in `docs/next-step.md` as the next slice's work. **202 tests**, clippy and fmt clean. |
| 2026-10-26 | v0.57 — **hit points, speed, gather rate and carry: the interpreter now covers all four stateful attributes.** Two rules, both user decisions, and both written as tests: (1) **a research grants health and speed to units that already exist** — a villager at 25 becomes 40 when Loom lands, one wounded to 10 becomes 25 (the grant is health, not a reset), a house at 550 becomes 605 and one at 100 becomes 155; (2) **gather rate and carry capacity are resolved when an order lands**, so a load in hand keeps the numbers it started with and Wheelbarrow changes what the *next* trip carries — which is why no in-flight task needed re-pricing. The maximum is **derived, not stored** (`max_hp(player, unit)` = table value + the researching player's modifiers), so `Entity` did not grow again; the last slice measured what that costs. Speed is refreshed at completion, at spawn and on upgrade; a trained unit's health and speed are now read at spawn rather than when the order was placed, so a tech researched while a unit sits in the queue still counts. `[tech_role]` joins `[role]` in `economy.toml` (techs have no name in the dat — only string-ids into the game's language files), and the scenario format gained `kind = "research"`, with `a_scenario_researches_a_tech_named_by_a_role` as the end-to-end demonstration: three villagers gather gold, the bank pays 50 for Loom, every villager in the final state has 40 health. **Behaviour-preserving when nothing is researched**: HEAD and this tree produce the same 600-tick scripted run hash, `0f5eea624d75d87a`; the four-hour probe still ends at 126 entities with the *unchanged* hash `936e71d5a641044f`, which is the strongest form of "no new state". Performance: 126-entity match 2.36–2.38 µs/tick / 28.0–28.3 ms per ten minutes (unchanged); the larger sizes read 3–8% above the last pass, but the box carried a load average of 3.0 against 2.3 then, and none of the new work is per-tick (spawn, order, damage resolution), so it is reported rather than claimed as a regression. **201 tests**, clippy and fmt clean. |
| 2026-10-26 | v0.56 — **kinds 4 and 5 land for attack and armour, and the importer does the decoding.** `class * 256 + amount` is a fact about the dat, so `ruage-tools` now emits a `modifiers` list beside each effect's raw `commands` (`{ op = "add", attribute = "attack", attribute_class = 4, unit = -1, unit_class = 6, value = 1.0 }`), and the engine never sees 1025. Twelve commands carry a *negative* packed value (U17) and 22 kind-5 commands on attack carry numbers that are neither multipliers nor clean packed pairs, so those are emitted as `unpacked:8`/`unpacked:9` rather than given an invented sign rule. In the engine: `World::adjusted` filters a modifier by the unit id or class it names, applies **additions before multiplications** in researched-tech order (so the result cannot depend on research order), and `check_attack` resolves both sides through it — `damage_between` takes the effective lists, `damage_dealt` stays as the unit-level wrapper. A classless attack/armour modifier is skipped (a change to a class cannot name none). Checks against published numbers: **Forging** (blow 4 → 5), **Scale Mail** (blow 4 → 3, and the victim's owner is who researches it), a class filter that keeps Forging off a class-4 villager, and a modifier being the researching player's alone. Stability and performance pass: four hours, 126 entities, no leak, same seed → `936e71d5a641044f`; **a realistic 126-entity match is unchanged (27.9–28.8 ms per ten minutes), while 2,126 entities are ~5% slower** — and the cause is not the new per-tick research scan (merging it into the production scan changed nothing) but `Entity` growing 16 bytes for `Option<Research>`, i.e. fewer entities per cache line. Hit points (220 commands), range, gather rate, speed and carry are **not** yet interpreted; hp is state, so it needs a stored maximum per entity first. **195 tests**, clippy and fmt clean; importer at `b1bed3c`, 24 tests. |
| 2026-10-26 | v0.55 — **research exists, and kind 3 upgrades a unit the way the data says.** `Command::Research { building, tech }` refuses in a fixed order (not a building, not finished, already researching, unknown tech, already researched, wrong building per the tech's own `location`, missing effect, cannot afford) and pays only after every check; the timer mirrors production including the `fresh` rule, so a 40 s tech at 20 Hz lands exactly 800 ticks after it starts. Applying it: a per-player researched set (hashed) and the effect's **kind-3** commands — "every unit `a` becomes `b`" — applied as a health-fraction-preserving swap, with speed from the new unit; other kinds are deliberately untouched and the doc comment says so. The tech used is the dat's Man-at-Arms (222, effect 211, `74 → 75`, 100 food + 40 gold, 40 s), identified by cost and time; a test asserts its effect contains nothing but kind 3, so the slice cannot apply half a tech whose data grows. Prices are now one type for both tables (`crates/data/src/cost.rs`): `[1, 25]` or `{ resource = "timber", amount = 25 }`, so a tech can be priced in an original pack's own resources. `techs.toml` and `effects.toml` join the content pack — **eight files**, and the hash changes with them, because a peer with different techs is playing a different game. Known gaps, named not hidden: `required` (tech prerequisites) is loaded and not enforced, and a unit already in a production queue is not upgraded. Three of my own test bugs were found by running: a queued `Spawn` that the helper never stepped, a `place()` that hardcoded `player: 1` (so "the enemy is untouched" asserted 1 == 1), and a `bank()` that gathered from one node until it ran dry. A second adversarial pass then found a real defect: two buildings could research the same tech at once, charging twice — now refused (`TechInProgress`). While measuring the data for that comment: **458 techs carry `location = -1` and 136 carry `0`**, so "no home building" is the majority reading and, for those, treating `location <= 0` as "any building" is an unverified reading rather than a fact (the 13 barracks techs are explicit, and are what the slice depends on). **189 tests**, clippy and fmt clean. |
| 2026-10-26 | v0.54 — **content names units by role, and the stability/performance pass says the tick is untouched.** The last D17 leak in behaviour code: `script.rs` matched `"barracks" => 12`, `free_tile_near` looked for `unit == 109`, and `config/scenario.toml` wrote `unit = 70` / `attacker = 74`. There is now one **`[role]` table** in `economy.toml` (town_centre 109, villager 83, house 70, barracks 12, spearman 74), and three consumers: `[start]` names roles so seeding never learns an id, the scenario format names roles, and the engine's own lookups (`start.town_centre`, `start.villager`) go through it. `SCENARIO_SCHEMA` is **2**. Load refusals: empty role table, a role mapped to unit 0, a `[start]` naming an undefined role; run-time refusal: a step naming an unknown role or resource, with the name in the message. Adversarial review of the deletion, on **stability and performance**: the four-hour probe still ends with 126 entities and reproduces `hash=508a746d89cc76e1`; the perf probe reproduces the previous numbers within noise (2.26–2.36 µs/tick at 126 entities, 32.99–33.44 at 2,126, flat ns/entity, 27–28 ms for a ten-minute match), so replacing the enum cost nothing measurable. Two findings fixed test-first: `free_tile_near` still knew the *word* `town_centre` (found by renaming the role in a test — the engine must not know a content word either), and a slice edit over `config/economy.toml` had eaten both `[[train.entry]]` blocks (the loader's `missing field 'train'` is what said so). One measurement pitfall recorded: the first perf run showed a *rising* ns/entity — the exact signature of the old O(n²) — and it was a background release build on a two-core container, not the code. **181 tests**, clippy and fmt clean. |
| 2026-10-26 | v0.53 — **the importer leaves, and the engine's history comes with it.** D4 said publish early with the generator split first; D12 said the split is mechanical. Both are now true: `tools/` moved to its own repository (`Rodelero1/ruage-tools`, GPL-3.0-or-later) carrying every commit that touched it, and this repository's history was **rewritten** so the GPL parser never appears in it (`git log --all -- tools/` is empty) — because deleting a directory on top of history would still publish it. The tooling side needed three changes the split forced: `tests/test_config_consistency.py` and `tests/test_integration.py` resolve `RUAGE_CONFIG`/`RUAGE_DAT` instead of walking to a sibling `config/` (and skip with the variable's name when unset), `pyproject.toml` declares GPL-3.0-or-later and the distribution name `ruage-tools`, and a README states the boundary. This side gains `README.md`, `LICENSE-MIT`, `LICENSE-APACHE` and the recipe in `docs/config.md` now clones the importer rather than running `cd tools`. Both repositories are **private** at the time of writing: the split is the prerequisite, the visibility flip is the owner's. **175 tests**, unchanged. |
| 2026-10-26 | v0.52 — **the resource kinds are content, and `ResourceKind` is gone.** Three commits pay down D17's most-cited leak. (1) The four cost sites (train/build, check/apply) resolve a cost's resource through the pack, and two content mistakes are refused rather than guessed: an id the pack does not carry (`UnknownResource` — the old code *skipped* such a cost, so a pack that could not price a house made it free) and an id in a column past the stockpile (`ResourceSlotOutOfRange`). (2) The stockpile's width becomes the engine's (`MAX_RESOURCES = 8`, because a row is an array) and the *pack's count* is checked at load (`TooManyResources`), which retires that range check entirely. (3) `ResourceKind`, `index()`, `dat_id()`, `from_dat_id()`, `COUNT` and the bridge test are deleted: a map node carries the pack's **column** (`map.toml` keeps readable names, `load_map_str` takes the pack and refuses a name it does not define), where a node may stand is content (`site = "forest"` on wood, open ground by default, both directions enforced), `[gather.resource_task]` is keyed by resource **name**, a scenario step names a resource and an unknown one is reported not skipped, `World::stock`/`slot_of`/`resources()` speak columns, and a cost may name its resource (`{ resource = "timber", amount = 25 }`) beside the reference form (`[1, 25]`), untagged so the 574 KB generated table is byte-identical. **Finish line, `a_content_pack_with_our_own_resources_loads_and_spends`:** a pack of five resources with timber in slot 4, a map whose wood is called timber, a three-unit table whose house costs `{ resource = "timber", amount = 25 }` — gather 25 timber into the fifth column, order the house, watch it finish at 550 hp, with grain (slot 0) asserted to stay 0 the whole way, so a fallback-to-column-zero pays in the wrong goods and the test says so. Two stale tests were rewritten rather than deleted (the economy join is keyed by name now; unit costs are `Cost`/`CostResource`, not a `(u8, u16)` pair). **175 tests**, clippy `--all-targets` and fmt clean, tools suite 22 passed. |
| 2026-10-25 | v0.51 — **eight sides, and the repository goes public soon.** D20: stockpiles are `[..; MAX_PLAYERS]` with `MAX_PLAYERS = 8` (4v4), not two; `Command::Spawn` refuses a ninth side in both the dry run and the apply, so the failure is `PlayerOutOfRange` rather than a panic on the array, and player 0 stays neutral because resource nodes live there. The movement hook `spawn_now` uses player 0, which is how the first version of the check broke six tests — the bound is a ceiling, not a range. D4 resolved: publish early, **generator split out first** (D12), because while `tools/genconfig` imports GPL-3.0 `GenieDatParser` in the same repository, publishing publishes its obligations too. Map identity stays inside the pack until map variety arrives, then becomes a per-match field. **161 tests.** |
| 2026-10-25 | v0.50 — **a match's content becomes one identified thing.** `ContentPack::load` reads all six files a match depends on and produces a **content hash** (FNV-1a over each file's bytes mixed with its name, fixed order). The hash is over **bytes, not parsed values**: it is what "same data" actually means, no ignored field can slip past, and it needs no second serialisation to keep in step. Cost accepted: a comment edit changes it, so a modded pack is republished rather than tweaked. `differences()` names the disagreeing file so a handshake reports which one; the identity goes into the match handshake and replay header beside `tick_rate_hz` and `fixed_point_scale` (D16), and a mismatch refuses the match. Not wired to networking yet — deliberately: the identity exists so the first netcode cannot be written without it. Also `pack_path` records that `units.toml` lives under `gamedata/` because it is generated (D8) while the rest are hand-written; a pack is one thing to a handshake regardless of the directory layout. **160 tests.** |
| 2026-10-25 | v0.49 — **the resource list becomes content, with a tested bridge to the enum it will replace.** `config/resources.toml` defines resources as `slot` / `name` / `dat_id`: the slot is the stockpile column the engine indexes, the name is what configs and scenarios use, and `dat_id` is the *reference pack's* numbering kept only for the bridge. The loader refuses a gap in the slots (a column nothing can address), a duplicate name (a lookup that silently takes the first), an empty name, an empty list, and one reference id claimed twice. `ResourceKind` is still a four-variant enum with `[[u32; COUNT]; 2]` stockpiles, so **the enum is still the pack** — `the_content_file_agrees_with_resource_kind` asserts the two agree on name, slot *and* dat id, so drift is a named failure rather than a silent mismatch. When the enum goes, that test goes with it. **154 tests.** |
| 2026-10-25 | v0.48 — **the engine is for a game, not for Age of Empires II.** D17 fixes four layers: **engine** (knows kinds, never particular ones), **content pack** (our schema, carrying the dat's values today), **importer** (all the AoE2 knowledge, in its own repo per D12), **asset pack** (entity + state → sprite reference). The rule: any id or number that means something only because of AoE2 stays outside `crates/`. D18 fixes the target: **A now, B-capable** — original graphics, tech trees and stats in use, own content scheduled but not blocked, divergent rules carried by the same schema if they ever happen. Graphics readiness turns out to be a *data* question first: the dat has a sprite table (`slp_id`, `layer`, `force_player_color`, `color_table`, `num_deltas`, `bounding_box`, `sound_id`), so the entity→sprite binding is generatable rather than hand-authored, and the sprite geometry the simulation needs (radius) is already in the unit table — swapping art cannot change gameplay. No renderer yet, deliberately. Leaks named and not yet paid down: `ResourceKind`'s fixed enum with `[[u32; COUNT]; 2]` stockpiles, `script.rs` naming dat ids, and dat semantics inside the data crate. **147 tests** (no behaviour change in this entry). |
| 2026-10-25 | v0.47 — **kinds 4 and 5 are pinned, by techs whose results are published rather than by lore.** The quintuple is `[kind, a, b, c, value]`: kind **4 adds**, kind **5 multiplies**, `a` is a unit (−1 = none), `b` is a unit **class**, `c` is the attribute (**0 hp, 1 range, 8 armour, 9 attack**). Loom (`[4,-1,4,0,15]`) is +15 hp to class 4 civilians; Masonry (`[5,-1,3,0,1.1]`) is buildings ×1.1 hp — *multiply*, or Masonry would add 1.1 hit points and do nothing, so the value forces the reading; Forging (`[4,-1,6,9,1025]`) is infantry +1 attack. **Attack is packed `class * 256 + amount`**: 1025 = 0x401 = melee class 4 + 1, 769 = 0x301 = pierce class 3 + 1 — and those class ids agree with the ones `combat.md` pinned independently from knight and archer stats, two roads to the same numbers. One gap found and left open rather than guessed: **six commands pack a negative attack** (`-136` ×3 in `Non-Research`, `-226` ×3 in `Shinkichon`), which an unsigned packing cannot express — registered as **U17**, named individually in the test. **147 tests.** |
| 2026-10-25 | v0.46 — **techs and effects load, and the census says what an interpreter must learn.** 801 techs, 798 effects, **3,306 commands** in thirteen kinds; `effect_id = -1` means "applies nothing" and is `Option<u32>`, not an error. The census is the to-do list: kind 4 (1,091) and 5 (576) are half the commands, kind 102 (942) is what a civ's tech tree is. **Kind 102 is pinned by the data**: its value resolves to a tech id in 941 of 942 commands, the exception being a `[102,-1,-1,-1,-1]` no-op in the Koreans tree that the test names. **Kind 101 is the interesting failure**: expected to be "enable/disable a unit", 13 of its 97 commands name ids that are not units — they are techs (202 double-bit axe, 203 bow saw) inside "free upgrade" bonuses — and checking both tables shows **0 unit-only, 84 in both, 13 tech-only**, so unit and tech ids share a range and an id cannot say which table it means. A `UnitTouch` reading would have been right 87% of the time and would have turned a free upgrade into a free villager, so the type now carries `AmbiguousId { id }` and the finding is **U16**. New `docs/mechanics/effects.md`. **142 tests.** |
| 2026-10-25 | v0.45 — **step 5 closes: a scripted ten-minute match runs end to end.** `config/scenario.toml` is the scripted opponent and the first form of the replay format: timed `Command`s in **seconds** (D16), naming roles rather than ids — "the first villager", "the second-nearest tree", "my finished barracks", "the newest spearman", "the nearest enemy". A step that cannot run is **reported**, never skipped: `step at 471.0s refused: not enough Food: 60 needed, 50 in the bank`. `ruage-headless` gained `--script`, `--combat` and `--report`, so the run prints its economy as it goes. The schedule is **measured, not derived**: three villagers bank ~0.75 wood/s once the walk home is counted, not the 1.17 the dat rates suggest, and one forager banks ~0.25 food/s — a schedule written from the dat rates cascades into refusals, which is how the real numbers were found. Two limits shaped the script and are recorded as limits: a villager **idles when its own node runs dry** (so the script re-tasks at 200 s), and a node inside a forest cluster is unreachable (the step was dropped rather than the check weakened). The determinism test compares the script against a shortened script, because the seed changes nothing here — nothing draws from the RNG streams and the map is authored. **133 tests.** |
| 2026-10-25 | v0.44 — **combat: the counter web falls out of the data.** `damage = max(minimum, Σ max(0, attack_i − armour_i))` per class, with `combat.toml` supplying the minimum and the missing-class sentinel; elevation loads but is unused because the slice has no terrain heights. A spearman does 4 to a villager, 3 comes back, 4 to a house and 1 to a town centre — the house number is the instructive one, since its 7 armour is pierce (class 3), not melee. `reload` becomes ticks, and a spearman's 2.0 s is **40** ticks, not 41: the countdown is decremented before the check. A spearman needs seven hits, so a villager dies 240 ticks (12.0 s) after the first blow, asserted to the tick. Review found two things: the death sweep ran before the position update, panicking with "entity list is stable within a tick" (now at the end of the tick), and — much worse — **`Task::Idle` moved entities by their `velocity`, and seeding set velocity to the unit's speed, so every idle villager marched east forever**. That had been true since seeding landed and no test noticed, because every test gave its villagers a task immediately. `Entity.speed` (capability) is now separate from `Entity.velocity` (motion); `an_idle_villager_stands_still` failed with entity 2 eight tiles away. **127 tests.** |
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
