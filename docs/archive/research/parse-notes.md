# Step 1 — dat extraction

**Status: complete.** Both dats parse fully, units included, to a JSON tree the engine can
load. Method, numbers, caveats and licences below.

## 1. Toolchain

| Piece | Where | Note |
|---|---|---|
| Rust 1.99.1 + cargo | `/opt/data/tools/{cargo,rustup}` | rustup, user-space, no root |
| uv 0.13.5 / uvx | `~/.local/bin` | **not in the base image** — installed via astral.sh during this work |
| Python for the parser | uv-managed 3.10.22 | provisioned by `uv sync` |
| `SiegeEngineers/genie-rs` | `/opt/data/tools/src/genie-rs` | cross-check oracle (GPL-3.0) |
| `Divy1211/GenieDatParser` | `/opt/data/tools/src/GenieDatParser` | the parser that actually covers units (GPL-3.0) |
| `Tapsa/genieutils` | `/opt/data/tools/src/genieutils` | read as a *format spec*; not built (needs Boost+LZ4, no root) |

## 2. Reproduce

```bash
export PATH="$HOME/.local/bin:$PATH"
cd /opt/data/tools/src/GenieDatParser
uv sync                              # env + deps, ~1 s warm
uv run python run_probe2.py \
  /opt/data/workspace/Age2HD/Age2HD/resources/_common/dat/empires2_x1_p1.dat \
  /opt/data/workspace/Age2HD/Age2HD/resources/_common/dat/empires2_x2_p1.dat
```

Output lands in `build/converted/<name>.json`. Parse is fast: 0.20 s (x1) / 0.53 s (x2).

## 3. Results

| | x1 | x2 |
|---|---|---|
| file version | `VER 5.7` (resolves to 5.7.1) | `VER 5.7` |
| civilisations (incl. Gaia) | 19 | 32 |
| units per civ (populated) | **672** of 866 slots | **1,166** of 1,402 slots |
| task lists | 866 | 1,402 |
| sounds | 506 | 548 |
| sprites | 7,367 | 11,175 |
| tech effects | 514 | 798 |
| techs | 460 | 801 |
| terrains | 42 | 100 |
| JSON size | 54.3 MB | 137.3 MB |

## 4. Cross-validation

Two independent implementations agree on every overlapping count:

| count | genie-dat | GenieDatParser |
|---|---|---|
| sounds | 506 / 548 | 506 / 548 |
| sprites | 7,367 / 11,175 | 7,367 / 11,175 |
| effects | 514 / 798 | 514 / 798 |
| civs | 19 / 32 | 19 / 32 |
| techs | 460 / 801 | 460 / 801 |
| task lists | 866 / 1,402 | 866 / 1,402 |

Plus an internal check that is hard to fake: **units per civ == task lists, exactly**
(866/866, 1,402/1,402). The format pairs one task list with each unit; a field-size error
anywhere upstream would break that equality.

Spot checks against known AoC values (civ 1 = British):

| id | internal name | HP | LOS |
|---|---|---|---|
| 74 | `SPRMN` (Spearman) | 40 | — |
| 83 | `VMBAS` (Villager) | 25 | 4.0 |
| 93 | `PKEMN` (Pikeman) | 45 | — |

## 5. Data-model finding — units are stored per civilisation

The unit table is **not** global. It is duplicated inside each civilisation: 866 units ×
19 civs (x1), 1,402 × 32 (x2). Civ 0 is `Gaia`.

Implication for the engine: load one civ as canonical and apply per-civ overrides, rather
than treating the dat as a flat unit list. Also means a naive export is ~19× larger than
needed — the JSON above carries every civ's full copy.

## 6. Caveats

- **Display names are not in the dat.** Units carry string IDs into the language files
  (`resources/en/`, the `.dll`/string tables). Resolving "Villager" needs a string-table
  lookup, not the dat.
- Internal names (`VMBAS`, `OLD-ACADEMY`) *are* in the dat for 5.7, but the parser hands
  them back as byte arrays rather than decoded strings. Decoding is a one-line fix at
  export time; don't assume `name` is populated in the JSON.
- Civ names in the dat are internal, not localised: `British`, `French`, `Germans` are
  Britons / Franks / Teutons. Match on internal names.
- `to_json` is a straight dump: field names are the parser's, not a design of ours. Treat
  `build/converted/*.json` as a staging format, not the engine's schema.

## 7. Licences — matters for D4

| Project | Licence |
|---|---|
| `genie-rs` / `genie-dat` | GPL-3.0 |
| `Divy1211/GenieDatParser` | GPL-3.0 |
| `Tapsa/genieutils` | LGPL-3.0 |

All three are usable as **tools**: run them, keep their output. None should be vendored
into a permissively-licensed engine — that would force GPL-3.0 on the whole thing. File
format facts are not copyrightable; their code is. So the plan stands: extract with these
tools now, write our own reader when the engine needs to read dats at runtime.

## 8. Remaining polish (not blocking)

1. Dedupe the export (canonical units + per-civ deltas) — cuts 137 MB to single-digit MB.
2. Decode internal name byte arrays.
3. Resolve string IDs against the language tables for display names.
4. Own Rust reader, when the engine needs it (§7 above).
