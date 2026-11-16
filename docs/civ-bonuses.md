# Civ and team bonuses: the design, ready, not implemented

**Ruling (the owner, 2026-11-15):** *"Team bonuses and other civilisation bonuses can wait. Let's just have the design ready for
it to be implemented in the future."* So this file is a **seam**, in the sense §7b of `docs/dependency-tree.md` uses the word: a
shape, with the numbers read from the data and nothing implemented on top of them. What the engine already does with the three
effect tables stays as it is; what it should look like when the bonuses are built is written down here, along with the test that
would prove the seam.

**Added 2026-11-16, the owner's requirement (D233):** *"I would like team bonuses and civ bonuses moddable in the future to
be able to easily add a civ by adding/changing config."* That does not un-defer the bonuses - it fixes their **shape**: a civ
is a row plus the effects it points at, never a name in `crates/`, and every bonus has to reach every consumer through one
table (this file's seam), because a bonus that lands in four walks out of five is a mod that looks right and does nothing.
The first half of that is now enforced where a config *can* be wrong: the pack's **attribute vocabulary is closed**
(`crates/data/src/techs.rs`: 13 names the engine acts on, 16 the data carries and nothing reads yet), and a modifier naming
anything else is refused at load with its effect id and the name. The remaining gap is the seam itself: five consumers,
three tables, and a sixth would be another place a new civ's bonus can be half-applied.

## What the data actually carries (measured 2026-11-15)

`config/gamedata/civs.toml`: **32 civ rows, 31 with a `team_bonus_effect`** (Gaia has none). Each civ also has a
`tech_tree_effect` and a 211-entry `resources` row.

Across the 31 team-bonus effects, the commands are:

| kind | count | what it is | who reads it today |
|---|---|---|---|
| 5 | 48 | multiply an attribute | `modifiers_for` - for the attributes it names |
| 4 | 37 | add an attribute | `modifiers_for` |
| 1 | 6 | write a **resource** | only resource 36 (the farm line) |
| 0 | 15 | set an attribute | `modifiers_for` |
| 3 | 2 | *unnamed* | **nobody** |
| 6 | 1 | *unnamed* | **nobody** |
| 102 | 2 | bar a tech - **inside a team bonus** | `barred_by` reads this only from the *tech-tree* effect |

The attributes those modifiers name, with the counts: `line_of_sight` 24, `gather_rate` 17, **`other:101` 16**, `population`
12, `attack` 11, **`other:23` 8**, **`other:100` 7**, `minimum_range` 3, `range` 2.

The resource rows are increments to **77, 178, 179, 36** and sets of **89, 210**.

**Two facts that decide the shape of the work:**

1. **Twenty-three of the ~100 team-bonus modifiers name an attribute nothing reads** (`other:100`, `other:101`), and eight more
   name `other:23`. So "team bonuses are applied" is not one switch: it is a table of attributes, and two thirds of it is still
   unnamed. Naming them is importer work (D214's shape), not engine work.
2. **Kind 3 and kind 6 appear in this data and nothing reads them anywhere**, and **two team bonuses carry a kind-102 row** -
   a bonus that bars a tech, which the tree's mental model ("the tech tree bars techs, the team bonus gives units") does not
   cover.

## What the engine does today, and why it is five walks of three tables

| consumer | table it walks | what it decides |
|---|---|---|
| `barred_by` | civ `tech_tree_effect`, kind 102 | may this player research this tech |
| `civ_barred_unit` | same effect, kind 2 | may this player have this unit |
| `modifiers_for` | researched techs **+ team-bonus effects**, kind 0/4/5, matched by **attribute string** | attack, armour, hp, range, line of sight, speed, gather rate, carry, population |
| `farm_food_bonus` | researched techs **+ team bonus**, kind 1, resource 36 | a plot's budget |
| `construction_rate` | researched techs **+ tech-tree effect**, kind 1, resource 195 | builder-ticks |

Five consumers, three effect tables, and each one re-decides which effects belong to a player. That is why a bonus can be
half-applied for years: the farm line and the construction rate were each "one resource row somebody forgot", and both were
found by accident while doing something else.

## The seam (R6): one resolved table per player, and consumers ask it

> **Built, in its first half, as D234 (2026-11-16).** `World.bonus_tables` exists and the five consumers read it: the rows a
> player has, per attribute, plus the kind-1 resource rows with their source, plus the tech-tree bars. **What is not built is
> the resolution to numbers** - the table caches rows, because the five consumers fold them differently and one resolved value
> would have meant one fold for five readers (a behaviour change, not a refactor). So rule 2 below (keying on the `Attribute`
> enum instead of a string) is still owed, and it now has one place to land in. Rule 1's "one list of effects per player" is
> what exists; the third source distinction (the tech tree's modifiers, which `modifiers_for` has never read) is preserved
> rather than merged, on purpose.

**The shape:**

```text
BonusTable per player, rebuilt when a tech lands and when the civ is set (the two events that can change it):
    resources:      BTreeMap<u16, Fixed>              // resource rows: 36, 89, 195, 210, 77, 178, 179 ...
    attributes:     BTreeMap<Key, Fixed>              // Key = (unit: Option<u32>, class: Option<i16>, attribute: Attribute)
    barred_techs:   BTreeSet<u32>
    barred_units:   BTreeSet<u32>
    granted_units:  BTreeSet<u32>
```

**Three rules it would carry, each already true somewhere in the code:**

1. **One list of effects per player** - researched techs (in id order) + the civ's team bonus + the civ's tech-tree effect -
   with the same last-statement-wins rule every modifier already follows. The five consumers above become five *readers* of one
   table, which is the only way a new attribute or resource lands in all of them at once.
2. **`Attribute` stops being a string in the hot path.** D214 showed the string *is* the interface today
   (`modifiers_for(player, unit, "range")`), and the cost is that a rename is a silent behaviour change and an unknown name is a
   silent no-op. A resolved table keys on the enum, so an unnamed attribute is a **load-time** report (*"effect 490 names
   attribute 101, which nothing reads"*) instead of a runtime shrug.
3. **A default is a statement, not a zero.** Two of the three resources that matter have **0** in every civ row meaning "not
   stated" (36 until D215, 195 until D218), and writing a 0 where a multiplier belongs is a game that does not build and does
   not eat. The table carries `Option`, and content decides the default (`[farm] food_per_farm`, 1.0 for a rate).

**The proof (a test that fails until the seam exists):** a fixture pack with a synthetic civ whose team bonus sets resource 36
to **+90** and names `line_of_sight` for a class, and a test that loads *that pack* and asks the existing consumers - a farm's
`food_per_farm` answer of **265**, and a tower's sight - with **no new engine word** anywhere: the pack says what the bonus is,
the engine only knows how to read a table. Today the same fixture would pass only for the attributes the five walks happen to
cover, which is exactly the difference the seam removes.

## What is deliberately not decided here

- **The numbers.** How much a team bonus gives is the data's; nothing in this file invents one.
- **`other:100`, `other:101`, `other:23`, kinds 3 and 6.** Unnamed and unread. Naming them is an importer change with a consumer
  each (D214's shape, twice), not part of the seam.
- **A team bonus that bars a tech** (2 rows). Whether the original applies a kind-102 row that sits in a *team bonus* is
  unverified; `barred_by` reads only the tech-tree effect today, so those two rows are inert. That is a question for the data,
  not for the design.
- **Unique units and unique techs.** They are not derived from the tech tree at all (tree row T5), and they are a content
  question before they are an engine one.

## Where it would go

With **T2** (the effect interpreter's remaining kinds) and before civ breadth (**M2**), because breadth multiplies every call
site the table replaces, and because T2 is what names the kinds the table would carry. It is not on any hot path except
`modifiers_for`'s current callers, so it needs a before/after tick measurement only where it replaces that function.
