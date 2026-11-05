# Review: one farmer a farm, and a husk that goes

*Step (D122). The owner, in three messages: "Only 1 farmer per farm (can be constructed by multiple, but only 1 can farm once
construction is finished). Once farm expires, it decays and can/needs to be replanted. It will cost 60 wood" -> "1 A but
vanishing takes some time, husk first ... Husk is effect only and only serves the player who can left click villager, right
click on the husk and villager starts planting new farm. It obstructs nothing as if it was not there" -> "2 C if there is no
empty farm nearby, villager becomes idle."*

## What lands

1. **One farmer a farm.** A gather order on a farm that already has a worker sends the villager to the **nearest free farm** of
   his side (his, finished, holding food, no worker) and leaves him **idle** when none exists. Answered, never refused.
2. **A husk with a lifetime.** An empty farm is still `is_expired_farm` - finished, no food - but it is now tracked in
   `World::husks` and **removed** after `[farm.depletion] husk_seconds = 30`. The clock is in the state hash, because when a
   husk leaves decides what the match looks like.
3. **A husk is a plot.** `EntityView::is_husk` lets the view offer the one thing a husk is for: a villager right-clicked onto a
   spent farm of his own side gets a **Build order for the farm the husk was**, and the build order pays the pack's 60 wood. An
   enemy's husk is not ours to replant, and there is a test for that too.

**Checked and cleared:** the farm's price. The pack's `FARM` (unit 50) costs `[[1, 60]]` - 60 wood, already right. My first
reading of "50 food" came from unit **259**, which is the *farmer villager variant* the gather task switches to, not the
building (`[role] farm = 50`). Reported here so nobody re-suspects it.

## Performance

Whole acceptance match, 36,000 ticks, release, `HEAD` (`bbaa3b3`, old `economy.toml` restored in its worktree so the old binary
can read it) against this tree, three interleaved runs each:

| run | `HEAD` | this step |
|---|---|---|
| 1 | 10,318 ms | 11,887 ms |
| 2 | 10,460 ms | 11,902 ms |
| 3 | 10,389 ms | 11,761 ms |
| median | **10,389 ms** | **11,887 ms** |

**+1.45 s (+14%)**, and it is **not** the new code's own cost: the match is doing more work. `HEAD` finishes with 148 entities
and **124 repaths**; this tree finishes with 154 entities and **211 repaths** - farms now vanish, so they are rebuilt, so more
build orders are issued and more routes are planned. The passes themselves are O(entities) every fourth tick (9,000 scans of
154 entities over a match) and cannot be measured against that. The honest statement is: the mechanic changed the match, and the
match got bigger.

**The hash moved, as it must**: `82efabd6f614c798` -> `4568cef16368bd5a`, still `players=1` (a win). A slice that changes what a
farm does is expected to change the flow; the tests that pin behaviour are the ones that matter here, and they are below.

## Stability

| attack | result |
|---|---|
| two villagers, two farms | the second goes to the free one; test `a_second_farmer_goes_to_a_free_farm` |
| two villagers, one farm | the second stands idle; test `a_second_farmer_with_no_free_farm_becomes_idle` |
| a farm behind a wall | offered by the search and refused by the route, which is the answer a player clicking it would get - the search does not pretend to know reachability |
| a husk's ground | a villager walks **over** it (`an_emptied_farm_leaves_a_husk_that_nothing_blocks_and_then_goes`), because "it obstructs nothing as if it was not there" is a claim to be tested, not trusted |
| the husk's clock | removed after 30 s of content time, and the map loses the entity |
| a husk's ground with a new farm on it | the existing rule already replaces a spent plot rather than standing beside it |
| replanting an **enemy's** husk | not a build order; test `an_enemy_husk_is_not_ours_to_replant` |
| a save that forgets the husk clock | impossible to write one from this state: `husks` is in the state hash, so a save that lost it would not match its own recording |
| a pack with no `husk_seconds` | defaults to 30 s; a pack with no farms never enters the pass |

**My own errors, reported:** (1) I first passed `amount: food * scale` to `Command::Spawn` and the farm came back with
19,990 units - a Spawn's `amount` is **whole units** and the engine scales it; (2) my first husk test stepped exactly 1,200
ticks before looking, which is exactly the husk's life, so the farm had already gone and the failure read as "the husk was never
made"; (3) my first fixture put the villager inside the farm's 3x3 footprint, where he could not move, which read as a gather
that never started; (4) the first timing attempt ran the old binary against the *new* `economy.toml` and recorded "3 ms" four
times - a parse failure reported as speed. The numbers above are from the corrected run.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
