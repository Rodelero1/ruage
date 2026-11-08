# Gates: what the game does, and what the open-source projects model (research only)

*2026-11-08. The owner: "Gates are more difficult than 'passable for owner and allies, solid for others'. Enemy unit can pass
together with your own. Please do mere research on the game and existing projects."* This file is research. **No code
changed**, and no design is decided here beyond correcting one sentence of D54.

## The game (Age of Empires II)

From the wiki's Gate page (ageofempires.fandom.com, read 2026-11-08):

> "Gates open automatically if a friendly unit approaches to let them through, and **only close again if no unit stands right
> under the Gate**. So, if friendly units (most notably Trade Carts) cause the Gate to open, **enemy units can also pass** or
> keep the Gate open on purpose. To prevent that, Gates can be **locked** so they will not open, even to friendly units."

and from the in-game description of the building itself:

> "Can be built into existing walls to allow movement of friendly units. Can be locked and unlocked. Cannot be converted by
> enemy Monks."

Other facts from the same page, each of which is a content number rather than an engine rule:

- **4×1 tiles**, built **inside** an existing stone or palisade wall, and "if they are placed over a wall piece or wall
  foundation, they will still get constructed".
- **70 s** build time; **30 stone**; **repair costs 45 stone** — one and a half times the build cost (the same asymmetry the
  town centre has).
- **Lockable while under construction** (since update 99311); unlockable at any time.
- Diagonal gates exist and can be rotated in DE, and DE fixed "side parts of diagonal gates being walkable during
  construction".
- Gate foundations had the finished building's armour in AoK/Conquerors and **0 melee / 0 pierce while under construction** in
  DE — i.e. a gate under construction is a soft target in DE, a hard one in HD.

**Player-side confirmation of the close rule** (Steam discussion, AoE2 HD general): *"Right now if an enemy gets one soldier
underneath an open gate, he can walk his whole army through it."* So the close condition counts **any** unit standing in the
gate, not only friendly ones — which is exactly the owner's point.

## The dat, from our own install

`config/gamedata/units.toml`, generated from `empires2_x2`:

| id | name | class | hp | los | radius | cost | dead_unit | train_time |
|---|---|---|---|---|---|---|---|---|
| 63 | `GTAA3` | **39** (gates) | 4000 | 6.0 | `[1.0, 0.5, 4.0]` | 30 stone | 144 | 70 s |

Class **39** is the gate class, and our `config/economy.toml` already records that class 39 is excluded from the conquest
condition ("gates (GTAA*, GTAB* …) excluded by the manual", line 363). There are 32 `GT*` entries — the variants are the
wall type the gate belongs to and the direction it faces.

So a gate is **a building**: it has hit points, it is built by a villager over 70 s, it leaves a ruin, and it projects sight.

## The closest open-source model: 0 A.D.

`binaries/data/mods/public/simulation/components/Gate.js` (0 A.D., **GPL-2+** — read for the model, not copied; we cannot
take its code). What it does, in its own terms:

- **State**: `opened`, `locked`, plus the list of allied units currently in range and an `ignoreList` of units that cannot
  move (they cannot trigger or hold a gate).
- **A range query** around the gate (`GetPassRange()`), restricted to **allied** players — only an ally's unit opens it.
- **`OperateGate()`**: close when `locked || !ShouldOpen()`, open when `ShouldOpen()`. The code's own comment states the
  close rule exactly as the wiki does: *"An ally must be in range to open the gate, but an unlocked gate will only close if
  there are no allies in range and **no units are inside the gate's obstruction**."*
- **Opening flips a global switch**: `SetDisableBlockMovementPathfinding(true, true, 0)` — both collision **and**
  pathfinding blocking are turned off for that obstruction; closing turns them back on. **It is not per-player
  passability.** Anyone can walk through an open gate, which is the same conclusion the owner reached.
- **Lock** closes at the next opportunity and stays shut even for allies; locking also destroys corpses under the gate so
  units do not try to gather an entity they cannot reach.
- **Placement**: 0 A.D. deletes entities that would sit inside a gate's obstruction when the gate is finished.

## openage

No gate component or module found in the sources reachable today (searches for "openage gate" return the wiki, forum threads
and unrelated projects). It is not a model to borrow for this.

## What this means for our engine

1. **D54's sentence is wrong and is corrected**: it says *"Not modelled: gates (owner-passable pathing needs a per-player
   grid)."* A gate is **not** per-player passability. It is a **time-varying global state**: while open, the gate's tiles
   are walkable for everyone; while closed, for nobody. That is `Walk::block`/`unblock` on the existing single grid (D54's
   writable layer), not a second grid per player.
2. **Three states, not two**: `closed`, `open`, `locked`. `open` is derived — an allied unit within the trigger radius — and
   the gate returns to `closed` only when **no unit of any side** stands on its tiles.
3. **The trigger radius is a number we do not have.** The wiki says "approaches"; 0 A.D. has `GetPassRange()`; the dat gives
   a gate LOS of 6 tiles. It needs a value, and it is a gameplay rule rather than something the data pins.
4. **Lockable, including while a foundation.** A locked gate is solid to its owner too, and the foundation must remember it.
5. **A gate replaces wall**: 4×1, placed over existing wall or wall foundation, and the wall segments it covers are gone (or
   never built) — with no refund of wall pieces already built.
6. **The hard part is invalidation**: a route planned through an open gate can become illegal the moment it closes. 0 A.D.
   dodges this by never closing while anything is inside the gate's obstruction — the same rule the wiki describes — which is
   cheaper than re-planning and is what the original appears to do.
7. **Two content numbers already exist** for free: repair at 1.5× build (45 vs 30 stone), and 0 armour while under
   construction only in DE (HD keeps full armour — our install is HD, so a foundation gate is armoured).

## Open questions this research cannot answer

- **Trigger radius**: 1, 2, or the gate's own LOS (6)? The data does not carry it.
- **Diagonal gates**: DE-only rotation; do we model one gate shape facing one way, or both?
- **A locked foundation**: the wiki says it can be locked while being built (a DE-era addition); is that in HD too?
- **Does a gate that is destroyed mid-passage free its tiles immediately?** Our death sweep frees a building's tiles already
  (D54), so the honest default is yes.

## Performance

Nothing changed, so no measurement of ours applies and none is claimed. The two costs this design implies are worth naming
now, because they are the only ones: **a proximity check per gate per tick** (0 A.D. uses a range query rather than a scan,
which is the model to follow — our engine already has `nearest_enemy`-style scans to avoid), and **a grid write when a gate
changes state** (`Walk::block` per tile, 4 tiles, on the ticks a gate opens or closes — never per tick while it stays open).

## Stability

- **No code changed**: this is research, and the only risk it carries is the one it removes — designing gates on a wrong
  premise. D54's "per-player grid" sentence would have sent the work down a dead end; the owner's correction, the wiki, the
  0 A.D. source and a player report all agree on the opposite model.
- **The sources are cited, and the licences are noted**: the wiki text is quoted as evidence about behaviour; 0 A.D. is
  GPL-2+ and is described here, not copied; the dat numbers come from our own generated pack.
