# D339 - P16: a body collides as a box, and the probe's 4,741 do not go to zero

**Crosscheck first.** `FUN_00840830` compares four quantities: `_DAT_00ddeef4` (the mover's `max_x`, computed as the candidate's
box entry `+0x20 + idx*4` **plus** the mover's inset at `+0x268` when `+0x266` is set) against `DAT_00ddeee0`/`DAT_00ddeee4`,
and the same for the vertical pair - four `<=`, i.e. two grown axis-aligned boxes overlapping inclusively. The first-party talk
says the same thing in words: the unit is shrunk to a point and the obstructions are grown. openage's formations note adds only
that a formation's members are exempt; nothing anywhere gives a **disc** for two bodies, and the disc was ours.

**Prior implementation.** The step's body test was squared distance against the summed radii (`motion.rs:544-551`,
`dx*dx + dy*dy >= reach*reach`). Everything downstream of it was **already a box**: `solid_boxes` (`lib.rs:3278`),
`target_contact` (`:3332`), `entry_parameter` (`:3373`), `body_touches` (`:3414`) and the plan's `Clear` boxes - so the
sequence's "with the arrival arithmetic in the same slice" was already satisfied, and P16 was one predicate, not an L.

**Landed:** `dx.abs() >= reach || dy.abs() >= reach` → continue. Overlap is **strict** (a body resting exactly at its radius is
at contact, not inside); the binary's inclusive `<=` and the `+0x268` inset are P3's slice, which is blocked on the matching
binary, and keeping them out keeps one variable on this branch.

## The witness, and two designs of it that were wrong

The final fixture is `crates/sim/tests/body_boxes.rs`: a mover walking a planned route with a stander dropped **inside the
pocket** - 0.350 on both axes, 0.495 away, inside the box and outside the disc - and the observable is that the tick
**counts a collision and re-plans** (`repaths` 0 -> 1) instead of taking the landing silently.

Both earlier designs are worth recording, because each passed or failed for the wrong reason:

1. **Stander placed first.** A collider that has not moved since the route was planned is *past* (D281/D320) and exempt, so a
   stander standing there when the route is made never reaches the step's body test at all - the fixture passed under the disc
   for a reason that had nothing to do with the shape.
2. **"The mover must not move."** He *does* move: he is already inside the box, and the re-plan sends him sideways - measured
   `(218790, 217680)` against the disc's `(218695, 218698)`. The rule is "the tick does not take a landing into the table", not
   "the body freezes"; freezing a body inside a box would be the congelation D290 already recorded.

**Both runs are quoted on the same tree, one variable, `HEAD` checked out per file and restored:**

> disc (`HEAD`): `decision Some(Idle) repaths 0` - the mover steps to 0.378 on both axes, inside the box, outside the disc.
> box (landed): `repaths 1` - the landing is a collision, the collider joins the avoid list, the step goes sideways.

## Performance

Same pack, seed 7, 48,000 ticks, release, idle box.

| | D338 (disc) | D339 (box) |
|---|---|---|
| acceptance hash | `527bf4ebb9520fa0` | **`36ceb342bd94acc9`** |
| entities / players alive at the end | 132 / 1 | **134 / 2** |
| re-paths | 41,473 | 40,127 |
| wall, median of three | 15.96 s | **15.93 s** (16.11 / 15.87 / 15.93) |

**The match changes and the wall does not.** Blocks are stricter, so the two sides stay alive to 48,000 instead of one collapsing;
re-paths are 3% lower and the wall is inside the noise of the previous slice. Nothing here is a speed-up or a slow-down to claim -
it is the same simulation answering a stricter rule.

**The sequence's number is refuted, and that is the finding.** `collision_probe::what_a_box_would_refuse_that_a_disc_takes`
measures end-of-tick states with both boxes overlapping and the discs clear. One variable, same tree, same 36,000 ticks:

- **disc:** 4,681 of 251,681 body-steps (**1.86%**)
- **box:** 3,156 of 153,061 body-steps (**2.06%**)

So the figure does **not** fall to zero - and the reason is in the probe's own split: the pockets it counts are pairs the engine
**exempts** (17,517 disc-overlapping states with the collider *past*, zero mates), not landings the disc waved through. The box
rule changes which landings are collisions; it cannot separate a crowd that is already exempt. What does move is the
**denominator**: 251,681 body-steps in 36,000 ticks become 153,061, a **39% fall**, because stricter blocks mean fewer bodies
get to move - the "crowds are stricter" cost the sequence warned about, visible as a number rather than a jam.

## Stability

- Suite: **217 result lines, 936 passed, 0 failed, 21 ignored** (`--workspace --no-fail-fast`, exit line read). No fixture was
  re-pointed and no `#[ignore]` was added: `collision`, `head_on`, `building_passage`, `crowd_piles_up`, `group_overlap`,
  `overlap`, `right_of_way`, `no_pushing` and `gather_ring` all stay green.
- Conformance **21 / 0 / 0** across 9 binaries. Clippy `--all-targets -D warnings` clean, `cargo fmt --check` clean.
- Determinism: the acceptance hash is identical across three runs (`36ceb…` three times).
- Attacked: the disc (fixture, `repaths` 0 quoted), the pass exemption (two fixture designs thrown away, above), crowds (the
  census above), and the arrival arithmetic (already a box - grepped, not assumed).

## The AI review (OCR, D209), and what was taken

`make ai-review` read 2 files (it **cut the rest for budget** - the docs were read by hand) and found two real defects, both in
the fixture, both accepted:

1. **`repaths()` is cumulative, and the collision fired on the stander's landing tick** - not on the tick the assertion named. The
   fixture was proving a bump it had already caused one step earlier.
2. **A refused step does not move the mover** (`walk_the_step` sets `here = start` on a refusal), so the `assert_ne!` "he moved"
   was incompatible with the rule it claimed to test - it passed only because of the already-fired bump.

**Fixed:** the baseline (`world.repaths()`) is taken immediately before the tick that both lands the stander and takes the
mover's next step, and the assertion is `repaths == baseline + 1` - the bump *on that tick*. The `assert_ne!` is gone: what the
fixture claims is the collision, not a displacement. Re-verified both ways after the fix: **disc 0, box 1**.
