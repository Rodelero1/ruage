# ADR 0005 — Four layers; only the content pack knows AoE2 (supersedes D17)

- **D-id alias:** D17 (2026-10-25)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

The engine must ship a game that is not Age of Empires II without being rewritten. Numbers that mean something only because of AoE2 must therefore live outside `crates/`.

## Decision

Four layers: **engine** (`crates/` — determinism, tick, commands, replays, pathing, entities, damage, effect verbs; knows kinds, never particulars), **content pack** (`config/`, our schema — carries AoE2's values today), **importer** (tooling repo — all AoE2 knowledge: packed attack, class ids, effect kinds), **asset pack** (entity+state → sprite reference; today the install's graphics, later ours). Rule: any id or number meaningful only through AoE2 stays out of `crates/`.

## Evidence

- **Game (rung 0):** Architectural, not behavioral. No dat/binary/save trajectory supports or refutes a layering.
- **Rung 2 (bounded, read-only):** openage's `nyan` engine/content split is the recorded contrast (D14 row, item 2 — boundary comparison, not adoption). `genie-dat` as importer-side format work is the recorded precedent (D13 row). The three new sources add nothing here in this slice: the decomp's module layout and openaok's structure are unread, and the Meeting C++ slides are pathfinding-only by their Q3 role. Marked unread, not assumed.
- **Owner:** Four-layer shape accepted 2026-10-25.

## Consequences

Paydown state, as recorded: `ResourceKind` deleted 2026-10-26 (resources are content, proven by `a_content_pack_with_our_own_resources_loads_and_spends`). Still open: scenario/build/train steps still quote dat ids for buildings (resource half done, building half not). `Attribute::from_id` knowledge belongs in the importer, not the engine.

## Fixtures

- Paydown proof: `a_content_pack_with_our_own_resources_loads_and_spends` (own resources load and spend).
- Open gap has no fixture yet — a step naming a building by role is the test to write.
