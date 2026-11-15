# OPEN - T9: five kind-4/5 values are negative, and their packing is not decoded

**Where it stands, 2026-11-15.** Three debts from the dependency tree are paid: **D214** (attribute 1 is line of sight, 12 is
range, in the importer and the engine - a sight tech had been moving a tower's *attack range*, and a sight upgrade marked no
new ground because exploration is stamped per move) and **D215** (a farm's base is the civ's own `resources[36]` = **175**,
read instead of the authored 250; and the Chinese team bonus, +45 on the same resource, had never been applied at all).
D215's price is fidelity, not frames: `repaths` **380 -> 552** in the acceptance match because a plot holds 75 less food,
which is why item 5 below matters more than it did. Reviews: `docs/reviews/2026-11-15-d214-attributes.md`,
`docs/reviews/2026-11-15-d215-farm-base.md`.

The order below is the user's ruling of 2026-11-14 - **live wrong numbers first**, then the effect interpreter, then the
economy's own rules. Do not start breadth, civs, or the renderer before items 1-5.

## The open items, in the order I would take them

1. **T9 - negative kind-4/5 values (U17).** Measured, so the next session starts from the data rather than the name: the pack's
   command tuple is `[type, unit, unit_class, attribute, value]`, and for kind 4/5 a **positive** value is the packed word
   `class * 256 + amount` - Forging's `[4, -1, 6, 9, 1025.0]` is class 6, attack class **4**, +1, and the importer decodes it
   correctly. Six commands carry a **negative** word instead, and the importer emits `unpacked:8` / `unpacked:9`, which the
   engine silently ignores: effect **217** (`[5, -1, 13, 9, -136.0]`, classes 13/54/22), effect **506** (Shinkichon, tech 445,
   `[4, 280, -1, 9, -226.0]`, mangonel/onager/siege onager), and effects **80/81/82** (`[4, 1120, -1, 8, -1025.0]`).
   **The open question is the sign convention**, and the two candidate readings of -226 are not the same game: floor division
   gives (class -1, amount 30) and truncation gives (class 0, amount -226), and neither is obviously Shinkichon's. Probe
   `Zaydme/openaok` and `genie`'s effect application first, then the exe; **refuse the effect by name** if no reading is
   defensible - a refusal that stops is better than a wrong number that ships.
2. **T10 - elevation is loaded and unused, and there is no chance-to-hit roll.** The published damage formula needs elevation,
   the pack has no heights, so the term is pinned to zero and `config/combat.toml:56` says `PROVENANCE: unknown`. Either an
   elevation field or a stated zero with a test that says so; today it is only a sentence.
3. **The construction rate (resource 195) is never read** - and the probe has a surprise in it: **every civ's `resources[195]`
   is 0.0 in this dat**, so reading it may change nothing at all. Decide that first (a probe over the 32 rows), then either
   wire the multiplier or record that HD carries no per-civ construction rate and close the debt as a measurement.
4. **`DisableTech` is parsed and never consulted** (`grep -rn DisableTech crates/sim` -> 0). A civ's **tech-tree bars do
   nothing**; only unit bars (kind 2) work. This is T2's kind-102 path and lands with the effect interpreter, not before it.
5. **No farm reseed queue, and the resource search is unbounded.** `grep reseed crates` -> 0; the game caps a resource search
   at **LOS+5 tiles and 30 reach tests** (U22). D215 made the reseed path hot: 43% more plots are laid per match.
6. **Two arrival failures remain, both `#[ignore]`d with their measurements** (off-tree bugs, not dependencies - they do not
   block items 1-5): a villager rallied to its own town centre never garrisons (`inside = 0` after 600 ticks, passes on
   `e0312f9`, fails from `e21b44d`), and a builder on a 6x6 map is asked for no step at all (`step = Idle` for 1,900 ticks,
   0.47 tiles from its waypoint, fails on old `main` too). Both are in the *arrival* family, not the collision one.
7. **The search's own cost, per plan, has never been isolated.** `crates/sim/tests/perf.rs` (ignored) at a fixed entity count,
   with and without a stander on the route. D211's decision says so in its own words.

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
