#![forbid(unsafe_code)]
//! Target-specific production composition from typed HIR through Core Wasm.
//!
//! This package composes already-accepted compiler and realization layers. It
//! owns no source entry-point, package, filesystem, backend-selection, or Runen
//! semantic policy.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use runen_core_ir as core;
use runen_core_lowering as lowering;
pub use runen_core_wasm::{ExternalProviderFailure, ExternalScalarValue};
use runen_core_wasm::{self as wasm, ExecutionOutcome, RealizationError};
use runen_hir as hir;

/// Failure while constructing one Core-Wasm realization from an accepted typed
/// HIR compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildError {
    /// A supplied provider key is not an external declaration in this compilation.
    ProviderNotExternal(hir::FunctionId),
    /// More than one provider was associated with the same external HIR identity.
    DuplicateExternalProvider(hir::FunctionId),
    /// A represented external declaration has no provider in the supplied environment.
    MissingExternalProvider(hir::FunctionId),
    /// The provider result shape disagrees with the accepted HIR declaration.
    ExternalProviderResultShapeMismatch {
        function: hir::FunctionId,
        declaration_has_result: bool,
        provider_has_result: bool,
    },
    /// Successful lowering did not preserve the required ordinary-function
    /// correspondence for one admitted HIR identity.
    MissingOrdinaryCorrespondence(hir::FunctionId),
    /// Successful lowering exposed the same ordinary HIR identity more than once.
    DuplicateOrdinaryCorrespondence(hir::FunctionId),
    /// Successful lowering did not preserve the required external-declaration
    /// correspondence for one represented HIR identity.
    MissingExternalCorrespondence(hir::FunctionId),
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

type NoResultProvider =
    dyn Fn(&[ExternalScalarValue]) -> Result<(), ExternalProviderFailure> + Send + Sync + 'static;
type ScalarResultProvider = dyn Fn(
        &[ExternalScalarValue],
    ) -> Result<ExternalScalarValue, ExternalProviderFailure>
    + Send
    + Sync
    + 'static;

#[derive(Clone)]
enum ProviderImplementation {
    NoResult(Arc<NoResultProvider>),
    ScalarResult(Arc<ScalarResultProvider>),
}

/// One provider implementation associated with an external declaration by its
/// existing per-compilation HIR identity.
///
/// This binding carries no Core identity, Core interface, source-name lookup,
/// symbol, ABI, or Wasm import identity.
#[derive(Clone)]
pub struct ExternalProviderBinding {
    function: hir::FunctionId,
    implementation: ProviderImplementation,
}

impl fmt::Debug for ExternalProviderBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalProviderBinding")
            .field("function", &self.function)
            .field("has_result", &self.has_result())
            .finish_non_exhaustive()
    }
}

impl ExternalProviderBinding {
    pub fn no_result(
        function: hir::FunctionId,
        provider: impl Fn(&[ExternalScalarValue]) + Send + Sync + 'static,
    ) -> Self {
        Self::try_no_result(function, move |arguments| {
            provider(arguments);
            Ok(())
        })
    }

    pub fn scalar_result(
        function: hir::FunctionId,
        provider: impl Fn(&[ExternalScalarValue]) -> ExternalScalarValue + Send + Sync + 'static,
    ) -> Self {
        Self::try_scalar_result(function, move |arguments| Ok(provider(arguments)))
    }

    pub fn try_no_result(
        function: hir::FunctionId,
        provider: impl Fn(&[ExternalScalarValue]) -> Result<(), ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            function,
            implementation: ProviderImplementation::NoResult(Arc::new(provider)),
        }
    }

    pub fn try_scalar_result(
        function: hir::FunctionId,
        provider: impl Fn(
            &[ExternalScalarValue],
        ) -> Result<ExternalScalarValue, ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            function,
            implementation: ProviderImplementation::ScalarResult(Arc::new(provider)),
        }
    }

    #[must_use]
    pub const fn function(&self) -> hir::FunctionId {
        self.function
    }

    fn has_result(&self) -> bool {
        matches!(self.implementation, ProviderImplementation::ScalarResult(_))
    }
}

/// One typed HIR compilation lowered and realized through the Core-Wasm target.
///
/// Function selection and external-provider association remain explicit and
/// compilation-local. Construction does not infer a `main`, source entry point,
/// package convention, provider from a source name, or Core declaration order.
pub struct RealizedCompilation {
    ordinary_functions: BTreeMap<hir::FunctionId, core::FunctionId>,
    realized: wasm::RealizedProgram,
}

impl RealizedCompilation {
    /// Lower and realize one already-built typed HIR compilation with an empty
    /// external-provider environment.
    ///
    /// Compilations with represented external requirements therefore fail through
    /// the same HIR-facing missing-provider boundary as provider-aware construction.
    pub fn new(compilation: &hir::TypedCompilation) -> Result<Self, BuildError> {
        Self::new_with_external_providers(compilation, Vec::new())
    }

    /// Lower and realize one already-built typed HIR compilation with providers
    /// associated by external HIR identity.
    pub fn new_with_external_providers(
        compilation: &hir::TypedCompilation,
        bindings: Vec<ExternalProviderBinding>,
    ) -> Result<Self, BuildError> {
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

        let mut providers = BTreeMap::new();
        for binding in bindings {
            let function = compilation
                .functions
                .iter()
                .find(|function| function.id == binding.function)
                .filter(|function| function.is_external())
                .ok_or(BuildError::ProviderNotExternal(binding.function))?;
            if providers.contains_key(&binding.function) {
                return Err(BuildError::DuplicateExternalProvider(binding.function));
            }
            let declaration_has_result = function.result.is_some();
            let provider_has_result = binding.has_result();
            if declaration_has_result != provider_has_result {
                return Err(BuildError::ExternalProviderResultShapeMismatch {
                    function: binding.function,
                    declaration_has_result,
                    provider_has_result,
                });
            }
            providers.insert(binding.function, binding);
        }

        let mut wasm_bindings = Vec::new();
        for function in compilation
            .functions
            .iter()
            .filter(|function| function.is_external())
        {
            let binding = providers
                .remove(&function.id)
                .ok_or(BuildError::MissingExternalProvider(function.id))?;
            let external = lowered
                .core_external_callable(function.id)
                .ok_or(BuildError::MissingExternalCorrespondence(function.id))?;
            let wasm_binding = match binding.implementation {
                ProviderImplementation::NoResult(provider) => {
                    wasm::ExternalProviderBinding::try_no_result_for_program(
                        lowered.program(),
                        external,
                        move |arguments| provider(arguments),
                    )
                }
                ProviderImplementation::ScalarResult(provider) => {
                    wasm::ExternalProviderBinding::try_scalar_result_for_program(
                        lowered.program(),
                        external,
                        move |arguments| provider(arguments),
                    )
                }
            }
            .map_err(|error| {
                BuildError::Realization(RealizationError::ProviderAdmission(error))
            })?;
            wasm_bindings.push(wasm_binding);
        }
        debug_assert!(providers.is_empty());

        let realized = wasm::RealizedProgram::new_with_external_providers(
            lowered.program(),
            wasm_bindings,
        )
        .map_err(BuildError::Realization)?;
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
