# Crosscheck: are today's six rulings in line with the original and the decomp?

Asked by the owner: *"Please crosscheck if those are all in line with original game and decomp."* What follows is per ruling: what the
claim rests on, whether the corpus still supports it, and anything that does **not** hold. The corpus is
`/opt/data/tools/rea/pseudo-aokhd` (15,465 functions, read by their log strings); the decomp is `samuelcaldas/age-of-empires-2-decomp`.

**First, the decomp's standing, because it decides how much weight it can carry:** it has **no movement and no gather code at all** -
its unit-movement and formation files are 16-47-line stubs (`docs/prior-art.md`). So "in line with the decomp" can only ever mean
"not contradicted by it", and for every ruling below it is silent. The evidence is the binary or the game's own artefacts.

| ruling | basis | crosscheck |
| --- | --- | --- |
| **D286** one box ruler; a click names its object | `FUN_00840830` (query box grown by the mover's radius), `FUN_007f9d80` (search grows objects by the radius), `FUN_008aa480` (the raw tile point test), and the epilogue's *"That collision is with our target, so we're done"* at `move_obj.cpp` 0x127d | **Holds.** All three functions are present in the corpus, and the 0x127d string is in `FUN_008a4360`. **One anchor name could not be re-confirmed:** `func_002690E0` does not appear in the corpus under that name, so the "both sizes subtracted per axis, clamped at zero" clause now rests on `FUN_00840830` alone. Not contradicted - but it is one source, not two. |
| **D287** the node re-lay deleted | our own machinery (D200/D202), removed because the original re-seats nobody | **Holds as an absence.** Nothing in the corpus implements a re-seat, and the decomp has no gather code to say otherwise. Recorded as measured-no-change (888/18/21 at the time). |
| **D289** a crowd goes to the tile it was ordered to; D271 withdrawn | the players' reports of men piling on one tree, and no engine-side slot assignment | **Holds, and the formation question was the real risk - it resolves in the same direction.** The binary does know formations, but every entry point is a **command handler**, not the movement path: `FormFormation(num=%d, pId=%d, formation=%d)` (`FUN_00807930`), `AutoFormation(pId=%d, on=%d)` (`FUN_008074e0`), `BreakFormation` (`FUN_008075b0`), `LockFormation`, `AboutFaceFormation` - all the same shape as the other script-console commands, which is how the game's own AI asks for one. So a move order does not auto-form; formations are an explicit request, and D206 already defers them (*"Formations later"*, owner 2026-11-14). **Residual divergence, to be written as such:** when formations arrive they will re-introduce the spacing D271's ring imitated - for grouped moves only, never for a gather. |
| **D290** no press floor anywhere | `FUN_008aa990` computes a full depenetration and its **only caller** (`FUN_008a4360`) never reads the result back into the unit's position | **Holds, strongest of the six.** Both halves were read directly: the resolver's separating-axis maths and its `*param_2 = local_14` write, then `local_64`'s three uses in the caller (declared, logged at 0x11df, passed to the resolver) with no position store. |
| **D291** the engine never crosses resource kinds | the game's AI carries the re-tasking (`sn-retask-gather-amount`, `sn-max-retask-gather-amount`, `up-retask-gatherers`), the same-kind shape in a replay-derived model of DE, and the decomp's silence | **Holds in direction, and its stated basis needed correcting**: this note's first version said *"the binary's radius is line of sight plus five tiles (D149)"* as though that were a measurement. **It is not** - D149 is the **owner's own ruling** (2026-11-07, *"the owner chose C - line of sight + 5"*). So the ruled-away behaviour (crossing kinds) rests on the AI-parameter evidence; the *radius* is his decision, not the binary's, and nothing in the corpus was found that filters a next-source search by object type. Medium confidence, labelled as inference. |
| **D292** no rule of ours stands a man down; `beside` and the forty-tick finish cut | the binary pauses (`getMaxVelFwd` returns exactly `0.0`) and re-paths; walls need an order to be rewritten | **Holds**, and the game's own materials agree in as many words: our first-party deck note reads *"Units bump into others, stop and repath around. **No pushing allowed.** Formations allow for some friendly overlap"* (`docs/pathfinding-from-the-talk.md`, quoted in D-of-2026-11-06). |

## What the crosscheck turned up that is not in line

1. **D291's basis was overstated** (above): the radius is the owner's D149 ruling, not a binary measurement. The decision stands; the
   sentence in `docs/decisions.md` does not, and is corrected.
2. **"No pushing allowed" - checked, and we already comply.** The first draft of this note claimed our engine still nudges a body 0.15
   tiles after forty refusals, quoting `world/step.rs:1807`. **That claim was wrong**: the line is a *comment recording a historical
   measurement*, not code. The audit that settles it: **no assignment to an entity's position exists anywhere outside the movement
   pass** (`grep '\.position = (' crates/sim/src` returns nothing), the only `scale / 20` left is a tolerance for "on his own place",
   and `no_pushing::a_crowd_at_a_tree_is_never_pushed` - which asserts exactly "nobody in the crowd is moved" - is green. The nudge
   was removed when D204 replaced it with "take the next node", and the comment outlived it. Nothing to do; the plan entry that this
   note added is withdrawn.
3. **A staleness to watch:** `docs/behaviour-diff.md` row 20 still describes the path-ignore list's ending as *"ours ends the
   exemption only when the route is dropped"*, while D281's re-test is in the code. Not one of today's rulings, but the same audit
   will be run over that file next, and it should not be trusted until it is.

**What this crosscheck cannot say:** anything about the decomp beyond "silent". Its movement and gather modules do not exist, so no
ruling here can be confirmed against it, and none is contradicted by it either.
