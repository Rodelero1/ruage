# OPEN - the farm reseed queue and the bounded resource search, the two halves D215 made hot

**Where it stands, 2026-11-15.** Six decisions, closing five debts and one carried regression from the dependency tree: **D214** (attribute 1 is line of sight, 12 is
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
`-d218-construction-rate.md`, `-d220-rally-inside.md`.

The order below is the user's ruling of 2026-11-14 - **live wrong numbers first**, then the effect interpreter, then the
economy's own rules. Do not start breadth, civs, or the renderer before items 1-2.

## The open items, in the order I would take them

1. **No farm reseed queue, and the resource search is unbounded.** `grep reseed crates` -> 0: a spent plot is a husk until a
   player replaces it, and a villager whose plot ran dry walks the *whole map* for the next node, where the game caps a
   resource search at **LOS+5 tiles and 30 reach tests** (U22). D215 made this hot: a plot holds 175 instead of 250, so 43%
   more plots are laid per match and `repaths` went **380 -> 552**. Two slices, and the cap is the one that can change a
   match's outcome; measure `repaths` either way.
2. **One arrival failure remains, `#[ignore]`d with a sharper diagnosis than it had** (off-tree, not a dependency):
   `build::a_content_pack_with_our_own_resources_loads_and_spends` - a house on a 6x6 map never leaves its site state for
   2,000 ticks. **Measured this session, and the cause is not the builder.** The builder does walk (it is at (4, 4) at t0,
   (5, 4) by t30) and stands adjacent, correctly; what blocks the site is **another villager parked on the footprint**
   (`site_is_clear` reports `standing [(EntityId(4), 1)]`, our own side). `step_aside` is called for it every tick and every
   tick returns early, because it records the attempt in `resuming` **even when the search failed** - so a single failure is
   remembered for the rest of the match. The first failure is real: the builder's standoff tile (5, 4) is the blocked
   villager's only exit, and a body standing on the goal makes the walker's step illegal. **Two defects, one slice:** (a) a
   failed step-aside must be *retried* (a retry deadline, not a permanent record - my first attempt at this used
   `ASIDE_RETRY_TICKS = 20` and the retry did fire, which is how (b) became visible), and (b) the standoff search must not
   hand the builder the only tile another unit can use to leave - either the builder waits, or the exit search excludes the
   standoff. Reverted rather than landed: the retry alone left the fixture red, and an unproven change is not worth a commit.
   The parked test's `#[ignore]` sentence now carries the measurement.
3. **The search's own cost, per plan, has never been isolated.** `crates/sim/tests/perf.rs` (ignored) at a fixed entity count,
   with and without a stander on the route. D211's decision says so in its own words.
4. **The effect interpreter's remaining kinds** (T2): 0/101/103 - tech cost, tech time, set-attribute rows that no consumer
   reads yet. This is the interpreter slice the tree puts in Phase 1; it is *not* a prerequisite for items 1-3.

## Ruled out of the current stretch, with the reason written down

- **Civ and team bonuses** (the owner, 2026-11-15: *"can wait ... just have the design ready"*). The design is
  `docs/civ-bonuses.md` and seam **R6** in `docs/dependency-tree.md`, and the reason it is a design rather than a switch is
  measured there: **23 of the ~100 team-bonus modifiers name attributes no consumer reads** (`other:100`, `other:101`), two
  bonuses carry a **kind-102** row (a bonus that bars a tech), and kinds 3 and 6 have no reader anywhere. D219.
- **The farm reseed queue** - a control feature (a new command, its own state, an AI policy), not a fix, and the owner has not
  chosen between implementing it small, shipping only the search bound, or recording it out of v1. Do **not** start it on a
  guess: the two arrival bugs below are independent of the answer and come first.

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
