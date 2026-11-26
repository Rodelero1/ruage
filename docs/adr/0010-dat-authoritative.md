# ADR 0010 — Dat is authoritative over community documentation (supersedes D5)

- **D-id alias:** D5 (2026-10-23)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Wiki numbers describe DE and are derived; the dat is the artefact the shipped game reads. Where they disagree, one of them is wrong, and guessing wrong poisons every rate built on it.

## Decision

The dat wins. Every mechanic number conflicting with community tables takes the dat-side value, with the conflict noted. First application: the farming rate (U11) is 0.400 — the farm building's own `work_rate` caps the villager variant's 0.530 (`work_value1 = 1.0` everywhere, so no hidden scaling). The construction-speed reading of `work_rate` was tested and falsified (`HOUS` builds in 25 s with `work_rate = 0.0`; build time lives in `creation_info`).

## Evidence

- **Game (rung 0):** Dat values direct: `FARM.task_info.work_rate = 0.400` vs `VMFAR = 0.530`; `HOUS.work_rate = 0.0` with 25 s build; `creation_info` holds 15/25/50 s for farm/house/barracks, matching published build times.
- **Rung 2 (bounded, read-only):** Wiki/infosheet/forum numbers used as the losing side of the comparison (farming 0.319 published vs 0.400 shipped) — contrast, never authority.
- **Owner:** Owner stated the dat-authoritative rule; U11 resolved by decision, not by re-measurement.

## Consequences

The farm rate is one named constant loaded from the dat, never inlined — a re-measure changes one number. Cost of being wrong recorded: 25–65% spread on the most important economy number. Cheap settler named: two saves 60 s apart with known villager count (food delta / villagers / 60).

## Fixtures

- `FARM.task_info.work_rate` as the single source (named constant, not inlined).
- U11 cheap-settler scenario (unrun — future measurement, not a missing gate).
