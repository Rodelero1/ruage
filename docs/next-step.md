# OPEN - D204 is on `main`; next is the carrier half of it, then register row 2

**State, 2026-11-14.** **D204** is built and green: a man who cannot get his square at a node **takes the next tile of the
same resource column**, and no body with a gather order is displaced by anybody. `push_the_standing` still nudges a body
with **no work at all** - a bystander in a doorway, a soldier in a corridor - which is the decision's own scope.

**Measured, `--seed 7 --ticks 48000 --report 400`:** food `45 / 49 / 268 / 120 / 660 / 450`, wood `80 / 96 / 135 / 75 /
395 / 185`, units `20 / 31 / 35 / 44 / 46 / 48`; summary `entities=160 players=2 hash=c3b10f4b09e17a43 repaths=324
made_room=147 repaths_body=83 repaths_ground=241`. Against `main` before D204 (`7d00be8`): food `4 / 14 / 484 / 594 / 1226
/ 1396`, wood 1215, units 37, `entities=143 players=2 hash=2d93236ebb16961b repaths=373 made_room=140`. **Eleven more
units and a smaller bank** - the match spends rather than banks - and the early game is the improvement: **45 food at 400 s
against 4**. Wall time 61.3 s -> 30.3 s; the like-for-like tick `7.4-8.4 us -> 6.5-6.8 us`. Gate: 185 binaries, 843 passed,
0 failed, 21 ignored, fmt and clippy clean.

**What the nudge was doing, measured before the change:** 310 displacements over the acceptance run - **232 builders, 76
loaded carriers, 2 idle, 0 gatherers**. The resource case was never a shove: it was **45,359 refusals of a square**, over 18
distinct villagers, one refused on **1,782 consecutive ticks**. Both halves are in
`crates/sim/tests/no_pushing.rs`; the record, with the gate that was written first and withdrawn, is
`docs/reviews/2026-11-14-d204-no-pushing.md`.

**The one cost, stated plainly.** The enemy is **not** eliminated by 40:00 (its town centre stands and `players=2` means
"the enemy still stands"). That is unchanged from the D200 merge and is a pacing question, not a stall.

## The plan, in order. Do not start a step while the previous one is red.

### 1. The carrier half of D204 *(next, test-first)*

A body with a **gather** order is never moved now; a **loaded carrier** still is - **76 of the 310** displacements the
acceptance run measured on `main` were men walking a load home (`Task::Return`), which is the other half of the owner's
*"villagers pushing other villagers"*. Extend the gate in `push_the_standing` to `Task::Gather | Task::Return`, and **write
the fixture first**: a loaded carrier shoved off his way home, watched red. The pieces to stage it: a bush and a town
centre, a villager ordered to gather until he is in `Return` with a load, then a stalled man whose next waypoint is the
carrier's tile.

### 2. Register row 2 - damage *(after that)*

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
is whether a **ninth** man should stand outside and wait (the crowd policy this work shipped) or walk to another node - and
D204 has now made "walk to another node" the engine's answer wherever the man was refused a square, which may be what the
saves were showing all along.

## Needs the owner - and only this

- the **A vs B** publishing answer (does `--load` live in a published engine, and is the engine public yet) -
  `docs/licensing.md`, "B now, A later";
- **whether "the enemy is eliminated inside 40 minutes" is a requirement.** With friends solid it is a pacing question,
  not a stall. Nothing in the suite depends on the answer.

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
without its tool. The REA corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 functions) and it is the *reading* instrument.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…`
(tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
