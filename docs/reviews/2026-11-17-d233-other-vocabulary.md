# D233 amendment - the `other:` vocabulary, and the build it broke

**Scope.** `crates/data/src/techs.rs` (`knows_attribute`), `crates/data/src/lib.rs` (export), `crates/data/tests/techs.rs`
(three fixtures changed or added), `docs/decisions.md` (D233 amendment).

**How it was found.** The owner, not a test: `git pull && make build && make run` after D233 died with
`effects.toml: effect[192].modifiers.attribute: "other:12" is not in the pack's attribute vocabulary`. His generator writes
`other:12` on Fletching; the pack this tree generated carries none. `config/gamedata/` is generated per machine and is not
tracked, so the list D233 closed was a **census of one machine**, presented as a vocabulary.

## Performance

**No steady cost, measured as unchanged work.** The change is inside `knows_attribute`, which runs **once per modifier at
pack load** and never on a tick: 798 effects, ~1,300 modifiers. It replaces two slice scans (`NAMED_ATTRIBUTES.contains`,
`UNREAD_ATTRIBUTES.contains` - 29 string comparisons) with **13 string comparisons plus a prefix test and a byte scan**, so a
load is the same or slightly cheaper; nothing is stored and nothing is allocated. The pack load the earlier entries measured at
**~80 ms** is unchanged by a sub-microsecond function, and no measurement is claimed beyond that: the acceptance match is
`2f1ab0ae663e2506` before and after at 48,000 ticks, which is a *behaviour* check, not a timing one.

## Stability

**Attack 1 - the fix opens the vocabulary too far.** It does not: only `other:<digits>` and `unpacked:<digits>` are accepted.
`attackk` is still refused by name (`an_attribute_name_the_pack_vocabulary_does_not_carry_is_refused` is unchanged and green),
and four malformed shapes are refused with the name in the message (`a_malformed_other_name_is_refused`).

**Attack 2 - the new fixture does not actually test the old rule.** Mutation-proved the other way round: restoring the closed
`knows_attribute` (the shipped line, unchanged) reddens `an_other_row_the_census_did_not_see_loads` and leaves the census
fixture green - which is exactly the shape of the original defect, since the local pack carries no `other:12`.

**Attack 3 - the end-to-end path is not the one that broke.** It was reproduced against the real binary, not a unit test: a copy
of `config/gamedata/effects.toml` with `other:12` added to effect 192's modifiers is **refused** by the pre-fix release build
(`EXIT 1`, the owner's exact message) and **runs** after it (`tick=100 entities=128 players=2 hash=266bcdf711e7824d`).
`ruage-view`'s `window` was rebuilt too, because `make run` is what failed.

**Attack 4 - the census test can only pass on one machine.** It could, and did - that was the second half of the bug. It now
asserts the invariant (*every name the pack carries is one the vocabulary can answer for*) and reports the count in the failure
message instead of pinning it at 29.

**Not attacked, and why.** The value a row carries is still not validated against its attribute (an `other:12` with any number
loads); nothing reads those rows, so there is nothing to validate against until a slice reads them, and inventing a range would
be the same mistake in a new place.
