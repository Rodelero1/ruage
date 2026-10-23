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
| `tools/**` | Our code, tests, lockfile. |
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

That is the point of `tools/genconfig` being shipped while its *output* is not: the code is
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

## Enforcement

`.gitignore` is the mechanism, and it is reviewed whenever a new artefact type appears. A
pre-commit check is a sensible later addition: fail the commit if any staged path matches
`Age2HD/`, `config/gamedata/`, `build/`, or a save/asset extension.
