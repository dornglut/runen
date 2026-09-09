#![forbid(unsafe_code)]
//! Typed semantic data structures and validation for the Runen Core proving kernel.
//!
//! This crate deliberately contains no interpreter, backend, platform model,
//! or source-syntax concerns.

mod common;
pub use common::*;

mod interprocedural;
pub use interprocedural::{BasicBlock, Body, Function, Program, Terminator};

mod interprocedural_validation;
pub use interprocedural_validation::{
    MirLocation, MirPoint, MirValidationError, MirValidationErrorKind, ValidatedProgram,
    validate_program,
};
