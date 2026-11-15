# OPEN - the reseed queue and auto-reseed at the mill

**Where it stands, 2026-11-15.** Seven decisions, closing five debts and one carried regression from the dependency tree: **D214** (attribute 1 is line of sight, 12 is
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

## There are no live defects - audited, not assumed (D224)

Every parked test that does not take minutes was **run** on 2026-11-15 (`cargo test -- --include-ignored`): all green. The
four items an earlier gap report called live were `depletion` (no test carried the attribute - the doc did), `retarget`
(D98a, green), `carrier_no_pushing` (D204, green) and `gather_contact` (D181, where the *fixture* measured to a tile's centre
and a corner contact is 0.93 from it by geometry - the engine was right). All four are closed or corrected in D224. What
remains is therefore the debt list, not a defect list.

## The open items, in the order I would take them

1. **The reseed queue and auto-reseed, both at the mill only** (the owner, 2026-11-15: *"please implement queue and
   auto-reseed, both are only in the mill"*, having been told that **auto-reseed is a Definitive Edition feature** and that HD/AoC
   has only the manual queue). Both are features, not fixes: a command that queues a replacement (**60 wood committed at the mill
   per entry**, as the game does), state for the queue and for the toggle, and the behaviour of a villager whose plot runs out -
   he takes the next entry and reseeds the same ground, and stands idle when there is none and the toggle is off. Note the screen:
   the dat's own resource for a farm's wood cost, not a constant in the engine. Gate: a fixture for the queue (wood drops, a farm
   reappears on the same tiles) and one for the toggle (no wood, no reseed; toggle off, the villager goes idle).
   **Anchors measured before starting** (so the next session does not re-find them): the mill is dat **68** (`name = "MILL"`,
   `config/gamedata/units.toml:1843`) and has **no `[role]` yet**; a farm is role `farm = 50`, **60 wood** (`costs = [[1, 60]]`);
   a spent plot already becomes a **husk** and is removed after `husk_ticks` (`step_husks`, `crates/sim/src/lib.rs:3886`,
   `is_expired_farm_ref`); commands are validated in `world/orders.rs`'s `check_*` match (:57) and applied in the `match` at
   :284; `start_build_site(builder, unit, at)` is the one path that raises a site, and `build_queue` already chains sites (D66),
   so a reseed is "pay, then hand that man a farm site on the old footprint".
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
