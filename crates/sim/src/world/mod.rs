//! The `World` split by subject: one file per question the engine answers.
//!
//! `World` stays one type in `lib.rs`; these are further `impl World` blocks. A move here is **mechanical** - no logic
//! changes - and the proof is the acceptance hash, which must not move (`f328ebc8d4d34b78` at the time of the split).
//! Stage 1 only moves code; cutting `step` (1,566 lines) into named phases is stage 2.

pub(crate) mod battle;
pub(crate) mod build;
pub(crate) mod garrison;
pub(crate) mod gates;
pub(crate) mod gather;
pub(crate) mod livestock;
pub(crate) mod memory;
pub(crate) mod movement;
pub(crate) mod orders;
pub(crate) mod report;
pub(crate) mod research;
pub(crate) mod seed;
pub(crate) mod shares;
pub(crate) mod sites;
pub(crate) mod state;
pub(crate) mod step;
pub(crate) mod train;
