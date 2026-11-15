# OPEN - T12: the tick's order, which the original's own binary has now given up

**Where it stands, 2026-11-15 (evening).** Seven decisions, closing five debts and one carried regression from the dependency tree: **D214** (attribute 1 is line of sight, 12 is
range, in the importer and the engine - a sight tech had been moving a tower's *attack range*, and a sight upgrade marked no
new ground because exploration is stamped per move) and **D215** (a farm's base is the civ's own `resources[36]` = **175**,
read instead of the authored 250; and the Chinese team bonus, +45 on the same resource, had never been applied at all).
D215's price is fidelity, not frames: `repaths` **380 -> 552** in the acceptance match because a plot holds 75 less food,
which is why the reseed queue below matters more than it did; **D216** refuses any packed attack value that no reading
explains instead of shipping it as silence; and **D217** makes the elevation term a real site held at 1.0 with two tests
proved by mutation; and **D218** reads the construction rate (dat resource 195) from the civ's tech-tree effect - a Spanish
builder is 30% faster, 500 builder-ticks become 385 - and corrects a **false debt**: the tree said a civ's tech bars do
nothing, but `barred_by` reads kind-102 rows and `check_research` refuses with `TechBarred`, tested since D23. Reviews:
`docs/reviews/2026-11-15-d214-attributes.md`, `-d215-farm-base.md`, `-d216-packed-attack.md`, `-d217-elevation.md`,
`-d218-construction-rate.md`, `-d220-rally-inside.md`, `-d222-builder-rate.md`.

The order below is the user's ruling of 2026-11-14 - **live wrong numbers first**, then the effect interpreter, then the
economy's own rules. Do not start breadth, civs, or the renderer before items 1-2.

## T12 - the original's intra-tick order, read from 5.7's own binary (2026-11-15)

`BaseWorld::update` is **`FUN_00738720` at `0x738720`**, and it is named three ways by the file itself: its profiler
string `"baseworld update"`, the `aoeworld-at-%08d.txt` status log it writes, and its assertions' own source path
`...\aoc\source\game\baseworld.cpp` (lines `0xb6b`, `0xc7e`, `0xcb7`). What it does, in order:

- **Players in index order.** The outer loop walks the array at **`+0x184 .. +0x188`** (8-byte stride; `FUN_0072b700` reads
  the same pair as a count) from index 0 upward. Index 0 is Gaia, so the order is `Gaia, players 1-8`.
- **Each player's objects, one at a time, inside that player's turn.** For a player whose list is non-empty it walks an
  object list (head at `+0x48`, node `next` at `+0x5c`, node owner at `+0x60`) and skips every node whose owner is not the
  player it is on. Per object there is **one virtual call that decides removal** (`+0xa0` returning 0 → unlink and destroy
  through vtable slot 0, the class's own destructor) and **otherwise one virtual call that runs the object's own update**
  (`+0x84`). Neither method can be named - both are virtual and the binary carries no symbols - but the shape is
  *sweep, else step*, and the state one object leaves is what the next object reads.
- **Death is applied inline, in the victim's own turn** - not in a pass after the tick, which is what our `sweep_dead` is.
  So a unit killed by an earlier-ordered attacker is gone before any later object's turn: it does not swing, and it does not
  step, gather or finish anything either.
- **Per-player virtual bracket**: `+0x130` before the object walk, `+0x68` after it, `+0x134` once for the players already
  visited; then three more per-player loops gated on `FUN_00727810()`.
- **The turn is real and it is `pturn`.** At the end of the update the rotating index at **`+0x100`** is advanced by one and
  wrapped at the player count, and `+0x100` is `pturn` - the field `docs/save-format.md` reads out of the save header. That
  is **T16**'s axis, read for free here; it is not this slice.

## Stage 2: the players take turns (2026-11-15, late)

**Landed after stage 1.** `Sim::step` now splits the tick's work into **one turn per player** - Gaia at index 0, then
1-8 - and applies each player's share as a unit: places, the fight, the harvest, the re-tasks, the switch, the herd, the
delivery, the site, the blow, the return. So a man killed by an **earlier** player's blow does nothing for the rest of the
tick, and a man whose turn came **first** has already worked when the blow lands. Two fixtures in
`crates/sim/tests/turn_order.rs` are the pair: the same build, players swapped, and opposite progress on the site.

The split is one pass per tick (`TickWork::bucket`), not one per player per list, and every item whose actor is dead is
dropped **twice** - at the split, and again when its turn comes (`TurnWork::keep_the_standing`), because the split can only
judge health at the tick's opening while an earlier turn may kill in between. Removing either half reddens a fixture.

**What it costs, and the match it changes.** Forced into a single turn, the same match is bit-identical to `main`
(`beb6c66f58e4045c`, 154 entities, 277 repaths) and runs at **20.71 s against 19.76 s** for 24,000 ticks - **+4.8%** for the
mechanism. Live, the match **diverges** - `cb1f6a83a119a2a3`, 152 entities, `repaths=256` - and takes 22.64 s
(**+13.3%**), of which the loop is the 4.8% and the rest is the different match; a phase probe inside a live match is owed
to separate them. **An empty-turn skip was tried and withdrawn**: `shepherd` resumes a gather on a corpse from
`self.slaughter`, which is state and not one of the turn's lists, so the skip stopped a villager working the sheep he had
just killed and two animal fixtures caught it.

**What is still not the queue:** inside one player's turn the objects are still simultaneous (their work is applied phase
by phase, not object by object), and `decide` still reads the state the tick opened with - so a decision cannot see what an
earlier object did. Movement, production, arrows, world timers and the death sweep remain global phases after the turns.

## The slice T12 takes, and what it deliberately is not

Our tick **decides** every entity against the state the tick opened with (`decide`, `&self`) and then **applies** the
collected work in a fixed phase order. A true per-object turn means deciding *and* applying inside one loop per object -
a rewrite of `world/step.rs` (2,402 lines), not a slice that lands in one pass. So this slice takes the half the two-pass
shape can honestly carry:

1. **The apply order becomes the original's**: the ordered blow pass (D88) is sorted `(player, id)` instead of by entity
   id - Gaia first, then players 1-8, and inside a player the id, which is creation order and therefore the slot order.
   An id that is not in the world sorts last. This is the one place the engine already *is* a sequence, so this is where
   the order is real rather than nominal.
2. **A building destroyed when the blow pass ends finishes nothing**: production and research skip an entity at or below
   zero health, so a barracks killed this tick does not hand over the unit it had one tick left on.

**The price of the two-pass shape, stated rather than discovered:** (2) is right for every victim whose killer's turn came
first - which is exactly the case the ordered blow pass now guarantees - and stricter than the original for a victim whose
own slot came first. And the phases that run *before* the blows (gather takes, deposits, construction) still let a doomed
entity act, because a take is applied before any blow lands.

**Not in this slice, named:** movement deltas are still applied in id order; an object's update is not merged with its
neighbour's; and the turn itself (T16) is unbuilt, so every player's objects are stepped every tick as the original does -
but the original runs only *one* player's unit AI per tick, which is the axis `pturn` is for.

**The gate on this slice, and what it actually measured.** The acceptance hash does **not** move:
`beb6c66f58e4045c` at 24,000 ticks and `4640552242d6b45b` at 48,000, on both sides of the change, with
`entities=154 repaths=277` at 24k and `148 / 552` at 48k. That is the honest result rather than the expected one: the sort
is reached (removing it reddens `tick_order::the_lower_player_acts_first_whatever_the_ids`), but in that scenario every
strike already lists the lower player first, so the order is a no-op there. The rule's evidence is therefore the fixtures
in `crates/sim/tests/tick_order.rs`, not the hash - which also means no re-baselining and no behaviour change to record for
the match.

## There are no live defects - audited, not assumed (D224)

Every parked test that does not take minutes was **run** on 2026-11-15 (`cargo test -- --include-ignored`): all green. The
four items an earlier gap report called live were `depletion` (no test carried the attribute - the doc did), `retarget`
(D98a, green), `carrier_no_pushing` (D204, green) and `gather_contact` (D181, where the *fixture* measured to a tile's centre
and a corner contact is 0.93 from it by geometry - the engine was right). All four are closed or corrected in D224. What
remains is therefore the debt list, not a defect list.

## The open items, in the order I would take them

1. ~~**The reseed queue and auto-reseed at the mill**~~ - **DONE (D226)**: `Command::Reseed` and `Command::AutoReseed`, per-player
   state, a paid-site path, and a husk that releases its ground. Four fixtures in `crates/sim/tests/reseed.rs`. What is left of it is
   presentation only: there is no panel or button because there is no renderer.
2. **A builder claims his place - measured-out, not built** (D225). The owner's rule is a free **space one diameter wide**,
   claimed by one worker, with a builder who finds none taking the next site in **his own queue** ("2 a"). The *claim* half has no
   reachable trigger in this model: own bodies are not walls to their own side (D189), a visible enemy on the footprint is refused
   at placement (`SiteOccupied`), an unreachable site is refused before the wood is spent, and the only construction that could
   freeze - a blind placement in fog over an enemy **unit** - is not known to freeze at all, because `settle_blind_sites` cancels
   only on an enemy building. Before any code: measure whether that last case stalls. If it does, the fallback has its fixture.
3. **The search cap, if it is ever wanted** (D225): the bound exists (`gather_reach`: the dat's `search_radius`, else sight + 5
   tiles); what does not exist is the note's "at most 30 reach tests", a guard whose candidate set is a handful, so it was not
   written.
 `grep reseed crates` -> 0: a spent plot is a husk until a
   player replaces it, and a villager whose plot ran dry walks the *whole map* for the next node, where the game caps a
   resource search at **LOS+5 tiles and 30 reach tests** (U22). D215 made this hot: a plot holds 175 instead of 250, so 43%
   more plots are laid per match and `repaths` went **380 -> 552**. Two slices, and the cap is the one that can change a
   match's outcome; measure `repaths` either way.
4. **The search's own cost, per plan, has never been isolated.** `crates/sim/tests/perf.rs` (ignored) at a fixed entity count,
   with and without a stander on the route. D211's decision says so in its own words.
5. **The effect interpreter's remaining kinds** (T2): 0/101/103 - tech cost, tech time, set-attribute rows that no consumer
   reads yet. This is the interpreter slice the tree puts in Phase 1; it is *not* a prerequisite for items 1-3.

## Ruled out of the current stretch, with the reason written down

- **Civ and team bonuses** (the owner, 2026-11-15: *"can wait ... just have the design ready"*). The design is
  `docs/civ-bonuses.md` and seam **R6** in `docs/dependency-tree.md`, and the reason it is a design rather than a switch is
  measured there: **23 of the ~100 team-bonus modifiers name attributes no consumer reads** (`other:100`, `other:101`), two
  bonuses carry a **kind-102** row (a bonus that bars a tech), and kinds 3 and 6 have no reader anywhere. D219.
- **The farm reseed queue** is item 2, not ruled out: D215 raised its price (`repaths` 380 -> 552) and the search cap is
  the half that can change a match. It stays below the claim because the claim is the rule the owner has already given.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored, so a fresh worktree has none, and
  every fixture does `let Some(mut world) = common::fresh() else { return; };`. A whole file reports `ok` without running an
  assertion. **Copy** the four `.toml` files in; never symlink them (the pack-loader tests write into that directory).
- **The pack is generated, and its names are the engine's interface.** `uv run python -m genconfig <dat> <config>` in
  `ruage-tools`; the simulation matches modifiers by **string** (`modifiers_for(player, unit, "range")`), so a rename is a
  behaviour change with no compiler to catch it. Attribute 1 is `line_of_sight`, 12 is `range` (D214).
- **A per-civ number is in the civ row, not in content.** `resources[36]` is the farm base (D215); the same row carries `[84]`
  starting villagers, `[88]` fish-trap food and `[195]` a construction rate that is 0 for every civ here. Content names the
  index (`[farm] food_resource = 36`), and `food_per_farm` is only the fallback for a pack with no civ row.
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swap test binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" was in a review and a
  comment for a day and did not exist. When a review claims a mechanism, `grep` for it.
- **A number that is right does not mean the mechanism is.** D214's sight arithmetic was correct while the fog still could not
  see the new ground (exploration is stamped per **move**); D215's base was right-able while the Chinese bonus applied to
  nothing. Four times in this run of work the fix was the second one.
- **A test whose route is planned *around* the body it means to collide with measures nothing.** With D210b/D213 live, a
  fixture that places a stander **before** the order gets a route round him: four fixtures had to move the body *after* the
  route exists (`collision`, `evasion`, `step_decision`, `repath_reasons`). The failure reads "the scene never met the body" -
  if you see that, the fixture is stale, not the rule.
- **A `#[must_use]` left over a deleted function** lands on whatever is inserted next to it: `put_route` inherited one from an
  orphaned standoff doc block and `-D warnings` refused the tree. Read the diff around a structural edit.
- **Timing on this box needs a quiet machine.** D214's first A/B read +35% and the truth was +7.2%: the document tooling was
  running during the change's half. Three interleaved pairs, nothing else running, and a gap under the reference's own spread
  is unresolved.
- **`make plan-check` resolves `ruage-tools/...` against the checkout's parent**, so a worktree outside `/opt/data/workspace`
  reports a good citation as missing. Check it in the workspace checkout.
