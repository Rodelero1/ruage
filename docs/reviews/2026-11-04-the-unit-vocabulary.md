# Review: the unit vocabulary, planned and inventoried

*Step (D119): the owner accepted the name-keyed proposal and asked for it, with a modding condition — "what if I chose that
scout cavalry will be upgrade of hussar, not the other way around?". This step is **the plan and the inventory**, not the
schema: `docs/unit-vocabulary.md` plus `tools/vocabulary.py`, which every number in the plan is measured against. No engine
code changed.*

**The inventory changed the proposal, which is why it came first.** I had written `upgrades_from = "militia"` per unit. The
pack cannot express that: **30 units have two possible predecessors** and the upgrade graph has **3 cycles** (527, 528, 1104).
The unit of content is therefore the **edge** — `from`, `to`, and the `tech` that performs it — which is also the modding
answer: reversing a line is a data edit and the loader must *detect* a cycle rather than assume acyclic.

Measured (2026-11-04, `empires2_x2_p1.dat`): **1,389** units, **56** unnamed, **28** duplicated names (`BARRICADE` ×4,
`BGAA` ×3, …), **223** upgrade edges, 43 bases with several variants (an age-up: normal), **40** roles the engine already uses
— every one naming a real unit, no two roles on one unit.

## Performance

No engine code, so no tick cost: the acceptance match is untouched by construction (`665 passed / 0 failed / 19 ignored`, hash
unchanged at `82efabd6f614c798`). The tool reads three TOML files (**~2.4 MB**) and runs in **0.7 s**, which is why it can gate
a future step rather than being a one-off script.

## Stability

| attack | result |
|---|---|
| a unit with two predecessors | counted (**30**) instead of silently picking one — the case that killed the per-unit field |
| a cycle | detected and printed (3 in the dat today), so the loader phase cannot loop |
| `[3, from, -1]` commands | filtered: a negative field is the dat's "no target", not an edge — the first run counted them and reported a phantom three-way ambiguity |
| an unnamed or duplicated name | counted, and the tool exits **1**, so a name-keyed schema cannot be declared ready on a pack that lacks keys |

**Two of my own errors, found by running the tool rather than trusting the first output:** the edge direction was labelled
backwards (the effect's `[3, from, to]` becomes `from → to`; my first version reported 67 "units upgraded by two effects" that
were simply age-ups with two variants each), and the phantom `-1` targets above. Both are why the plan's numbers were
re-derived after the fix rather than quoted from the first run.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
