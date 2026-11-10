# The ellipse was bigger than the body (D176)

*2026-11-08. The owner: *"units still collide with the impassable building diamonds (the ellipse cannot overlap with it at all, not just center of
it)."* Half of that was already true, and the half that was not is the reason he sees it.*

## What was measured first

A new test walks the village's own map with the skirmish script for **2,000 ticks** - villagers crowding round trees, soldiers past the town centre -
and asks of every unit `clearance >= 0`: the distance from its centre to the nearest **blocked tile**, minus its own radius. It passes on the
shipped engine. So **the bodies were never inside a wall**; our ground test asks about the centre's tile, but the paths it produces keep the centre
half a tile from any wall.

What overlapped was the **drawing**. `config/economy.toml` had `[ring] scale = 1.5`, so a villager's 0.2 body wore a **0.3** selection ellipse:
clear body, ring a tenth of a tile into the house. The picture was saying something true about a circle that does not exist.

## The change

`[ring] scale = 1.5 -> **1.0**`, which is what the file's own comment already described as the alternative: *"Set 1.0 to make the ring be the
collision circle exactly."* The reason for 1.5 was to keep the ring from cutting through a sprite's feet, and there are no sprites yet (D18); when
they arrive the number can go back up, and it will be a decision about art rather than about collision.

The engine also gains two queries so the rule can be stated and tested rather than assumed:

- `World::clearance_at(position, radius)` - distance from a point to the nearest **blocked tile**, minus the radius. Zero is touching, negative is
  inside. This is the original's *"shrink the unit to a point, expand every other obstruction"* in one integer expression.
- `World::body_clearance(id)` - the same for a unit, using its own body radius.

## The test that proves the picture

`crates/view/tests/ring_is_the_body.rs` reads the **shipped pack** and asserts the ring is drawn at **1.0** times the body and that the radius
drawn is the villager's **0.2** - not the dat's third field (2.0, which is a selection *size*, not a footprint). **Mutation-proved**: with
`scale = 1.5` restored it fails with *"the pack draws the ring at 1.5 times the body: the ellipse is not the collision circle"*.

## What this does **not** do

It does **not** change collision. The body rule was already true and is now guarded by `body_clearance`; the gather standoff (D173) is unaffected,
because the jump is about *where a walk may end*, not about clearance. The next step there is still the movement change: a walk whose destination
may be a contact point.

## Performance

One content number and two read-only queries; the queries are not on the tick path. **Measured**: the acceptance run is byte-identical -
`6d23d672eb18ef40`, 153 entities, **24 repaths** (unchanged) - and the gate is recorded with the commit. The new sim test costs 1.1 s, the view test
0.8 s.

## Stability

- **A number in content, with the reason written beside it**, so the next person does not "fix" the ring back to 1.5 without knowing what it buys.
- **Both halves have a test**: the body never inside a wall (2,000 ticks, whole village), and the ring never drawn wider than the body.
- Suite: **781 passed / 0 failed / 20 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
