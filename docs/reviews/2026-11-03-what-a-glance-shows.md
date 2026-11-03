# Review: silhouettes, pictograms, and a speed selector (items 6 and 5's neighbours)

*Step D of session 3: the three things the owner asked to see - villagers a square and soldiers a triangle, a mark on each
building, and a speed selector. All three are presentation, so all three are pure functions with tests (D55: what cannot
be seen on this host must still be testable).*

## What changed

1. **Silhouettes.** `EntityView.shape: Silhouette` is `Square` for the start villager's family, `Triangle` for anything
   content calls military (`[metrics] military_roles`, the list the army count uses), `Dot` otherwise. The renderer draws a
   square, an **upward triangle** (`Frame::fill_triangle`) or a dot, in the player's colour. Content decides, the view only
   draws: `World::is_military` / `World::is_worker` are the two predicates behind it.
2. **Pictograms.** `[mark] <role> = "<shape>"` in `economy.toml`, a table of 13 roles; `World::mark_name(unit)` resolves a
   unit's role to it (by exact id first, then by family, so an upgraded barracks keeps its mark); `Mark::named` turns
   content's word into one of five glyphs (`ring`, `roof`, `sword`, `grain`, `cross`) drawn in `palette::MARK` on the
   footprint. **A mark naming a role the pack lacks is a load error**, naming the key - a typo that silently drew a plain
   building is the shape of mistake this loader refuses everywhere else.
3. **A speed selector.** `Speed` (1x, 1.5x, 2x, 3x, 5x) and `Pace`, which runs whole ticks per interval and **carries the
   fraction** (1.5x is 1, 2, 1, 2 ticks, never rounded down for ever). The HUD draws five boxes (a segmented control - the
   HUD font has digits, not words) and `ui::speed_at` maps a click to a speed; the window's keys 1-5 do the same, its
   title shows the pace, and a change is written to the window's own log line.
   **Decision (D101):** a speed change is allowed mid-match and is **not** match state - the simulation is fixed-rate
   (D16), a replay is a tick stream, and how fast a person watches is not part of it.

## Tests

| test | what it pins |
|---|---|
| `shapes::a_villager_and_a_soldier_at_one_pixel_differ` | the same unit at the same pixel is a square versus a triangle, by **row widths** (a square's are constant, a triangle's grow) - the owner's own acceptance line |
| `shapes::a_building_carries_the_mark_its_role_names` | `[mark] house = "roof"` reaches the snapshot and the pixels; the same house without the entry is plain |
| `shapes::the_speed_selector_picks_one_of_five` | five non-overlapping boxes, each picking its own speed, nothing outside, the live one filled |
| `shapes::the_speeds_are_exact_and_a_change_mid_match_keeps_the_carry` | 200 intervals at each speed are exactly 200/300/400/600/1000 ticks, and a change mid-match never gives or takes a partial tick |
| `economy::a_mark_naming_no_role_is_rejected` | a `[mark]` key that is not a role is refused by name |

**Four mutations, four caught:** the square drawn as a triangle → *"a villager and a soldier paint different rows"*; the
roof glyph not drawn → the mark test's pixel sets equal; the pace's carry dropped → *"the half tick is carried"*; the
mark-key check disabled → the economy test fails at the missing error.

## Performance

**The frame probe** (`frame_cost`, release, 960x720, 126 entities): **488 us per frame** with the marks and shapes drawn;
the same build with `draw_mark` stubbed out measured **510 us** - i.e. the additions are inside the probe's own spread, and
the honest claim is **no measurable cost**, not zero. A frame is still ~30x cheaper than the 16.7 ms a 60 Hz window has.

## Stability - what was attacked, and the two things that broke

- **A mark is part of the building's paint.** `depth::a_unit_behind_a_building_is_covered_by_it` asserted the exact
  building colour at the villager's tile and went red the moment the town centre carried a ring: the pixel it sampled was
  the mark. The assertion is now "not the unit's colour, and the building's own paint" - which is what the test meant.
- **A HUD click must not also be a world click.** The selector is drawn over the map, so the window checks `speed_at`
  before anything else and returns; otherwise clicking "5x" would also select whatever stood under it.
- **The pace's unit.** A float multiplier would round every frame and drift; the accumulator is in thousandths and
  `Pace::ticks` carries the remainder, proved over 200 intervals at each speed.
- **Not covered, and named:** the window itself is not run on this host (no display), so the wiring - keys, click, title -
  is untested except by the compiler; the *logic* it calls (`speed_at`, `Pace`, `speed_selector`) is tested. The mark
  vocabulary is the view's, so a content pack with a mark this build does not know draws a plain building rather than
  failing to load: `Mark::named` maps unknown names to `None` on purpose.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
