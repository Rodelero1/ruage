# Review: a militia was called a champion

*Regression fix (the owner: "player trains champions in dark age and has hussar instead of scout cavalry"). It is a **name**
bug, not a simulation one.*

`role_name` finds a unit's role by **family root**, because an age-up rewrites a building's id and the exact id then has no
role (the Feudal town centre is 71, and the panel once said "unit 71"). The fallback had no tie-break: `economy.role` is a
map ordered by name, so `find` returned the **alphabetically first** member of the family — and for an upgrade line that is
the *top* tier. A Dark Age militia (74) read as **champion** (567); the opening scout (448) read as **hussar** (441).

**The simulation was always right.** A headless dump at tick 20 shows unit **448** spawning and **74** queued by the barracks;
`units.toml` names them SCOUT and SPRMN with militia-class stats. Only the label was wrong — on the barracks' card, in the
panel, and anywhere else that asks the pack for a word.

**Fix, in `role_name`:** its own role first (exact id), then the **family's base role** (the role whose id *is* the root — what
the fallback exists for), then any role in the family by **lowest id**, so the answer is an ordering decision rather than
alphabetical luck.

**Why the suite missed it:** the card tests compared `role_name(x)` against `role_name(y)`, which is consistent when both are
wrong. The new test (`a_unit_is_named_by_its_own_role_not_by_a_later_tier`) compares against the **role names the pack
declares** — militia, scout_cavalry, spearman, knight, villager — and it failed with `Some("champion")` before the fix.

## Performance

No cost: the lookup is one map scan over ~40 roles, on a panel or card build, never in a tick. The acceptance match runs
36,000 ticks in **10.39-10.77 s** (three runs before this change) and is **unchanged**, hash `82efabd6f614c798` — which is the
proof that this is a display fix and not a simulation one. The three card tests that exercise the lookup finish in **3.6 s** at
debug.

## Stability

| attack | result |
|---|---|
| a base unit of a line (74 militia, 448 scout) | named by its own role; `champion`/`hussar` no longer appear |
| an age-up building variant (71, and 463 for a house) | still named `town_center` / `house` by the family base — the case the fallback exists for |
| a role-less id in a named family | lowest id in the family wins, deterministically, instead of alphabetical order |
| a unit the pack does not name | `None`, as before — the panel's "unit N" fallback |

**The real lesson, and it is the owner's next question:** this bug exists because the engine carries **two vocabularies** — dat
ids for the payload, role names for the rules — and a lookup has to cross between them. A name-keyed pack (D118, proposed in
`docs/next-step.md` Session 16) deletes the crossing: the key *is* the name, so a unit cannot be called by a later tier.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
