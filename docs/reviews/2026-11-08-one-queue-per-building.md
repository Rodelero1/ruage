# One queue per building: units and techs in order, with a bar (D157)

*2026-11-08. The owner: *"When units are trained in a building, there should be a training queue with visible progress. Same for
tech research. Units and tech can be queued in the building, but they are performed one after another not in parallel."*

## What was already true, and what was not

Training and research were **already timed** before this step - `train_time` and `research_time` from the pack × tick rate, with
`a_villager_costs_fifty_food_and_appears_after_its_create_time` and
`research_takes_the_techs_own_time_and_then_upgrades_what_its_owner_has` (Man-at-Arms at exactly 800 ticks) proving it. Nothing
was instant, so nothing needed "making slow"; the sim's 36,000-tick run already pays that cost.

What was missing was the **queue**:

- units had a queue (`self.queued`), techs did not - a second research order was **refused** with `AlreadyResearching`;
- a building could **train and research at the same time** - two independent timers, exactly the parallel behaviour the owner
  ruled out;
- the interface showed a **count** of pending units and nothing else: no progress, and no tech at all.

## What shipped

**`Job` - one queue, in order.** `self.queued` is now `BTreeMap<EntityId, Vec<Job>>` where
`Job::Train(Training) | Job::Research { tech }`. The running job still lives on the entity (`training`/`researching`), because
those are the timers the tick loop reads.

- `start_train` and `start_research` both **queue when the building is busy**, and validate first: a queued job is paid for
  **when it starts**, so a queue cannot spend money the player has not got yet (D66's rule, now applied to techs).
- `check_research` no longer refuses a busy building. It refuses the *same* tech twice - in this building's running slot, in
  this building's queue, or in another of the player's buildings - which is the double charge `TechInProgress` was always about.
- `start_next_jobs` runs at the **end of `step_production`**: any building with nothing running takes the head of its queue.
  A tech at the head starts; a unit at the head starts. A job that can no longer be paid for is **dropped with a refusal** in
  `last_refusal`, rather than silently skipped.
- `World::production(building) -> Production { running: Option<(QueuedItem, done, total)>, waiting: Vec<QueuedItem> }` - one call
  for the interface, so a HUD cannot draw three pictures that disagree.
- The queue's tag goes into the **state hash** (a queue holding unit 83 must not hash like one holding tech 83).

**The interface**: `EntityView.production` → `SelectedRow.production` → `ui::production_bar`, a 4-pixel bar under the health bar of
the selected building, filled `done / total` in `palette::PROGRESS`, with `+N` for the count waiting. An idle building draws an
empty track and no text.

## The bug this uncovered - mine, and the test found it

Removing the old "pop the next unit" from `spawn_from_building` left the **training slot never cleared**: it used to be cleared as
a side effect of being overwritten by the next queued unit. The result was a town centre that spawned a villager **every tick** -
the test reported **23 villagers** where it expected 4, twenty of them with sequential ids `9904..9923`. Clearing the slot
explicitly fixed it. This is the second time in this session that a "harmless" refactor of a queue lost an implicit invariant;
both were caught by counting entities rather than by reading the diff.

## The tests

**Engine** (`crates/sim/tests/production_queue.rs`, 3 tests): a unit then a tech, a tech then a unit, and two units plus a tech
taking **75 s rather than the longest 25** - each asserting the *time the thing lands* and the `production()` readback in between.
Also: a fresh match starts at the house cap (5/5) and a capped job freezes at "done", which cost one debugging round
(`pause=None`, `pop=5/5`) - the tests now build houses first, because a queue test that never builds a house tests the cap.

**One existing test needed to change**, and honestly: `a_barracks_trains_the_spear_line_only_as_far_as_it_is_unlocked` orders a
spearman and then a tech at the same barracks without stepping, so the tech now waits behind the spear and the helper's 45 s
budget was no longer enough. It now waits until the barracks has nothing pending - and that wait needed **one step first**,
because a command only lands on the next tick (`command_apply = next_tick`): asking `production()` immediately after the order
returns "nothing running" and the spear starts later, in the middle of the resource gathering.

**Interface** (`crates/view/tests/production_hud.rs`, 3 tests): the HUD's numbers are the queue's numbers after ten of
twenty-five seconds (`done` between 8 and 12 s, `waiting == [Tech(22)]`); the bar fills a quarter of a 100-pixel track in the
progress colour and leaves the rest in the track colour; and the bar reaches the screen inside the panel's selected zone.

## Performance

**The acceptance match changed, and that is the news**: hash `45596da62c9e3668` → **`a120ed0721c8768c`**, entities 135 → **150**,
repaths 125 → **88**, made-room 116 → **100**, and **both players alive at 30 minutes instead of one**. Serialisation is a real
gameplay change: a tech ordered behind units now waits, so the AI's research completes later and its army arrives later. The
numbers are recorded here rather than dressed up as an improvement.

**Measured**: the full gate took **365,094 ms** of test wall clock for **759 tests**, the new `production_queue` suite **4.8 s**
(three tests that each simulate 50-80 s of match time), and the interface suite **2.0 s**. The acceptance run stays near its old
cost: 36,000 ticks in the same ~10 s range, with fewer entities moving (88 repaths against 125).

**Cost**: `start_next_jobs` is one early return when no building has a queue (`self.queued.is_empty()`), and otherwise one pass
over the queue map comparing against the entity list's idle buildings. `production()` is called per entity in `Snapshot::of`, so
it is a `find` plus a `BTreeMap` lookup - not a search; the bar itself is two `fill_rect`s per selected card.

## Stability

- **A queue cannot double-charge**: the same tech in two places is refused at validation, and payment happens at start.
- **A queue cannot silently vanish**: a job that cannot start is recorded in `last_refusal`.
- **The hash covers the queue's kind**, so a unit and a tech with the same id cannot collide in a replay check.
- **The halt reason still wins**: `TrainingPause` (no room, population cap) freezes the *running* job as before; the queue behind
  it simply waits, which is what "one after another" means.
- Suite: **759 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.

## Not done, deliberately

- **The villager build card**: still the largest interface gap - a player can place a wall with `B` but nothing else, and the
  production bar is visible only for a building that already has a queue.
- **Cancelling a queued job** (right-click to remove the last one): the queue is visible now, so this is the next obvious
  affordance.
- **A tech's *name* on the bar**: the bar says `+1`, not "Loom", because names come from content in the window, not the view -
  the card could show it once the row carries the economy.
