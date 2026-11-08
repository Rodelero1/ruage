# Getting the AI out of the binary (D151)

*2026-11-07. The owner: "By extract AI I mean removing all the AI controls from the binary and moving it exclusively into ai
files. Do this first." And: "every unit control should be removed from game code and moved to AI."*

This is the inventory that has to come first: you cannot remove what you have not enumerated, and an engine "behaviour" that
is really a simulation rule must **not** be moved. Every entry below was read in the tree, not remembered.

## The line between the two

- **Simulation** — the world's own rules: movement and collision, pathfinding, damage, harvest rates, production timers,
  vision, death, rot. No choice is being made, so nothing moves.
- **AI control** — a **choice** made for a player: which node a worker goes to, where a building goes, which unit is trained,
  who a soldier fights, when a villager is redistributed. These belong in `config/ai/*.toml`, not in Rust.

The game's own AI agrees with that split: its scripts set `sn-*-gatherer-percentage` and **the engine assigns the villagers**
(`docs/ai-of-the-game.md`). So the engine keeps *mechanisms* and the AI keeps *decisions* — including the ones we have
hard-coded today.

## Inventory: choices currently made in Rust

| where | what it chooses | verdict |
|---|---|---|
| `sites.rs:432` `nearest_free_farm` | **which free tile a new farm is placed on** | **AI** — placement is a decision |
| `script.rs` `build{ where = "<resource>" }` resolution | which tile a building lands on, from a resource name | **AI** — the same choice, one level up; must become an explicit tile chosen by the AI |
| `sites.rs:196` `nearest_drop_site` | which drop site a carrier walks to | **AI** (the game's AI does pick drop sites) |
| `battle.rs:219` `nearest_enemy` | which enemy a unit engages on its own | **AI** |
| `battle.rs:268` `retaliate` | whether a unit turns on whoever hit it | **AI** |
| `step.rs:76` `resume_fights` | hunts starting fights, patrols turning, defensive units going home | **AI** (stance/patrol policy) |
| `step.rs:692` `resume_idle` | what a unit does when its job ends | **AI** |
| `research.rs:283` `grow_plots` | re-seeding farms when a tech lands | **AI** |
| `shares.rs:25` `step_shares` | which villager moves to which resource | **half** — the *table* is AI (already in the personality file), the *assignment* is the engine, which is what the original does |
| `gather.rs:17` `nearest_like` | a worker's next node when its own runs out | **engine** — the original's engine continues a villager on a nearby resource; this is the same mechanic, and D148/D150 bounded it |
| `sites.rs:117` `site_fits`, `site_refusal` | is a placement **legal** | **engine** — validation, never policy |
| `lib.rs:3103` `approach_tile` | where to stand to enter a building | **engine** — geometry |
| `build.rs:21` `settle_blind_sites` | cancelling a foundation that can no longer stand | **engine** — bookkeeping |
| `lib.rs:2024` `step_leashes` | how far an animal strays | **engine** — the animal's own rule |
| `livestock.rs:14` `step_conversion` | a herdable joining the first player who sees it | **engine** — a game rule, not a choice |
| `step.rs` `shepherd`, `stand_down`, `apply_switches` | carrying out an order already given | **engine** |

**Nine choices** to move (the bolded **AI** rows), two of them the placement ones the owner named — a farm's tile today, and
a building's tile through `where = "<resource>"`.

## The order, and why

1. **Placement first** (`nearest_free_farm`, `where = "<resource>"`): it is the one the owner named, it is needed by the gold
   loop (a mining camp next to a found pile), and it has a clean boundary — the engine keeps `site_fits`/`site_refusal`, and
   the AI chooses a tile.
2. **Then the gold loop**: AI scouts, remembers a sighting, asks for a camp tile by that sighting, trains villagers, assigns
   them. This needs one engine gap filled: a **build-at-a-tile** order, and a **sighting** query the AI can read.
3. **Then combat choices** (`nearest_enemy`, `retaliate`, patrol/stance in `resume_fights`), which are the most entangled
   with the tick's blow order (D88) and must move one at a time with tests.
4. **Then `resume_idle` and `grow_plots`**, the two "what happens after" choices.
5. **`nearest_drop_site`** last: it is inside the deposit loop, which every economy test depends on.

**`nearest_like` stays.** It is the engine continuing a villager on a nearby resource, which is what the original engine does
too; D148 (only looked-at nodes) and D150 (the look-around) already made it a rule rather than a cheat.

## How fidelity will be shown (the owner's "explain like I'm five")

Three simple checks, because we cannot run their game beside ours:

1. **Every rule says where it came from.** A rule transcribed from `Economy/Dark Age.per` carries that file and the values it
   uses, so a reader can open the file and see the same numbers. No rule without a source.
2. **We compare counts, not matches.** Their script says "28 villagers before Feudal", "31 then 40 through Feudal", "a market
   and a blacksmith for Castle with two builders each". We run our match and read our own villager count, market and
   blacksmith times against those numbers — like checking a cake by its layers when the original photo is gone.
3. **The same seed gives the same match, twice.** If our own AI cannot reproduce itself, it is not an AI yet.

**Pass condition, per the owner**: not victory. A **working economy** (villagers keep producing, resources keep banking) and a
**military training queue** that fills and empties on its own.

## Performance

Nothing was moved in this step — this is the inventory, and it changes no code path, so no measurement applies and none is
claimed. The programme's cost shape is known and small in the hot path: every listed choice runs **once per decision** (a
selection, a placement, a retarget), never per entity per tick, except `resume_idle` and `retask_workers`, which run on the
tick a job ends. Moving them to rules turns a compiled comparison into a rule evaluation, which is the one place this will
cost something — and the rule engine already evaluates every rule every few seconds in the acceptance match, so the budget
exists.

## Stability

- **Nothing changed**, so nothing can regress in this step. The risk lives in the moves that follow, and it is named per move:
  placement (the economy's farm ring depends on it), combat choices (D88's blow order), and `nearest_drop_site` (every economy
  test).
- **The inventory is checkable**: each row cites a file and line that exists today, and the "engine" verdicts are the ones
  that keep the simulation intact — moving `site_fits` or `approach_tile` to a rule would be a mistake, and the table says so
  before anyone tries.
