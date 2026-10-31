# Review: the Town Centre's footprint mask (D76)

*Change: `[footprint.<role>]` in `economy.toml`, `Footprint` in `ruage-data`, `solid_tiles` + mask blocking and freeing in
`ruage-sim`, `Walk` counted layer used for both.*

## Performance

The probe reads **29.2 / 27.2 / 27.0 / 25.6 / 24.3 ns per entity per tick** at 126 → 2,126 entities, against **27.1 / 26.2
/ 26.4 / 26.1 / 28.5** measured on the previous commit (`a9ea683`) an hour earlier in this session. Every row is inside the
probe's ~8–10% between-build sensitivity except the largest, which improved by 15%; **the change is neutral**, which is what
the shape predicts: `solid_tiles` runs on **insert** and on **demolition**, and on the refusal path of a build order - never
in the tick. Its worst case is a 4x4 mask (16 cells) after one `BTreeMap` lookup.

The acceptance match still runs to a result in the same wall-clock envelope (`make headless`, 48,000 ticks, release).

## Stability

Attacked, and the attacks found three defects and one open one:

1. **A building's solid core now lives in the walk layer, so a site on it reported the wrong reason.**
   `a_site_on_top_of_something_is_refused` failed with `SiteBlocked` ("not open ground" - a message about terrain) instead of
   `SiteOccupied` ("something already stands here") for a tile a town centre holds. Fixed by asking *which building holds the
   tile* before answering, so both messages stay true.
2. **A foundation fixture was building inside the town centre's new core.** `a_foundation_i_can_see_blocks_me` placed player
   2's house at (17,16), which was legal while the town centre blocked one tile and is inside `##..` now. The fixture moved
   to (19,16)/(20,16) - the engine was right and the test was stale, which is the direction to prefer.
3. **OPEN - the masks are not finished:** `a_civilisation_that_cannot_build_houses_gets_the_whole_cap` trains **18 of 20**
   villagers, and doubling the budget to 200,000 ticks does not add the last two, so **two units are dropped rather than
   delayed**. The likely cause is unit placement: a finished unit needs a free tile beside its building, and a core that
   occupies four tiles plus a crowded yard has fewer. It is `#[ignore]`d with that reasoning and recorded in
   `docs/next-step.md` as the blocker for this step, not as acceptable behaviour.

My own mistakes, both caught rather than shipped: the `[footprint.*]` tables were first inserted **in the middle of
`[build]`**, which would have moved `repair_hp_per_second` and the build prerequisites into `[footprint.farm]`; and a
scripted insert duplicated a `#[derive(Debug, Clone)]` pair, which the compiler refused. Both are the documented traps of
editing a large TOML and a large Rust file with scripted replacements.

**And the hash moved: `f91872681ba4a3a8` → `91e869fcc92d606c`.** That is expected here - a mask changes paths, so a
path-equivalent change is exactly what it is *not* - and the property that matters is preserved: the match still ends
`players=1` at 48,000 ticks. Any future run that changes the hash without changing pathing is the one to investigate.
