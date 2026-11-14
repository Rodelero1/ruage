# OPEN - D205 is parked on `d205-pause`: the pause is right, the pather is the missing half

**State, 2026-11-14.** D205's rule is **implemented and measured** on branch `d205-pause` (from `bb467dd`), and it **cannot
land as specified**. The tie-break is faithful to the binary - lower entity id has right of way, the higher id pauses a
uniform 0.25-1.50 s from the seeded stream and keeps its route - and the three improvisations it replaces (the give-way
mutex, the axis/perpendicular slide, `push_the_standing`'s nudge) are gone. The consequence is measured, not debated:
**the winner's bounded step is exactly zero at the pair's reach and its re-plan is tile-based, so the pair stands while the
loser is paused.** The acceptance match freezes at 400 s - wood 100, food 10, units 13, buildings 6 at every report point to
2400 s, against `main`'s 48 units and food `45 / 49 / 268 / 120 / 660 / 450` - and **33 tests in 21 binaries are red**,
including `acceptance_economy`. Record: `docs/reviews/2026-11-14-d205-pause.md`.

**Do not merge it and do not delete it.** The branch is the evidence for the next step, and its three fixtures
(`crates/sim/tests/right_of_way.rs`) are the acceptance for whatever fixes the gap: the pause length and the kept waypoint
list already pass there; what fails is that the pair never gets past each other.

**D204 remains the state of `main`** (`bb467dd`): food `45 / 49 / 268 / 120 / 660 / 450`, wood `80 / 96 / 135 / 75 / 395 /
185`, units `20 / 31 / 35 / 44 / 46 / 48`, summary `entities=160 players=2 hash=c3b10f4b09e17a43 repaths=324 made_room=147
repaths_body=83 repaths_ground=241`, acceptance wall 30.3 s, like-for-like tick 6.5-6.8 us. Gate: 185 binaries, 843 passed,
0 failed, 21 ignored, fmt and clippy clean.

## The plan, in order. Do not start a step while the previous one is red.

### 1. A re-plan that can route round a **body**, not a tile *(next - the only step that unblocks D205)*

The blocker is a disc (`blocker_at`, radius-summed); the plan is a tile set (`short_range` and `find_path_avoiding` both
take `&BTreeSet<(u16,u16)>`). Two consequences, both measured on `d205-pause`:

- **Same tile**: the avoid set cannot express the blocker, because the plan drops the mover's own start tile as an obstacle
  ("an avoid set that holds it makes the search leave the tile it starts in"). The re-plan returns the straight line it
  already had, and the mover is `WaitBlocked` from tick 70 to tick 2,500 of a 2,500-tick run.
- **Adjacent tile**: the re-plan does produce a detour, and it does not help: the mover's own disc already overlaps the
  avoided tile's expansion by 400 raw units, so the graph's first waypoint is the **far** corner -
  `[(264000,156000), (290000,170000)]` for a man at `(236400,170000)` - and the step toward it is still blocked. Two men
  stop at **8000 raw = exactly the two 0.2 radii**, where `bound_to_reach` permits no step at all.

**Done when** `crates/sim/tests/right_of_way.rs` is green on a branch that keeps the give-way, the slide and the nudge out:
head-on (test 1), the owner's carrier-and-gatherer picture (test 2), and two men jammed on one tile (test 3). The two cheaper
alternatives are recorded in the review and need the owner's word, because both bring back something a decision removed:
a lateral step for the winner only (D205's slide), or a step through a body it has just paused (D189's pass-through, which
D200 withdrew and `collision.rs::a_walker_never_ends_a_tick_inside_a_friend` forbids).

### 2. Register row 2 - damage

`0x744d60` has five call sites (`0x6bb100`, `0x741a30` x2, `0x744ff0`, `0x745060`); `docs/behaviour-diff.md` records what
each reads (multiplier 1.0 everywhere, the wrapper sums) and that the per-entry drop stands. Open: which call site is
combat's path, and whether the game drops each sub-1 class entry or floors the total (`combat.toml`'s `minimum = 1`).
Two seconds a query with craft-anything, then reading the owned function.
**Done when:** row 2 is ruled `same` or `different` with the address, and a `different` becomes a test-first fix.

### 3. `World::from_state` and the tool-side converter (B shape) - `--load`

A versioned world description the engine builds from, produced by a GPL tool now and by a reader inside the engine later. The
18 saves in the install become a differential oracle: run 1,200 ticks from `brit7`, compare with `brit8`.
**Done when:** `ruage-headless --load <save>` runs and the converter's output describes a *world*, not a transcript.

### 4. The importer into `tools/` and a bootstrap step

One repository, GPL-3.0 directory with its own licence file, invoked once at install time.
**Done when:** a clean clone with only `Age2HD/` present builds and produces `config/gamedata/`.

### 5. Content (D201): names by three lists, prose rewritten, campaigns ours only

### 6. Publish cleanup: quote-and-cite the five docs, licence plumbing, CI gates, README

### 7. The gather capacity, restated as a question the saves can answer

The game shows seven to eight bodies at **0.66** on a node's touching envelope; our ring hands out places at 0.66 first and
0.40 when the ring refuses. Both are now exercised by the acceptance run, so the difference is visible; what is not settled
is whether a **ninth** man should stand outside and wait (the crowd policy this work shipped) or walk to another node - and
D204 has made "walk to another node" the engine's answer wherever the man was refused a square, which may be what the saves
were showing all along.

## Needs the owner - and only this

- **which way past a paused body is allowed**: a body-aware re-plan (no decision is undone; this is step 1), a lateral step
  for the winner only (the slide comes back), or a step through a paused body (the pass-through comes back). Nothing lands
  on `main` until this is answered;
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
the value from seeding and reads as `Idle`. `World::route_points(id)` - added by D205's branch - returns the waypoint list a
stalled body is still holding, which is what says whether a re-plan produced a detour or gave up.
`crates/sim/tests/acceptance_economy.rs` runs the acceptance world itself and is the guard that would have caught this
regression; keep it green. Probes on a `Task` arm are env-gated and removed in the same session - `no_float` catches a
leftover `as f64` in production code.

craft-anything's CLI is `/opt/data/tools/craft-anything/bin/craft-anything-cli`; the MCP entry is registered **deep**
(`mcp --all-tools --deep`, 323 tools, `connect_timeout` 180) and its `PATH` must carry the JDK 21 `bin` and both Ghidra 12.2
directories. **Every identification number in the evaluation came from its built-in engine**, which needs no Ghidra; a
Ghidra-backed call pays ~6.5 min and 1.4 GB. Its function count is **19,484** where Ghidra's is **15,465** - never quote one
without its tool. The REA corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 functions) and it is the *reading* instrument.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…`
(tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
