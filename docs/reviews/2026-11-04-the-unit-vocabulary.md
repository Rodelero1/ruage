# Review: the unit vocabulary, planned and inventoried

*Step (D119): the owner accepted the name-keyed proposal and asked for it, with a modding condition — "what if I chose that
scout cavalry will be upgrade of hussar, not the other way around?". This step is **the plan and the inventory**, not the
schema: `docs/unit-vocabulary.md` plus `tools/vocabulary.py`, which every number in the plan is measured against. No engine
code changed.*

**The owner caught an error in my first reading, and it is the more useful half of this note.** I reported "30 units have two
possible predecessors" as evidence a per-unit `upgrades_from` could not work. *"Halberdier is reached from something else than
pikeman? How? Upgrade path is clear pikeman -> halberdier"* - and the pack agrees with him: both the spearman (93) and the
pikeman (358) edges for the halberdier (359) sit **inside effect 189, the Halberdier tech itself**. One tech in this dat
replaces *every* earlier tier of a line, so a chain that skips a rung still lands where it should. The correction, measured
after the tool was fixed to group by effect: **22** units have that same-effect fan-in and only **2** are reached from
genuinely different effects (`ARRG3` from effects 102 and 133; `WALL3` from two effects that both name `WALL2`). The two units
on a cycle are the fishing ships (`RMSHP` 527 and `SDGAL` 1104), which two effects really do upgrade into each other.

**The conclusion survives, for the right reason:** the *tech* is the actor in an upgrade, so the edge (from, to, tech) is the
unit of content, and a per-unit field would be a lossy projection of it. Also unchanged: the modding requirement, since
reversing a line is a data edit either way.

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
