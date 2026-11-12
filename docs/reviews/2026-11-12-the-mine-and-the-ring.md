# The mine, and the ring all three share

**A measurement, not an engine change.** No rule moved; the acceptance run is untouched.

## Performance

`tools/savegame_positions.py` reads the whole inflated body (3.3 MB) offset by offset and prints the radii: **1,035 ms** per save
on this box, run once per save rather than per tick. Nothing in the simulator changed, so the acceptance baseline stands at
`hash=ef7cdba92a83f173`, 142 entities.

## The measurement

| node | save | men on the ring | radii | mean | closest pair |
|---|---|---|---|---|---|
| tree | `test.aoe2spgame` (2nd) | 6 | 1.105-1.189 | **1.156** | 0.404 |
| bush | `bush.aoe2spgame` | 10 | 1.100-1.174 | **1.143** | 0.400 |
| gold mine | `mine.aoe2spgame` | 8-9 | 1.041-1.188 | **1.114** | polluted |

The owner: *"tree, mine and berries are all the same in that aspect."* The data agrees - all three means sit inside 1.11-1.16, and
`GATHER_RING_MILLI = 1.156` (taken from the tree) is inside all three bands.

## Stability

**The tool had a hidden assumption and the mine save broke it.** The first version required the **229-byte stride** the bush save
chains its records at; the mining save lays them out differently (gaps of 68, 30 and 192 in the same window) and the tool reported
*"no run of object records"* on a save that plainly has them. It now reads **every float pair in the window** and collapses copies
nearer than 0.2, reporting the stride it saw as a note - simpler, and save-independent. The measurement never needed the stride;
only the diagnosis did.

**Why the mine's closest pair is not a spacing.** That save stores each object's position **more than once** (copies 0.10 apart -
seen as the 68- and 30-byte gaps). Collapsing at 0.2 leaves survivors that can still be one body, so its 0.241 is an artefact. Its
**radii** are the number that can be trusted, and they match the other two.

**What this does not settle.** Our engine's cap is **nine** and the game put **ten** on a bush; `2 * pi * 1.143 / 0.400 = 17.9` says
the ring holds eighteen. The gap is our packing (the settle and the collision pass fight at 0.345 against 0.40 bodies), not the
game's rule, and it is written down in `docs/next-step.md` as the next attempt to make.
