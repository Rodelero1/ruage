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

## Where T12 stands: two stages landed, and the inner loop priced (2026-11-16)

**The original's order is read, and two stages of it are in the engine.**

- **The binary** (`FUN_00738720`, `0x738720`): `BaseWorld::update` walks the players array at `+0x184..+0x188` from index 0
  up - Gaia, then 1-8 - and inside a player's turn walks that player's object list one object at a time (*sweep, else step*:
  `+0xa0` decides removal, `+0x84` runs the update), so death is applied **in the victim's own turn**. It also advances
  `pturn` at `+0x100`, which is T16's axis. Full write-up in **D229** and `docs/mechanics/determinism.md`.
- **Stage 1 (D229)**: the ordered blow pass sorts `(player, id)`, and a building dead when that pass ends finishes no
  production or research.
- **Stage 2 (D230)**: `Sim::step` splits the tick's work into **one turn per player** and applies each player's share as a
  unit, so a man killed by an earlier player's blow does nothing for the rest of the tick and a man whose turn came first has
  already worked. Two fixtures, `crates/sim/tests/turn_order.rs`, same build with the players swapped.

**Stage 3 - the inner loop - is priced, not started.** It means deciding *and* applying one object at a time inside a
player's turn, and what it would still buy after the two stages is narrower than it looks. The quantities a decision reads
that an earlier object of the same player can change in the same tick are: node amounts (already clamped at apply time in
`grant_takes`, so the sum matches), ring reservations and places (already applied in order), `spent_nodes` (a retask that
sees the opening amount lags **one tick**), free ground for a spawn (`free_spot_near` already runs at apply time), and the
**population ceiling** - which was the one real defect in the list, fixed as **D231**: the ceiling was read once at the
tick's opening, so two barracks finishing with one slot left both delivered (measured: population 6 against a cap of 5; the
fixture is `population::the_ceiling_is_read_at_the_spawn`).

The price of the rest is a `decide`-per-object rewrite: `decide` is one 700-line `&self` pass over the whole entity list,
the walk would have to be keyed by id rather than index (a unit born mid-tick shifts every index after it - the hazard
`references/performance.md` names), and construction's per-site hand count and movement's collision set are aggregates that
cannot be per object at all. Against that, every remaining case is a **one-tick latency on a quantity that is already
clamped**, and none of them has a match-visible outcome. **Recommendation: take T16 (the turn) before stage 3** - the tree
sequences it first anyway, its evidence is in hand from the same function, and it is the axis that decides *whether* an
object's update runs at all.

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
