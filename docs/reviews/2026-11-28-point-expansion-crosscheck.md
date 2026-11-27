# Point-expansion crosscheck: does the game shrink the mover to a point and grow obstacles by its radius?

*2026-11-28. Claim under test: "for pathfinding setup each unit/villager becomes a single
point and obstacles get an additional border added at half the moving circle's diameter
(i.e. radius)". Per-claim verdicts below, game leg first. Disagreements parked, not smoothed.
Owner scope note 2026-11-28: we are adopting the New System from DE — HD-exe findings
below are HD-only and may not transfer; DE behaviour stays open until read from DE.*

## 1. Game leg (first rung)

**Install absent.** `Age2HD/` is not present at the tree root (checked 2026-11-28; no
`Age2HD/` directory, no `AoK HD.exe` anywhere reachable), so no new binary read was possible.
What follows is (a) the binary readings already extracted in-tree, quoted as game observations,
and (b) the game's own data (`config/gamedata/units.toml`).

**The game's collision query is point-vs-grown-box, expanded by the mover's own radius.**
`docs/archive/research/movement-in-the-binary.md:521-531`: `FUN_00840830` builds the object's
box from its radius table (`*(float *)(obj + 0x20 + index*4)`, four direction indices, i.e. the
dat's radius quadruple), **grows the query box by the mover's radius** (`in_ECX+0x268`), and
collides on AABB intersection with `<=` (touching edges collide). The mover side of the test is
therefore its position plus its own radius against the object's box — algebraically the same as
a point against a box grown by the mover's radius.

**The movement update's query radius is the unit's own radius field.** Same file, lines
127-129: `FUN_008a92a0` → `FUN_00840440` queries at the new position with radius
`*(float*)(unit+0xc) +0x34) * param_7`, i.e. the unit's own radius field times a multiplier.
The multiplier (`param_7`) is 1.0 at both movement-collision sites (0.6 at the
waypoint-advance check) — read 2026-11-28, see §5.

**The game's own log string says the search plans for the unit's radii.**
Same file, lines 549-556: the route defect against the town centre is diagnosed with the game's
own words, *"the boundaries of where this unit's radii will go"* — the search answers for where
the unit's radii go, not where its centre goes. Arrival is box-to-box: `func_002690E0` computes
`max(0, |dx| - radius_x)` per axis then the hypotenuse, overlap collapsing to zero (lines 533-540),
and the owner's half-diameter rule is exactly range zero (lines 541-543).

**Terrain tiles are NOT grown — point test, no radius.** Same file, lines 484-487 and 517-519:
`FUN_008aa480` bounds-checks the tile, applies the unit's terrain passability mask, and asks
whether the tile's bits are zero. Arguments cast to `int` (tile indices); **no radius appears in
it**. So "obstacles get an additional border" holds for *object boxes in the collision query*;
terrain passability is a point-in-tile test. Whether the original's *search* (`FUN_008007f0`)
grows blocked terrain tiles by the mover radius is settled — see §5: it grows nothing.

**Game data: the radii in question.** `config/gamedata/units.toml`: villager `VMFAR` id 259
carries `radius = [0.2, 0.2, 2.0]` (line 6975); tree `TREEA` id 399 carries
`radius = [0.5, 0.5, 2.0]` (line 10535), i.e. its box is its tile. A villager's "half the moving
circle's diameter" is therefore 0.2 tiles in this data.

## 2. Deck leg (published source, second rung)

Raymi Klingers (Forgotten Empires), "25+ Years of Pathfinding Problems with C++", Meeting C++
2025, 104-page PDF (local text extraction 2026-11-28; figures viewed as rendered pages):

- **PDF p. 22**: "Units and buildings are axis aligned rectangles in AoE."
- **PDF p. 23** (original algorithm, step 2): **"Shrink A to a point and expand all other
  obstructions."** Figure: solid rectangles (original footprints) each inside a dashed expanded
  outline; A drawn as a labelled point, B as a small circle.
- **PDF p. 65**: "Calculate intersection points of rectangles. **This is compatible with the
  radius expansion approach.**" Only place in the deck that names the width quantity: *radius*.
- **PDF p. 83** (replacement algorithm, step 1): **"Shrink ourselves to a point and expand the
  obstructions."** Same figure as p. 23.

The deck never states the expansion width as a number or as "half the diameter". Its whole
vocabulary on the point is: *shrink to a point / expand obstructions / radius expansion
approach*. "By A's radius (Minkowski trick)" is this project's gloss
(`docs/archive/research/pathfinding-from-the-talk.md:12`), not a deck quote.

## 3. Web leg (third rung)

- **openage reverse-engineering notes** (`SFTtech/openage`,
  `doc/reverse_engineering/game_mechanics/pathfinding.md`, after Pottinger/Shelly 1999): AoE2 uses
  three pathfinders (mip-mapped hierarchical long-range on passability ignoring moving objects;
  tile-based short-range; polygonal convex-hull for combat and crossing paths) plus two
  obstruction systems. Corroborates a hull-based short-range layer; says nothing about
  point-vs-expansion.
- **kindatechnical.com, "Configuration Space and Minkowski Sums"** (generic planning theory, not
  game evidence): "For a disc centred on the reference point… `O ⊕ R` is the set of points within
  distance r of `O`… Growing every footprint by the robot's radius and planning for the centre
  point is the whole method for a round robot." Key takeaway verbatim: **"the radius and not the
  diameter"**. Directly matches the claim's parenthetical; carries no game weight.
- **Dre Dyson Godot RTS tutorial** (tertiary blog retelling the Klingers talk): "expand all
  obstacles outward by the unit's radius (called the 'margin' or 'Minkowski sum')". Retelling only.

## 4. Per-sub-claim verdicts

| Sub-claim | Verdict | Basis |
|---|---|---|
| Each unit/villager becomes a single point for pathfinding setup | **Confirmed** | Deck pp. 23, 83 ("Shrink A to a point"); game's query is position + own radius vs object boxes (`movement-in-the-binary.md:521-531`); game's search plans for "where this unit's radii will go" (:549-556). |
| Obstacles get an additional border | **Confirmed for object boxes, not for terrain** | Deck pp. 23, 65, 83 ("expand…", "radius expansion approach", dashed figures); game grows the query/object boxes in `FUN_00840830`. But `FUN_008aa480` tests terrain at a point with no radius (:484-487) — the border does not apply to the ground test. |
| The border is half the moving circle's diameter (i.e. its radius) | **Consistent, not deck-stated** | Deck's only width word is "radius" (p. 65); game's query base is the unit's own radius field (:127-129); theory source states "the radius and not the diameter" explicitly. No source writes "half the diameter" or any number; the equivalence is arithmetic, and the binary's `param_7` multiplier is 1.0 (see §5). |

## 5. Parked questions: settled 2026-11-28 from the binary (read-only)

Binary: `/app/Age2HD/AoK HD.exe` (32-bit PE, base `0x400000`; build per embedded
PDB path `Age2HD_Archive_5.7_with_Hotfix`). This worktree has no `Age2HD/`, so the
reads below are `rizin` disassembly of that install — nothing copied, nothing
written back. Addresses are virtual addresses; in `.text`
(file offset = VA − `0x400c00`).

1. **SETTLED — the search grows nothing by the mover radius.** `FUN_008007f0`'s
   neighbour loop (`0x80148e`–`0x8015ac`) tests each single tile twice: a stamp
   test (`test [tile],[ebp-0x45c]`, e.g. `0x801404`) with mask `~passability |
   0x4000` (built at `0x800a8c`–`0x800ad0` via two `FUN_008427c0` calls), then the
   ignore-list body test `FUN_00801a20(x=esi, y=ebx)` (six sites
   `0x80149f`…`0x80158d`; returns 1 = blocked). No `0x840830` / `0x840440` /
   `0x843410` radius call exists inside `0x8007f0` — only the `0x8427c0` mask
   fetches. The one radius-grown box in the search is the *goal* rect
   (`0x80093d`–`0x8009b3`): target pos `[edx+0x3c]/[edx+0x40]` ± the *target's
   own* def radii `[[edx+0x0c]+0x34]/[+0x38]`, minus `0.01`
   (`[0x9b0ac8]` = `0x3c23d70a`), tested at `0x800e53`–`0x800e77`. The mover's
   radius is never read in the search (the only `+0x34` reads in the function
   are `0x800961`/`0x800966`, on the target's def). So blocked terrain tiles are
   single-tile stamp tests, bodies block by single-tile occupancy, and no
   Minkowski expansion happens at search time. The expansion the game *does* have
   lives in the movement collision query (`FUN_00840830` grows the query box by
   `[ecx+0x268]` at `0x8405a4`–`0x8405bb`).

2. **SETTLED — the multiplier is 1.0 at the two movement-collision sites (0.6 at
   a third).** `FUN_008a92a0` loads the unit's own radius (`0x8a930b`:
   `eax=[esi+0x0c]`; `0x8a9315`: `xmm0=[eax+0x34]`) and multiplies by
   `[ebp+0x20]` (`0x8a931a mulss`), storing the product via `FUN_00843410` into
   `[ecx+0x268]` (`0x843421`) — the field `FUN_00840440` grows its box by. All
   three `FUN_008a92a0` callers in `FUN_008a4360` push a float literal there
   (7 args each, `add esp,0x1c`; `0x8a2610` is cdecl, `ret` at `0x8a2674`, so the
   literal survives as `[ebp+0x20]`): `1.0` (`0x3f800000`) at `0x8a69af` for the
   straight-line check (`0x8a69d6`, after `"Looking for a STRAIGHT LINE path:"`)
   and at `0x8a7c05` for the new-position check (`0x8a7c25`, before `"We have %d
   ObSystem collisions…"`), and `0.6` (`0x3f19999a`) at `0x8a6e9d` for the
   waypoint-advance check (`0x8a6ed2`, terrain query skipped,
   `[ebp+0x1c]=0`). Query radius = unit radius × 1.0 = the radius itself.

3. Answered from the `/app` install, not this worktree (no `Age2HD/` here) and
   without the REA corpus — pure `rizin`/`rabin2` disassembly, read-only. HD questions
   settled; DE New System behaviour remains open (owner scope 2026-11-28).

## 6. DE New System (read 2026-11-28 from the DE exe, read-only)

Binary: `/app/AoE2DE/AoE2DE_s.exe` (64-bit PE, image base `0x140000000`,
compiled 2026-10-14 per header, PDB path
`F:\Agents\PhoenixBuilder-07\p4\portland_group\Build\Steam\Game\Retail\Output\AoE2DE_s.pdb`).
All reads below are `rabin2`/`rizin`/raw-byte reads of that install; nothing
written into `/app/`, nothing copied into the repo. VA mapping used:
`.text` fileoff → VA = fileoff − `0x400` + `0x140001000`;
`.rdata` fileoff → VA = fileoff − `0x3195C00` + `0x143197000`.
A one-pass LEA-rip scan of `.text` (140,113 RIP-relative LEAs, 79,634 unique
targets) plus raw `E8 rel32` call-target scans supply the xrefs.

### (a) DE SEARCH: terrain tiles are point-tested, no mover-radius growth visible

The closest DE analog of HD `FUN_008aa480` (terrain point test) + the search
tile loop is the `ActionGather.cpp` routine at **`0x140b7b300`**
(assert `__FILE__` LEA at `0x140b7b37a` → VA `0x143224e50`,
`"…\Aoc\Source\Game\ai\Actions\ActionGather.cpp"`, `r9d` = `0x42a` = 1066):

- bounds: float pos → int tile (`cvttss2si` at `0x140b7b339`/`0x140b7b34b`),
  `cmp edi,[global+0xa0]` / `cmp ebx,[global+0xa4]` map width/height
  (`0x140b7b35e`/`0x140b7b36a`); out-of-range returns 0 = blocked;
- passability: `movzx edx,word [obj+0xa6]` → table lookup `call 0x14111f580`
  (`0x140b7b3c6`/`0x140b7b3cd`; that helper reads `[rcx+0x90]` → `0x1416027b0`
  → `dword [rax+rbx*4]`, i.e. per-unit-type passability row);
- occupancy: `xor eax,0xff; imul ebx,[rcx+0xa0]; add ebx,edi`
  (tile linear index), then **`test qword [rcx+rdx*8], rax`**
  (`0x140b7b3f3`) — a single-bit test in a per-tile 64-bit mask array;
  `jnz 0x140b7b57d` (return 0 = blocked).

No `addss` of any mover radius onto tile bounds exists in this routine: the
tile test is bounds + passability-word + one bit test — structurally the same
point-in-tile test as HD's `FUN_008aa480`. One scaled float appears just after:
`movss xmm2,[rax+0x78]; mulss xmm2,[0x14369ff60]` (`0x140b7b441`/`0x140b7b446`)
where `[0x14369ff60]` = `0x3F733333` = **0.95** — but `rax` there is a
string-decrypt-sequence pointer (the `add rax,0x65141ee66b383496` /
`add rax,0x4bf9501e7914af14` idiom used before every decrypted literal), so
what `+0x78` is cannot be resolved statically; recorded as open, not as a
radius.

The `MapAnalysisSystem.cpp` search/LOS walk (assert at **`0x1410598cc`** →
`"…\Aoc\Source\Game\Map\MapAnalysisSystem.cpp"`, `r9d` = `0x3ae` = 942;
function starts `0x141059760`) walks tile indices `r14`→`r15` against the tile
count (`call 0x141072970` at `0x141059920`), fetches 12-byte tile structs via
the bounds-checked accessor **`0x141076260`** (`[rcx+0x438]`/`[rcx+0x440]`
array, `lea rax,[r9+rax*4]` after `lea rax,[r8+r8*2]` = 12-byte stride), reads
tile positions as doubles (`movsd` at `0x141059a13`), walks the segment in
fixed steps (`divss xmm1,xmm8` with `xmm8` = **7.5** from `0x14329c728` =
`0x40F00000`, at `0x141059a75`; per-step blocked test `call 0x1410729e0` at
`0x141059a97`). The stepping/sampling math is point-based; no mover-radius
term appears anywhere in the readable stretch. The two decisive callees —
per-pair segment test `0x14111d8e0` (called `0x141059990`) and per-sample
blocked test `0x14111d0c0` (called `0x1410599ce`, returns bool in `al`,
`cmp al,1`) — sit in high-entropy pockets (7.1 bits/byte over 256 B vs
5.4–5.6 for surrounding real code) and disassemble as garbage: **function-level
at-rest encryption**. So the terrain verdict is *readable-code* strong, with
the final bool buried in an encrypted helper — the same hedge as HD §5's
"settled" claim cannot be fully closed on DE.

Corroborating point-geometry: the MapAnalysis closest-point routine at
`0x1410d1b14` (stack-cookie prologue; calls tile-count `0x141072970` at
`0x1410d1b62` and tile accessor `0x141076260` at `0x1410d1ba7`/`0x1bb9`/`0x1cd5`/`0x1ce4`/`0x1d19`)
projects the query point onto every tile-pair segment (clamped `t` via
`comiss`/`minss` against 0.0/1.0, `0x1410d1c1e`–`0x1410d1c30`), keeps the min
**squared** distance from `FLT_MAX` (`0x7F7FFFFF` at `0x14369ffdc`,
`0x1410d1b4e`), adds accumulated length and returns it (`0x1410d1d56`–`0x1d5b`).
Pure point-vs-segment distance; no radius is added or subtracted.

### (b) DE movement/collision query radius: NOT recovered — open

`motion.cpp` (`0x1432a3070`), `PathingSystem.cpp` (`0x14329cec0`),
`zone_map.cpp`/`TerrainRestriction.h` (`0x1432d82d0`/`0x1432d8240`), and every
log-format string probed (`Checking area…collisions`, `NewWaypointBehaviour`,
`Pathfinding`, `SyncObstruction`, `Collision with Unit`,
`mFriendlyCollisionCircleScale`, influence-map logs) have **zero** LEA-rip refs
and zero absolute/RVA pointer occurrences anywhere in the exe: DE's retail
logging/attribute names are data-driven (hashed names) or compiled out, so
string-led discovery fails completely on DE. No HD-style `radius × multiplier`
load (`movss [reg+0x34]; mulss`) was found in any readable movement-adjacent
code, and no unit-radius field offset was identified (struct layouts differ;
`+0x34`-style HD offsets do not transfer). The DE unit attribute system names
(`mFriendlyCollisionCircleScale`, `sn-dock-avoidance-factor`,
`up-get-path-distance`, `xsSetUnitPosition(..., checkCollision)`) prove a
collision-circle + explicit `checkCollision` gate exist, but the numeric
radius × multiplier is unreadable statically. OPEN.

### Biggest HD-vs-DE structural differences (do not force HD addresses onto DE)

1. **Function-level at-rest encryption of game-logic code.** Readable and
   garbage regions interleave at ~100-byte granularity (e.g. clean search walk
   `0x141059760`+ calls directly into encrypted `0x14111d0c0`/`0x14111d8e0`/
   `0x1410729e0`; a clean 8-instruction helper at `0x14111f580` is immediately
   followed by garbage at `0x14111f5b0`). `aaa` (26,131 functions) leaves gaps
   exactly over game-logic spans (`0x141057310`→`0x14105a520`,
   `0x141060670`→`0x14106ae10`, …). HD had no such pockets.
2. **New System module layout.** Embedded `__FILE__` paths give the DE
   authoritative structure: `Source\Game\Map\PathingSystem.cpp`,
   `Source\Game\Map\MapAnalysisSystem.cpp` (+ `RGE_Map_Analysis::GameInterface`
   callbacks taking `BaseWorld` + `vector<TerrainRestriction>` + `BPath`
   lambdas), `Source\Game\Map\map.cpp`, `Source\Game\objects\motion.cpp`,
   `Source\Game\objects\Types/{MovingObject,MasterMovingObject,…}.cpp`,
   `Source\Game\Terrain/{zone_map.cpp,TerrainRestriction.h}`,
   `Source\Game\ai\Searching\aisearch.cpp` — vs HD's flat `FUN_` soup.
3. **All HD log anchors are gone.** `Looking for a STRAIGHT LINE path:`,
   `We have %d ObSystem collisions`, `the boundaries of where this unit's
   radii will go` have zero occurrences (narrow and wide) in DE. DE's
   observable vocabulary is instead: influence maps
   (`Adding impassable influence for Group %d…`, `SyncObstruction`),
   `NewWaypointBehaviour` / `InstantWaypoints` / `PathingLine`,
   `MapTile…`/`xsGetMapTileAttribute`, and hashed attribute names.
4. **Tile data model differs.** DE tiles are 12-byte structs in a
   bounds-checked array (`0x141076260`), occupancy is a per-tile qword bitmask
   array indexed by linear tile id (`test [rcx+rdx*8],rax`), and positions are
   doubles — vs HD's byte-tile stamp test + `FUN_00801a20` ignore-list body
   test. The point-in-tile verdict transfers; the mechanism does not.

### Still open (DE)

- Whether the *object-box* side of DE search/collision grows boxes by the
  mover's radius (Minkowski) or tests point-vs-box: the segment/blocked-test
  callees (`0x14111d8e0`, `0x14111d0c0`, `0x1410729e0`, `0x14111c660`) are
  encrypted at rest; needs a running process (decrypted `.text` dump) or
  dynamic tracing — explicitly beyond this static pass.
- The movement-query radius field offset and multiplier (0.95? 1.0? something
  else): `motion.cpp` code paths unidentified; the single 0.95-scaled float at
  `0x140b7b446` is unattributed.
- `PathingSystem.cpp` emits no referenced `__FILE__`/assert strings at all in
  `.text` (0 LEA refs) — its code was not located; only its name and the
  `RGE_Map_Analysis` callback shape are known.

## 7. DE live-dump (read 2026-11-27 from the running process, decrypted)

Source: live minidump `/app/AoE2DE/AoE2DE_s.DMP` (MiniDump, 18 streams; read-only,
nothing written into `/app/`, no game bytes copied into the repo).
Module `AoE2DE_s.exe` base in-dump `0x7ff726a70000` (size `0x4f24000`; ASLR-relocated
from image base `0x140000000`). `.text` carved from the dump's Memory64List
(5,510 runs): VA range `0x7ff726a71000`–`0x7ff729c06000`, 99 overlapping runs,
**full `0x3195000`-byte coverage**, zero gaps — scratch only in `/tmp/dedump/`
(`AoE2DE_s.text.bin`).VA mapping: file VA = dump VA − `0x7ff5e6a70000`;
i.e. the §6 VAs apply unchanged. Control: the §6-readable regions are
byte-identical dump-vs-exe (`0x141059760` search walk, `0x140b7b300`
ActionGather incl. ±32 B around `0x140b7b446`) — never encrypted. All four
§6-encrypted callees plus `0x141116800` and `0x141078af4`±64 B **differ from
the exe at rest** — they were function-level encrypted and are now decrypted.
objdump cites below are against the carve rebased at file VA `0x140001000`.

### (a) Object-box side: SETTLED — conditional Minkowski growth, not point-vs-box

All three decrypted box-test variants share one idiom — `cmpb` a flag byte at
ctx+`0x57`, and when set, grow the tested box by the float at ctx+`0x58`
(`subss` the two min corners, `addss` the two max corners):

- `0x14111d0c0` (per-sample blocked test, called `0x1410599ce`, returns bool in
  `al`, `ret` at `0x14111d544`): growth at `0x14111d1b4`–`0x14111d1d2`
  (`cmpb $0x0,0x57(%rdi)`; `movss 0x58(%rdi),%xmm5`;
  `subss %xmm5,%xmm4/%xmm3`; `addss %xmm1,%xmm6`; `addss %xmm2,%xmm5`).
- `0x14111d8e0` (per-pair segment test, called `0x141059990`, `ret` at
  `0x14111da02`): growth at `0x14111d91f`–`0x14111d94e`
  (`cmpb $0x0,0x57(%rcx)`; `movss 0x58(%rcx),%xmm1`; same subss/addss shape).
- `0x14111c660` family (box test taking explicit bound args into
  [rcx+`0x7c`/`0x80`/`0x84`]): growth at `0x14111c785`–`0x14111c7a3`
  (`cmpb %r15b,0x57(%rcx)`; `movss 0x58(%rcx),%xmm6`; subss/addss), plus the
  margin recompute at `0x14111cd27`–`0x14111cd34`: `movss xmm0,[rbx+0x84]` /
  `mulss [rbx+0x58],xmm0` / `movss [rbx+0x58],xmm0`.

So the hypothesis "point-vs-grown-box" is half wrong: the query keeps its own
geometry (segment endpoints min/maxed, `minss`/`maxss`) **and** the obstacle box
grows by the margin — box-vs-grown-box. When the flag is 0 the test is
ungrown; both phases exist back-to-back in ActionGather (see (b)): grow+test,
then reset. Terrain tiles are still point-tested (unchanged `test [rcx+rdx*8],rax`
at `0x140b7b3f3`, unencrypted region). `0x1410729e0` decrypts as a
bounds-checked tile-array accessor (12-byte stride math + call), not a blocked
predicate — the §6 "per-step blocked test" label for it was wrong; the per-step
verdict now rests on `0x14111d0c0` directly.

### (b) Movement-query radius: FOUND (field +0x78 × multiplier; identity open)

The HD analog (`movss [reg+0x34]`; `mulss`) exists as
`movss xmm2,[rax+0x78]` / `mulss [0x14369ff60]` at `0x140b7b441`/`0x140b7b446`,
where `[0x14369ff60]` = `0x3F733333` = **0.95** (read from exe `.rdata`,
fileoff `0x369eb60`); neighbour `[0x14369ff64]` = **1.0**. The product feeds
the margin setter `0x141121350` (`movss [rcx+0x58],xmm2`;
`mov [rcx+0x57],dl`; `ret`) — called at `0x140b7b44e` with `dl`=1
(`mov $0x1,%dl` at `0x140b7b401`), i.e. margin = [X+`0x78`] × 0.95, grow
enabled — then box test `0x14111c660` at `0x140b7b4cc`, then reset via the same
setter at `0x140b7b4dd` with `edx`=0 (`xor %edx,%edx`; margin 0.0, flag 0).
In the MapAnalysis search walk the setter is called unscaled:
`movss xmm2,[rax+0x78]` at `0x1410598be` → `call 0x141121350` at `0x1410598c3`
(`dl`=1, set `0x14105989b`), second call at `0x141059bbf`. Callers of the setter
(15 found: `0x140b7b44e`, `0x140b7b4dd`, `0x1410598c3`, `0x141059bbf`,
`0x1410612f8`, …) and of `0x14111c660` (15: `0x140b7b4cc`, `0x1410618b0`, …)
span ActionGather, MapAnalysis and movement code — margin plumbing is global
(ctx singleton `[0x144357fe0]`), not per-site.

### (c) PathingSystem.cpp: FOUND — 5 LEA refs in decrypted code

The string at `0x14329cec0` has 5 RIP-relative LEA refs, all assert sites
(`lea r8,[PathingSystem.cpp]` + `mov r9d,line` + call assert helper
`0x14111f580`): `0x141078af4`, `0x141078c51`, `0x141079264` (line `0x431`=1073),
`0x1410792c0`, `0x14107a49e` (line `0x33b`=827). Enclosing function starts
≈`0x141078690` (verified `mov %rsp,%rax` / push prologue). Bonus from the same
LEA scan (142,729 unique targets): `motion.cpp` (`0x1432a3070`) has 1 ref at
`0x141116897` (line `0x6f4`=1780, function in decrypted region ≈`0x1411166ed`+);
`zone_map.cpp`/`TerrainRestriction.h` refs at `0x1414eb3b2`…/`0x1414e8cee`….
`0x14369ff60` (the 0.95) has 0 LEA refs — it is only ever reached by RIP-relative
`mulss`/`movss`, as expected.

### What changed vs §6 static reading

1. The "still open" object-box question is closed: Minkowski growth confirmed,
   conditional on ctx+`0x57`, margin ctx+`0x58` — but it is box-vs-grown-box,
   and the margin is per-call-site (0.95-scaled in ActionGather, unscaled in the
   MapAnalysis walk), not a single global rule.
2. The 0.95 float is attributed: the ActionGather margin multiplier
   (`0x140b7b446`), with the HD-analog field load (`[rax+0x78]`) directly above
   it (`0x140b7b441`). The §6 "unattributed" hedge is lifted for the mechanism.
3. `PathingSystem.cpp` and `motion.cpp` code regions are located (assert LEAs),
   closing the "0 LEA refs" finding — it was an encryption artefact, not absence.
4. Correction: `0x1410729e0` is a tile accessor, not the per-step blocked test.

### Still open (DE, after live-dump read)

- Identity of the +`0x78` margin-base field: its pointer chains pass through the
  string-decrypt add idiom (`[rsi/rcx+0x10]` + `0x65141ee66b383496` +
  `0x4bf9501e7914af14`, e.g. `0x140b7b418`/`0x140b7b42d`, `0x14105989d`/`0x1410598aa`),
  so whether it is the mover's own radius (vs another radius/attribute) is not
  statically attributable — needs struct recovery or a watchpoint on +`0x78`.
- Why 0.95 at the ActionGather site vs unscaled in the MapAnalysis walk, and what
  the +`0x84` multiplicand (`0x14111cd27`, caller-passed; 1.0 stack slot at
  `0x140b7b479`–`0x140b7b490`) is at each `0x14111c660` call site.
- Dynamic confirmation (arrival/touch behaviour, which flag phase wins) still
  needs tracing a live process, not this static carve.
