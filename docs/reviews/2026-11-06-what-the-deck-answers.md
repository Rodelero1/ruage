# What the deck answers about pathfinding, and what it does not

*2026-11-06. Asked to check the research thread's findings (`discord .../1567454248696807449`) for pathfinding answers.
Verdict: **yes for policy and for the algorithm's shape, no for the layer D131 lives in** - plus two documentation defects
found while checking, both fixed here.*

## The source, re-read rather than trusted

The thread's claim is that a first-party description exists. It does: **Raymi Klingers (Engineering Director, Forgotten
Empires), "25+ Years of Pathfinding Problems with C++", Meeting C++ 2025**, slides at
`meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf`. I fetched the deck and read the pathfinding
pages again rather than taking the summary's word for it. It agrees with the summary, and it was **already harvested** into
`docs/pathfinding-from-the-talk.md` on 2026-11-04 - so nothing was lost; the thread's value was the pointer.

The one thing the summary did not carry, and it is the thing we needed, is on page 9:

> In Age of Empires 2: Units bump into others, stop and repath around. **No pushing allowed**. Formations allow for some
> friendly overlap.

## What that answers

| our open question | the deck's answer |
|---|---|
| D131: should head-on bodies slip past each other in a corridor? | **No.** They bump, stop, and re-path. Overlap belongs to formations, not to a pass. Option B cannot be argued as fidelity |
| D131: is a stall in a one-tile corridor a bug? | **No** - it is the documented behaviour of the game in that geometry |
| D131: what shape should the fix take? | the deck's model: a unit is a point, every obstruction is expanded by its radius, so the other body **is** an expanded obstacle - separation comes from the plan's geometry, not from a reach test applied after the step |
| why fixed point? | the edge walk uses only `> < == + -`, because precision is what broke the original: nearly-colinear points flip left/right and the walk clips through obstructions. `ruage-fixed` already satisfies this |
| what does a planner do when it cannot reach? | **"give the closest point path"**, after expanding the search area once - the same rule as D134's `nearest_reachable` |
| is a plain A* over the tile grid what AoE2 does? | **No.** AoE2 has three pathfinders: a mip-mapped hierarchical route (passability-subdivided, buildings update it, moving units ignored), a tile-based short-range one, and a polygonal convex-hull one for combat and for units crossing paths. Ours fuses the first two. Fine at 141 entities; the hierarchy is the answer if a match ever needs thousands |

## What it does not answer

- **The follower layer.** The deck ends at "brute-force smooth". No arrival radius, standoff, unstick or approach-to-moving-
  target - and D131's 628-unit overlap is a *following* defect, not a generation one. A third reading of the same deck will
  not settle it; `docs/pathfinding-from-the-talk.md` already said so and this check confirms it.
- **HD 5.7 specifics.** The talk is the DE-era rebuild. It describes behaviour and design, never the tick order we need for
  determinism.

## Two documentation defects found while checking, both fixed

1. **`docs/sources.md` carried the pathfinding section three times** (lines 8, 62 and 110 before the fix) - a patch
   duplication of the kind the pitfalls file warns about, invisible because each copy was byte-identical and every copy was
   correct. Now one section, with the second source appended to it.
2. **A missing slash in a citation I added** (`AgeOfAgentsdoc/` for `AgeOfAgents/doc/`), caught on re-read before commit.

The deck's policy quote is now recorded in **D131.8** so the pending decision carries the evidence instead of a memory of it.

## Performance

No code changed. The deliverable is a source verdict; the only measurable claim it touches is AoE2's own, quoted: their
replacement walk is **2× faster on average**, **40× faster in the old worst case**, and **10% faster in the new worst case**
than the gift-wrapping original - on their machine, for their map, with their code, so it is context and not our measurement.
Our acceptance match is unchanged: **`f328ebc8d4d34b78`**, 141 entities, 188 repaths, **10.7 s** per 36,000 ticks.

## Stability

- No engine code touched, so the suite is the same **718 passed / 0 failed / 20 ignored** it was before these edits.
- `docs/sources.md` lost 25 lines of duplication and kept every distinct section: the seven `##` headings were counted before
  and after (7 → 7), and each section's first line was printed to confirm the removal took the copies and nothing else.
- The risk in a `sources.md` edit is silently dropping a distinct paragraph that happens to look like the duplicated one;
  the guard used here was deleting **by line range**, listing every heading afterwards, and diffing the count of the
  repeated heading (3 → 1).
