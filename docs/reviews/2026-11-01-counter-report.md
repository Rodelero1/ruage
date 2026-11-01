# The counter report: cost, and what it refused (2026-11-01)

Scope: `tools/counters.py` and `tools/test_counters.py` (new), plus one content line - `knight = 38` in
`config/economy.toml`'s `[role]` table. Analysis tooling; no engine code, nothing read at runtime by the sim. This is
the authoring half of **D91**: the numbers a countering `[[rule]]` is written from, derived from the pack.

## Performance

Measured on this container (2 CPUs, 4 GiB), in-process:

| what | number | how |
|---|---|---|
| load the pack (1,389 units, 28 roles, three TOML files) | **209 ms** | `counters.load()`, dominated by parsing `units.toml` |
| damage/dps over 49 ordered pairs of seven roles | **0.10 ms** | 42 `dps` calls |
| the full matrix for five roles (the CLI) | **0.4 s** wall including the 209 ms load | `python3 tools/counters.py …` |
| this module's tests | **0.61 s**, 9 tests | `python3 -m unittest discover -s tools -p 'test_counters.py'` |
| the whole `tools/` suite | **3.2 s**, 55 tests | `python3 -m unittest discover -s tools -p 'test_*.py'` |

The load cost is TOML parsing, not arithmetic - 0.10 ms for 49 pairs says the matrix itself is free. If this ever became
hot it would move into `crates/data` beside the other load-time derivations, which is where **D91** says it goes when
the engine needs it; today the engine does not, because **D93** keeps the choice in the script.

## What it says, as a check on itself

Against a knight (100 hp, 60 food + 75 gold), damage per resource-second: **halberdier 0.200, pikeman 0.133, spearman
0.089, militia 0.013** - the order an AoE2 player expects, and derived rather than written down. The knight answers all
four for 10 a blow, which is what inflicting 0 in armour class 4 means.

## Stability

Attacked, with the outcome of each:

- **A role the pack does not name** - `UnknownRole`, not a silent skip; pinned by a test. A silent skip is how a report
  comes back one row short and looks authoritative.
- **A role that names a unit the pack does not carry** - also `UnknownRole`, naming both the role and the id. The two
  failures are different and say which one happened.
- **A unit with no cost** - `dps/cost` is left at 0.0 rather than dividing by zero, and the report still prints the row.
- **A unit with no reload** - `dps` returns 0.0 instead of raising; a zero reload means the data is wrong, and the
  report showing 0 is a louder signal than a crash in a loop over 1,389 units.
- **A class the target does not list** - contributes **nothing**, because the engine assumes a huge armour value there
  (`armour_missing_sentinel = 10000`). This is the one place a plausible-looking reimplementation gets the game wrong:
  treating an unlisted class as zero armour would make every unit a counter to every other. Pinned by
  `test_a_class_the_target_does_not_list_contributes_nothing`.
- **The floor** - the total never goes below `combat.toml`'s `minimum`, which is read from the file rather than
  hardcoded, so the tool cannot drift from the engine on it.
- **Bad TOML / missing files** - the CLI prints the error and exits 1 rather than a traceback.

### My own errors, in this slice

- **I asserted the wrong number and the tool was right.** My real-pack test expected **16** damage from a pikeman to a
  knight, because I had read unit 93's row. The pack's `pikeman` role is unit **358**, the upgraded pikeman, whose
  cavalry bonus is **22** - so the answer is **24**. The test now says 24 and explains why; the comment matters, because
  the next reader will make the same mistake with the same two units.
- **The test found a missing role rather than a bug.** `economy.toml` had no `knight` at all: the role table had only
  ever held what the engine needed, and the test asked for the one unit a spear line exists to answer. `knight = 38` is
  now content, with the reason in a comment.

## Not attacked, and named

- **Cost is a plain sum across resources.** Wood is not gold, and a market makes the exchange rate explicit; the report
  prints the per-resource split beside the total so a reader can overrule the sum. It is an author's yardstick, and the
  docstring says so rather than pretending to a valuation.
- **The matrix ignores range, speed and hit points per cost.** A knight's speed is why it beats archers in practice; a
  damage-per-resource table cannot see that. This is a first-order tool for writing rules, not a balance model.
- **Nothing here reads modifiers** (tech bonuses, civ bonuses). The engine's damage uses *modified* attacks/armours; the
  tool uses the base row. That is deliberate - a countering rule should be written against base units - but it means the
  numbers are pre-upgrade, and the docstring on `load` should be read as "the pack as shipped" rather than "in a
  Castle-Age fight".
