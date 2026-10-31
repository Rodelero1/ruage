# Review: the RE toolchain — REA, Ghidra, and what it cost

*Change: REA 6.8.0 + Ghidra 12.1.4 + Temurin 25 + Node 26.13 installed under `/opt/data/tools`; REA registered with
Hermes as a lazy MCP server (229 tools); `docs/re-toolchain.md`, D86, a `sources.md` row block and a `next-step.md`
compact. Nothing under `crates/`. Two docs commits.*

## Performance

Measured on this box (4 GiB / 2 CPU cgroup, load average 3.7–4.7 because a second session was running `cargo test`):

| Operation | Measured | Cost of getting it wrong |
|---|---|---|
| Ghidra headless, full default analysis of the 7,521,280-byte i386 PE, `-max-cpu 2`, `-Xmx2G` | **6 min 30 s wall / 8 min 13 s CPU**, peak RSS **1.42 GB**, exit 0 | — |
| REA's first query with its shipped default | **`provider_timeout` at 330 s**, analysis discarded | one full analysis lost per attempt |
| One REA query once analysed (fresh import) | **7–8 min** (`inspect` 7m51s, the function query 5m57s) | the reason the persistent-project route exists |
| MCP schema cost avoided by `lazy: true` | 229 tool schemas not injected into every session | every session pays them permanently otherwise |

The design conclusion this buys: **REA is the evidence instrument, not the iteration loop.** Ten questions at 8 min each
is an afternoon; the same ten against a persistent `AoKHD.gpr` (built once, 6.5 min) is minutes. That split is written
into the skill so the next session does not re-derive it.

## Stability

Attacked, and each attack found something real:

- **Timeout destroys work silently-ish.** The first run wrote 22,806 bytes of JSON that *looked* like a result. It was
  an error envelope (`error: Analysis failed`, `category: timeout`). I misread it as a completed inventory before
  opening it. Rule now written down: check `error`/`code` before treating any REA output as data.
- **Memory.** Peak 1.42 GB inside a 4 GiB cgroup with the gateway resident. No `OutOfMemoryError` was reported, and the
  failure path says so explicitly rather than guessing. `GHIDRA_HEADLESS_MAXMEM=2G` is the ceiling used; 3G was
  rejected as too close to the cgroup limit with another session's cargo build resident.
- **Project reuse is not what the flag name suggests.** `REA_GHIDRA_PROJECT_PATH` was set to a project I built, and REA
  still re-imported: its real invocation is `analyzeHeadless … -import <scratch>/target-<sha>.exe -readOnly
  -deleteProject`. The ephemeral project is deleted on every close by design, so no cache accumulates in `cache/scratch`.
- **Self-inflicted kill.** `pkill -f rea.mjs` matched the wrapping shell's own command line and SIGTERM'd the job
  controller. Recovered by killing PIDs; the rule is in the skill.
- **Secrets/config.** No credential touched. The MCP entry carries only paths (`JAVA_HOME`, `GHIDRA_INSTALL_DIR`, `PATH`),
  written through `hermes mcp add` / `hermes config set`, never by hand-editing `config.yaml`.
- **The game install stayed read-only.** REA copies the target into its own scratch directory (`target-cbd10d81b936.exe`,
  sha256 `cbd10d81…4650`); nothing was written under `Age2HD/`.

## Errors I made in this step, for the record

Three, all reported rather than buried: (1) installed Ghidra 12.2, which REA rejects, and JDK 21, which its JDK check
rejects, on the first attempt; (2) extracted Ghidra with `python -m zipfile`, losing the exec bit on `analyzeHeadless`
and the native decompiler, which `rea doctor` correctly called "missing"; (3) read a 22 KB failure envelope as a
successful inventory.
