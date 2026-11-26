# ADR 0007 — Fixed-point domain is simulation state only (supersedes D9)

- **D-id alias:** D9 (2026-10-23; scale clause superseded by D10, see ADR 0008)
- **Status:** Signed 2026-11-26.

## Context

Determinism needs exact arithmetic in state. It does not forbid floats where state never goes.

## Decision

Values entering simulation state are `Fixed` (i64 at `sim.fixed_point_scale`). Load-time constants that never enter state (pathing weights, heuristic costs) stay f64. The boundary is explicit: `Fixed::parse` refuses sub-precision instead of rounding. (The "scale must divide a power of ten" clause was arithmetically inverted and is superseded — ADR 0008.)

## Evidence

- **Game (rung 0):** Architectural. The dat gives decimal values (e.g. `1.41421356` has no exact representation at scale 20000), so rounding at the boundary silently changes behavior — the refusal rule follows from the data's shape, not from any trajectory.
- **Rung 2 (bounded, read-only):** `librematch/genie` keeps f32 and instruction-level conversions to match DE bit-for-bit — recorded in the bank itself as contradicting this rule, which is exactly why fixed-point is valid only under the standing goal (behave like the original within stated tolerances, never bit-for-bit). Cited as the boundary of this decision, not as authority over it.
- **Owner:** "No floats in sim state" requirement.

## Consequences

Enforced by tests in `crates/fixed` and `crates/data`, not by prose. A bit-for-bit goal would delete this ADR (keep f32); under the standing goal it stands.

## Fixtures

- `crates/fixed` suite (parse refusal of sub-precision).
- `crates/data` loader tests (strict scale/boundary checks).

## Sign-off note (2026-11-26)

Signed as written, plain-worded (no codes). Standing interpretation: outside behavior may be checked against the original within tolerance; bit-exact arithmetic is out. D9 bank row deleted on sign-off per the 2026-11-26 rule.
