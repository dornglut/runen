#![forbid(unsafe_code)]
//! Bounded, private WGSL emission for validated Core U32 scalar computations.
//!
//! This crate currently emits target code only. It does not instantiate a GPU,
//! provide a Runen ABI, or claim conformance from WGSL text alone.

use runen_core_ir::{
    BasicBlockId, FunctionId, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator,
    TypeKind, ValidatedProgram, Value,
};

/// Failure to admit a valid Core function into this deliberately small target subset.
///
/// These are target-coverage errors, never source-validation errors or Runen faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageError {
    UnknownFunction,
    UnsupportedSignature,
    UnsupportedLocal,
    UnsupportedControlFlow,
    UnsupportedStatement,
    UnsupportedOperand,
    UnsupportedDestination,
    UnsupportedReturn,
}

/// One private target artifact. Its text and binding layout are not a stable Runen ABI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U32Kernel {
    wgsl: String,
}

impl U32Kernel {
    #[must_use]
    pub fn wgsl(&self) -> &str {
        &self.wgsl
    }
}

fn complete_root(place: &Place) -> Result<String, CoverageError> {
    if !place.projections.is_empty() {
        return Err(CoverageError::UnsupportedDestination);
    }
    Ok(format!("v{}", place.local.0))
}

fn u32_operand(operand: &Operand) -> Result<String, CoverageError> {
    match operand {
        Operand::Constant(Value::U32(value)) => Ok(format!("{value}u")),
        Operand::Move(PlaceAccess::Direct(place)) | Operand::Copy(PlaceAccess::Direct(place))
            if place.projections.is_empty() =>
        {
            Ok(format!("v{}", place.local.0))
        }
        _ => Err(CoverageError::UnsupportedOperand),
    }
}

/// Emit WGSL from exactly one selected function of a canonically validated Core program.
///
/// Validated Core is the language-validity authority. Admission here recognizes only
/// the physical subset that has a faithful direct U32 expression translation.
/// Generated shader code is a private artifact pending separately proven GPU execution.
pub fn compile_u32_kernel(
    validated: &ValidatedProgram,
    selected: FunctionId,
) -> Result<U32Kernel, CoverageError> {
    let program = validated.as_program();
    let function = program
        .functions
        .get(selected.0 as usize)
        .ok_or(CoverageError::UnknownFunction)?;
    let is_u32 = |ty| {
        program
            .types
            .get(ty)
            .is_some_and(|item| matches!(item.kind, TypeKind::Scalar(ScalarType::U32)))
    };
    if function.parameters.len() != 1
        || !is_u32(
            function
                .parameter_type(0)
                .ok_or(CoverageError::UnsupportedSignature)?,
        )
        || !function.result.is_some_and(is_u32)
    {
        return Err(CoverageError::UnsupportedSignature);
    }
    if function.body.blocks.len() != 1 || function.body.entry != BasicBlockId(0) {
        return Err(CoverageError::UnsupportedControlFlow);
    }
    if function.body.locals.iter().any(|local| !is_u32(local.ty)) {
        return Err(CoverageError::UnsupportedLocal);
    }

    let mut scalar = String::from("fn runen_scalar(value: u32) -> u32 {\n");
    for (index, _) in function.body.locals.iter().enumerate() {
        scalar.push_str(&format!("    var v{index}: u32;\n"));
    }
    scalar.push_str(&format!("    v{} = value;\n", function.parameters[0].0));
    let block = &function.body.blocks[0];
    for statement in &block.statements {
        match statement {
            Statement::Init { dst, src } => {
                scalar.push_str(&format!(
                    "    {} = {};\n",
                    complete_root(dst)?,
                    u32_operand(src)?
                ));
            }
            Statement::IntegerAdd { dst, left, right }
            | Statement::IntegerMul { dst, left, right } => {
                let op = if matches!(statement, Statement::IntegerAdd { .. }) {
                    "+"
                } else {
                    "*"
                };
                scalar.push_str(&format!(
                    "    {} = {} {op} {};\n",
                    complete_root(dst)?,
                    u32_operand(left)?,
                    u32_operand(right)?
                ));
            }
            Statement::Drop {
                place: PlaceAccess::Direct(place),
            } if place.projections.is_empty() => {
                // Cleanup of a plain U32 has no physical destructor or result effect.
            }
            _ => return Err(CoverageError::UnsupportedStatement),
        }
    }
    match &block.terminator {
        Terminator::Return(Some(value)) => {
            scalar.push_str(&format!("    return {};\n", u32_operand(value)?));
        }
        _ => return Err(CoverageError::UnsupportedReturn),
    }
    scalar.push_str("}\n\n");

    let mut wgsl = String::from(
        "@group(0) @binding(0)\nvar<storage, read> runen_inputs: array<u32>;\n\
         @group(0) @binding(1)\nvar<storage, read_write> runen_outputs: array<u32>;\n\n",
    );
    wgsl.push_str(&scalar);
    wgsl.push_str(
        "@compute @workgroup_size(64)\n\
         fn runen_main(@builtin(global_invocation_id) invocation: vec3<u32>) {\n\
         \x20   let index = invocation.x;\n\
         \x20   if (index >= arrayLength(&runen_inputs)) {\n\
         \x20       return;\n\
         \x20   }\n\
         \x20   runen_outputs[index] = runen_scalar(runen_inputs[index]);\n\
         }\n",
    );
    Ok(U32Kernel { wgsl })
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_core_lowering::lower;
    use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
    use runen_syntax::parse_source;

    fn compile_source(source: &str, name: &str) -> Result<U32Kernel, CoverageError> {
        let parsed = parse_source(source.as_bytes()).expect("valid UTF-8");
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        let compilation = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
            .expect("fixture must be source-valid");
        let selected = compilation
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("selected function")
            .id;
        let lowered = lower(&compilation).expect("accepted HIR must lower to Core");
        compile_u32_kernel(
            lowered.program(),
            lowered.core_function(selected).expect("HIR mapping"),
        )
    }

    #[test]
    fn source_defined_transform_emits_actual_arithmetic_and_bounds_checked_mapping() {
        let shader = compile_source(
            "external fn input() -> U32; \
             fn transform(value: U32) -> U32 { return value * value + 2; } \
             fn cpu_entry() -> U32 { return transform(input()); }",
            "transform",
        )
        .expect("bounded transform admitted");
        assert!(shader.wgsl().contains(" * "));
        assert!(shader.wgsl().contains(" + "));
        assert!(shader.wgsl().contains("= 2u;"));
        assert!(shader.wgsl().contains("arrayLength(&runen_inputs)"));
        assert!(shader.wgsl().contains("runen_scalar(runen_inputs[index])"));
    }

    #[test]
    fn target_output_changes_when_the_same_source_function_changes() {
        let original = compile_source(
            "fn transform(value: U32) -> U32 { return value * value + 2; }",
            "transform",
        )
        .unwrap();
        let edited = compile_source(
            "fn transform(value: U32) -> U32 { return value * value + 7; }",
            "transform",
        )
        .unwrap();
        assert_ne!(original.wgsl(), edited.wgsl());
        assert!(edited.wgsl().contains("= 7u;"));
    }

    #[test]
    fn ordinary_function_reordering_does_not_affect_selected_function_translation() {
        let a = compile_source(
            "fn unrelated() -> U32 { return 99; } \
             fn transform(value: U32) -> U32 { return value * value + 2; }",
            "transform",
        )
        .unwrap();
        let b = compile_source(
            "fn transform(value: U32) -> U32 { return value * value + 2; } \
             fn unrelated() -> U32 { return 99; }",
            "transform",
        )
        .unwrap();
        assert_eq!(a.wgsl(), b.wgsl());
    }

    #[test]
    fn target_coverage_fails_closed_for_non_u32_and_non_admitted_ops() {
        assert_eq!(
            compile_source("fn f(value: I32) -> I32 { return value * value + 2; }", "f"),
            Err(CoverageError::UnsupportedSignature),
        );
        assert_eq!(
            compile_source("fn f(value: U32) -> U32 { return value - 1; }", "f"),
            Err(CoverageError::UnsupportedStatement),
        );
        assert_eq!(
            compile_source(
                "fn helper(value: U32) -> U32 { return value; } \
                 fn f(value: U32) -> U32 { return helper(value); }",
                "f"
            ),
            Err(CoverageError::UnsupportedControlFlow),
        );
    }
}
