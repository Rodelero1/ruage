# Review: the villager-share table moves into the personality (D94's other half)

*Change: `[policy]` was the last piece of AI living in `economy.toml`. `Economy` loses the field, `Personality` gains
`policy: Policy`, `Scenario` carries the resolved table so a runner has something to install, and
`World::apply_content_policy()` becomes `apply_policy(&Policy)` at its five call sites (`headless`, `batch`, `window`,
`scenario.rs`, `frame_cost.rs`). `Policy` now deserialises itself in `crates/data` — the sum-to-100 refusal moved with
the table and names the personality file — a personality that sets shares but no rules is a legal AI, a scenario may
carry its own shares only when it writes its own rules, and an `economy.toml` that still carries `[policy]` is refused
by name with the destination. Content: 19 rows moved from `config/economy.toml` to `config/ai/base.toml`.*

## Performance

Everything here is **load time**, once per match; `git diff 57bf7c1..HEAD -- crates/sim/src/lib.rs` has no hunk inside
`step`, and the probe below is the number that says the table did not become a per-tick cost (release, 200 reps for the
loads, 1,000 for the install):

- **pack load + identity, 11 files hashed: 92.70 ms each.** The walk hashes `economy.toml` and every `ai/*.toml`; the
  same bytes moved from one file to the other, so the walk reads the same total. The 92.7 ms is parsing `units.toml`
  (521 KB, 1,166 records), not the hash — the hash on its own was 12.8 µs when it was measured separately.
- **personality parse (14 rules + 19 share rows): 234.8 µs each.**
- **`apply_policy` (19 rows resolved to stockpile slots): 1.47 µs each, once per match.**

The stronger claim than any microbenchmark: **the acceptance match is byte-identical across the change.** 36,000 ticks
(30:00 game time, seed 7) on a `57bf7c1` worktree with the same `config/gamedata/` and on this tree produce the same
521 report lines, the same rule firings `[316, 6, 15, 1, 19, 28, 29, 1, 2, 17, 41, 1, 79, 41]` and the same final
`hash=98c1c2a051d044f6`. A table that changed hands but not behaviour is exactly what that diff should show.

## Stability

**Six mutations, each caught by the test that was written for it** (delete the guard, watch the intended test fail, put
it back):

| Mutation | Test that failed |
|---|---|
| sum-to-100 refusal removed (`if total != 100` → `if false`) | `a_share_row_that_does_not_sum_to_100_is_refused_with_its_file` |
| stale-`[policy]` hint removed (falls back to `unknown field`) | `an_economy_that_still_carries_the_policy_block_is_refused_by_name` (`contains("ai/")`) |
| `scenario.policy = personality.policy` deleted | `a_personality_carries_the_villager_shares` (`left: 0`, "both rows arrived") |
| emptiness check reverted to rules-only | `a_personality_that_only_sets_shares_is_still_an_ai` |
| scenario's own `[policy]` arm dropped from the both-sources check | `a_scenario_with_shares_and_a_personality_is_refused` |
| `install_policy` made a no-op | `a_personalitys_shares_reach_the_world` (`left: 0`, "every villager should be on wood: []") |

**The gap the review found, and closed.** The loader's comment promises that a scenario *without* a personality may
carry its own shares; nothing exercised that arm — every policy test went through `ai = "base"`. It is now
`a_scenario_that_carries_no_personality_may_carry_its_own_shares`, and it passes on the same code (a guard for a
documented promise, not a defect fixed).

**A behaviour change, named rather than buried.** A headless run with **no `--script`** used to install the economy's
share table regardless; it now installs nothing, because there is no AI to take a table from. That is D94 read
literally, `make headless` always passes `acceptance.toml`, and a bare `--ticks` run is a probe rather than a match —
but it is a change, so it is written down.

**Refusals, so a wrong file is loud:** a share row that does not sum to 100 fails at the personality parse with the file
and the count ("sums to 90%, not 100%"); an economy carrying `[policy]` is refused with the key `[policy]` and the
sentence that it lives in `config/ai/<name>.toml`; a scenario carrying both a personality and its own shares is refused
and says which behaviour it found. A personality with neither rules nor shares is still refused, as before.

**Known and accepted:** `RawShare`'s flattened map turns a misspelled key into a share for a resource of that name
(pre-existing; only a row that then sums to 100 would survive, and the share is skipped by `slot_of`); a share naming a
resource the pack lacks is skipped rather than refused (pre-existing, deliberate — the pack owns the columns); and the
SPEC changelog has **no rows for v1.53–v1.56** — the AI folder, the names work and the knob deletion landed without
them, which is not this slice's doing and is not reconstructed here.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
