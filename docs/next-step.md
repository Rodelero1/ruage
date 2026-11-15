# OPEN - T4: a dark-age farm holds 250 food in the original and 175 here

**Where it stands, 2026-11-15.** The collision story is closed (D210, D210b, D211, D212, D213) and **the first two debts in the
dependency tree are paid**: `T1a` + `T1b` landed as **D214** - the dat's attribute **1 is line of sight** and **12 is attack
range**, in the importer and in the engine, with one `effective_los` consumer at the three raw `los` reads, a panel that now
shows effective range and sight, and `restamp_player` so a sight upgrade marks new ground (exploration had been stamped per
*move*, so a tower's +4 sight revealed nothing). Measured: a watch tower's `los` 1000 -> 1400 and its range stays 800; the
pack moved 40 modifiers to `line_of_sight` and 26 to `range`. Reviews: `docs/reviews/2026-11-15-d214-attributes.md`,
`2026-11-14-d211-shoulders.md`, `2026-11-15-d213-last-moved.md`.

The rest of the debt list is unchanged and is the order below: it is the user's ruling of 2026-11-14 - **live wrong numbers
first**, then the effect interpreter, then the economy's own rules. Do not start breadth, civs, or the renderer before §1-§4.

## The open items, in the order I would take them

1. **T4 - the farm's base is 175 in the dat and 250 in `economy.toml`.** `config/economy.toml`'s farm entry is 250; the dat's
   farm (unit 50) carries **175**, so a Horse Collar farm holds 325 where the game holds 250 - a 30% over-credit on the first
   food building every player makes. Fix is data plus the fixtures that were written against 250 (grep `250` in
   `crates/sim/tests/gather.rs` and `acceptance_*`); expect acceptance hashes to move, and say so in the review rather than
   "restating" a hash quietly.
2. **The construction rate (resource 195) is never read.** `grep -rn '\b195\b' crates` is empty, so build times ignore the
   per-civ rate and Spanish build at British speed. The mechanism exists for gather rates already (the variant's work rate);
   this is the same shape on `Building`'s construction task, and it needs a test that a 1.2x civ finishes in 5/6 the ticks.
3. **`DisableTech` is parsed and never consulted** (`grep -rn DisableTech crates/sim` -> 0). A civ's **tech-tree bars do
   nothing**; only unit bars (kind 2) work. The data is in the pack; the check belongs where a research order is validated,
   beside the age and building prerequisites.
4. **No farm reseed queue, and the resource search is unbounded.** `grep reseed crates` -> 0; the game caps a resource search
   at **LOS+5 tiles and 30 reach tests** (U22), and ours can walk the whole map. Two separate slices; the cap is the one that
   can change a match's outcome.
5. **Two arrival failures remain, both `#[ignore]`d with their measurements** (off-tree bugs, not dependencies - they do not
   block §1-§4): a villager rallied to its own town centre never garrisons (`inside = 0` after 600 ticks, passes on `e0312f9`,
   fails from `e21b44d`), and a builder on a 6x6 map is asked for no step at all (`step = Idle` for 1,900 ticks, 0.47 tiles
   from its waypoint, fails on old `main` too). Both are in the *arrival* family, not the collision one.
6. **The search's own cost, per plan, has never been isolated.** `crates/sim/tests/perf.rs` (ignored) at a fixed entity count,
   with and without a stander on the route. D211's decision says so in its own words.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored, so a fresh worktree has none, and
  every fixture does `let Some(mut world) = common::fresh() else { return; };`. A whole file reports `ok` without running an
  assertion. **Copy** the four `.toml` files in; never symlink them (the pack-loader tests write into that directory).
- **The pack is generated, and its names are the engine's interface.** `uv run python -m genconfig <dat> <config>` in
  `ruage-tools`; the simulation matches modifiers by **string** (`modifiers_for(player, unit, "range")`), so a rename is a
  behaviour change with no compiler to catch it. Attribute 1 is `line_of_sight`, 12 is `range` (D214).
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swap test binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" was in a review and a
  comment for a day and did not exist. When a review claims a mechanism, `grep` for it.
- **A number that is right does not mean the mechanism is.** D214's sight arithmetic was correct while the fog still could not
  see the new ground: exploration is stamped per **move**. Three times in this run of work the fix was the second one.
- **A test whose route is planned *around* the body it means to collide with measures nothing.** With D210b/D213 live, a
  fixture that places a stander **before** the order gets a route round him: four fixtures had to move the body *after* the
  route exists (`collision`, `evasion`, `step_decision`, `repath_reasons`). The failure reads "the scene never met the body" -
  if you see that, the fixture is stale, not the rule.
- **A `#[must_use]` left over a deleted function** lands on whatever is inserted next to it: `put_route` inherited one from an
  orphaned standoff doc block and `-D warnings` refused the tree. Read the diff around a structural edit.
- **`make plan-check` resolves `ruage-tools/...` against the checkout's parent**, so a worktree outside `/opt/data/workspace`
  reports a good citation as missing. Check it in the workspace checkout.
