# What the game's own AI does, read from the install (2026-10-29)

Read for item 9, after three attempts at a timestamped build order stalled. The install ships the AI as **Perl-style
scripts** under `resources/_common/ai/` and `resources/_common/drs/gamedata_x1/`; this file records what they *do*,
with the file it was read from, and **not** their text (D22: facts from the install may be recorded, its expression may
not be copied).

## The build order is a rule system over state, not a timetable

`Economy/Dark Age.per` is the whole Dark Age in five active rules. The villager rule trains while: the age is Dark,
villagers are below the maximum, production is enabled, **food is under 400 or villagers are under 28**, and Loom is
done **or** there are fewer than 12 villagers. The age-up rule is: allowed, **villagers ≥ 28**, and `can-research
feudal-age`. Loom has its own rule: can't train a villager, **or** 12 villagers, and it can be researched.

`Economy/Feudal Age.per` continues the same way - villagers to 31, then 40 with a Wheelbarrow guard - and builds a
**market** and a **blacksmith** only when it has none, with `up-assign-builders` dedicating two builders to each.

**Nothing in any of it names a clock.** Not one rule says "at 2:10". Every guard is a count, an amount, a `can-*`
predicate or an age, and the AI is re-evaluated every few seconds forever. That is the single most useful thing in the
file: a build order that cannot go stale, because it reacts to the state it is actually in.

**Numbers worth knowing** (they are the AI's plan, not the game's rules): **28 villagers before Feudal**, 31-40 through
Feudal, Market and Blacksmith for the Castle Age with two builders each.

## The engine owns villager distribution, and the AI sets percentages

`Economy/Low-pop Villager Distribution.per` is 76 active `set-strategic-number` lines and no orders at all: for each
villager count from 1 to about 20 it sets `sn-wood-gatherer-percentage`, `sn-food-gatherer-percentage` and the gold and
stone equivalents. At 8 villagers or fewer, food is **100%**; it becomes 88/80/72/66 as wood takes 12/20/28/34 at 9, 10,
11 and 12 villagers. The AI does not choose which tree a villager works - **it states a share and the game assigns idle
villagers to meet it.**

That is the mechanism the acceptance script has been faking with timestamps: "keep four on wood" is a percentage in the
game's own vocabulary, and the engine is what moves the villagers.

## The AI's own words on the boar, and on what is switched off

`Economy/Boar Hunting.per` is **entirely commented out** in HD - every rule, including the ones that would set
`sn-minimum-number-hunters` and `sn-minimum-boar-lure-group-size`. The owner's recollection that *"the factory AI never
tries to lure the boar"* is exactly right, and the disabled file shows what the lure would have used: a
`dropsite-min-distance boar-hunting` guard, one hunter, then eight, and `up-retask-gatherers food c: ...`.

Much of `Economy/Balance.per`, the resource-rebalancing file, is commented out too. HD's AI is not a complete
implementation of the design its file names suggest, which is worth remembering before treating any of it as the
game's intended behaviour rather than as what shipped.

## What this says about our own plan

1. **A `[[rule]]` form in the scenario format** (condition → one order, re-evaluated) would make the acceptance match
   expressible without timestamps: "if villagers ≥ 28 and no market, build a market beside the mill". It is also
   strictly more testable, because a rule can be unit-tested against a state instead of a clock.
2. **A villager-distribution policy in the engine** - target shares per resource, applied to idle villagers - is the
   missing piece behind "keep four on wood", and it is how the game itself does it. It also gives the balance runs
   something better to vary than 35 timestamps per script.
3. **`up-assign-builders` names the third thing we lack**: dedicating N builders to a building *type*, rather than
   prescribing which villager walks where.
