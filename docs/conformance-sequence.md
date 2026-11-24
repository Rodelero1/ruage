# The road to 100%: the ordered steps, and what "100%" can and cannot mean

*Requested 2026-11-21: the order of steps needed for 100% conformance on pathfinding and obstacle avoidance with the
original, crosschecked against online sources. Sources read for this document: `docs/parity-plan.md` (the live register),
`docs/pathfinding-*.md`, `docs/movement-in-the-binary.md`, the HD corpus (`/opt/data/tools/rea/pseudo-aokhd`), the AoC
decompilation (`src/zone/`), and four online sources named in §2.*

## 1. What "100%" can mean, in three classes

The owner's target is "blunt copy". Three kinds of claim get confused under it, and only the first is reachable:

- **R - reachable from the sources in hand.** A rule is read (corpus line, decomp module, or first-party slide), it is
  implemented, and a witness was watched failing first. Ten of the thirteen steps below are R.
- **B - needs the matching binary or the running game.** The corpus is HD 5.7 but this box's `AoK HD.exe` is a *different
  build* (D328: the descriptor table's address is outside its image, the corpus's strings sit `0x8346` lower, non-constantly).
  `+0x268`'s inset formula, the 100/600 terrain bits, the octile table's values and the equal-`f` sift order sit here. Each is
  named with what would unblock it rather than guessed.
- **U - undefined, or deliberately different.** Bit-exact route equality is **not** a target and cannot be: the binary's `g`
  is a float, its tie-break is a heap order we have not read, its pause draws from a `rand()` its own code calls "not sync
  safe", and the search window's padding is unread. G2 rejected bit-exactness; P10 keeps a reproducible pause on purpose.

So: **100% = every R step landed and witnessed + every B step either unblocked or recorded as out of reach with its reason +
the "unit stuck" class test green (§5)**. It is not "our routes equal the game's route on every click", and a plan that
promised that would be lying about three of its own constants.

## 2. The crosscheck: what the online sources add

**First-party - Raymi Klingers (Forgotten Empires), "25+ Years of Pathfinding Problems with C++", Meeting C++ 2025**
(slides, 104 pp.).

- The **legacy short-range algorithm** (slides 21-38) is hulls: obstructions are AABBs, the unit is shrunk to a point, the
  rectangles are gift-wrapped into convex hulls, cardinal rays escape a hull, edges are walked, a ray at the goal is tried at
  every vertex, **capped at 64 iterations**, then brute-force smoothing. Its own verdict: "undocumented, not performant
  enough, functioned most of the time".
- The **replacement** (slides 82-96) - ray at the goal, edge-walk the union of rectangles, only `> < == + -` so it runs in
  fixed point - was written for the **modern** releases. The talk frames it as what they built, not what AoK/AoC shipped.
- Player-visible rules, in the talk's words: *"units bump into others, stop and repath around. No pushing allowed"*;
  *"formations allow for some friendly overlap"*; units *"usually cannot overlap"*.

**Third-party, structural - `openage/doc/reverse_engineering/game_mechanics/pathfinding.md`** (cites the Ensemble postmortem,
Pottinger's GDC 2000 talk and the official AoK strategy guide):

- *"Age of Empires 2 uses three different algorithms for pathfinding plus two obstruction systems."*
- The high-level one *"uses mip-mapping ... Criteria for the mip-map subdivision is passability. **It ignores moving objects,
  e.g. units or wildlife**, though it takes static objects like buildings into account. Whenever a building is constructed the
  pathfinder gets notified and updates the corresponding mip-map chains."*
- The other two are *"tile-based"* and *"a polygonal pathfinder that dynamically generates convex hulls ... used in combat
  situations or when units cross paths"* - which is the same three-solver shape our register carries as P11.

**Third-party, formations - `openage/.../formations.md`:**

- *"Units within the same formation don't collide with each other when the formation is established."*
- *"Villagers, trade carts and fishing ships will not be part of any formation ... each of them takes their own path."*
- A marching formation is used when the order is more than **10 tiles** away.

**Third-party commentary on the talk** (daily.dev's summary and a pathfinding-benchmark post) agree the fixed-point
concave/edge-walk algorithm is the *modern* fix and that the legacy engine phased units through buildings when float
precision broke. No source contradicts our readings; two of them change the plan, below.

### Three corrections the crosscheck forced

1. **P14's gate is backwards.** openage says the mip-map **ignores moving objects**; the corpus agrees independently -
   `FUN_007ee980`'s body calls `FUN_007fd880`, `FUN_007ed5c0`, `FUN_007f0be0`, `FUN_007ede40`, `FUN_007edb80`,
   `FUN_007eeda0`, `FUN_007ee760`, `FUN_007ee600`, and **no** stander inserter (`FUN_007f9d80`'s only callers are
   `FUN_007f96b0` and `FUN_007fbab0`, reached from `FUN_007fb940` -> `FUN_007f9f10`/`FUN_007fa8a0` -> `FUN_007f7460`, not
   the levelled search). So the game runs the coarse search *and ignores bodies in it*; ours answers
   `avoid.is_empty() == false` by dropping to the flat search, which is the opposite. The step is to **drop the gate**, not to
   teach the tier about bodies. (What `FUN_007f9d80` is called *for* - which entry point inserts standers into which search -
   is still one read.)
2. **P11 targets the legacy algorithm, not the replacement.** HD 5.7 is the 2013 rebuild of the AoK/AoC engine; the
   fixed-point edge walk is the DE-era rewrite. The skill's movement reference had the DE design listed as *"the design to
   implement"*; that is now marked as DE-only, and the polygon pather's target is slides 21-38 (hulls, rays, 64 iterations,
   smoothing).
3. **The mate exemption belongs to a formation, and villagers never join one.** Our same-tick rule (every id ordered in one
   tick is a mate) is broader than the game's on both counts. Closing it either needs a formation model or makes villagers
   solid - a ruling, not a read (step 11).

And three **gaps the superseded gap register listed and no live row carries**: waypoint substitution
(`mConsecutiveSubstituteCount`), the order's path vocabulary (forced waypoint / teleport-to-waypoint), and the border cost
nudge. They are steps 5, 9 and 7 below.

## 3. The sequence

Order is by dependency, then by blast radius: the step that moves **every** route goes before the ones that move some, so each
measurement is against a settled base. Every step is one slice: witness watched failing, change, suite, acceptance A/B
(re-paths, stockpile at 2,400 s, hash, wall), review note, push.

| # | step | source | witness it owes | size | blocker |
|---|---|---|---|---|---|
| 1 | **P15 - a scene where the 20,000 cap binds.** A ~1,140-tile sealed map (`25 x euclid > 20,000`), assert `expansions == 20_000` | `FUN_008007f0:198` (read) | red today at the count, green with the cap | S | none |
| 2 | **P13 - the frontier's key `g + octile`.** The A/B is measured (28.9 vs 11.8 expansions/search; 54% vs 5.8% exhausting the budget) | `FUN_008007f0:177, 316` (read) | a scene the two keys answer differently, red on the octile key before the switch | M | none; moves every route, so the full suite + acceptance after it |
| 3 | **P14 - drop the tier's `avoid` gate (and re-test the 16-tile floor).** The coarse tier ignores bodies, as the game's does | `FUN_007ee980` body; openage pathfinding.md | a `World::plan_path` walk whose route comes from the hierarchy (the existing row calls `Levels` directly, so it cannot see this) | M | the 16-tile floor's removal is measured in D322 as a 7-point staircase - re-measure after 2 |
| 4 | **P16 - body collision as boxes** (four `<=`, grown by the mover's radius), with the arrival arithmetic moving in the same slice (`reach_between`, `target_contact`, `body_touches`) | `FUN_00840830`; talk: units are AABBs | the new probe's 4,741 box-only steps (2.17%) fall to 0, and `building_passage`, `collision`, `head_on` stay green | L | crowds are stricter; if the suite reddens on jams, the pause carries it or the slice parks with its numbers |
| 5 | **Waypoint substitution** - a waypoint a re-plan keeps returning is skipped and the next is reached | the log string `mConsecutiveSubstituteCount`, *"We won't reach our waypoint either this or the next update ... faking it"* | a walk whose waypoint is blocked re-plans and then **moves past** it (today it re-plans for ever) | M | the mechanism's exact trigger is a read of `FUN_008a4360` |
| 6 | **The search's body visibility: a waiting walker is an obstacle.** Ours skips any body with a route; the binary's test is velocity zero | `FUN_007f9d80:45` (`[vtbl+0x2a0] == 0`) | a jammed walker that a third man plans **round** (today he plans through) | S | none |
| 7 | **The levelled search's cost rows: the 300 predicate** is an object of type 8 or `0x20` on the cell or beside it, not "a blocked neighbour" | `FUN_007ed280` via `FUN_007f11f0` | a cliff-adjacent cell costs 10 (not 300); an object-adjacent cell costs 300 | S | none for the predicate; the window-distance term stays parked on the window's padding (B) |
| 8 | **The refinement's scan direction** - the binary scans the span outward from the interpolated point (forward, then back), ours from the low end | `FUN_007eeda0` (read) | two boundary crossings for the same step, only one of which ours picks | S | none |
| 9 | **The order's path vocabulary: forced waypoint, teleport-to-waypoint, per-path hit** | `ADDmove`'s fields (gap register, unread) | a forced waypoint honoured even when the search would route another way | M | a read of `ADDmove`'s field writers first |
| 10 | **P12 - the four-bit mask: close or port.** The conformance doc reads it as a derived cache of `!blocked()`; the register still calls it unread | `func_001D29F0` MIPS + C | either a row that the mask equals the derived form on every neighbour, or the mask landed | S | a decision, not a read |
| 11 | **The mate exemption: formation members only.** Villagers never join a formation, so a villager crowd must be solid | openage formations.md; talk | eight villagers ordered to one tile in one tick: min pair distance >= reach on **every** tick, or the ruling that they may press | M | **owner's ruling**: (a) formation model, (b) villagers solid + pause carries the jams, (c) keep the same-tick rule as a recorded deviation |
| 12 | **P11 - the polygonal short-range pathfinder**, legacy algorithm: AABBs, shrink the unit, gift-wrap hulls, cardinal rays, edge walk, ray at the goal at every vertex, 64-iteration cap, brute-force smooth | slides 21-38; openage pathfinding.md (third solver) | a combat approach round a building cluster that is **not** a tile staircase; the tile chain's corner count as the control | L | a project; scope separately, and it lands only if it beats the tile tier on the same scene |
| 13 | **P3 - touching collides, with the inset**, plus the 100/600 terrain rows | `FUN_00840830` `+0x266`/`+0x268`; `FUN_007ed280` bits 3/6 | the trebuchet's one-tile passage (D306) stays green through an inclusive test | M | **B**: the inset's formula and the terrain descriptor table need the matching binary |

Parked, with what unblocks each: **the equal-`f` sift order** (one read of `FUN_00801e90` - it is in the corpus, so this
should be R, not B: the sift is 30 lines); **the search window's padding** (`D299`, unread in the corpus); **the octile table's
values** (`DAT_00afd2a0` is runtime storage - read the generator, or run the game once and dump the table at `0xafd2a0`);
**the straight-line tier** (`#10` of the crosscheck: the caller of `FUN_008000d0`); **`FUN_007f9d80`'s real callers** (step 6
does not need it, but "which search sees standers" does).

## 4. Why this order

- **1 before 2**: the cap's witness is conformance-only and cannot move a route, so it is free and it retires a row that is
  currently decoration.
- **2 before 3 and 4**: the key changes every route in the engine. Landing the tier's gate or the box rule before it would
  charge those changes for a route shape the key caused.
- **3 before 4**: with the tier answering again, a body-related regression can be attributed to the box rule; with the tier
  gated off, every body regression lands in the flat search and the tier's own behaviour is never measured.
- **5, 6, 7, 8**: independent, small, and each has a named observable. They can be done in any order between the big ones.
- **11 and 12 last among the code steps**: both are structural (formations, a new solver), and both are cheaper to judge once
  the routes are the game's.
- **13 last**: it is the only step blocked on a binary this box does not have, and it is the *only* one whose rule cannot be
  read in hand.

## 5. The definition of done

Two things, and the second is the one that would have caught the last month of defects:

1. **Every R step landed, each with the witness that failed before it** - the register's rows P13-P19 plus steps 5-10 and 12
   carrying a `DONE` and a test name.
2. **The "unit stuck" class test**, from the talk's own method: *any* unit with a live order that does not move for N ticks
   fails, whatever the mechanism. Not a fixture per mechanism - one property over the acceptance match and the skirmish:
   no unit stands with a live order for more than N ticks, and every unit that arrives stands within the game's box gap of
   its target. Run on the acceptance match it is a number to report beside the stockpile; run on the crowd fixture it is the
   thing the last four slices kept regressing.

Bit-exactness stays out (class U), and `docs/parity-plan.md` says so at the top. What "blunt copy" buys here is that every
**rule** is the game's, every **constant** is read or marked B, and no mechanism of ours survives for a reason other than a
recorded decision.
