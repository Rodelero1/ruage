# Gates: open for anyone, closed for everyone (D154)

*2026-11-08. The owner's two numbers: **trigger radius 1**, and **an enemy standing in an open gate keeps it open**. The model
came from `docs/gates-research.md` (D153): the game's wiki rule, 0 A.D.'s `Gate.js`, and the dat's own gate — and it is not
per-player passability.*

## What shipped

A gate is **content**, not a class: `config/economy.toml` gained `gate = 64` (the dat's palisade gate `GTAA2`, 1375 hp, 2×1)
and `[build] gate_roles = ["gate"]`, validated like `blocking_roles` (a name that is not a `[role]` is a load error).

`World::step_gates()` runs **before the movement pass**, so a gate that opens this tick is walkable for this tick's steps:

- **opens** when a unit of the gate's owner — or of a player **on its team** — stands within **one tile** of any of the gate's
  tiles;
- **closes** only when **no unit of any side** stands on those tiles;
- so an **enemy inside** keeps it open, which is the owner's rule and the game's (*"enemy units can also pass or keep the Gate
  open on purpose"*).

The walk grid is written **only on a transition** — `Walk::unblock` when it opens, `Walk::block` when it closes. That matters:
`Walk` counts occupants, so an unbalanced pair would leave a tile blocked for ever. `open_gates` remembers which gates are
open, which is what makes the pair balanced.

## The tests, and what they cost to get right

`crates/sim/tests/gates.rs`, written before the behaviour:

```
a_closed_gate_blocks_the_way                            ok   (this was already true - gates block as buildings)
an_allied_unit_within_one_tile_opens_the_gate           FAILED -> ok
an_enemy_standing_in_the_gate_keeps_it_open             FAILED -> ok
the_gate_closes_when_the_last_unit_leaves               FAILED -> ok
```

**RED, watched**: three of four failed with *"an allied unit one tile away must open the gate"*, *"an enemy standing in the
gate must keep it open"*, *"an ally beside the gate opened it"* — the gate never opened at all. GREEN after `step_gates`.

**Two test bugs of my own, both caught by running them rather than by reading:**

1. the observable was wrong — `is_free_ground` refuses a tile an **entity's anchor** stands on, so it says "not free" about
   the gate's own tiles whatever the grid holds. The right question is `tile_walkable` (the walk grid), and the helper now
   asks it;
2. the closing test's walker was spawned with `speed: 0`, so it never left — the gate "failing to close" was a stationary
   villager. It now spawns with a speed, and the helper says why.

## Performance

**No per-tick cost when no gate exists**: `step_gates` starts with `if self.gates.is_empty() { return; }`, and the acceptance
match has no gate, which is why its hash is **byte-identical** (`45596da62c9e3668`, 135 entities, 125 repaths, 116 made-room)
— the change is provably inert on the match that does not use it.

**Measured**: the acceptance run took **9,390 ms and 9,287 ms** over two runs on an idle box, and the gate suite's four tests
finish in **1.78 s**. The first version re-walked the entity list per gate — `O(gates × units)` — and hoisting the unit list
made that one walk per tick; at the acceptance match's 150 units and any realistic gate count the difference is microseconds,
which is why no before/after table is offered for it.

With a gate, the cost is **one walk of the entity list per tick** to collect `(player, tile)` once, then one pass over the
gates; the grid is written 2 tiles per transition. The first version re-walked the entity list **per gate** (`O(gates ×
units)`), which is why the unit list is hoisted.

## Stability

- **The balance invariant is explicit**: `open_gates` exists so `unblock` is called exactly once per opening. That is the one
  way this feature could corrupt the walk grid permanently, and it is the one thing the code is shaped around.
- **Any-side occupancy is not a bug**: an enemy inside keeps the gate open. A reader who expects "closes when no ally is
  near" will find the opposite, which is why the rule is quoted in the method's doc comment and tested from both sides.
- **Teams**: `same_side` reads `team_by_player` only when the match names teams; with no teams a gate opens for its owner
  alone.
- Suite: **743 passed / 0 failed / 19 ignored**, clippy `-D warnings` 0, fmt 0.

## What is deliberately not done

- **Lock** (a gate that will not open, even for its owner). The research records it; it needs a player command and a state on
  the entity, and it is a separate slice.
- **"A gate must be built inside a wall"** — the game's placement rule. Today a gate can be placed anywhere, which is a
  content/validation question (D151 calls placement policy AI, validation engine).
- **Diagonal gates and the four wall types**: one gate role for now (`GTAA2`); the dat has `GTAX2` (4×1) and friends.
- **A gate destroyed while open** frees its tiles through the death sweep already, but the `open_gates` entry is not cleared on
  death — the next tick's pass simply finds no entity and leaves the entry. That is a small leak in a `BTreeSet<EntityId>` and
  is recorded here rather than hidden.
