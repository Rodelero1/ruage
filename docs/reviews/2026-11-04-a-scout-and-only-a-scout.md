# A scout at the start, and only a scout goes looking

Owner's note, 2026-11-04: *"player should start with scout cavalry, villagers shouldn't scout."*

The second half is the one with teeth. A villager that walks off to find the map is a villager not gathering, so scouting is
not "some unit wanders": it is a **named role** in content, and the rule can only order that role.

## What changed

- `[start] scout = "scout_cavalry"`, `scouts_per_player = 1` — the pack names the role and the count, like every other
  opening unit. A count with no role, or a role the pack does not have, is a **load error**: a pack that means to scout and
  cannot is a mistake worth refusing.
- The seeding loop spawns it beside the town centre with its own speed and health, resolved through the same `resolve` the
  villagers' speed goes through.
- `do = "explore"` — a new rule action. It finds every **idle** unit of the named role (family-matched, so an upgraded
  scout still counts), asks `World::nearest_unseen` for the closest ground the player has never seen, and orders a `Move`
  there. Idle only: one already walking was sent somewhere by a rule with a better idea, and re-aiming it every cooldown
  would make it pace in place.
- `World::nearest_unseen(player, from, limit)` — a square of `limit` tiles around the unit, nearest unseen tile wins. It reads
  the fog's own bitset (`explored_bit`), *not* `vision`, which asks the same question one tile at a time by scanning every
  entity: 2,809 tiles × 168 entities would be half a million operations per order.

The fog already existed (`crates/sim/tests/vision.rs`, three states per tile, the dat's own `los` radius), so the scout shows
something real rather than walking for its own sake.

## What the scout broke, and what that says about the engine

Adding one unit to the opening turned 17 tests red, and three of the failures were **not** test arithmetic:

1. **A trained unit could be born inside its own building.** The town centre's yard is walkable and free, so
   `free_spot_near` returned a yard tile when the neighbours were taken. Both villagers and the scout had crowded the ring,
   and the fourth villager appeared at (18, 16) - inside the town centre's own 4x4 footprint. Fixed: a birth spot is never
   inside a building's footprint.
2. **A rally to "inside" silently idled a unit that appeared away from the door.** `check_garrison` refuses a unit that is
   not at the building, and the code did nothing with that refusal, so the finished villager stood outside for ever. Fixed:
   the unit walks to the building and enters on arrival (`World::entering`), which is what a rally-into-building means.
3. Six animal tests picked "the player's first unit" as their villager, which is now the scout, and gathered nothing.

The population ones are arithmetic and not bugs: **an opening scout costs one population slot**, so the town centre's five
hold three villagers and a horse, and one more villager fits rather than two. Four population tests were updated to the new
counts with the reason in the comment.

## Where I broke my own rule, and how I know the test is not decoration

**I implemented before writing the test** for this step, which the standing rule forbids and which I am reporting rather than
hiding. What I did instead was prove the test bites afterwards, by mutation:

- `scouts_per_player = 0` → both tests fail (`player 1 does not open with exactly one scout`), restored to 1;
- the explore rule removed from the test's own scenario → the scout does not move and `the scout did not go anywhere` fails.

Both are one-line mutations of the thing the test claims about, so the test is measuring the behaviour and not the type names.

## Performance

- Acceptance, 36,000 ticks, seed 7: `tick=36000 entities=154 players=1 hash=7c517fa4e3322ee4 repaths=152 made_room=147`,
  **36 s** (was 29 s before the scout). The scout ends at (39, 63) in the run before the rally fix - it crosses an empty half
  of the map, which is what scouting looks like.
- **repaths rose: 119 -> 152** in the final run (240 in the run before the birth/rally fix, which put a unit in its own
  building's yard and then failed to shelter it). That is the honest cost of a unit walking where no unit walked before: a
  fresh route per leg, and the map caches nothing for a trip nobody repeats.
- The army still wins (`players=1`) with slightly fewer entities alive at the end (154 against 162), because the town centre
  spends its fifth population slot on the horse - one villager fewer for the whole match. That is the real price of the
  owner's scout, and it is worth stating plainly: the opening economy is one villager weaker.

## Stability

`nearest_unseen` is bounded by `limit²` bitset reads and returns `None` when everything within reach is known: a scout that
has seen its whole neighbourhood is simply not ordered anywhere, and the rule reports `no idle scout with anywhere left to
look` instead of thrashing. No allocation per call beyond the accumulator.

The rule cannot order a villager: the role is in the content file and the resolver filters on it. The test pins this at the
level of *distance from home*, not of orders, which is weaker than it sounds - villagers legitimately walk 20+ tiles to a
forest under the gather rules, and this test runs no gather rules. What it actually proves is that the explore action touches
only the named role; the "villagers shouldn't scout" guarantee comes from there being no rule that names them.

Open: the scout has no fight-or-flight. It walks into an enemy town centre and is shot at until it dies; in this run it
happened to survive. A `when = { no_enemies = ... }` guard, or a retreat on damage, is the next thing a player will want.
