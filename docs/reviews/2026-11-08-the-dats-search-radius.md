# The dat has a search radius, and it is per unit (D175)

*2026-11-08. The owner: *"If the worker cannot reach the place it will search for another nearest source. ... Can you check in the file (using rea
if need be) or do online research?"* and then, of the three options put to him, **B**: take the dat's `search_radius` per unit.*

## What the file says

Read out of `Age2HD/resources/_common/dat/empires2_x2_p1.dat` with the parser (`task_info.search_radius`), 750 of the 1,402 slots carry it:

| unit | `search_radius` |
|---|---|
| `VMBAS` villager | **4.0** |
| `SCOUT` | 4.0 |
| `COGXX` cog | 6.0 |
| `FSHSP` fishing ship | 12.0 |
| `TREBU` / `PTREB` trebuchet | 17.0 / 18.0 |
| widest in the file | 21.0 |

Median 6.0. This is the number the original bounds its "find another nearest source" search by - **not a placement rule, a radius**.

**And the standoff is not in this file.** The per-task fields that would hold it - `work_range`, `auto_search_targets`, `search_wait_time` - are
**Definitive Edition** fields (dat v7.x); the Age of Kings HD file is v5.7.2 and carries `search_radius`, `work_rate` and drop sites only. So in
AoK HD the touching distance really is the unit radii - 0.2 villager + 0.5 node = **0.70** - which is what D169 already implements. The owner's
model is the file's model; there is no hidden work-range to copy.

## What changed

`Unit` grows `search_radius` (`task_info.search_radius`), and `World::gather_reach` uses it **when the pack carries it**, falling back to the old
sight-plus-five for every pack generated before this field existed. The fallback is not politeness: the generator lives in **another repository**
(D12), so this checkout cannot regenerate `config/gamedata/units.toml` - the engine gains the field and the pack gains it when `genconfig` emits it.

**Proven load-bearing by mutation**: with the new branch disabled the test reports *"a villager's search radius is 4 tiles in the dat, and the
engine looked **9** instead"* - the 9 is sight 4 + 5, the number the game does not use.

## The fallback rule the owner asked for already exists

"If the worker cannot reach the place it will search for another nearest source": `start_gather` refuses an unreachable node, the refusal is
remembered in `share_failed` (D138), and the picker - which sorts candidates by distance and asks reachability of the nearest eight - moves on to
the next one. `crates/sim/tests/gather_pick.rs::a_walled_off_wood_is_tried_once_and_then_left_alone` proves it. What was wrong was the **radius**
the search ran over, and that is what this step changes.

One number to note for the importer's owner: `share_retry_ticks` is **600** (30 s at 20 Hz), where the DE default for `search_wait_time` is around a
second. That is a separate decision and is **not** changed here.

## Performance

One lookup on the worker's variant in `gather_reach`, and that function is called from the picker, not per tick. **Measured**: the new suite runs in
**0.84 s**; the gate's wall clock is recorded with this commit. Acceptance hash is unchanged (`6d23d672eb18ef40`) on this machine, because the
generated pack carries no `search_radius` yet - the behaviour changes the day it does, and that is the point.

## Stability

- **Nothing changes for the pack we have**: absent field, old reach, byte-identical acceptance. A clone with an older `config/gamedata/` behaves
  exactly as before.
- **The field is optional by construction** (`#[serde(default)]`, `0.0` = absent, and no unit in the dat searches zero tiles), so an old pack cannot
  be silently misread as "search zero tiles".
- Suite: see the gate line in the commit; clippy `-D warnings` and fmt clean, `review_check` 0.
