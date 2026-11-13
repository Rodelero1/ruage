# OPEN - the Return that aims at a blocked anchor: fix it, then re-land D200

**State, 2026-11-13 (second session of the day).** `main` is the pre-D200 behaviour, restored by `git revert -m 1 2c7d8e1`:
the whole workspace gate is green and the acceptance match is back to `entities=128 players=1 hash=ace125e60ea99b60
repaths=579 made_room=0`, food 56 / 397 / 667 / 690 at 400 / 800 / 1200 / 2400 s.

**Why it was reverted.** The D200 merge passed every test and both gates and killed the acceptance match's food economy: after
it, the bank held **0 food at every report** from 400 s to 2,400 s, wood climbed 70 -> 790, units fell 43 -> 14, and player 2 -
four villagers and an untouched town centre - was still standing at 48,000 ticks. The mechanism is measured, not guessed:
four loaded carriers freeze 1.7-2.4 tiles from their town centre, each with a **one-point route aimed at the building's own
blocked anchor** (`next=(320000,320000)` = tile `(16,16)`, `touching=false`, 419 identical probe ticks), so the deposit test
never fires and `approach_tile` is only consulted when there is *no* route. Full measurement, the wrong claims in the merged
review (its hash and its `players` explanation), and the probe output: `docs/reviews/2026-11-13-d200-reverted-food-stall.md`.

**Branch `d189-owner-blind` keeps every commit of D200** (last `1ef3a85`, merged as `2c7d8e1` before the revert). Nothing is
lost; `main` is green.

## The plan, in order. Do not start a step while the previous one is red.

### 1. The `Return` whose waypoint is not standable ground *(next, test-first)*

A `Return` route that ends on a building's own anchor can never complete. The fix belongs in the `Task::Return` arm
(`crates/sim/src/world/step.rs:~1959`): a waypoint that is not walkable ground is replaced by `approach_tile`, whether or not
a route exists - today `approach_tile` is the `unwrap_or_else` of `next_waypoint`, so a route *to the anchor* wins.

**The fixture comes first, and it must be watched red:** a drop site that is a **town centre with its own footprint** (not a
mill beside grass), a bush two tiles off, four carriers approaching from open ground - the acceptance match's own geometry,
which every existing drop test sidesteps. `a_crowd_all_banks_its_loads` passes before and after the change and is therefore
not the harness for this.

### 2. Re-land D200 on top of the fix

Re-merge or re-apply `d189-owner-blind`, run the acceptance match, and require the two numbers that were red: **food reaches
the bank** (a non-zero bank at 400 s and a rising unit count), and the summary line's `players` is stated with its mechanism
(highest surviving player id) rather than explained away. The workspace gate and the review numbers go in the same commit.

### 3. Register row 2 - damage *(investigation started; two doc commits landed before the revert and stay)*

`0x744d60` has five call sites (`0x6bb100`, `0x741a30` x2, `0x744ff0`, `0x745060`); `docs/behaviour-diff.md` now records what
each one reads (multiplier 1.0 everywhere, the wrapper sums) and that the per-entry drop stands. Open: which call site is
combat's path, and whether the game drops each sub-1 class entry or floors the total (`combat.toml`'s `minimum = 1`).
**Done when:** row 2 is ruled `same` or `different` with the address, and a `different` becomes a test-first fix.

### 4. `World::from_state` and the tool-side converter (B shape) - `--load`

A versioned world description the engine builds from, produced by a GPL tool now and by a reader inside the engine later. The
18 saves in the install become a differential oracle: run 1,200 ticks from `brit7`, compare with `brit8`.
**Done when:** `ruage-headless --load <save>` runs and the converter's output describes a *world*, not a transcript.

### 5. The importer into `tools/` and a bootstrap step

One repository, GPL-3.0 directory with its own licence file, invoked once at install time.
**Done when:** a clean clone with only `Age2HD/` present builds and produces `config/gamedata/`.

### 6. Content (D201): names by three lists, prose rewritten, campaigns ours only

### 7. Publish cleanup: quote-and-cite the five docs, licence plumbing, CI gates, README

## Needs the owner - and only this

- the **A vs B** publishing answer (does `--load` live in a published engine, and is the engine public yet) -
  `docs/licensing.md`, "B now, A later";
- anything that changes a rule he set. **D200 is his ruling and it is not in question**: friends are solid, and the engine
  will do it - but it re-lands only when the acceptance match still feeds itself.

**Decided elsewhere, not on this plan:** pacing/AI (owner: later, with the AI), the renderer (D18), the `Vision` split (no
behavioural content - last on purpose).

## Standing rules for this plan

Test-first, with the red run shown. Measure before and after. Every step ends committed and pushed. A step that cannot go
green is parked on a branch with its diagnosis, **never left red on `main`** - which is exactly what happened here.

## Instrument notes, so a fresh session does not re-derive them

The acceptance command that matters (release, 40 minutes of game time):

```
./target/release/ruage-headless --config config/sim.toml --map config/map.toml --economy config/economy.toml \
  --units config/gamedata/units.toml --resources config/resources.toml --combat config/combat.toml \
  --techs config/gamedata/techs.toml --effects config/gamedata/effects.toml --civs config/gamedata/civs.toml \
  --script config/acceptance.toml --seed 7 --ticks 48000 --report 400 --dump build/dump.csv
```

`--dump` writes `kind,player,id,unit,slot,x,y,amount,hp,carry,path,task`; `path` is the route's length, which is how the
carrier stall was found (`path=1`, one point, the anchor). A probe on a `Task` arm is env-gated and removed in the same
session - `no_float` catches a leftover `as f64` in production code.

craft-anything's CLI is `/opt/data/tools/craft-anything/bin/craft-anything-cli`; the MCP entry is registered **deep**
(`mcp --all-tools --deep`, 323 tools, `connect_timeout` 180) and its `PATH` must carry the JDK 21 `bin` and both Ghidra 12.2
directories. **Every identification number in the evaluation came from its built-in engine**, which needs no Ghidra; a
Ghidra-backed call pays ~6.5 min and 1.4 GB. Its function count is **19,484** where Ghidra's is **15,465** - never quote one
without its tool. The REA corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 functions) and is the *reading* instrument.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…`
(tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
