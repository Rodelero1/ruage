# The AI's identity and its name: closing the hole the review found (2026-11-02)

Scope: `crates/data/src/pack.rs` (`AI_DIR`, the directory walk, the hash feed, `differences` over the union of parts,
`PACK_FILES` 10 -> 9), `crates/data/tests/pack.rs` (`copy_pack`, one new test, the parts count),
`crates/sim/src/script.rs` (`personality_path`), `crates/sim/tests/scenario.rs` (one new test).

This is the landing gate from the OCR review: **item 2** (a personality outside `PACK_FILES` was not hashed - the
`civs.toml` hole from `bdbcf48` reintroduced from the other side), **item 1** (a personality name could leave
`config/ai/`) and **item 10** (`ai = ""` resolved to `ai/.toml`). One mechanism, because both are about "what may a
personality name be, and what does the pack know about it".

## What changed

- **The names are walked, not listed.** `PACK_FILES` is a static array and cannot say "whatever is in `ai/`"; the loader
  now reads every `*.toml` under `ai/`, **sorted by name**, and feeds each one into the hash after the fixed files. A
  scenario may name any personality, so the pack must know about every one it could name.
- **`differences` compares the union of parts** rather than a fixed list, so a file one peer has and the other does not is
  named - which is exactly the case a refusal must explain.
- **A name must be a name**: lowercase letters, digits, `-`, `_`, non-empty, at most 64 bytes, checked **before any
  filesystem access**. Verified against the real `Path::join` with `rustc` before the rule existed: `"../economy"` gave
  `config/ai/../economy.toml` and `"/etc/passwd"` gave `/etc/passwd.toml`.

## Performance

Measured on this container, `rustc -O`, 2,000 iterations of the added step (read `ai/`, read the 5,146-byte
`base.toml`, FNV-1a over it, sort the names):

**12.8 us per pack load**, 25.6 ms for 2,000 loads.

That is the whole added cost: it runs once per `ContentPack::load`, not per tick, and it is 0.0128 ms against a load
that already reads nine files and parses five of them. The pack suite (8 tests, each loading the pack at least once and
several copying it to scratch) runs in **8.44 s**, which is the same order as before the change and dominated by the
scenario tests elsewhere.

`differences` is now a filtered set difference over two `BTreeMap`s of 11 parts instead of a loop over 10 names: 22
lookups, no allocation beyond the result vector. It is called at handshake time, not in a match.

## Stability

- **Determinism of the walk**: names are sorted before hashing, so the identity cannot depend on `read_dir`'s order,
  which is not specified. A test pins the *content* case; the ordering case is pinned by construction (the sort is the
  only ordering).
- **An unlisted personality is now visible**: adding `ai/rush.toml` changes the identity and `differences` names
  `ai/rush.toml`. That is the whole point, and it is the test that failed before the change.
- **A name that could escape is refused before the filesystem is touched**, so a scenario naming `../economy` gets a
  sentence about names rather than a confusing "cannot be read" from a file it was never meant to reach.
- **A pack without an `ai/` folder still loads** (the walk is skipped), so a minimal pack is not broken by this; it
  simply has no personalities.
- **The identity changes for every existing checkout again** - a personality is now fed after the fixed files rather than
  inside them. Expected, and the reason two builds from either side of this commit should not join.

### RED, as it was watched

- `a_personality_outside_the_fixed_list_is_part_of_the_identity` failed with `assertion left != right failed: a
  personality the peers do not share is a different game` - the old pack hashed nothing for `ai/rush.toml`.
- `a_personality_name_that_could_escape_the_folder_is_refused` failed with the real escape in its own message:
  `names personality "../economy", but .../ai/../economy.toml cannot be read` - which is the defect, printed.

## Not attacked, and named

- **The walk is more permissive than the loader.** `ai/Bad-Name.toml` is hashed by the pack but cannot be named by any
  scenario (the allowlist refuses capitals). Harmless - it makes the identity stricter than the loader, so it can only
  cause a spurious refusal, never a missed one - but it is an asymmetry worth knowing.
- **A `.toml` that is not a personality at all is hashed, not parsed.** The pack does not validate personalities; only
  the scenario loader does, when one is named. A pack can therefore carry a broken personality file and load fine.
- **Symlinks are followed.** A `ai/link.toml` pointing outside the folder is read and hashed. It cannot be *named*
  (the allowlist is about the name, not the target), and anyone who can write `config/ai/` can already change the game,
  so this is a note rather than a hole.
- **Non-UTF-8 filenames** are refused with an error naming the directory, not skipped; a pack that cannot name a file is
  a pack whose identity is ambiguous.

## Addendum: the personality's own schema, an empty refusal, and two hygiene items (2026-11-02)

Second commit of the same review, items 3, 4, 8 and 12. The landing gate above is `c370599`.

- **`PERSONALITY_SCHEMA = 1` now exists and is checked.** Item 3 was right in substance and wrong in constant: the
  personality is not a scenario, so it moves on its own version (`SCENARIO_SCHEMA` is 2), and the refusal names both
  what it read and what this build reads. The scenario's `schema` check still lives in `Runner::new` (item 5, deferred
  with the code that touches it).
- **An empty personality is refused** (item 4). It would play the match doing nothing - the same silent failure that
  `load_scenario_str` refuses for `ai`, and the contradiction the review pointed out.
- **The one-tier count assertion is gone** (item 8). `assert_eq!(scenario.rule.len(), 14)` coupled a sim test to content
  tuning; it is now "the rules arrived and are non-empty", with the behavioural proof left where it belongs - the
  acceptance run, which plays the opening for a minute and refuses if it does not work.
- **The Loom comment now describes the rule it is attached to** (item 12). The 120 s gate is ours, not the game's, and
  the comment says so and why (50 food against villager production at twelve). Removing it stays an experiment with
  acceptance numbers, not a code change made on a hunch.

### Mutation testing

| mutation | result |
|---|---|
| `personality.schema != PERSONALITY_SCHEMA` -> `if false` | **FAILED** - `a_personality_from_a_different_schema_is_refused` |
| `personality.rule.is_empty()` -> `if false` | **FAILED** - `an_empty_personality_is_refused` |

Both guards are pinned.

### Honest note on ordering

Items 3 and 4 were written **implementation and test in the same edit**, then proven by mutation rather than by a
watched RED. That is the third time today the tests did not come first; the mutation run is what establishes they bite,
and it is recorded here rather than presented as test-first.
