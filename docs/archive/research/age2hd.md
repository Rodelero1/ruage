# AoE2 HD → AI-assisted recreation — handoff notes

Written 2026-10-23 (session started 2026-10-22). Purpose: hand off all state to a new session.

## Project goal

User owns Age of Empires II: HD Edition (Steam, legally purchased). Long-term plan
agreed across the session: reverse-engineer and rebuild the game with AI coding agents,
using the **original-assets route** (engine reads the user's own copy of the game files,
like openage — assets are never redistributed). User wants to continue in a new session
with a stronger model.

## Decisions made (with rationale)

- **Route:** engine reimplementation + behavior spec from the binary, loading original
  HD assets. Chosen because it sidesteps the unsolvable asset problem (frame-consistent
  isometric sprite generation) and the legal wall.
- **Language: Rust recommended and user-obtained agreement leaning this way.**
  Rationale: deterministic lockstep sim (bit-exact multiplayer/replays), rustc filters
  agent-written bugs at compile time, headless `cargo test` enables cheap AI-vs-AI
  self-play balancing (Battle Drones proved this pattern with 1,200 simulated matches).
  Suggested stack: pure-Rust sim crate (deterministic, no rendering deps) + Bevy/wgpu
  rendering + Rhai or Lua (mlua) for mod scripts and the legacy `.per` AI DSL interpreter
  + Python converters (openage-style) emitting JSON via serde.
- **Prior art / references found by research (Oct 2026):**
  - `Age of AI` — github.com/alexvilelabah/age-of-ai (MIT), playageofai.com. Browser
    multiplayer AoE-like made with AI coding agents; author calls it a "functional
    draft". Proof the core loop is achievable in months.
  - `Battle Drones` — github.com/simonw/battledrones (Apache-2.0, Oct 20 2026).
    Simon Willison's C&C-style vibe-coded RTS; 321 agent commits, self-play balancing,
    online 1v1. Proof of agent-built RTS at playable quality.
  - `REA` — github.com/morluto/rea (MIT, ~278k stars, created 2026-04). MCP server
    wiring agents into real RE tooling (Ghidra/Hopper/IDA: pseudocode, symbols, call
    refs; also JS/Electron, .NET, WASM, HAR captures). Evidence-based results. This is
    the chosen tool for Phase 2 (behavior analysis of the exe). Runs via
    `npx rea-agents setup` (supports Claude Code, Codex, Cursor, Gemini CLI) or CLI
    (`rea analyze...`). Needs Node ≥22 (container has 26.7 ✓) and a procps `ps` on PATH
    on Linux. Native analysis needs Ghidra/Hopper/IDA installed.
  - Long-standing OSS clones (not AI-built): openage (14.5k★, C++/Python, 13 yrs,
    unfinished — cautionary reference for difficulty; its `convert/` code documents
    Genie formats), freeage (AoE2 **DE** reimpl, C++), Chariot (AoE1, Rust+SDL2,
    stalled), 0 A.D.
- **Difficulty conclusion:** core loop = months (proven by Age of AI); faithful HD =
  multi-year community project historically, but REA compresses the reverse-engineering
  bottleneck (the worst part) to weeks. Remaining hard parts: deterministic lockstep at
  8 players, content scale (27 civs w/ expansions), balance long tail (20+ yrs of
  patching), 25-year-old engine behavior.

## Environment facts (IMPORTANT — this container)

- Hermes runs as a **rootless podman quadlet**: `hermes.container` is in the workspace
  root. Image `docker.io/nousresearch/hermes-agent:stable`, Exec `gateway run`,
  Memory=4G, CPUs=2.0, Network=podman (IP 10.88.0.202), ports 9119 (dashboard) + 8642.
  Host is 192.168.60.139.
- Bind mount: host `%h/apps/hermes` ↔ container `/opt/data` (`:Z`). **Files the user
  drops into workspace arrive owned by `nobody:nogroup`** (host-UID mismatch) —
  readable but possibly not writable; if a write fails, `cp` the file to a hermes-owned
  dir first.
- **No sudo / no root** (sudo is not installed). User-space installs only: portable
  tarballs into `/opt/data/tools`, `npm --prefix`/npx, pip. apt is NOT available.
- OS Debian 13 (trixie). Present: Node 26.7.0, npm 12.0.2, Python 3.14.7, objdump,
  git. **Missing:** java, Ghidra, `file`, 7z, pwntools, Wine (no Wine ⇒ the exe can be
  analyzed statically but never *run* here; dynamic/empirical checks would need the
  user's gaming PC or a Wine attempt), Steam.
- Disk: 868G free on /opt/data. RAM cap 4G — when running Ghidra headless use
  `-Xmx2G` or less or the OOM killer will visit.
- Outbound internet works (web_search/web_extract/npm/downloads verified during
  session). web_extract saves big pages under /opt/data/cache/web/.
- Container restarts (`Restart=always`) — persist all state to /opt/data, never rely
  on in-memory kernel state between sessions (execute_code kernel state does not
  survive).

## Game data (verified in workspace)

Location: `/opt/data/workspace/Age2HD/` — 2.6 GB, complete Steam install of AoE2 HD
(app id 221380; `221380_install.vdf` depot manifest at root). All files
`nobody:nogroup`, readable.

- Main binary: `AoK HD.exe` (7.5 MB). Verified PE32, machine 0x14c (x86), 6 sections,
  link timestamp 1534962379 (2018-08-27).
  sha256 `cbd10d81b93601ffb26773d250b7478969da92d70792a40ae423294202e14650`
- `Launcher.exe` also present (skip for analysis), plus steam_api.dll and Twitch-era
  dlls (libcurl-ttv etc. — irrelevant).
- **Game data files (the crown jewels)** in `resources/_common/dat/`:
  - `empires2_x1_p1.dat` — main game data (units/buildings/techs/civs/effects)
    sha256 `28b50786d62cc956679bf56b42b272ec04eb93642268e997484dbc63bb24e6cb`
  - `empires2_x2_p1.dat` — HD expansions data
    sha256 `21591ac67251d8674635d5634f2d8e9ff80ad90f3c792f9503d60dd658d61058`
  - helpers: blendomatic_x1.dat, FilterMaps/lightMaps/LoQMaps/PatternMasks/STemplet.dat,
    TileEdge/BlkEdge.Dat, view_icm.dat
- `resources/_common/drs/`: gamedata_x1, gamedata_x2, graphics, interface,
  retail-campaigns, sounds (DRS archives — Genie container format, community-documented).
- Loose trees under `resources/_common/`: `slp/` (loose sprites; 3,623 `.slp` files
  total in install), `terrain/` (blends/textures/water), `sound/` (2,246 wav),
  1,564 mp3, fonts, shaders, strings, credits, movies.
- Scripts: 122 `.rms` random-map scripts, 74 `.per` + 30 `.per2` AI scripts, 49 `.scx`
  scenarios, SaveGame/ dir. `resources/en/` = 865M language pack; `_packages/` 224M.
- Docs/ has official PDFs/RTFs.

## Progress so far

1. Researched prior art + REA; decisions recorded above.
2. Located and inventoried the full install in workspace (top level + resources tree).
3. Parsed PE header of `AoK HD.exe` (valid x86 PE) and recorded SHA256 of exe + both
   `.dat` files (hashes above — provenance anchors; re-verify if anything looks off).
4. Toolchain install attempt failed (no sudo) → switched plan to user-space portables.
   **No RE tooling is installed yet.**

## Next steps (suggested order)

1. **Tooling (user-space):** download Temurin JDK 21 tarball + latest Ghidra release zip
   (github.com/NationalSecurityAgency/ghidra/releases) into `/opt/data/tools`; confirm
   `java -version`; `npm install -g rea-agents` may fail without root for the global
   prefix — use `npm config set prefix ~/.local` or just `npx rea-agents@latest`. Also
   `pip install py7zr` (pip may need `--break-system-packages` on Debian 13 / or use
   a venv under /opt/data).
2. **Phase 1 — data extraction:** parse `empires2_x1_p1.dat` + `empires2_x2_p1.dat`
   → JSON. Reference implementations: openage's `convert/` (Python) and community Genie
   format docs; note HD's `p1` dat variant differs from AoK's vanilla dat — openage
   handles AoE2/HD variants, freeage targets DE. Then DRS archives → loose files, SLP →
   PNG frames (SLP format is documented; openage convert / reimplement in Python).
   Deliverable: `converted/` JSON + PNG tree the future engine loads.
3. **Phase 2 — behavior spec via REA + Ghidra headless on `AoK HD.exe`:** target the
   undocumented rules — combat damage resolution, gather rates, pathfinding, lockstep
   sync/RNG, random-map gen. Output: written spec with evidence citations (function
   addresses/pseudocode). Keep `-Xmx2G` for headless analysis (4G container RAM).
4. **Phase 3 — engine build:** Rust per the stack decision, loading Phase 1 output +
   original assets from the user's copy only.
5. Wire REA's MCP into whatever agent is used in the new session (`npx rea-agents setup`
   lists supported agents; for Hermes itself, CLI mode `rea --help` works regardless).

## Guardrails to keep respecting

- User owns the game: analysis and personal-use engine OK. **Never copy/publish game
  assets** — ship only engine + converters + instructions pointing at the user's own
  install.
- Nobody-nogroup uploads: read fine, write carefully (cp first if needed).
- RAM/CPU cap (4G/2 CPU): batch heavy work, persist intermediates under /opt/data.

## Quick pointers for the next session

- Game install: `/opt/data/workspace/Age2HD/`
- This handoff: `/opt/data/workspace/age2hd.md`
- Scratch: `/opt/data/cache/scratch` (pruned after 24h idle — don't keep anything
  valuable there; use `/opt/data/workspace/` or a new `/opt/data/tools/`)
- Suggested first command chain: `mkdir -p /opt/data/tools && cd /opt/data/tools` then
  fetch JDK + Ghidra, then `npx rea-agents@latest setup` (or CLI-only), then Phase 1.
