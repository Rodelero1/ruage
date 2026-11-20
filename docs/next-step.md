# OPEN - P1 (the search budget), then route-shape coverage, then the six reds

**Where this branch stands, measured (`blunt-port`, `8073007`).** P11 is **done**: the levelled search is the land-unit default behind the
straight line and in front of the flat search (D296-D302), cached per ground change (D301), and it took **acceptance from 33.74 s to
22.96 s**. The suite is **918 passed / 6 failed / 21 ignored**, plus `radius_route` parked (D303). The six reds are at the foot of this file.

## Ready to start now - no evidence needed

1. **P1, the search budget.** The witnesses already exist and are `#[ignore]`d only because a *flat* search could not afford them:
   `crates/conformance/tests/budget.rs::a_hopeless_search_is_bounded_by_its_own_estimate` and the 20,000-cap row. Long walks now start on the
   levelled search, which is what D279 said the budget needed. Un-ignore them, enforce `estimate` / `EXPANSION_CAP` in `search_reporting`,
   and the truncation that parked them ("a villager one tile from its sheep answered a node fifteen tiles east") should no longer be
   reachable.
2. **Route-shape coverage.** D302's own review flagged it: no fixture asserts a *per-tile polyline* any more, so the red-set diff cannot see
   that blocked routes are now coarse chains. Two fixtures: the flat fallback answers tile-level corners; the hierarchy answers cell centres.
3. **The six reds**, named at the foot of this file - two `reseed` economy ones, the drop-site invariant, walled-off wood,
   `both_sides_field_an_army`, and the skirmish refusal.

## Ready after one short read each

4. **`FUN_007f0da0`** - the function that stamps grown obstacles into the level-3 cells. It decides the **granularity** of the radius growth,
   and therefore whether a one-tile gap survives at all (D303's parked witness). It also settles difference #1's window, #3's distance term
   and #4's h, because `FUN_007f0be0` already gave the window its shape: the **goal's box grown by `h + radius`**.
5. **`FUN_007eeda0`** - refinement of an edge to a **tile on the boundary** (#5): the fix that stops our coarse routes being 8-tile spaced.
6. **The straight-line tier** (#10): trace `FUN_008a4360` / `FUN_008a39a0` / `FUN_008a3a20` for a pre-search line test - if there is none,
   tier 1 is ours alone and should be named as such.
7. **The re-path's ignore list** (#8): four entities, cap 255, `0x1265`. We have `plan_path_avoiding`; the cap and the choice of entries are
   the missing pieces.

## Needs the owner

8. **The trebuchet-in-a-gap test in HD** (his, "later"). If a packed trebuchet passes a one-tile gap, the fix is `fits` from `> 0` to
   `>= 0` - the game's depenetration is computed and discarded (D290), so tangency is a passage - and D303's witness flips to assert the
   passage. If it does not pass, the growth has to match `FUN_007f0da0`'s granularity instead.
9. **Bodies priced at 2000** (#6). The game's behaviour is known; the decision is whether a crowd's own routes should stop routing round
   itself. Changes every crowd route.
10. **A terrain cost class** (#2's 100/600 rows): needs a `config/` and a map-format change, not an engine one.

## What is *not* owed

- Any further work on the levelled search itself: four levels, the descent, the cost table, the chain and the refinement are all landed,
  measured and gated.
- The tile search's parity: octile table, `25 x euclid` budget, 20,000 cap, tie-break and best-settled fallback all match.

## The six reds, verbatim

```
a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot
a_villager_drops_only_within_half_a_diameter_of_the_drop_site
a_walled_off_wood_is_tried_once_and_then_left_alone
auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out
both_sides_field_an_army
the_skirmish_only_refuses_what_the_fight_has_taken
```
