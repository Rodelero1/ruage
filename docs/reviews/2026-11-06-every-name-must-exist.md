# Every name must exist

*D137. The second review's `economy.rs:1010/1065/1083` findings, implemented. Seven tests, watched failing first, and one
half of the review refused with the pack's own comment as the evidence.*

## The defect, as it actually is

The 41-finding pass filed this as *"the loader copies role and resource names into its tables without checking they
exist"*. Reading the validation block before writing any code **corrected that**: four of these names are already refused -

- `build.farm` (line 891)
- `build.farm_spacing` (897, a range)
- `build.spaced_roles` (908, with a comment that says exactly why: *"or the rule silently applies to nothing"*)
- `age.order` (882, against `[tech_role]`)

and six are not:

| key | what a typo does today |
|---|---|
| `build.requires` (building **and** its prerequisite) | the prerequisite never matches; the building is buildable from the start |
| `build.blocking_roles` | the wall segment blocks nothing; units walk through it |
| `footprint` keys | the footprint is never used; the role falls back to a solid tile |
| `hunt.herdable_roles` / `animal_roles` / `carcass_roles` / `butcher_roles` | the list names nobody: sheep are not livestock, corpses are not butchered |
| `age.requires` keys and their `of` / `single` | the age can never be counted, so it is never reachable |
| `[metrics] military_roles` | **nothing, and that is by design** - see below |

## The refusal

```
economy.toml: hunt.herdable_roles: "sheeep" is not a name in this pack's [role] table
```

The key **and** the name, because "invalid" is what the typo already was. The message says which table was consulted, so a
role that belongs in `[tech_role]` is distinguishable from one that belongs in `[role]`.

## `[metrics] military_roles` is refused, deliberately

The review asked for this one too. The pack answers it at line 377:

> *A role that the pack cannot name is skipped rather than refused - a metric is a measurement, not a rule of the game.*

That is the right rule and it is already the code's behaviour (`read_army` filters by `role.contains_key`). Adding a refusal
would turn a **measurement** into a **load failure** and prove nothing about the game. So this half of the finding is
**refused**, with the comment as the evidence - recorded in D137.4 rather than quietly dropped.

## Test-first, and the RED

Seven tests added to `crates/data/tests/economy.rs`. Each takes the **real** `config/economy.toml`, swaps exactly one name
for one the pack does not have, and asserts the refusal names both the key and the swapped name. Using the real pack, not a
fixture, is deliberate: it also proves the shipped content passes all seven.

Helper, written before the tests it serves:

```rust
fn pack_with(from: &str, to: &str) -> String {
    let text = std::fs::read_to_string(repo("config/economy.toml")).expect("config/economy.toml");
    assert!(text.contains(from), "config/economy.toml no longer contains {from:?}: this test's premise has moved");
    text.replacen(from, to, 1)
}
```

The `assert!` on the premise matters: a test that silently swaps nothing would pass for the wrong reason, which is the
D130 defect in a new dress.

**RED, watched** (`cargo test -p ruage-data --test economy`):

```
test result: FAILED. 30 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

failures:
    a_blocking_role_the_pack_lacks_is_refused
    a_build_prerequisite_named_by_a_role_the_pack_lacks_is_refused
    a_build_prerequisite_whose_role_does_not_exist_is_refused
    a_footprint_for_a_role_the_pack_lacks_is_refused
    a_hunt_list_naming_a_role_the_pack_lacks_is_refused
    an_age_requirement_counting_a_role_the_pack_lacks_is_refused
    an_age_requirement_for_a_tech_role_the_pack_lacks_is_refused
```

Every failure was `expect_err` returning an `Economy`: the loader accepted all seven typos, which is the finding confirmed
by execution rather than by reading.

**GREEN**: `test result: ok. 37 passed; 0 failed` in the same 0.05 s.

## The shipped pack was clean before the check existed

A cross-check script read `config/economy.toml`, collected `[role]` (40 names) and `[tech_role]` (6), and walked every
reference the new check covers: **0 mismatches**. So this change refuses nothing today - which is the point: it refuses the
typo that *will* be made, and it costs zero runtime (it runs once, at load, over 40 + 6 names).

That also means the acceptance hash must not move, and it does not: **`f328ebc8d4d34b78`** before and after.

## What was not done

- `start.stock` is **not** checked: it names resources, and this loader does not hold the resources table. It belongs with
  whatever validates a pack as a whole (there is no such pass yet), not here.
- The other 34 findings from this block are unchanged: 17 confirmed, 15 confirmed-but-low, 3 partly, 2 refuted. They are
  listed with verdicts in `docs/reviews/2026-11-07-second-review-the-rest.md`.

## Performance

- **Load**: the check is six `BTreeMap::contains_key` walks over a **40**-entry `[role]` table and a **6**-entry
  `[tech_role]` table. The whole `economy` test binary - 37 tests, each parsing a TOML and 7 of them parsing the real
  1,100-line pack - finishes in **0.05 s**, so the check is inside the measurement noise. Nothing allocates on the success
  path; the `format!` strings exist only in the error arms.
- **Runtime**: **zero**. It runs once, at load, before `Economy` is handed out, and no simulation code touches it.
- **The match is unchanged and unmoved**: `f328ebc8d4d34b78`, **141** entities, **188** repaths, **10.7 s** for 36,000
  ticks in release - the same hash, count and time as before the change, which is the only admissible proof that a load-time
  check did not alter behaviour.

## Stability

- **It cannot panic.** Two `contains_key` lookups and a `format!`; no index, no `unwrap`, no arithmetic.
- **The failure mode moves from silent to loud**, which is the whole point: a pack with a typo used to load and misbehave at
  tick 5,000 (a wall that blocks nothing, an age that never arrives); it now refuses at load and names the key and the name.
- **No false refusal today**: a cross-check of `config/economy.toml` against its own tables found **0** mismatches over 40
  roles and 6 tech roles, and the whole suite - **718** tests, every fixture that needs the pack loading that file - is green with it.
- **The D130 rule is preserved**: pack absent -> fixtures skip; pack present and unloadable -> fixtures fail. Seven of those
  fixtures now fail for a new, well-named reason, which is the direction the rule wants.
- **Scope is bounded on purpose**: `start.stock` is not checked (it names resources, and this loader holds no resources
  table) and `[metrics] military_roles` is excluded by the pack's own stated rule. A check that guesses outside its table
  would refuse valid content, which is worse than the typo it catches.

**718 passed / 0 failed / 20 ignored** (+7 new tests); clippy `-D warnings` clean; `cargo fmt --check` clean.
