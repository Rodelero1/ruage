# Licensing and repository hygiene

Rule: **the repository contains our work, never the game's data.**

Not legal advice. It is the operational policy this project follows, so that a future
session does not accidentally commit something that cannot be taken back.

## Why this project is lawful

Reverse engineering for interoperability is permitted for a lawful user under the EU Software
Directive (2009/24/EC), Art. 6: decompilation is allowed where it is indispensable to obtain
the information necessary to achieve interoperability of an independently created program.
Art. 5(3) separately permits observation, study and testing of a program's functioning.

The project owner owns a legitimate copy (Steam install `221380`) and is porting the game to
run on their own hardware. That is the classic interoperability case.

Three limits matter for what we do with the results:

1. **Art. 6 does not authorise distributing the information obtained**, where it reproduces
   the original program's expression. So: use the knowledge, don't ship their bytes.
2. **The Database Directive (96/9/EC), Art. 7** grants a sui generis right over a substantial
   extraction of a database. `empires2_x2_p1.dat` is exactly that: 1,166 units, 801 techs,
   798 effects, 32 civilisations. Copying it out wholesale is the risky act, regardless of
   whether individual numbers are "facts".
3. **Assets are separate and stricter.** Sprites (`.slp`), sounds (`.wav`/`.mp3`), string
   tables and campaign files are creative works. We never copy, convert for redistribution,
   or commit them — the engine reads them in place from the user's own install.

Individual game *facts* (knight has 100 HP, DBA is +20% wood) are not copyrightable in
themselves. A systematic extraction of the whole database is a different proposition, which is
why the line is drawn at "generated files stay local".

## Repository

`https://github.com/Rodelero1/ruage.git` — **private**. Pushed 2026-10-23: 35 files, 175,185 bytes
of blobs, zero derived game data (verified against the remote tree, not just the ignore rules).

## What is in the repository

| In git | Why it is ours |
|---|---|
| `SPEC.md`, `docs/**` | Our analysis, decisions, sources. Facts and formulas, with small illustrative values quoted for explanation. |
| `config/*.toml` (except `gamedata/`) | Hand-written engine constants and overrides. Our choices, not their data. |
| `tools/**` | **Moved out** (2026-10-26, D12): the importer's own repository. |
| `.gitignore` | The policy itself. |

## What is never in the repository

| Excluded | Reason |
|---|---|
| `Age2HD/` | The game install — the user's own copy, incl. exe, assets, saves. |
| `config/gamedata/*.toml` | Full extracted dat: the substantial-extraction case in §2 above. |
| `build/` | Inflated dats, converted JSON, probe output, Ghidra projects. |
| `*.aoe2spgame`, `*.aoe2record`, `*.mgz` | Player content; may embed map/unit state. |
| `config/MANIFEST.toml` | Describes excluded files and names the install. Regenerated locally. |
| `tools/.venv/`, caches | Machine state. |

## A clone is still useful

Everything excluded is regenerable from a legitimate install:

```bash
cd tools
uv run python -m genconfig ../Age2HD/resources/_common/dat/empires2_x2_p1.dat ../config
```

That is the point of the importer (`ruage-tools`) being shipped while its *output* is not: the code is
ours to publish, the data is not ours to distribute.

## Documentation rule

- **Allowed**: formulas, mechanics descriptions, field layouts, counts, and a handful of
  values quoted to illustrate (e.g. "knight 100 HP, 2/2 armour, 60F 75G").
- **Not allowed**: complete dumps of unit/tech/effect tables pasted into markdown, or any
  copied asset. If a doc needs a whole table, it links to the generator instead.
- Current docs quote roughly twenty units and ten gather rates for explanation. If **D4**
  ever resolves to "publishable", these get re-read against this rule before anything is
  pushed publicly.

## Precedent

`openage` (the long-running open-source reimplementation) keeps reverse-engineered data out of
its repository for exactly this reason, and ships an engine that reads the user's own
installation. We follow the same separation.

## Where the GPL actually reaches (corrected 2026-10-23)

The dat parser is a **build-time tool**, not a component of the game, and the dependency edge is
short:

```
tools/genconfig  ──imports──▶  GenieDatParser (GPL-3.0, Divy1211)
        │
        └──writes──▶  config/gamedata/*.toml  ──read by──▶  crates/* (MIT/Apache)
```

- **The engine links nothing.** `grep -r GenieDatParser crates/` returns nothing. At runtime the
  engine reads TOML; the parser never runs during gameplay.
- **The parser is not openage's converter.** It is `Divy1211/GenieDatParser`, a standalone
  Python library for reading and writing AoE2 `.dat` files. openage has its own `convert/`
  module, which SPEC §5.2 explicitly rejects as a reference.
- **The GPL covers the program, not its output.** The generated TOML is not GPL-licensed — it is
  derived game data, restricted by D8 instead, which is a different and stricter rule.
- **Consequence.** Publishing the engine cannot be affected by this licence; only the packaging
  of the generator is a question. **Decided (D12), and done 2026-10-26:** it moved to its own repository,
  GPL-3.0, together with a fork of the parser if we have to modify it — and we probably will,
  since `VER 5.7.2` already raises `VersionError` for the per-task list and cannot read
  `unique_unit_effect_ids` or `name2`. Forking a GPL library is permitted and stays GPL, so the
  licence boundary becomes a repository boundary instead of a directory convention. The
  remaining alternatives, if that ever proves wrong: keep the generator private, or write our
  own reader (which would also remove the dependency entirely).

## Enforcement

`.gitignore` is the mechanism, and it is reviewed whenever a new artefact type appears. A
pre-commit check is a sensible later addition: fail the commit if any staged path matches
`Age2HD/`, `config/gamedata/`, `build/`, or a save/asset extension.

---

# Licence survey of comparable projects (2026-10-25)

Checked against the GitHub API, not from memory:

| project | declared licence | note |
|---|---|---|
| `SFTtech/openage` | GPL-3.0-or-later (NOASSERTION on GitHub) | copyleft |
| `librematch/genie` | **AGPL-3.0** | copyleft *with the network clause* — worst case for reuse |
| `SiegeEngineers/genie-rs` | GPL-3.0 | dormant since 2023 |
| `SiegeEngineers/genieutils-py` | **LGPL-3.0** | the only "lesser" one in the field |
| `SiegeEngineers/sld-extractor` | **none declared** | all rights reserved by default: no permission at all |
| `SiegeEngineers/WololoKingdoms` | GPL-3.0 | |
| `SiegeEngineers/aoc-reference-data` | none declared | data, but no explicit grant |
| `OpenRA/OpenRA` | GPL-3.0 | |
| `OpenRCT2/OpenRCT2` | GPL-3.0 | |
| `openblack/openblack` | GPL-3.0 | |
| `OpenTTD/OpenTTD` | GPL-2.0 (NOASSERTION) | |
| `0 A.D.` | GPL-2.0+ code, CC-BY-SA art | art is separately licensed |

**The field is uniformly copyleft.** Our MIT/Apache dual is the outlier, and that is a deliberate
choice rather than an accident: a permissive engine can be embedded, packaged by distributions, read
by modding tools and reused by other engines, and permissive code can be taken *into* a GPL project
(MIT and Apache-2.0 are both GPL-3.0-compatible). The reverse is not true. Compatibility runs one
way, and this is the direction that costs us nothing.

## What that means concretely

- **We may take code from:** nothing in the survey. Every comparable engine is GPL-3.0, and
  `librematch/genie` is AGPL-3.0, which extends copyleft to network use.
- **We may take ideas from:** all of them. Reading a GPL implementation and writing our own from the
  data and the documented format is what D17 already requires for a different reason.
- **`sld-extractor` has no licence at all.** "No licence" is not "free": default copyright applies.
  It may be read for what it does, never copied.
- **`genieutils-py` is LGPL-3.0**, the only licence in the field that would technically permit a
  separate-component arrangement. Not needed: `GenieDatParser` is already separated (D12).

## Correction to D13

D13 said "adopt only `librematch/genie` file-format readers in a separate tooling repository". With
the licence checked, that is **withdrawn**: AGPL-3.0 on a tool that reads game data would put the
strongest copyleft obligation we could have chosen onto the part of the pipeline that touches the
game's files. `GenieDatParser` (GPL-3.0, local build-time tool, separate repository) is the *less*
restrictive option and stays the choice.

## The policy to keep (D21)

| artefact | licence | why |
|---|---|---|
| engine (`crates/`) | MIT OR Apache-2.0 | permissive, GPL-3.0-compatible, distributable |
| content pack schema + our authored `config/` | MIT OR Apache-2.0 | a modder must be able to copy a schema without a licence question |
| original art and audio (later) | CC-BY-SA 4.0, per-asset attribution | art wants attribution, not source disclosure |
| game data | not shipped (D8) | it is not ours to license |

"Compatibility" worth engineering is **format** compatibility, not licence compatibility: a documented,
permissively licensed content-pack format is something other engines and modders can target, and it is
the one thing the GPL projects cannot offer back.
