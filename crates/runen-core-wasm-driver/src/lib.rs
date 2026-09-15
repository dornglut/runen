#![forbid(unsafe_code)]
//! Target-specific production composition from typed HIR through Core Wasm.
//!
//! This package composes already-accepted compiler and realization layers. It
//! owns no source entry-point, package, filesystem, backend-selection, or Runen
//! semantic policy.

use std::collections::BTreeMap;

use runen_core_ir as core;
use runen_core_lowering as lowering;
use runen_core_wasm::{self as wasm, ExecutionOutcome, RealizationError};
use runen_hir as hir;

/// Failure while constructing one Core-Wasm realization from an accepted typed
/// HIR compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// This first composition slice does not expose source/HIR correspondence
    /// for Core external-provider identities.
    ExternalDeclarationsUnsupported,
    /// Successful lowering did not preserve the required ordinary-function
    /// correspondence for one admitted HIR identity.
    MissingOrdinaryCorrespondence(hir::FunctionId),
    /// Successful lowering exposed the same ordinary HIR identity more than once.
    DuplicateOrdinaryCorrespondence(hir::FunctionId),
    Lowering(lowering::LoweringError),
    Realization(RealizationError),
}

/// Failure while selecting or executing one function in an already-realized
/// compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    /// The supplied per-compilation HIR identity has no ordinary non-generic
    /// function correspondence in this compilation.
    FunctionNotSelectable(hir::FunctionId),
    Realization(RealizationError),
}

/// One typed HIR compilation lowered and realized through the Core-Wasm target.
///
/// Function selection remains explicit and compilation-local. Construction does
/// not infer a `main`, source entry point, package convention, or Core function
/// order.
pub struct RealizedCompilation {
    ordinary_functions: BTreeMap<hir::FunctionId, core::FunctionId>,
    realized: wasm::RealizedProgram,
}

impl RealizedCompilation {
    /// Lower and realize one already-built typed HIR compilation.
    ///
    /// External declarations are deliberately rejected before lowering because
    /// this first composition slice does not expose HIR-to-Core external-provider
    /// correspondence.
    pub fn new(compilation: &hir::TypedCompilation) -> Result<Self, BuildError> {
        if compilation.functions.iter().any(hir::Function::is_external) {
            return Err(BuildError::ExternalDeclarationsUnsupported);
        }

        let lowered = lowering::lower(compilation).map_err(BuildError::Lowering)?;
        let mut ordinary_functions = BTreeMap::new();
        for function in &compilation.functions {
            if function.is_external() || !function.type_parameters.is_empty() {
                continue;
            }
            let core_function = lowered
                .core_function(function.id)
                .ok_or(BuildError::MissingOrdinaryCorrespondence(function.id))?;
            if ordinary_functions
                .insert(function.id, core_function)
                .is_some()
            {
                return Err(BuildError::DuplicateOrdinaryCorrespondence(function.id));
            }
        }

        let realized =
            wasm::RealizedProgram::new(lowered.program()).map_err(BuildError::Realization)?;
        Ok(Self {
            ordinary_functions,
            realized,
        })
    }

    /// Execute one explicitly caller-selected ordinary non-generic HIR function.
    ///
    /// The `FunctionId` is interpreted only relative to the `TypedCompilation`
    /// used to construct this realization. Backend entry admissibility remains
    /// owned by Core-Wasm and is returned unchanged through `ExecutionError`.
    pub fn execute(&self, function: hir::FunctionId) -> Result<ExecutionOutcome, ExecutionError> {
        let function = self
            .ordinary_functions
            .get(&function)
            .copied()
            .ok_or(ExecutionError::FunctionNotSelectable(function))?;
        self.realized
            .execute(function)
            .map_err(ExecutionError::Realization)
    }
}
