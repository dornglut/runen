#![forbid(unsafe_code)]
//! Typed semantic data structures for the Runen Core proving kernel.
//!
//! The accepted feature branch is migrating the former one-body model to a
//! program-level interprocedural Core relation. `model` preserves the proven
//! one-body implementation only while that migration remains in draft.

mod model;
pub use model::*;

pub mod interprocedural;
pub mod interprocedural_validation;
