# Review: the slide deleted and the refusal traced - the plan's first leg is made from a point the body is not on

*Change: on branch `d276-no-slide`, `StepDecision::Slide` is no longer returned from the ground branch, and the animals
family was run to reproduce D264's measured state. **Not for `main`**: three fixtures are red by construction, and the trace
below says why they cannot be fixed in the mover. A probe printed each refused step; it is removed from the commit.*

## Performance

The branch changes one arm of `collide`: a blocked ground step now goes straight to the re-plan and `WaitGround` instead of
trying two axis steps first. That makes the mover **cheaper**, not dearer - the two `step_is_walkable` probes per refused step
are gone - but the number that matters is not CPU:

**Animals with the slide deleted: 8 passed, 3 failed** (`a_live_animal_is_a_gather_source`, `the_villager_follows_the_sheep`,
`the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`) - exactly D264's 3, reproduced.

Timed, same tree, debug, `cargo test -p ruage-sim --test animals`: **5.46 s green on `main`** against **5.58 s on this branch**
(11 fixtures against 8 passing + 3 failing early). So the deleted arm costs nothing measurable, and the two `step_is_walkable`
probes it removes are not where the time goes - **2.2%**, and on a run whose failures end early, which is not a speed claim.

## Stability

**What each red says, in its own words** (the messages, because they are the symptom per fixture):

- `the_villager_follows_the_sheep` - *"the villager has to reach the sheep it was sent to, not stop half-way"*
- `a_live_animal_is_a_gather_source` - *"a villager working a sheep must end up carrying meat"*
- `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` - *"the shepherd has to strike the animal, not have it
  turn into a corpse on arrival"*

**All three are approach and arrival; none is an overlap.** That is the strongest support for the trace's conclusion below: with
the slide gone nothing gets *inside* anything - what stops is a man 0.35 tiles off his tile's centre never taking his first step.

**The trace, which is the finding D265 asked for** (one fixture, `DBG_GROUND=1`, 270 refusals from tick 31):

```
tick 31  here=(316980,316980) step=(566,566)   clear_now=270  clear_next=-530  len=5
         route=[(319999,319999), (319999,360001), (360001,360001), (399999,380001), (440000,440000)]
tick 32  here=(316980,316980) step=(793,-104)  clear_now=270  clear_next=-164  len=3
         route=[(370000,310000), (390000,390000), (440000,440000)]   (repeats to tick 300)
```

Read against the scale (20,000 raw to a tile): the body stands at **(15.849, 15.849)** - i.e. 0.35 tiles from its **own tile's
centre**, (15.5, 15.5) - and the re-planned route's first point is **(18.5, 15.5)**, the centre of a tile 2.6 tiles east. The
straight line from where he stands to that point grazes a blocked tile by **164 raw (0.008 of a tile)** and is refused; the
line from the tile centre to it would be clear.

**So the first failing element is the plan's first leg, and its cause is that the leg is made from a point the body is not
standing on.** `short_range_clear` and the pull plan between *tile* points; the body is 0.35 tiles off its tile's centre,
which is exactly the distance `settle_workers` and body collision put it there. Every re-plan reproduces the same leg, which
is why the refusal repeats 270 times with an unchanged route - the mover is not the stage that is wrong.

**What this rules out, with the number:** a re-plan budget (raised 2 -> 40 in D254: identical trace), a slide repair (16 and
14 tests red in D253), and a "tolerance" on the step test (the 164 raw is a corner graze *of a leg the plan chose*, not a
brush on a legal plan).

**Two probes later, and the answer moved again (both probes removed, both findings kept):**

1. **`DBG_PLAN` on `plan_path_mode`**: the sheep fixture makes exactly **two plans, both through `short_range_clear`** - no
   fallback at all, so "the tile search answered" (D263's question) is **not** this fixture's case. Both plans are logged from
   `from=(300000,300000)` - the **corner** of tile (15,15), which is where `Command::Spawn` put the body.
2. **The follower is 0.85 tiles from that point when it first refuses** (316980,316980 at tick 31): so the route is *planned
   from where the body was born* and followed from where it now stands. The tick-32 re-plan does return a **different** route
   (3 points, first at 18.5,15.5), and then never changes again for 270 ticks - so the re-plan is not stale, it is
   **wrong by the mover's measure**: its first leg is clear to the planner (a *tile* line test) and refused by the mover (a
   *disc* step test) at 164 raw.

**Fix attempted and reverted**: prefixing the fallback route with the body's own tile centre. It cannot help - the route does
not come from the fallback - and it was reverted the moment the animals family stayed at 8/3, rather than left in as a
plausible-looking change.

**What it leaves, named:** the disagreement is **inside `short_range_clear`'s answer**: its pull and its line test are
tile-based (`line_clear_avoiding`), so a leg it certifies can still be a leg the disc step test refuses. D261 bundled that fix
with four others and measured 3 red; **it has never been measured alone**, and that is the next slice: give the pull the
body's radius (`line_clear_body`), change nothing else, and report the animals count and the acceptance run.

**What it leaves, named:** the first leg must start where the body stands - either by planning from the body's position rather
than its tile's centre, or by making the walk aim along the current leg (D259's recommendation) so the plan's segments are the
walk's segments. That is slice 1's real content, and it is larger than "delete the slide".

Known and accepted: the branch is parked, not merged. `main` is green; this branch carries the measurement and the deleted arm
so the next slice starts from it rather than from the argument.

