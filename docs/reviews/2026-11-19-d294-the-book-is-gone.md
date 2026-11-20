# Review: the seat book is deleted (D294)

*D293 stopped the engine reading the book; the commit before this one took the fixtures off it. This is the deletion, and its only
honest test is that **nothing moves**.*

## What went

| item | where |
| --- | --- |
| `standoff_point` (the ring walk, 36 samples) | `lib.rs` |
| `Standoff` (`Point` / `NoRoom`) | `lib.rs` |
| `place_is_live`, `gather_places`, `gather_place_public`, `gather_place_of` | `lib.rs` |
| `reserve_gather_place`, `release_gather_place` | `lib.rs` |
| the `gather_place` field and its constructor line | `lib.rs` |
| `GATHER_WALK_SLACK_MILLI` (the score it was judged by) | `lib.rs` |
| `reserve_places` and its call in the tick | `world/step.rs` |
| the release call in the site's clear path | `world/sites.rs` |
| two aims that preferred "his place" | `world/step.rs` |

The two aims are now `Some(point)` and `target` instead of `place.unwrap_or(point)` / `place.unwrap_or(target)`. That is the only
line in the diff that reads like a behaviour change, and it is not one: `place` was already `None` everywhere after D293.

## Performance

- build `cargo build --workspace`: **8.22 s** warm, no errors; `cargo clippy --workspace --all-targets -- -D warnings`: **clean**,
  which is itself the finding that the deletion left no half-wired reader behind;
- full suite, before the deletion: **895 passed / 11 failed / 21 ignored**; after: **895 / 11 / 21**;
- the red sets are **byte-identical**: `diff` of the two sorted name lists prints nothing;
- no fixture's timing moved by more than noise, and the `research` group that carries the 400,000-tick banking fixture is unchanged.

## Stability

**The risk in a deletion is silent behaviour change**, so the gate was the red-set diff rather than a count: a count can hide a swap
(one green, one red) and a name list cannot. It is identical, so no rule that any fixture measures changed hands.

**Two things this proves, and one it does not.** It proves the book had been truly inert since D293, and it proves that the fixtures
which used to read it were the only readers. It does **not** prove the gather model is now the original's - that stays where D290 and
D293 left it, with the press witness (`gather_press_in`), the jam witness (`crowd_piles_up`) and the owner's own file as the
evidence.

**A false step on the way, recorded**: the first cut used brace counting from a signature anchor and took out a closing brace of the
impl block along with the function, leaving a fragment of `gather_place_of` in place; the compiler caught it (`unexpected closing
delimiter`) and the deletion was redone from line ranges measured from `grep` anchors instead.
