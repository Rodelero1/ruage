# Scrutiny: the external review, checked claim by claim

*The owner handed over a 141-line review (`ruage_review`) and said "please scrutinize". Its method was static only - it says so:
"Could not run cargo clippy/cargo test because the Rust toolchain is not installed in this environment" - so every claim here was
checked against the tree, and where a claim turns on arithmetic, the arithmetic was computed.*

**Verdicts: 15 findings. 9 confirmed, 2 wrong in severity (right in principle, not reachable in this pack), 4 confirmed but
low-value. 5 fixed in this step; 4 deferred with reasons. The line numbers are accurate** - the reviewer read the current tree.

| # | Claim | Verdict | Action |
|---|---|---|---|
| 1 | `Fixed::div_int` asserts and uses `expect` | **Confirmed as written**, but it has **no production caller** - only `crates/fixed` tests call it. Not reachable from the tick. | Kept; the precondition is documented. |
| 2 | `raw * 2` overflow in the kiting reflection | **Not reachable.** It would need a raw position above 4.6e18; the largest a `u16` map can hold at the finest scale that fits `i64` is 2.1e9. | Fixed anyway (`i128` + clamp, no cost). |
| 3 | Squared distances overflow `i64` | **Latent, not live.** The shipped pack: 64 tiles at 20,000 is **3.2e12** against `i64::MAX` **9.2e18** - seven orders of margin. Even a full 65,535-tile map at 20,000 fits (3.4e18); overflow starts above **scale 32,768** on a full map. The reviewer's example (720,000) does overflow, so the concern is right and the severity is not. | **Fixed at load**: `SimError::GeometryOverflow` + `max_scale_for`, tested against the arithmetic itself. |
| 4 | `headless`/`batch` panic on `load_resources` | **Confirmed**, both sites. | Fixed: `map_err(...)?`. |
| 5 | `frame` panics if the villager vanished | **Confirmed**. | Fixed: reports and exits. |
| 6 | `find`/`find_mut` are O(n) in hot loops | **Confirmed** and architectural. | Deferred: the id→index map is a real slice with a state-hash review of its own, not a patch. |
| 7 | Three consecutive `find(worker)` scans in the share pass | **Confirmed** - and one of them hid an `expect`. Per villager per tick. | **Fixed**: one lookup. Measured below. |
| 8 | `expect` in movement application | **Confirmed**, invariant-guarded ("entity list is stable within a tick"). | Deferred: the honest fix is #6's index, which removes the lookup rather than the panic. |
| 9 | `nearest_like` may run up to 8 path searches | **Confirmed**, already documented as a former hotspot (1,336 searches a tick, now capped at 8). | Deferred with #10. |
| 10 | `nearest_resource` runs A\* per candidate node | **Confirmed, and the most valuable performance item in the review.** `nearest_nodes` next door avoids `can_reach` for exactly this reason - the file's own comment records "37x the whole rest of the run". | Deferred: needs the same treatment (short list first, reachability only on the head) plus a measurement; not a drive-by edit. |
| 11 | `frame` swallows `load_resources` with `.ok()` | **Confirmed**. | Fixed: reported, non-zero exit. |
| 12 | `raw * 100` overflow in the stat percentage | **Not reachable**: the largest raw it sees is a unit stat (≈1.2e5), not a position; the bound would need 9.2e16. | Fixed anyway (`i128`), because the next caller might pass a position. |
| 13 | `mask.width as u16` truncation | **Confirmed but bounded**: the mask is a content footprint, at most a handful of tiles. | Deferred: an assert at load belongs with the footprint loader. |
| 14 | O(n²) duplicate RNG-stream check | **Confirmed, negligible**: the list is a handful of names, once per load. | Deferred: cosmetic. |
| 15 | `tile_of` clamps out-of-bounds to `(0, 0)` | **Confirmed and documented** as what a test fixture needs. | Kept; a `debug_assert` would fight the fixtures it exists for. |

**What the review could not check, and therefore did not say:** whether any of it changes behaviour. It also missed the three
defects this project found by *running* things in the same period (the O(n²) husk pass, the stale-route freeze, the minimap
seams) - all of them invisible to static reading. That is not a criticism of the method it had; it is the case for the method
this project uses.

## Performance

The five fixes are behaviour-preserving by construction, and the **acceptance hash proves it**: unchanged at
`e4bdd521acaaf342`, same 152 entities, same 237 repaths. That makes the timing a clean like-for-like - the same match, run
slower or faster:

| | 36,000-tick acceptance match |
|---|---|
| before (`e3ecba9`) | 13,396 / 13,520 / 13,588 ms |
| after this step | 12,243 / 12,391 / 12,761 ms |
| median | **13,520 -> 12,391 ms (-8.3%)** |

The only change with a plausible mechanism is #7: three full-entity scans removed from a pass that runs **per villager per
tick**. The `i128` fixes are one multiply-wide op each and the binary changes run once at startup.

## Stability

| attack | result |
|---|---|
| a pack whose scale cannot be squared | refused at load with the numbers and the largest scale that fits (`a_scale_too_fine_for_the_map_is_refused`) |
| the bound itself | `the_bound_is_the_arithmetic_it_claims`: at the reported limit `2*((tiles-1)*scale)^2 <= i64::MAX`, and one past it overflows - for 2, 64, 512 and 65,535 tiles |
| the shipped pack | still seeds (`the_shipped_pack_seeds`) |
| a resource-pack mismatch in `headless`, `batch` | reported, non-zero exit, no panic |
| a resource-pack mismatch in `frame` | reported, non-zero exit, no silently different picture |
| a vanished selected unit in `frame` | reported, no panic |
| the reflection arithmetic | total: `i128` then a clamp, so no position can wrap it |
| behaviour | unchanged: acceptance hash, entity count and repath count are identical to before the step |
| the `no_float` guard | green - the new integer square root is integer, and a `f64` first draft was caught by it |

**699 passed / 0 failed / 19 ignored**; clippy and fmt clean.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
