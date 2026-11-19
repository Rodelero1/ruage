# Review: the plan walks through its own order - one ignore list, read by the search as well as the step

*Change: the tick's group is formed **before** the first plan rather than after the last order (`drain_orders` pre-scan);
`stander_tiles` and `stander_bodies` skip that group, so a tile held only by a man's own order is free ground to his search
(`FUN_00801a20`); `World::group_mates` becomes the one definition of the group, read by the plan, by the collision block and
by a re-plan (`Route::mates`); and a movement click is no longer refused by the submit-time dry run
(`walk_goal_permissive`, D134). The red it closes is `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`. Basis,
and the reading it withdraws: D284.*

## Performance

**The gate (D278), the acceptance match, 48,000 ticks, release, seed 7, two interleaved pairs on the same box:**

| run | wall | verdict |
|---|---|---|
| `7eabb70` pair 1 | **269.2 s** | `entities=153 players=1 hash=3e9cad93e2883c7c repaths=1749` (`body` 218, `ground` 1531) |
| this change pair 1 | **267.5 s** | identical, same hash |
| `7eabb70` pair 2 | **264.9 s** | identical, same hash |
| this change pair 2 | **264.9 s** | identical, same hash |

The match **resolves** (`players=1`, a win), its last refusals are `no enemy matches "enemy_town_center"` - the beaten side is
gone, not starved - and the hash and **every re-path counter come back bit-identical on all four runs**. That is the proof
this change is **inert in the acceptance match**, and it is expected: the script orders **one unit per tick**, so no group
ever forms and the new branch never fires there. By the skill's rule a hash identity is inertness in *that scenario* and not
an absence of cost, so the branch is exercised by the fixture instead (below), and the overlap probe (below) prices the one
thing it could have made worse.

**The standard fixture, debug, 800 s of game time, two interleaved pairs:** `7eabb70` **100.44 s / 89.05 s**; this change
**90.49 s / 89.25 s**. The **spread inside one tree (11.4 s, 11%) is larger than the difference between trees**, so no cost is
resolvable from it - the change adds one `group_mates` scan per plan, and an empty set when the mover has no group.

**One number that moved and is not this change's:** both trees read **89-100 s** where the notes record **26.6 s** for the same
fixture. The parent commit reads it too, on this box, today; the earlier session already measured 58.84 s "not attributable"
on a shared box. It is quoted so the next session does not read today's absolute figures as a regression of this slice.

**The overlap probe** (`collision_probe`, 36,000 ticks, release, both trees): **371,804 overlapping pair-ticks of 17,160,468
inspected (2.1666%), deepest 8,000 raw, 243,298 deeper than half a footprint** - **identical** in both trees. Letting a plan
cut through its own group's tiles did not put a single body deeper into another in a real match.

## Stability

**The trap this slice walked into, and the fixture that catches it.** `a_crowd_walking_through_a_gap_stays_on_free_ground`
asserts that nobody stands on blocked ground - which a **refused** order satisfies perfectly by never moving. Its green would
therefore have been hollow if the order had simply been dropped one layer down. So the slice adds
`a_crowd_ordered_through_a_gap_arrives`: every one of the twelve ends within three tiles of the goal, **furthest 1.51 tiles**
measured. Watched failing first, twice: on `7eabb70` in a `before-group` worktree both fixtures panic with
`NoPath { from: (6, 20), to: (19, 19) }` before the code existed, and both pass after it.

**What was attacked.** That the exemption leaks (a bystander becoming free ground): it cannot, because `group_mates` filters
the **tick's orders**, not the map - a body with no order this tick is still a tile wall, and `Route::mates` on a re-plan is
the same set that was stamped at order time. That villagers get the exemption too (D237 forbids it): `group_mates` answers
empty unless the mover is military, and the villager families are green - `gather` **22/22**, `no_pushing` **2/2**,
`carrier_no_pushing` **5 passed 1 ignored**, `group_overlap` **4/4**. That the pre-scan changes the group the collision block
sees: it cannot - a refused order is removed again, so `stamp_group` receives exactly the set it received before, and the
gate's hash identity confirms it.

**Suite.** `cargo test --workspace --no-fail-fast`: **203 result lines over 203 binaries**. The first read was
**2 failed** - `a_broken_file_in_a_present_pack_fails_the_fixture` and `a_present_but_broken_marker_fails_rather_than_skipping`,
both panicking with *"the source path is neither a regular file nor a symlink to a regular file"*. **That is my own reading
error and it is named:** `config/gamedata` in this worktree was a **symlink**, `fs::copy` refuses that source, and both tests
are the fixture loader's own D130 guards rather than engine behaviour. Replacing the symlink with a real copy of the same pack
and re-running that binary gives **3 passed, 0 failed**. Movement sweep, 13 binaries, **65 passed, 2 ignored, 0 failed**
(`movement` 4, `evasion` 6, `gather` 22, `gather_motion` 6, `collision` 3, `group_overlap` 4, `no_pushing` 2,
`carrier_no_pushing` 5, `right_of_way` 4).

**Known and accepted.** (1) "Same tick" is still not "same selection" (D207): two clicks landing in one tick form one group
here and two in the game, so a military man can plan through another military man ordered elsewhere in the same tick. The
explicit group id on the command stays the open register row. (2) The submit-time dry run answers the click point when it
finds no route, so **an order can be accepted and then refused when it lands**; a goal that is genuinely unreachable still
fails there, which is what keeps "a tree behind a wall is a refusal" (item 9). (3) The one-tile stander exemption and the
`2 x radius` re-seed are unchanged by this slice - they are the *start* rule (`FUN_007fb220`), not the ignore list, and the
search no longer needs them widened to let a crowd out.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
