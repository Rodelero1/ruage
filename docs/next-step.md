# OPEN - D200 is on `main` with its crowd fix; next is register row 2, damage

**State, 2026-11-13 (second session).** `main` carries **D200** - friends are solid, a tile seats seven at the measured
0.66, a body at rest is nudged only after forty refused ticks, a place a body stands on is stale, and **a body on the
mover's own tile can now be walked around sideways**. The last one is what made the difference: without it, seven villagers
stood on two tiles in `WaitOwnTile` for the whole match and the acceptance bank held **0 food from 400 s to 2,400 s**.

**Measured, `--seed 7 --ticks 48000 --report 400`:** food `4 / 14 / 484 / 594 / 1226 / 1396`, wood `40 / 75 / 300 / 415 /
805 / 1215`, units `20 / 29 / 33 / 37 / 37 / 37`; summary `entities=143 players=2 hash=2d93236ebb16961b repaths=373
made_room=140 repaths_body=77 repaths_ground=296`. Against `main` before D200: food `56 / 397 / 667 / 388 / 650 / 690`, wood
2019, units 43, `entities=128 players=1 hash=ace125e60ea99b60 repaths=579 made_room=0`. The branch farms **more** food and
holds less wood - a different opening, not a stalled one. Gate: 184 binaries, 841 passed, 0 failed, 21 ignored, `EXIT 0`,
fmt and clippy clean.

**The one cost, stated plainly.** The enemy is **not eliminated by 40:00**: its town centre is at **1451 of 2400 hp** with
four spearmen in `Attack` on it, where `main` had already razed it with eleven soldiers. `players` is the highest surviving
player id, so `players=2` means "the enemy still stands", not "the match is broken". The record, with every withdrawn rule
and its measurement: `docs/reviews/2026-11-13-owner-blind-collisions.md`; the revert and what it taught:
`docs/reviews/2026-11-13-d200-reverted-food-stall.md`.

## The plan, in order. Do not start a step while the previous one is red.

### 1. Register row 2 - damage *(next)*

`0x744d60` has five call sites (`0x6bb100`, `0x741a30` x2, `0x744ff0`, `0x745060`); `docs/behaviour-diff.md` records what
each reads (multiplier 1.0 everywhere, the wrapper sums) and that the per-entry drop stands. Open: which call site is
combat's path, and whether the game drops each sub-1 class entry or floors the total (`combat.toml`'s `minimum = 1`).
Two seconds a query with craft-anything, then reading the owned function.
**Done when:** row 2 is ruled `same` or `different` with the address, and a `different` becomes a test-first fix.

### 2. `World::from_state` and the tool-side converter (B shape) - `--load`

A versioned world description the engine builds from, produced by a GPL tool now and by a reader inside the engine later. The
18 saves in the install become a differential oracle: run 1,200 ticks from `brit7`, compare with `brit8`.
**Done when:** `ruage-headless --load <save>` runs and the converter's output describes a *world*, not a transcript.

### 3. The importer into `tools/` and a bootstrap step

One repository, GPL-3.0 directory with its own licence file, invoked once at install time.
**Done when:** a clean clone with only `Age2HD/` present builds and produces `config/gamedata/`.

### 4. Content (D201): names by three lists, prose rewritten, campaigns ours only

### 5. Publish cleanup: quote-and-cite the five docs, licence plumbing, CI gates, README

### 6. The gather capacity, restated as a question the saves can answer

The game shows seven to eight bodies at **0.66** on a node's touching envelope; our ring hands out places at 0.66 first and
0.40 when the ring refuses. Both are now exercised by the acceptance run, so the difference is visible; what is not settled
is whether a **ninth** man should stand outside and wait (the crowd policy this work shipped) or walk to another node.

## Needs the owner - and only this

- the **A vs B** publishing answer (does `--load` live in a published engine, and is the engine public yet) -
  `docs/licensing.md`, "B now, A later";
- **whether "the enemy is eliminated inside 40 minutes" is a requirement.** With friends solid it is now a pacing question,
  not a stall: the town centre is at 60% with four attackers on it at 40:00. Nothing in the suite depends on the answer.

**Decided elsewhere, not on this plan:** pacing/AI (owner: later, with the AI), the renderer (D18), the `Vision` split (no
behavioural content - last on purpose).

## Standing rules for this plan

Test-first, with the red run shown. Measure before and after. Every step ends committed and pushed. A step that cannot go
green is parked on a branch with its diagnosis, **never left red on `main`**.

## Instrument notes, so a fresh session does not re-derive them

The acceptance command that matters (release, 40 minutes of game time):

```
./target/release/ruage-headless --config config/sim.toml --map config/map.toml --economy config/economy.toml \
  --units config/gamedata/units.toml --resources config/resources.toml --combat config/combat.toml \
  --techs config/gamedata/techs.toml --effects config/gamedata/effects.toml --civs config/gamedata/civs.toml \
  --script config/acceptance.toml --seed 7 --ticks 48000 --report 400 --dump build/dump.csv
```

`--dump` writes `kind,player,id,unit,slot,x,y,amount,hp,carry,path,task`. **`world.trace_steps()` is the instrument for a
stuck body, not `Entity::last_step`**: that field is only recorded when tracing is on, so reading it on a normal run returns
the value from seeding and reads as `Idle`. `crates/sim/tests/acceptance_economy.rs` runs the acceptance world itself and is
the guard that would have caught this regression; keep it green. Probes on a `Task` arm are env-gated and removed in the
same session - `no_float` catches a leftover `as f64` in production code.

craft-anything's CLI is `/opt/data/tools/craft-anything/bin/craft-anything-cli`; the MCP entry is registered **deep**
(`mcp --all-tools --deep`, 323 tools, `connect_timeout` 180) and its `PATH` must carry the JDK 21 `bin` and both Ghidra 12.2
directories. **Every identification number in the evaluation came from its built-in engine**, which needs no Ghidra; a
Ghidra-backed call pays ~6.5 min and 1.4 GB. Its function count is **19,484** where Ghidra's is **15,465** - never quote one
without its tool. The REA corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 functions) and is the *reading* instrument.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…`
(tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
