# The scenario: a scripted match

`config/scenario.toml` is the step-5 artefact and the scripted opponent in one. It is also the
replay format's first form: timed `Command`s, applied at fixed moments.

## Time is seconds

Every step has `at_seconds`, never a tick (D16). The runner multiplies by the match's own tick rate
when it is constructed, so the same file means the same match at 20 Hz, 60 Hz or 144 Hz. A script
that named ticks would silently change meaning the moment the rate changed.

## Steps name *what*, not *which id*

Ids belong to seeding order and would rot the moment the map changed, so a step names a role:

| field | means |
|---|---|
| `villager = 0` | this player's first villager (by entity id) |
| `resource = "wood"`, `nth = 1` | the second-nearest wood node to that villager |
| `role = "house"` | a unit, by its role in `economy.toml`'s `[role]` table |
| `building = "barracks"` | this player's finished barracks, also by role |
| `attacker = "spearman"` | this player's newest spearman, also by role |
| `target = "nearest_enemy"` | the closest enemy unit |

**Schema 2 names roles; schema 1 wrote dat ids** (`unit = 70`, `attacker = 74`). The role table is
the engine's whole vocabulary for units - the start of a match uses it too - so a scenario can be
replayed against a pack whose ids are entirely different, which is what "content" means here (D17).
An unknown role is refused with its name, and so is an unknown resource name.

`nth` exists for a specific reason: three villagers each told "the nearest tree" all pick the *same*
tree, exhaust its 200 wood, and idle. Telling them 0, 1 and 2 puts them on three trees.

A step that cannot be resolved — no such villager, no finished barracks, cannot pay — is
**reported**, not skipped quietly: `ruage-headless` prints `step at 471.0s refused: not enough
Food: 60 needed, 50 in the bank`. The test asserts every step in the project scenario ran.

## The ten-minute run

`cargo run -p ruage-sim --bin ruage-headless -- --config config/sim.toml --map config/map.toml \
  --economy config/economy.toml --units config/gamedata/units.toml --combat config/combat.toml \
  --script config/scenario.toml --seed 7 --report 60`

What it does, with the times it actually reaches rather than the times it asks for:

| time | event |
|---|---|
| 0 | three villagers on three trees, three opponent villagers likewise |
| 120 s | house ordered (25 wood) and finished 25 s later |
| 200 s | the first trees are bare; the villagers are re-tasked onto the next ones |
| 320 s | barracks ordered (175 wood), finished at ~370 s |
| 330–380 s | the villagers switch to forage and gold for the spearman |
| 500 s | spearman ordered (60 food + 20 gold, 21 s) |
| 525 s | the spearman is sent at the nearest enemy and walks the map |
| ~585 s | first blow; the villager dies 240 ticks later, inside the run |

The schedule was **measured**, not derived: three villagers bank about 0.75 wood per second once the
walk home is counted, not the 1.17 the dat's 0.39/s per villager suggests, and one forager banks
about 0.25 food per second. A schedule written from the dat rates alone lands short and every step
after it is refused in a cascade — which is exactly how the numbers here were found.

## What the script works around, and why it should not have to

Two engine limits shaped the script, and both are recorded as limits rather than features:

- **A villager idles when its own node runs dry.** There is no "seek the next node". That is why the
  script re-tasks villagers at 200 s, and why the comment in the file says so. When the engine
  learns to look for another node, those steps become unnecessary.
- **A node inside a cluster cannot be reached.** One re-tasked villager aimed at a tree surrounded
  by forest and was refused with `no walkable route ... the target is walled off` — correct
  behaviour by the pathfinder, and the step was removed rather than the check weakened.

## What it proves

- the whole slice runs: gather → build → train → fight, on the authored map, with data from the
  tables;
- the match is **deterministic** — the same script twice gives the same state hash, and a script
  with one step fewer gives a different one (the seed is irrelevant here: nothing draws from the RNG
  streams and the map is authored, so testing the seed would prove nothing);
- a refused step is visible rather than swallowed.
