# OPEN - the crowd plans through its own order; next is the bound that replaces the window

> **2026-11-19, state of `main` (D284).** The red the last slice left is **green**:
> `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground` - twelve militia through one gap, no `NoPath`, and every
> man arrives (the new arrival fixture, furthest **1.51 tiles from the goal**). The cause was never the width: the game
> builds its path-ignore list **in the search** (`FUN_008007f0`) and reads it as **passability** (`FUN_00801a20`), so a
> group's plan walks through its own members. Ours had that list in the collision block only (`Route::mates`) and stamped it
> *after* the batch, so the first of twelve men was planned while the other eleven read as tile walls. Landed: the group is
> formed **before the first plan** (`drain_orders` pre-scan), the search **skips it** (`stander_tiles`/`stander_bodies`, one
> `group_mates` definition for both readers, `Route::mates` on a re-plan), and a movement click is **never refused** by the
> submit-time dry run (`walk_goal_permissive`, D134). **D207's sentence "the search never reads the route's ignore list" is
> withdrawn** by the 2026-11-16 read of that function; its rule stands.
> Review: `docs/reviews/2026-11-19-the-group-is-on-the-order.md`.
>
> **Gate (D278), this run** (release, seed 7, 48,000 ticks, two interleaved pairs against `7eabb70`): **`players=1`** - the
> match resolves, and the last refusals are `no enemy matches "enemy_town_center"` rather than a resource wall -
> **`entities=153`**, **`repaths=1749`** (body 218, ground 1531), `hash=3e9cad93e2883c7c`, and **every one of those numbers
> is identical on both trees**: the script orders one unit per tick, so the group never forms there. 269.2 / 264.9 s against
> 267.5 / 264.9 s; the overlap probe identical too (371,804 pair-ticks, 2.1666%).
>
> **Next, in the order they block each other:**
>
> 1. **The segment-test budget** - the bound that replaces the short-range window without paying **1.63x** (`b2accfe`: the
>    window's removal made the conformance set green and cost the acceptance fixture 26.6 s -> 43.38 s). The bound is
>    **work**: `nodes x (nodes-1)/2` segment tests, each weighted by its tile length, against `h x 25` capped at 20,000,
>    decided *before* the graph is built. Until it lands, the conformance red
>    `divergences::the_original_solves_a_long_walk_in_its_hierarchy` stays red and `MAX_SIDE` stays where it is.
> 2. **The expansion budget** - `expansions > h`, cap 20,000 (`FUN_008007f0`). **Blocked on a read**: both breaks land on a
>    block that *pushes* onto the frontier (`0x801861`-`0x80189d`), so what a budget-exhausted search answers is unread. No
>    guess goes in.
> 3. **Plan for the radius** (register row 21) - the graph layer still judges passage by tiles; `path.rs` has no radius.
> 4. **The walk** - no slide, the waypoint snapped to and spent, the body kept on the leg, leg by leg (D259/D260). Needs 3.
> 5. **The collision block's vocabulary** (rows 20, 22, 23, 25, 29, 30, 36) - `TakeAtRest` out, the avoid set out of the
>    search, the pause re-drawn, the caps gone, the order's path vocabulary.
> 6. **The place model** - a gather place the body fits on, and an approach it can walk (D257).
>
> **Two compromises carried, both labelled:** "same tick" is still not "same selection" (D207's stand-in: two clicks in one
> tick form one group here and two there, and the explicit group id stays open), and a submit-time dry run that finds no
> route answers the click point rather than refusing (D134), leaving the real refusal to the plan made when the group
> exists.
