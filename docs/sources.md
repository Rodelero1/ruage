# Sources

Everything harvested from outside this machine, with what it was used for and how far to
trust it. Version caveat throughout: **most community documentation describes DE**, while
this project targets HD 5.7. Where they disagree, the dat wins and the disagreement is
logged as a question (e.g. U11 farming).

## Numbers used in `docs/mechanics/`

| Source | Used for | Trust |
|---|---|---|
| [Armor class (Age of Empires II)](https://ageofempires.fandom.com/wiki/Armor_class_(Age_of_Empires_II)) | Damage formula, step-by-step derivation, min-1 rule, bonus stacking | High — agrees with two others |
| [Pierce armor](https://ageofempires.fandom.com/wiki/Pierce_armor) | Closed-form damage formula, elevation multipliers (1.25 / 0.75) | High |
| [Attack](https://ageofempires.fandom.com/wiki/Attack) | Full formula with k_res / k_mul terms | High |
| [aoe2database — Damage Formula](https://www.aoe2database.com/damage_formula/en) | Third independent statement; sentinel armour for absent classes | Medium — self-described approximation |
| [Resource](https://ageofempires.fandom.com/wiki/Resource) | Base gather rates per source (0.388 wood, 0.379 gold, 0.359 stone, 0.310 berry, 0.280 fishing ship) | High — matches the dat |
| [Villager (Age of Empires II)](https://ageofempires.fandom.com/wiki/Villager_(Age_of_Empires_II)) | Gather rate table, carry capacities, tech bonus list | High |
| [Food](https://ageofempires.fandom.com/wiki/Food) | Hunter carry 35, practical farm rate ~0.356 | High |
| [aoe2db — Farming & Economy Upgrades](https://aoe2db.com/farming-rates) | Double-Bit Axe +20% wood, Gold/Stone Mining +15%, farm food per upgrade | High — used to *verify* the dat's `c=13` multipliers |
| [aoe2.ai methodology](https://aoe2.ai/en/methodology) | Practical vs theoretical rates (farms 20.3–24 food/min) | Medium — planning assumptions |
| [aoe2-de-tools](https://aoe2-de-tools.herokuapp.com/villagers-required/) | Independent practical rates (0.39 wood, 0.34 farm) | Medium |
| [Jineapple's Infosheet (Steam)](https://steamcommunity.com/sharedfiles/filedetails/?id=2169547705) | Attack delay, reload resampling to 1/60 s, Matt Pritchard's float/OOS quote | High for mechanics, DE-era |
| [Tech tree lies (Steam)](https://steamcommunity.com/sharedfiles/filedetails/?id=191310499) | Proof that speed bonuses multiply **reload** (Celts ×0.8, Japanese ×0.75) | High — fits the dat's type-5 encoding |

## Tooling and format references

| Source | Used for |
|---|---|
| [SiegeEngineers/aoc-dev-resources](https://github.com/SiegeEngineers/aoc-dev-resources) | Index of the ecosystem (this is how `geniedoc`/`aoeres` were found) |
| [aap/geniedoc](https://github.com/aap/geniedoc) | Documents the *other* `.dat` files (terrain, blendomatic, filters) — **not** `empires2*.dat` |
| [withmorten/aoeres](https://github.com/withmorten/aoeres) | Versioned resource dumps, AoC/AoK — not yet mined |
| [stefan-kolb/aoc-mgx-format](https://github.com/stefan-kolb/aoc-mgx-format) | Recorded-game format; its header section is a **stub**, so it did not help with saves |
| [Divy1211/GenieDatParser](https://github.com/Divy1211/GenieDatParser) | The parser that produced `build/converted/*.json` (GPL-3.0 — tool use only) |
| [SiegeEngineers/genie-rs](https://github.com/SiegeEngineers/genie-rs) | Cross-check oracle for block counts (GPL-3.0) |
| [Tapsa/genieutils](https://github.com/Tapsa/genieutils) | Read as the format *specification* (LGPL-3.0) |
| [AoE forums — sharing saves](https://forums.ageofempires.com/t/how-to-find-and-share-save-game-and-recorded-game-files/84162) | Confirms `.aoe2spgame` = full state, `.aoe2record` = replay |

## Rules for using these

1. **The dat outranks any wiki.** Community numbers describe DE; where the dat differs, the
   dat is what the sim must implement, and the difference is a question (U11).
2. **Two sources or it is `inferred`.** Every formula in `combat.md` has at least two
   agreeing statements; every multiplier decode was verified against a published bonus *and*
   found in the dat.
3. **Never assume an id.** The one wrong conclusion in this project so far — "the DBA
   multiplier is missing" — came from assuming tech 200 was Double-Bit Axe. Match by cost
   and research time. This is now a rule, not a note.

## Found via enhansome/enhansome-aoe2 (curated list, checked 2026-10-28)

The repository is an "awesome AoE2" index rather than a tool. Most of it is tournaments, streamers and civ
pickers; four entries are worth keeping on the watch list, in order of expected use:

| Source | Why it matters | Trust |
|---|---|---|
| [AoE2DE UGC Guide](https://ugc.aoe2.rocks/) ([source](https://github.com/Divy1211/AoE2DE_UGC_Guide)) | Divy1211 is the GenieDatParser author, and this documents user-generated content - scenario, AI and data modding. The place to check when a dat field's *behaviour* matters rather than its layout (build categories, class semantics). Python, updated this month. | High - same author and lineage as the parser we already trust |
| [AoE2 AI Scripting Encyclopedia](https://airef.github.io/) | The game's own AI API: object names, per-object facts, and the commands an AI may issue. Our scenario runner is a poorer cousin of this, so it doubles as a checklist of orders we have not modelled (and its object classes may settle questions like "is a farm a gathering building"). | Medium - documents DE's AI, not the sim |
| [Build order guide](https://buildorderguide.com/) / [AoE Companion](https://aoecompanion.com/) | Expert build orders with uptime and villager counts. This is what answers "can a player reach Castle in 15 minutes" with numbers instead of guesses - directly relevant to D59b's pending siege decision. | High for timings, community-maintained |
| [aoe-twools](https://aoetwools.firebaseapp.com/) ([source](https://github.com/Kheyzell/aoe-twools)) | Tech tree per civ, civs per unit and per tech. Useful if the pack ever grows past one civ, since it is the same "which civ may have what" question our `civ_barred_unit` answers for one. | Medium - DE data |

**Not useful yet:** unit-counter charts, civ pickers, replay packers, tournament boards. Recorded here so the
next session does not re-read the list looking for the same things.
