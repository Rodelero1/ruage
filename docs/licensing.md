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

## What "learn from it" does and does not mean

Yes: **reading a GPL project to understand how something works is lawful, and reimplementing that
understanding in our own code is lawful.** Copyright protects the *expression* - the particular code,
comments and structure - not the idea, the algorithm, the file format or the behaviour. Every
compatible implementation ever written rests on this: Compaq's BIOS, Samba, Wine, ReactOS, the whole
of the open-source RTS scene.

The line that matters is not "did you look at it" but "did you copy the expression":

- **Allowed**: reading code to learn an approach, reading a *specification* or a format description,
  measuring behaviour in the original game, and writing our own implementation from that
  understanding.
- **Not allowed**: paraphrasing their code - renaming variables, restructuring a function, keeping the
  same sequence of steps - which is a derivative work regardless of how it is spelled. Nor copying
  their data files, test corpora, or anything they themselves license from the game's owner.
- **The practical discipline**: do not write our implementation with their editor open. Write down the
  *behaviour* first (this repository already does that in `docs/mechanics/`), then implement from the
  note. Where a piece of our code was informed by reading a GPL project, say so in the module - that
  record is what makes the provenance defensible years later, and it costs one comment.

Fully clean-room (one person writes a spec from the code, another implements from the spec) is the
strongest form of this defence, and is worth doing deliberately for anything we copy closely in
spirit. It is not required for using a published file format.

**Separate from copyright, and not solved by any of this:** patents (software patents exist in some
jurisdictions; nothing relevant to twenty-five-year-old RTS mechanics is in play here), trademarks
(the game's name and art are not ours, which is what D8 and D21 already enforce), and the fact that
those projects' *assets* are licensed to them by the game's owner, not by their GPL.

## The save reader: what the decision actually is (2026-11-13)

D4 = B keeps `docs/mechanics/`, SPEC §3/§4, `docs/oracle.md` and `config/gamedata/` out of the published set, but it does not
say where a **format reader** belongs. `crates/save` + `--load` needs an answer before any code ships, so here is the question
split into the parts that actually differ.

**What the reader is.** `tools/savegame.py` and `docs/save-format.md` describe a container (raw DEFLATE), a versioned header
(`VER 9.4`, a float32 save version), a `0x0A60`-framed string record, and a world section with a turn counter and 229-byte
object records. Every one of those facts was obtained by **reading the game's loader and writer as behaviour** - what they
compare, what version they refuse, which fields they branch on - and then by measuring the owner's own files. No decompiled
text is reproduced anywhere in the reader or its documentation.

**Where the legal weight actually sits.** Three different regimes, and only one of them is about the code:

| Question | Regime | Answer |
|---|---|---|
| may we decompile to learn the format? | Software Directive 2009/24/EC Art. 6 | yes - interoperability, lawful user, indispensable information |
| may we write and publish our own reader from that understanding? | Art. 6 + case law on functionality (CJEU *SAS v WPL*, 2012: functionality, a language and a file format are not protected expression) | yes - a format description is functional, not the program's expression |
| may we publish a save file, or a test fixture made from one? | Database Directive 96/9/EC Art. 7 + D8 | **no** - that is the game's output and player content; it is the substantial-extraction case |

So the risky act is **shipping artifacts**, not shipping a parser. The parser is the same posture as Wine reading PE files or
openage reading `.slp`: an independently written description of a format, with the original's bytes and code left behind.

### The options

| | Where the reader lives | `--load` for a public clone | Licence consequence | Risk |
|---|---|---|---|---|
| **A** | `crates/save` in the engine | yes | engine stays MIT/Apache | the reader's field offsets were learned by decompilation; a strict reading of Art. 6's "do not distribute the information obtained where it reproduces the expression" can be raised, and answered with *functional, not expressive* - the SAS line - plus the provenance record |
| **B** | the tooling repository (GPL-3.0), beside the importer | no: `save → TOML/JSON` becomes a build step | mirrors D12 exactly; engine keeps linking nothing | none that is new: the same bastion the dat parser already sits behind |
| **C** | nowhere - local scratch only | no | none | `--load` and the snapshot oracle stay unshipped, which is today's state |
| **D** | not built | - | - | the only oracle that can prove a mechanic per-tick is lost |

**Recommendation: B now, A later, and only if the engine is published.** B unblocks the work this week - the 18 saves, the
tick counter, stockpiles, the entity list - inside the repository boundary that already exists for the dat, and it makes no
publishing decision about decompiled-derived offsets. A becomes a packaging change (move the module, keep the licence split)
if and when the engine goes public, and it should carry the provenance paragraph that says the offsets came from the game's
loader *as behaviour*, with the function addresses cited as evidence rather than as code.

Three things are genuinely the owner's, and nothing in the binary answers them:

1. **Does `--load` have to exist in the published engine, or is a tool-side conversion enough?** (A vs B.)
2. **Is the engine being published at all yet**, and if so is a save reader inside or outside that set?
3. **Is the provenance rule acceptable** - format facts cited as behaviour, no decompiled text, no fixtures committed,
   synthetic tests plus SHA-256-keyed real-fixture tests that skip when the install is absent (the shape `tools/savegame.py`
   already uses)?

### B now, A later: what makes that migration cheap, and what must be true before it starts

B is the tooling repository (GPL-3.0, `save → our own state file`, engine reads the state file). A is the engine reading the
save directly, MIT/Apache, `--load` in a published clone. **The migration is a change of producer, not of engine**, and that
is the whole design rule: everything below exists so that A is "swap the reader behind an unchanged model".

**The one decision that makes A cheap, taken now.** The engine's entry point is `World::from_state(state)` - a plain,
versioned description of a starting position (map, players, stockpiles, ages, techs, entities with owner/hp/task/position,
tick). In B a tool writes it; in A a reader builds the same struct in-process. If this model is designed late, or grows
save-specific fields, A becomes a rewrite of the engine's seeding path instead of a one-module swap.

**Ordered steps between the two.**

1. **Freeze the state schema and version it** (`state v1`), with strict loading (D6): an unknown or missing key fails, so a
   format change cannot pass silently. This is the contract both sides read.
2. **Write the tool-side converter against the format notes**, not against the eventual engine reader, and keep every
   format fact in one module with its evidence cited as *behaviour* (loader gate, version field, record framing) - never a
   pasted decompilation.
3. **Write the engine-side reader from `docs/save-format.md` alone**, ideally in a different session from the converter -
   the discipline that makes "functional, not expressive" a record rather than a claim. If a step cannot be written without
   opening the GPL converter, the notes are incomplete, and *that* is the finding.
4. **Diff the two producers on every save in the install** (31 files today): field-for-field equality, with any difference
   named and explained. This is the migration's acceptance test, and it runs before any switch.
5. **Switch `--load` to the direct reader**, keep the converter as a tool for formats the engine does not read, and leave
   both paths alive until the suite is green on the engine side.
6. **Re-run the publish checklist**: `cargo tree` shows no GPL crate in the workspace, the published tarball carries no
   fixtures or saves, the docs rule still holds (no complete tables), and `make check` plus the decision and next-step gates
   are green.

**Prerequisites before step 3 may start** - not steps, gates:

- the state model is stable (no schema churn for a release);
- `docs/save-format.md` is complete enough that a fresh reader can be written from it without the converter's source;
- the provenance note exists for the module (what was learned, from where, as behaviour);
- **the engine is actually going to be published**, and D4's publish set has been re-confirmed - if it is not, B is the
  destination and A never has to happen, which is the cheapest outcome of all;
- a mechanical licence check exists in CI (a dependency-graph assertion, not a promise in prose), because the failure mode
  here is a GPL edge appearing in the workspace months later and nobody noticing.

**What makes it expensive if done out of order:** letting the converter's output be a *transcript of the file* (offsets,
padding, sections) instead of a description of a world - then A must publish the transcript, which is the Art. 6 problem
again, and the engine carries format detail it should never know.

## What must be redone before this is distributable (2026-11-13)

The owner's goal: **clone, point at `Age2HD/`, play — nothing else needed**, with game *data* and (later) art replaced by
ours. This is the honest inventory, measured against the tree today, grouped by what it actually costs.

### 1. Things that are not ours at all

| What | State today | What "distributable" requires |
|---|---|---|
| the content data (1,166 units x ~200 fields, 801 techs, 798 effects, 32 civs) | generated into `config/gamedata/`, gitignored, produced by an importer in **another repository** | either **generate at first run** (importer in this repo) or **author our own numbers**. Shipping theirs is the Art. 7 problem, not a packaging one |
| sprites, sounds, string tables | read **in place** from the install | nothing - this is already lawful. Replacing them is an optional endgame, not a blocker |
| names and trademarks - "Age of Empires", civ and unit names, the visual style | used as-is | a public project must not read as official; renaming arrives with authored content |

**Measured:** a clean clone `cargo build`s **now** (the engine needs no game data to compile), and runs a world from the
tracked `config/*.toml`; it cannot produce `config/gamedata/`, which is exactly the gap above.

### 2. Our own writing that quotes their code

| What | Count | Fix |
|---|---|---|
| the game's own source paths, e.g. `c:\users\jakes_workstation\...\move_obj.cpp` | **5 files** (`movement-in-the-binary.md`, `behaviour-diff.md`, `binary-class-map.md`, `decisions.md`, `reviews/2026-11-07-the-binary-names-itself.md`) | rewrite as findings: cite the **address** and the **behaviour**, drop the quoted path (and the third party's username with it) |
| the game's log/assertion strings, e.g. `"We have %d non-past collisions and %d terrain collisions, repathing"` | **3 files** | same: paraphrase the branch, cite the address; a quoted line of their code is the expression Art. 6 says not to distribute |
| decompiled pseudocode fragments in reviews | several | keep the conclusion and the address, delete the listing |

This is mechanical, and it does **not** weaken the analysis: an address plus a described branch is stronger evidence than a
paraphrase of Ghidra's output, and it survives a re-decompile.

### 3. Structure, so that "clone and run" is true

1. **The importer lives in-repo** - `tools/` with its own GPL-3.0 licence file (or rewritten in Rust, which removes the
   question entirely). Reverses D12's repository boundary into a directory boundary; the engine still links nothing.
2. **A bootstrap step and an install-path argument**, failing with one clear sentence when the install is missing.
3. **Licence plumbing**: per-directory `LICENSE`/`NOTICE`, the engine dual MIT/Apache untouched, and a **CI assertion** that
   no engine crate depends on a GPL one - the failure mode is an edge appearing months later, which prose cannot catch.
4. **README/quickstart**, and the path-based content gate moved into CI (the pre-commit hook is not installed by
   contributors).
5. **Decide what the docs are**: D4 = B keeps `docs/mechanics/`, SPEC §3/§4, `docs/oracle.md` and the decision log internal.
   The public set is engine + tooling + a README, and code comments currently cite `docs/mechanics/…`, which will not ship.

### 4. History

Zero game data has ever been committed (verified against the remote at first push, and the pre-commit hook has refused it
since). **That stays true only if the generated tables are never committed** - a scrub is a history rewrite, not a delete.

### The order that costs least

1. Importer into `tools/` + bootstrap (days) - unblocks "clone and run".
2. Quote-and-cite rewrite of the five docs (days) - unblocks publishing the analysis.
3. Licence plumbing, CI gates, README (days).
4. Authored content pack (months) - the only route that also makes the *data* ours.
5. Own art and audio (unbounded) - optional forever.

## Content policy: names, prose, campaigns (2026-11-13, D201)

The owner's rule, in his words: *"The only campaigns and scenarios to be shipped by the project are the one made for it.
Never the ones taken from original game. 100%."* The rest follows from what each thing actually is.

### Three lists for names

| list | what goes in it | why |
|---|---|---|
| **keep** | historical ethnonyms and generic unit nouns - Persians, Huns, Goths, Mongols, Teutons, Saracens, Britons, Franks, Celts, Vikings; knight, archer, crossbowman, light cavalry, spearman, mangonel, trebuchet | no authorship, no trademark: a people's name and a common noun cannot be owned |
| **rename** | names that exist only because AoE2 coined them (the "Woad Raider" is the clearest) | a coinage is their expression; our unit gets our name |
| **rewrite** | **all prose** - unit, tech and building descriptions, tooltips, hints, campaign dialogue | sentences are copyrightable; this is the part that is actually mandatory |

**The test for the first two lists:** would a historian recognise the word on its own? Then keep it. Does it exist only
because the game invented it? Then rename. The medieval vocabulary is the right register anyway - "Achaemenids" is 0 A.D.'s
antiquity, not ours; "Persians" is period-neutral and stays.

**Britons -> English** and **Franks -> French** are historical choices, free to make or not make. Only the *presentation* -
title, logo, README, any "HD Edition" - is where trademark actually bites.

### Campaigns and scenarios

- **Shipped:** only scenarios and campaigns authored for this project. No exceptions.
- **The game's own:** read **in place** from the install and played locally - that is the user's copy, used as the assets
  are - and **never converted into a shipped artifact, committed, or quoted in public documentation**. A `.scx` converted to
  our format is a derivative of their work; the moment it leaves the machine it is distribution.
- **A cache of converted scenarios** lives in `build/` or scratch, under the same rule as inflated dats (D8).
- The reader itself (a `.scx`/`.cpx` parser, a trigger system) is **our code** and is publishable; D15 already puts scenarios
  outside the current slice, so this is a priced "later" item.

### Art and audio

Read in place now; replaced by our own (CC-BY-SA, per-asset attribution) if and when the owner chooses (D21). Nothing here
blocks distribution today, because nothing is copied.
