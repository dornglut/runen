#![forbid(unsafe_code)]
//! Executable reference semantics for validated Runen Core MIR.
//!
//! `legacy` preserves the accepted one-body oracle only while the draft
//! interprocedural machine is proven beside it. The legacy surface is removed
//! before #241 may be accepted.

mod legacy;
pub use legacy::*;

pub mod interprocedural;
