# Handover — pathfinding pile / plan-cost thread (2026-11-24)

Tree: clean at `7db1e9a` (Phase B calibration). SPEC v1.223. This box has **no cargo** and **no `/opt/data/cache/`** — Rust cannot be built or run here; Python tooling only. A session that can run `cargo test` should do the measurement slice below.

## Where the engine stands

- D357 landed: arrival is a placement. Acceptance scene: 153 entities, 138,588 re-paths, 24.83 s, **wins** (was 124 / 12,847 / 10.10 s, draw). Pile halved; deepest overlapping pair still 8,000 raw.
- Seat-off control: same build is 9.93 s / 12,847 re-paths. The whole delta is plan cost: men standing a body apart stamp twice as many tiles as blockers.
- Source of truth for this thread: `docs/next-step.md` (full detail, read it first), `docs/divergence-register.md` §1 (11 DEFECT rows, each with source + closing step), `docs/live/pathfinding-movement.md` (rulings D52–D268+).

## What I verified this session (static, no run)

- The per-reason counter exists: `StepDecision`, 7 variants (`crates/sim/src/lib.rs:672`), `WaitGround` vs `WaitBlocked` vs `WaitTarget` vs `Paused`; counted in `step_counts` (`lib.rs:980`), plus `repaths()` (`lib.rs:1709`) and `plan_count` (`lib.rs:975`). "Measure per-reason first" is executable.
- The three D350 ports are as described: `inside_a_body` is tile-centre based (`crates/sim/src/ground.rs:1001-1010`); `smooth` certifies with body-blind `chain_leg` (`ground.rs:986`); `refine` interpolates the crossing geometrically (`crates/sim/src/levels.rs:307`).
- In-tree witnesses exist: `crates/sim/tests/pile_probe.rs` (mate 0 / past 500 / neither 164 at `ed064fb`), `crates/sim/tests/collision_probe.rs`, `crates/sim/tests/crowd_piles_up.rs`.
- Correction to my own earlier claim in chat: I doubted the D197/D193 counter — it is real (the `StepDecision` array). Trust the code lines above over the chat.

## Blockers / stale paths (do not guess, re-check on a full box)

- `/opt/data/cache/scratch/age2hd-pending-fixtures/` does not exist here, so `body_blocks_its_tiles.rs` and `spent_pile.rs` (cited by `next-step.md`) are unrecoverable from here. They may exist on the owner's box — check before rewriting history.
- `config/gamedata/` (units/techs/effects/civs) exists here; the acceptance scene + generated data path is the thing to confirm where cargo runs.

## Proposed next slice (test-first, measurement-only)

1. Run the 36k-tick acceptance scene (seed 7) with step tracing on; report `step_counts` by reason + `repaths()` + wall time + pile census (deepest group, pairs inside a fifth of a tile).
2. Attribute the 138k-vs-12k gap (expect `WaitBlocked` dominance from the double-stamp) **before** touching the plan. Invariant: pile stays halved.
3. Only then: D350 ports one variable per commit (tile rectangle → smoothing bodies → refinement span), each with census + wall + re-path count.

## Open questions for the owner (unchanged)

- The pile's last inch: with no pushing and no place model (D293), two men sharing one destination can only be kept apart by the plan — his call, numbers in the review.
- Whether the arrival rule covers a `Return`ing carrier walking home into another (the deepest 8,000-raw pair), or whether that is the walk's business.
- Evenly-mirrored skirmish fixture (player-2 barracks timing skews post-~1985 s comparisons).
