# OPEN - T16: the turn is a per-player clock, and the AI split it gates does not exist

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

## T16 and T12 stage 3: both read, both priced, neither next (D229-D232)

**T16 is a clock, not an order.** `pturn` (`+0x100`) rotates each update and the nine-word array `+0x104..+0x124` accumulates
per-player milliseconds, zeroed for the player whose turn it is (`FUN_00738720`, reset `FUN_00735960`, getter `FUN_0072d0b0`).
The DE port uses it to give the turn player's objects the **full** update and everyone else a *between-turns check*, so building
it needs a per-player clock and a two-tier unit AI - the second a game-behaviour decision, not a reading. The **saved fields**
(`pturn`, the nine clocks) are owed by P1 either way.

**T12's order is in the engine (stages 1 and 2, D229/D230); stage 3 is priced and not worth it yet**: per-object decide-and-apply
is a rewrite of a 700-line `&self` pass, keyed by id (a newborn shifts indices), with construction's hand count and movement's
collision set as aggregates that cannot be per object - and every remaining case is a **one-tick latency on a quantity already
clamped** (`grant_takes`, `free_spot_near`), with the one real defect (the population ceiling, `D231`) already fixed.

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

## Landed: villagers are solid, and D236's "missing mechanism" was a guard (D237, 2026-11-16)

The owner's rule is in: a villager ordered with others is no longer a path-ignore mate (`stamp_group` forms mates among
**soldiers** only), and a body in the way can be planned round **inside the goal's tile** - the clause that refused it was
written for ground and is fatal for the one case a gatherer lives in. Measured: entities **149 -> 155**, `repaths` **432 -> 497**,
wall **+2.7%**, hash `8cf433ad4e0205f4` (the behaviour changed on purpose). `eight_villagers_ordered_to_one_tile_never_walk_
through_each_other` is green and off `#[ignore]`.

**What that leaves open, in this area:** the gather ring's seats are 0.66 apart against a 0.40 standoff, so the ink a crowd makes
is still wider than the original's; and `STALLED_TICKS` (40 = two seconds) is the wait before a blocked man re-tasks. Neither is
touched - one variable per slice - and both are now cheap to try, because a blocked crowd routes instead of standing.

## Now: P1b - the converter, once the save's world section is read (2026-11-16, D235)

**Landed (P1a): the engine's half.** `World::state(&pack)` writes the world as MessagePack (59,108 bytes, 180 us) and
`World::from_state(&pack, bytes)` reads it back (2,882 us), with the pack's tables installed from the pack's identity and the
two derived caches rebuilt. The round trip is **hash-identical immediately and after 100 ticks**; a cut state and a foreign
pack are both refused by name. `--state-out` in the harness writes one and reads it back through the engine.

**What P1b needs, in order:**

1. **Walk the save's sections 2-4** (`docs/save-format.md`): the scenario (`FUN_00545460`/`FUN_00567630`: civs, stockpiles,
   ages, disabled techs), the map (`FUN_00836830`), and the world's object loop (`FUN_00730600`: `pturn`, `numPlayers`, the
   objects). None of the three is walked today; the object records are only *found* per save (the 229-byte chains of
   `float32 x, y`) and their other fields are unknown.
2. **A `tools/` converter** that emits the state format above for the fields the save supplies, and says which it does not.
3. **`--load`** in the harness, taking a **pack directory** (`--config <dir>`) rather than eight files, because a state names
   the pack it was written under.
4. **A version stamp** in the document before any converter ships: a state from a newer build currently loads and drops the
   fields it does not know (`deny_unknown_fields` cannot be combined with `flatten`).

**Then P2**: 1,200 ticks from `brit7` against `brit8`, which is the oracle that settles T4, T7, T11 and T12 by comparison rather
than argument.

## Landed: the master list (R6, D234, 2026-11-16)

`World.bonus_tables` holds, per player, the rows the five consumers ask for - modifiers per attribute in application order,
kind-1 resource rows **with their source**, and the tech-tree bars - rebuilt on the two events that can change it (a tech
landing, the civ being set). All five consumers read it: `barred_by`, `civ_barred_unit`, `modifiers_for` (11 call sites),
`farm_food_bonus`, `construction_rate`. The acceptance hash is unchanged (`cb1f6a83a119a2a3` at 24,000), which is the check
that a refactor stayed one.

**Two things it deliberately did not settle, both still open:**

1. **What a bar inside a team bonus means.** Two of the pack's team bonuses carry a kind-102 row and the original's treatment
   is unread. The table takes bars from the tech tree only - what `barred_by` has always read - because folding every source
   answered the question by accident (and moved the acceptance hash to `051e29ec69087825`).
2. **Resolution to numbers keyed on the `Attribute` enum.** The table caches rows, not a folded value, because the five
   readers fold differently. That is the design's second half (`docs/civ-bonuses.md`, rule 2) and it now has one place to land.

**Cost, measured, and it is not a win.** The seam is **+2.6%** on the acceptance match at 24,000 ticks (21.71 s before against
22.28 s after; three interleaved pairs, oracles both built from source). Cloning every modifier per player and hashing a
`String` attribute each lookup costs more than the walks it saves. **Next slice, and it is cheap:** index each **effect** by
attribute once - the effect table is immutable for a match - and let a player's table hold **effect ids per source** instead
of a copy of the rows. Measure it before claiming it.


## A standing requirement on shape: a civilisation is config (D233, 2026-11-16)

*"I would like team bonuses and civ bonuses moddable in the future to be able to easily add a civ by adding/changing
config."* The bonuses stay deferred (D219); what this settles is the shape, and one half of it is enforced in the loader
already: the **attribute vocabulary is closed** (13 names the engine acts on, 16 the data carries unread) and a modifier
naming anything else is **refused at load, by name** - so a mod's civ cannot load and silently do nothing. The other half is
the **R6 seam** (`docs/civ-bonuses.md`): one resolved table per player, because five consumers already walk three effect
tables and a sixth is another place a new civ can be half-applied. Read that before adding any bonus consumer.

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
