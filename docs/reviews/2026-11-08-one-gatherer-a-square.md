# One gatherer a square (D182)

*2026-11-08. The owner's rule, verbatim: **"Can you make the villagers to stand at the distance of half of diameter of the circle from the square they
are gathering any resource from? If there is no space at half diameter from another villager and the the resource try another nearest resource of the
same kind"**.*

Half the diameter of the ring **is** its radius - 0.2 of a tile for a villager - so the ring touches the square's edge with the centre at
`0.5 + 0.2 = 0.70` from the tile's centre. D169 adopted that number; what was missing is that a worker *reaches* it, that a square holds **one**
worker, and that the next man is answered with another source instead of crowding the ring.

## The three parts

1. **The distance is already right and is now tested**: `a_gatherer_stands_a_body_radius_from_the_square` measures the gap from the villager's centre
   to the square's **edge** and requires 0.20. It passes on arrival at the shipped standoff - the defect the owner saw was never the number, it was the
   worker standing *off* the ring.
2. **The standoff is the side the worker came from, not four sides**. D171 fanned workers round a node onto whichever side was free; measured, **five
   villagers all worked one bush** - the crowding in the photograph. `standoff_point` now offers the approach side only: a second worker from another
   direction still gets its own side (the side follows the approach), and a second worker from the *same* direction gets **no room**.
3. **A taken square sends the next gatherer elsewhere.** `start_gather` refuses to put a second worker on a square somebody is already gathering from,
   and answers the order with `nearest_like_free` - the nearest source of the same kind that nobody is working - or with standing idle when there is
   none, exactly as a farmer with no plot already does.

## The performance trap, and the number

The first version applied the rule **everywhere `start_gather` is called**, including the internal re-tasks that run per tick (a deposit completing, a
node running dry, a share being reassigned). Each of those now planned a route per candidate:

| | acceptance wall time |
|---|---|
| before | **16.3 s** |
| rule applied on every re-task | **> 180 s** (abandoned at 20,000 of 36,000 ticks) |
| rule applied to **player orders only** | **10.9 s** |

`start_gather` (a player's order) spreads; `start_gather_internal` (the engine's own re-tasks) does not. That is the whole difference, and it is also
the right place for the rule: the owner is describing what happens when *he* clicks a resource.

## Performance

**Measured** on the acceptance run (`36,000` ticks, seed 7, release build): **16.3 s** before this step, **10.9 s** after it, and **> 180 s** with the
rule applied to internal re-tasks (the version that was abandoned). The gate's own suite ran in **511,073 ms** (`gate108.log`) under load; the two new
tests are 0.9 s and 0.8 s. The claim check is one `O(units)` scan per order, and the alternative search is the existing one.

## Acceptance

| | D181 | D182 |
|---|---|---|
| entities | 160 | **137** |
| repaths | 254 | **618** |
| made-room | 112 | 307 |
| hash | `009e5cfe81b182ae` | **`dc9cf9905f65b61c`** |

**Both moves are costs and are recorded as such**: 23 fewer entities and 364 more re-plans, because a village whose workers can no longer share a
square must walk further for the same food and wood. That is a gameplay trade the owner chose; the engine's job is to make it behave and to say what
it costs.

## What was rewritten, and why

Two tests asserted the old rule and could not both be true:

- `gather_spread::two_woodcutters_on_one_tree_take_different_sides` -> `..._do_not_share_its_side`, now asserting no overlap and at most one gatherer
  on the square.
- `gather_motion::three_gatherers_on_one_bush_keep_their_own_places` -> `..._do_not_pile_up`, same shape.

Both carry the reason in their own headers rather than being deleted.

## Stability

- **Clippy caught a `clone` on a `Copy` type in the new test after the gate** (`CLIPPY=101` in `gate108.log`); fixed, and a second clippy run reports
  `CLIPPY_RC=0`. Recorded because the gate log still says otherwise.
- The `NoRoom` arm does **nothing** on purpose: re-tasking from it fires every tick a worker cannot reach its side, which is the 180 s loop above. The
  rule lives where it is cheap.
- Suite: **788 passed / 0 failed / 21 ignored**, fmt clean, `review_check` 0.
