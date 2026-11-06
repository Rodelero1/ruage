# The second review, the rest: 41 loader findings checked

*2026-11-06/07. The triage of 2026-11-05 verified **14 of 205** findings and sorted the rest by shape. This pass takes the
next block - every finding filed against `crates/data/src/`, the content loaders - and gives each one a verdict, with the
code it points at read. The remaining 164 (sim 63, tests 59, config 26, other 16) are named at the end with what each class
needs, and are **not** claimed as read.*

## Method

For each finding: open the cited file at the cited lines, read the claim's mechanism, and decide one of

- **confirmed** - the code does what the review says, and it is worth fixing
- **confirmed, low** - true, but the pack cannot reach it or the cost is trivial
- **refuted** - the mechanism is wrong, or the premise is
- **fixed** - already dealt with between the review and tonight

Line numbers in the review have **drifted**: two of the 41 point at code that has since moved (`economy.rs:252` is now
`struct Train`, `economy.rs:935` is `rate_from`). Every verdict below is against the **current** tree, not the quoted line.

## A class-level refutation first

**Four findings rest on a false premise about Rust's `as` casts.** `f64 as i32` and `f64 as u32` are **not** undefined
behaviour and do **not** wrap: since Rust 1.45 they **saturate** (NaN → 0, too large → `MAX`, too negative → `MIN`). A cast
can therefore be *unvalidated* - and some are - but it cannot be UB, and it cannot wrap. Findings [#127 units], [#935
economy], [#493 techs] and part of [#2] repeat this; each is marked below for what it actually is.

Second, `format!("{value}")` on an `f64` never produces scientific notation (`{}` is the Display form, decimal always;
`{:e}` is the exponent one), so `crates/data/src/lib.rs:188`'s round-trip is safe from that specific worry - though NaN and
infinity do reach `Fixed::parse`, which refuses them rather than rounding.

## The 41 findings

| # | where | verdict | what the tree says |
|---|---|---|---|
| 1 | `civs.rs:34` `Civ::bars` always `false` | **refuted as a bug, confirmed as a stub** | It really does return `false`. It also has **no caller** in the workspace (checked twice, last at D128), so nothing is misreported today. It is a dead API, not a broken rule. |
| 2 | `civs.rs:49` duplicate civ ids accepted; `by_id` is a copy of `get` with a name-lookup doc | **confirmed** | `get` and `by_id` are the same body; `by_id`'s doc says "look a civ up by the name content uses". The loader has no duplicate-id check. |
| 3 | `civs.rs:43` `civs` public, `by_name` private | **confirmed, low** | The field is `pub`, the index is private: an outside writer can desynchronise them. No such writer exists today. |
| 4 | `civs.rs:92` any negative value becomes `None`, not just `-1` | **confirmed, low** | `u32::try_from(value).ok()` - so `-2` is treated as "no effect". Needs a loader-side floor of `-1`. |
| 5 | `combat.rs:58` hardcoded `SCALE = 1_000_000` | **confirmed** | A scale the pack does not own, `unwrap_or(i32::MAX)` masking an out-of-range sentinel, and `i32::MAX / 2` for an unresolved one. |
| 6 | `combat.rs:175` `garrison_eject_fraction` range unchecked | **confirmed** | The raw field is a plain `f64` with a default and no range guard; the simulation's `unwrap_or(Fixed::ZERO)` then decides silently. |
| 7 | `combat.rs:155` `projectile_speed` / `accuracy_scatter` / `aggro_radius` accept negatives | **confirmed** | Same shape as #6: defaults are documented, negatives are not refused. |
| 8 | `combat.rs:61` `damage_rules()` returns an unnamed `(i32, i32)` | **confirmed, low** | True, and it is the crate's only unnamed pair. |
| 9 | `combat.rs:189` `bonus_resist_default` parsed and never read | **confirmed, low** | No reference outside the loader; it is a placeholder that a pack can set to no effect. |
| 10 | `combat.rs:184` NaN/Inf reach the resolver | **confirmed, low** | True; `resolve` then refuses, so the failure is loud but arrives late. |
| 11 | `combat.rs:65` integer division truncates the sentinel | **confirmed, low** | `9999.9` → `9999`. Armour classes are integers, so a whole-number check at load is the right fix. |
| 12 | `economy.rs:935` `scale_percent` cast unchecked | **partly confirmed** | It is unchecked: `(raw.ring.scale * 100.0).round() as i32`. It does **not** wrap - it saturates. A bounds check is still worth having. |
| 13 | `economy.rs:252` `fell_hp_per_tick` per tick, `f64` in the tick | **fixed** | D129: the rate converts once at load into `fell_milli_per_second` and the tick carries an integer remainder. The review's arithmetic underneath was wrong by 1,600×. |
| 14 | `economy.rs:1010` cross-references not validated (`build.requires`, `footprint`, `hunt.*`, `age.*`, `start.stock`) | **confirmed** | None of those vocabularies is checked at load; a typo becomes a silent no-op. This is the single largest real find in this block. |
| 15 | `economy.rs:430` share total uses `u32` sum | **confirmed, low** | Arithmetic is right that it can overflow; reaching it needs a row summing above 4.29e9 %, which only a hand-written pack can write. A `u64` accumulator is one token. |
| 16 | `economy.rs:442` duplicate `at_villagers` rows accepted | **confirmed** | The row sum is validated (`sums to {total}%, not 100%`); duplicate thresholds are not. |
| 17 | `economy.rs:1083` `[hunt]` role lists unvalidated | **confirmed** | Same as #14, narrower: four lists, none checked against `[role]`. |
| 18 | `economy.rs:1065` `age.requires` keys unchecked | **confirmed** | `age.order` *is* validated against `[tech_role]`; the requirement entries are not, so a typo silently blocks an age. |
| 19 | `jobs.rs:91` duplicate `target`/`column` accepted | **confirmed, low** | `name_for` takes the first match; nothing rejects a duplicate. |
| 20 | `lib.rs:188` `resolve` round-trips through `format!` | **refuted in mechanism, confirmed as a guard gap** | `{}` never emits an exponent for `f64`, so the round-trip is exact; NaN/Inf do reach the parser, which refuses them. A `is_finite()` guard is cheap tidiness, not a defect. |
| 21 | `map.rs:75` `is_walkable` accepts only Grass | **fixed** | D132, tonight: `Grass \| Dirt`, with the owner's own words in the comment. Its interchange with `Terrain::walkable()` is now documented at both ends. |
| 22 | `map.rs:226` negative `hp` accepted | **confirmed** | `amount == 0` is refused; a negative hit-point resource is not. |
| 23 | `map.rs:279` two resource nodes on one tile accepted | **confirmed** | No occupied-tile set during loading. |
| 24 | `overrides.rs:262` negative `los`/`speed`/`work_rate`/`range`/`reload`, no `range >= min_range` | **partly confirmed** | `Hp` and `Garrison` are validated (`hp <= 0` is refused by name); the five physical fields are not. The blanket claim is too wide; the specific one stands. |
| 25 | `overrides.rs:303` `Change::old()` prints `MaybeFixed(Some(1.0))` where `written()` prints `0` | **confirmed, low** | Two formatting paths for one log line, plus an unreachable arm that suppresses exhaustiveness. |
| 26 | `overrides.rs:251` negative `train_time` accepted | **confirmed** | Not in the validated set. |
| 27 | `overrides.rs:410` negative attack/armour `class` or `amount` in `pairs` | **confirmed** | Cast to `i16` with no sign check. |
| 28 | `pack.rs:99` `bytes` map built and never used | **confirmed** | `bytes.insert(name, data)` and then every loader re-reads the path. Double I/O, and the handshake hash and the parsed values can come from two different reads. |
| 29 | `pack.rs:225` FNV-1a is not collision-resistant | **refuted** | There is no multiplayer and the hash is a determinism check, not a security boundary - recorded at D128. A cryptographic hash would be justified by a *networked* handshake, which does not exist. |
| 30 | `pack.rs:77` docstring says `(name, length, bytes)`, code has no length | **confirmed, low** | Documentation drift in a security-adjacent comment. |
| 31 | `techs.rs:493` hardcoded scale `1_000` for effect ids | **confirmed, low** | `resolve(1_000)` then `raw / 1_000`: a fractional id truncates rather than being refused. |
| 32 | `techs.rs:501` negative `command.a` becomes id 0 | **confirmed** | `u32::try_from(command.a).unwrap_or(0)` - a malformed command looks like a valid id 0. |
| 33 | `techs.rs:93` `from_id(9)` returns a decoy `Attack { class: 0, amount: 0 }` | **confirmed** | True as written; the only production caller special-cases 9. |
| 34 | `techs.rs:505` attribute 9 emitted without validating the packing | **confirmed** | The kind-4/5 arm exempts attribute 9 from `from_id`, then passes the raw value through. `Attribute::split` exists (`None` when it does not fit) and is not used here. |
| 35 | `techs.rs:449` `required.len() > u16::MAX` silently capped | **confirmed, low** | `unwrap_or(u16::MAX)` changes "all" into "65,535". Unreachable from generated data, reachable from a hand-written pack. |
| 36 | `techs.rs:451` `min_required > required.len()` accepted | **confirmed** | No cross-check. |
| 37 | `techs.rs:461` any negative `effect_id` becomes `None` | **confirmed, low** | Same shape as #4. |
| 38 | `units.rs:127` `f64 as u32` is UB outside range | **refuted** | `as` saturates; there is no UB. The guard `if tiles >= 1.0` also means the only reachable conversion is ≥ 1. |
| 39 | `units.rs:129` negative radius silently becomes one tile | **confirmed, low** | Real, and the loader is the right place to refuse it. |
| 40 | `units.rs:308` variant parents unvalidated (missing parent, cycle > 1) | **partly confirmed** | `root()` has an eight-hop cycle guard and its doc says so, so a cycle cannot hang the tick - it returns the wrong unit, silently. Load-time validation is absent. |
| 41 | `units.rs:308` (second finding, same lines) self-loop rejected, longer cycle not | **confirmed, low** | Same code as #40. |

**Tally, counted from the table rather than by eye: 17 confirmed (a real fix behind each), 15 confirmed but low, 3 partly
confirmed, 2 fixed tonight (D129, D132), 2 refuted outright, and 2 that are "refuted in mechanism, confirmed in effect" -
`Civ::bars` (a dead stub, not a broken rule) and `resolve`'s round-trip (safe as written, missing a finite guard).**
The two that matter most are **#14/#17/#18** - one defect wearing three hats: the economy loader copies role and resource
names into its tables without checking they exist, so a typo in content is a silent no-op at runtime instead of a refusal
at load.

## What is not in this pass

| class | findings | what it needs |
|---|---|---|
| `crates/sim/src` | 63 | the engine itself: read each against `step`, `apply` and the mover. The largest and most consequential block |
| `crates/*/tests` | 59 | 10 already refuted or fixed (the `fresh()` hole is D130; "discarded `submit` Result" is not a thing - `submit` returns `()`) |
| `config/` | 26 | content claims: mostly the owner's calls (army size is #27 of the first triage) |
| docs, tooling, other crates | 16 | read last; several are comments that drifted |

## Questions this pass raises (no deadline)

1. **Loader strictness.** For values that are wrong but not dangerous - negative resource `hp`, two nodes on one tile,
   negative radius - do you want a hard `ConfigError` (a pack refuses to load) or a warning? The project's rule so far is
   "a config that cannot mean what it says fails", and I would keep that: refuse.
2. **`Civ::bars`.** Delete the stub, or implement it against the effects table? Deleting is honest; implementing is the
   tech-tree work D3 has not asked for yet.
3. **`pack.rs`'s `bytes` map.** Wire it through so each file is read once (kills the hash-vs-parse split), or delete it?
   I would wire it through: the split is a real determinism hole in the pack identity.
4. **The pack hash.** FNV stays unless you want a multiplayer handshake. If you do, it becomes SHA-256 and a dependency.
5. **Duplicate `at_villagers` rows.** Refuse, or let the later row win? I would refuse - the file is authoring, and the
   ambiguity is invisible otherwise.

## Performance

The only measured claim in this block was #13, and it was measured in D129: a 20 hp tree fell on its **first** blow
(**30 hp/tick** at 20 Hz against an intended **0.075**), and after the fix it falls on tick **267** = **13.35 s**. Nothing in
this pass changed code, so nothing else is measured - the loader findings are all load-time and cost **0 ms** at runtime.

**711 passed / 0 failed / 20 ignored** (unchanged; this pass is a document).
