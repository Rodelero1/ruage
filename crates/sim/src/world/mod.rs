//! The `World` split by subject: one file per question the engine answers.
//!
//! `World` stays one type in `lib.rs`; these are further `impl World` blocks. A move here is **mechanical** - no logic
//! changes - and the proof is the acceptance hash, which must not move (`f328ebc8d4d34b78` at the time of the split).
//! Stage 1 only moves code; cutting `step` (1,559 lines) into named phases is stage 2.

pub(crate) mod memory;
pub(crate) mod movement;
