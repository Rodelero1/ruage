# Review: the card in the bottom bar

*Step (D120): the owner asked (2026-11-04) for **unit stats in the bottom central panel**, to *"remove the big long health
bar"*, for a villager to be **renamed to the job he is doing** with his stats and carried resources, and for units to read
like the game's own selection bar (`Selection-Bar.png`: name, portrait, attack 4, armour `0/0`, range 4, `30/30`). This step
lands the **card and the numbers**; the job names are the follow-up slice and are not in this commit.*

## What is on the card

A card per selected thing, in a row, as many as fit (the owner's answers: **3A** several cards, **4A** the wide bar goes and
the small one under the portrait stays, **5A** attack + both armours + range, buildings health and armour).

- the pack's name, uppercase, at the top;
- a **framed portrait** - empty for now: the renderer has no art, and an empty frame that says nothing is honest, while an
  invented silhouette would read as content the pack does not have;
- the **small health bar** under the portrait with `hp/max` beneath it (a resource node prints `HP n`, because nothing records
  what it started with);
- the numbers beside it: `ATK`, `ARM m/p`, and then `RNG` for a fighting unit or `WRK` for a worker, plus `WOOD 7/10` for a
  load in hand in the pack's own word for the column.

## The numbers come from the owner, not the viewer

`World::stats_of(player, unit)` returns `UnitStats` **after the owner's upgrades** (`adjusted`, the same path combat uses), so
an enemy's card shows what *their* research produced - the rule D112 set for maximum health, applied to every number on the
card. Attack is the larger of class 3 (pierce) and class 4 (melee), which is what the two photographs agree on: an Archer
`[[27,3],[21,0],[3,4],[17,0],[13,0]]` reads **4**, and a villager `[[11,3],[4,3],[13,6],[14,0]]` reads **3**, not the 6 that
class 13 carries, which is a bonus against buildings.

Fractional values are integer **hundredths** across the boundary (speed 96 = 0.96 tiles a second), so nothing on this path is a
float and nothing can desync a tick. Hit points and capacity are counts and stay whole.

## Performance

Measured with `cargo run --release -p ruage-view --example hud_cost` (added by this step so the figure can be re-taken), a
`HEAD` git worktree against the working tree, **same `config/`**, three interleaved runs each, 2,000 HUD draws a run, tick 200
of a real match with one villager selected:

| run | `HEAD` (wide bar + row list) | this step (cards) |
|---|---|---|
| 1 | 170.7 µs | 170.1 µs |
| 2 | 168.2 µs | 180.3 µs |
| 3 | 168.5 µs | 170.8 µs |
| median | **168.5 µs** | **170.8 µs** |

**+2.3 µs a frame (+1.4%)**, which is at the edge of this machine's noise (the same binary varied by 12 µs between runs): the
card draws a filled ground of 7,316 pixels, a portrait frame and up to four short text lines, where the wide bar and six row
bars were 1 bar of 944x6 plus six of 60x4 and a handful of digits - a wash. At 60 fps the difference is 0.14 ms a second
against a 16.7 ms budget, and the whole HUD is **1% of a frame**. Cost does not grow with the selection: the cards are capped by
`selection_cards_that_fit`.

**Two measurements of my own were wrong and are reported here:** my first single runs read 263.9 µs (new) against 291.4 µs
(HEAD), from which I nearly wrote "the card is 27 µs *faster*". Both were the first run after a build, with a cold cache; the
interleaved medians above are the measurement, and the honest statement is "about the same".

The acceptance match is untouched: **no tick code changed**, and its hash is unchanged at `82efabd6f614c798`; the full suite is
**669 passed / 0 failed / 19 ignored**.

## Stability

| attack | result |
|---|---|
| a pack with no unit behind the entity (a resource node) | `stats` is `None`, the card prints `HP n` and the node's remaining amount; the tests for both node and farm were rewritten for the card and pass |
| an unknown unit id | `stats_of` answers `None` rather than inventing numbers |
| a fractional value that would not fit the scale | the accessor resolves it, and an unresolved one reads 0 rather than a float |
| more selected than fit | cards stop at the zone's width (`selection_cards_that_fit`), no drawing outside the panel |
| the wide bar returning | a test counts the longest run of the bar's track colour anywhere in the selected zone and fails above 30 pixels - the bar the owner asked to remove cannot come back unnoticed |

**My own errors this step, reported:** (1) I first patched the old drawing block to `if false {` to keep it compiling while I
wrote the replacement, which would have left dead code clippy rejects - the block is deleted, not disabled; (2) my first card
probe read the card's ground rather than its bar and I reported a failure as "the card is not drawn" until I checked the
coordinates (the bar starts four pixels into the card, not at its edge); (3) the earlier `let mut world` change made a test
borrow fail and clippy caught it, which is the third time this session that a piped command hid a compile error - the
unpiped run is the only one that counts.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
