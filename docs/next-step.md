# OPEN - step 1: six tests, one of which may be a real defect

**State, 2026-11-13.** `main` at `a722263`, green (183 binaries, `EXIT 0`, clippy 0, fmt 0), acceptance `91572e72c33cee93`.
The owner-blind collision rule (D200) and the measured geometry are built on **`d189-owner-blind`**, `67dabd1`: gather places
`0.66` apart, spread evenly, **7 places** on one tile - the same seven his saves show. **79 test binaries green, 8 tests red**,
every red one crowd delivery or a test that pins the old spacing.

## The plan, in order. Do not start a step while the previous one is red.

### 1. The nudge landed; the drop is the next stall *(branch work, no decision needed)*

**`WaitBlocked` forever is fixed.** A body at rest is nudged out of a walker's way only after **40 refused ticks**, only for a
body on the walker's **own tile**, never for a body somebody is working: the 0 A.D. shape, damped hard. Measured on the branch
(`0ef5de2`): the trebuchet fixture **ok in 6.29 s** where it took 46 s and banked 200 of 400, and the rules the engine already
states are intact - `evasion` 11/11 including all three *"the body it went round did not move"*, `animals` 6/6 including the
sheep that must stand still, `gather_motion` 5/5.

**Suite: 113 binaries green, 11 tests red.** Six were already there (content-pack house, two tests written for 0.40 spacing, the
woodcutter joining a patch, the reservation book, the withdrawn friend-on-goal rule). **Five are economy and skirmish**:
`two_players` 790 of 900 banked, `the_spear_line` 400 of 700, `both_sides_field_an_army`, `the_skirmish_only...`,
`a_friend_on_the_goal_tile_is_a_take`. At 400,000 ticks with `repaths 29, made_room 0` these look like a **second stall, at the
drop**: carriers hold places (`Return` keeps one, by design), and two carriers blocked by each other are both "busy", so
neither is nudged and neither yields.

**Done when:** the five economy rows pass, the trebuchet fixture stays at ~6 s, and the branch merges.

### 2. Restate the three tests that pin the withdrawn rule

`step_decision::a_friend_on_the_goal_tile_is_a_take` (a friend on the goal tile is now an obstacle and the mover stops beside
it), `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives` (arrives *beside*, not on), and
`evasion::two_villagers_crossing_pass_through_and_both_arrive` (they go round each other). Two gather tests compute places
with `2 * radius` and expect 10-11; they should call the engine's own capacity so they cannot drift.

### 3. Land D200 on `main`

Whole-workspace gate (fmt, clippy `-D warnings`, every test), acceptance numbers recorded, review updated, merged, pushed.

### 4. Register row 2 - damage *(investigation, no code until ruled)*

`0x744d60` has five call sites (`0x6bb100`, `0x741a30` x2, `0x744ff0`, `0x745060`). Which is combat's path, and does the game
drop each sub-1 class entry (as `0x744d60` does) or floor the total (as our `combat.toml` `minimum = 1` does)? Two seconds a
query with craft-anything, then reading.

**Done when:** row 2 is ruled `same` or `different` with the address, and a `different` becomes a test-first fix.

### 5. `World::from_state` and the tool-side converter (B shape) - `--load`

The seam that makes B -> A a swap: a versioned world description the engine can build from, produced by a GPL tool now and
by a reader inside the engine later. 18 saves in the install become a differential oracle: run 1,200 ticks from `brit7`,
compare with `brit8`.

**Done when:** `ruage-headless --load <save>` runs and the converter's output describes a *world*, not a transcript of the file.

### 6. The importer into `tools/` and a bootstrap step

The structural half of A-prime: one repository, GPL-3.0 directory with its own licence file, invoked once at install time.
**Done when:** a clean clone with only `Age2HD/` present builds and produces `config/gamedata/`.

### 7. Content (D201): names by three lists, prose rewritten, campaigns ours only

### 8. Publish cleanup: quote-and-cite the five docs, licence plumbing, CI gates, README

## Needs the owner - and only this

- the **A vs B** publishing answer (does `--load` live in a published engine, and is the engine public yet) -
  `docs/licensing.md`, "B now, A later";
- anything that changes a rule he set.

**Decided elsewhere, not on this plan:** pacing/AI (owner: later, with the AI), the renderer (D18), the `Vision` split (no
behavioural content - last on purpose).

## Standing rules for this plan

Test-first, with the red run shown. Measure before and after. Every step ends committed and pushed. A step that cannot go
green is parked on a branch with its diagnosis, **never left red on `main`**.

## Instrument notes, so a fresh session does not re-derive them

craft-anything's CLI is `/opt/data/tools/craft-anything/bin/craft-anything-cli`; the MCP entry is registered **deep**
(`mcp --all-tools --deep`, 323 tools, `connect_timeout` 180) and its `PATH` must carry the JDK 21 `bin` and both Ghidra 12.2
directories - without the JDK's `bin` the CLI reports java and Ghidra "not found" whatever `JAVA_HOME` says. **Every
identification number in the evaluation came from its built-in engine**, which needs no Ghidra; a Ghidra-backed call pays
~6.5 min and 1.4 GB. Its function count is **19,484** where Ghidra's is **15,465** - never quote one without its tool. The
REA corpus is `/opt/data/tools/rea/pseudo-aokhd` (15,465 functions) and is the *reading* instrument.

**Owner saves live in scratch, never in the tree** (D8/D22): `/opt/data/cache/scratch/owner-saves/`, SHA-256 `6bc5f8a6…`
(tree), `b9455053…` (mine), `bbc92744…` (bush), save version 12.5.
