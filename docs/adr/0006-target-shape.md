# ADR 0006 — Target shape: original content now, replaceable later (supersedes D18)

- **D-id alias:** D18 (2026-10-25)
- **Status:** Accepted. Old row untouched until owner sign-off.

## Context

Owner's words: keep the vibe and dynamics; start with original graphics, tech trees and stats; be ready to replace graphics; move toward own content as far as cost allows. Reference content is what makes mechanics verifiable against known-good values.

## Decision

(a) Now: dat numbers + install art. (b) Scheduled, not blocked: own resources, tech tree, stats — the engine must accept them unchanged (ADR 0005 buys this). (c) Not planned: divergent rules stay expressible in the same schema, nothing designed against it. Engine addresses visuals by content id, never filename; simulation geometry (radius, footprint) lives in the unit table, so swapping art cannot change gameplay.

## Evidence

- **Game (rung 0):** Sprite-table fields (`slp_id`, `layer`, `force_player_color`, `color_table`, `num_deltas`, `bounding_box`, `sound_id`) read from `GenieDatParser`'s model at `VER 5.7.2` — dat-adjacent, single-source.
- **Rung 2 (bounded, read-only):** openage's SLP/DRS asset pipeline is the recorded item to learn from (D14 row, item 1). Decomp sprite coverage and openaok art handling unread in this slice.
- **Owner:** Owner's (a)/(b)/(c) split, 2026-10-25.

## Consequences

No renderer exists, deliberately — headless is the harness until the sim is worth watching. **Unproven:** the unit → sprite link and how numbered graphics are stored (DRS indexing) are unchecked; that is the first asset-pack task, not an assumption. The gameplay/art-independence rule is the test to write when the first renderer lands.

## Fixtures

- Sprite fields: importer's dat model at `VER 5.7.2` (no engine fixture; importer-side).
- `unproven`: unit → sprite link, DRS layout.
- Future: art-swap gameplay-invariance test (to be written with renderer).
