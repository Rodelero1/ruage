# Review: the 2000 row priced a rule the original does not have, and the two terrain bits cannot be named from this install (D328)

*Change: `Levels::step_cost` loses its `body: bool` parameter and its 2000 branch; the fixture that pinned it is re-specified; the doc now
names the three rows it does not carry and why. Two reads behind it: the grid the binary's 2000 keys on, and the terrain flag byte - the
second one blocked by a build mismatch that is itself the finding.*

## Read 1: what `[0x4d]` actually is

`FUN_007ed280` overrides any cost to **2000** when `*(char*)([0x4d] + x*4 + y) != 0`. `FUN_007ef3f0` (157 lines) fills and refines that grid:

- `[0x4c]` starts **all ones** and is **zeroed** as runs are consumed (`*([0x4c] + x*4 + y) = 0` inside the walk);
- `[0x4d]` is a grid of **eight one-bit direction masks** (`bVar8 & 1`, shifted `local_28` times over 8 slots, `0x01 << n` per side);
- for each tile with a non-zero mask it walks **maximal straight runs** in each of the 8 directions while the bit stays set and the tile's
  blocked byte agrees, allocating one 0x1c-byte node per run (`*node` = length, `node[1]` = blocked, `node[4..5]` = the run's end cell).

So it is a **run/visibility structure over the direction grid**, not a list of bodies. The only body-aware test anywhere in the path system is
`FUN_00801a20` - "a tile is passable when every body on it is in the path-ignore list" - and that is called from the **flat** search. **The
2000 row is not a body penalty**, and D298's sentence saying it was is wrong.

## Read 2: the terrain bits, and the wall

`FUN_0076f430(x, y)` → `*(DAT_00af541c + (FUN_0083ae40(x,y) & 0xff) * 4)`: a **table of terrain descriptors indexed by the tile's terrain
id**, which makes bits 3 and 6 of the descriptor's `+0x28` terrain flags rather than unit flags. `FUN_00842bf0` classifies every tile with
exactly those bits (`& 7` → class 1, `& 0x18` → class 2, `bit 5` → class 4), which is a **four-way terrain class**, not a cost.

Naming the bits needs the descriptor table read out of the binary, and that is where this install stops being the right binary:

- `DAT_00af541c` is **outside the image** of `Age2HD/AoK HD.exe` (`.data` ends at `0x9e3044`, `.reloc` at `0xa43794`; the table is at
  `0xaf541c`);
- the same string constants the corpus quotes are present but **shifted**: `"That's a path ignore unit"` is at corpus VA `0x9ea438` and at
  our VA `0x9e20f2` - a **0x8346** difference that is not constant across the image.

So the exe on this box is **a different build** from the one the corpus was decompiled from, and the flag words cannot be read from it by
address. Recorded rather than guessed: the rows stay unstated, and the reason is now "wrong binary", not "unknown rule".

## Test-first

`coarse_cost::clear_ground_is_ten_and_a_body_is_two_thousand` becomes `clear_ground_is_ten`: the `body = true → 2000` assertion is **removed,
not deleted silently** - its doc names the read that retired it, so the next person finds the reason where the rule used to be. Removing the
parameter then broke **five** call sites in the fixture, which is what a removal should look like (`E0061` before the production edit, green
after).

## Performance

- **Acceptance is bit-identical**: `c7191f531bd81b1b`, 141 entities, 7,125 repaths, 12.9 s. That is the proof the branch was dead - a live
  2000 row would have moved something.
- Suite and conformance: see the logs staged with this change. Nothing per-tick was added; one branch per expansion was **removed**.
- The two reads cost one 157-line function and one 116-line function plus a PE header walk (~40 lines of throwaway Python at
  `/opt/data/cache/scratch/read_terrain_table.py`).

## Stability

- The signature is simpler and there is no longer a parameter that must always be `false`; the cost table's remaining two rows are the ones
  the code can actually express.
- **The risk this leaves**: if the matching binary ever arrives, the 100/600 rows become readable and `step_cost` needs a third argument
  again (a terrain class, not a bool). That is named in the doc rather than left for a future reader to rediscover.
- **My own error, twice in one thread**: D298 recorded "a body on it costs 2000" from a partial read of the same function, and it survived a
  day because the row was inert. Both halves are now corrected in place (D327 in the levels doc, D328 here and in `step_cost`).
