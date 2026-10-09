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

#[cfg(test)]
mod gpu_contract_tests {
    use super::*;
    use runen_core_lowering::lower;
    use runen_gpu as gpu;
    use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
    use runen_syntax::parse_source;

    fn generated(source: &str) -> U32Kernel {
        let parsed = parse_source(source.as_bytes()).expect("Runen source should be UTF-8");
        assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
        let compilation = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
            .expect("Runen source must be accepted");
        let selected = compilation
            .functions
            .iter()
            .find(|function| function.name == "transform")
            .expect("test transform")
            .id;
        let lowered = lower(&compilation).expect("typed Runen source must lower");
        compile_u32_kernel(
            lowered.program(),
            lowered
                .core_function(selected)
                .expect("exact ordinary function mapping"),
        )
        .expect("bounded U32 target must admit source function")
    }

    fn admitted_pipeline(kernel: &U32Kernel) -> gpu::GpuComputePipelineDescriptor {
        let owner = gpu::GpuProgramSourceOwnerId::allocate().unwrap();
        let identity = gpu::GpuProgramSourceIdentity::new(
            owner,
            gpu::GpuProgramSourceKey::new("runen.core.u32.proof").unwrap(),
            gpu::GpuProgramSourceRevision::try_from_raw(1).unwrap(),
        );
        let mut registry = gpu::GpuProgramSourceRegistry::new(2, 16 * 1024).unwrap();
        let admitted = registry
            .admit_wgsl(
                identity,
                kernel.wgsl(),
                gpu::GpuProgramSourceProvenance::new("runen-validated-core-gpu-test", None)
                    .unwrap(),
            )
            .unwrap();
        gpu::GpuComputePipelineDescriptor::ordinary(admitted, "runen_main")
            .expect("generated Runen WGSL must pass RunenGPU's canonical program admission")
    }

    fn input_buffer(scope: &mut gpu::GpuResourceScope, inputs: &[u32]) -> gpu::GpuBufferHandle {
        let prepared = gpu::PreparedGpuData::<gpu::TransferData>::ordinary_pod_transfer(
            "runen U32 scalar-map inputs",
            inputs,
        )
        .unwrap();
        scope
            .buffer(
                gpu::GpuBufferDescriptor::ordinary_owned(
                    "runen U32 scalar-map inputs",
                    gpu::GpuResourceLifetime::Transient,
                    gpu::GpuReconstruction::SourceBacked,
                    prepared.layout().byte_len(),
                    [
                        gpu::GpuBufferUsage::Storage,
                        gpu::GpuBufferUsage::CopyDestination,
                    ],
                    gpu::GpuBufferInitialization::Prepared(prepared),
                )
                .unwrap(),
            )
            .unwrap()
    }

    fn output_buffer(scope: &mut gpu::GpuResourceScope, count: usize) -> gpu::GpuBufferHandle {
        scope
            .buffer(
                gpu::GpuBufferDescriptor::ordinary_owned(
                    "runen U32 scalar-map outputs",
                    gpu::GpuResourceLifetime::Transient,
                    gpu::GpuReconstruction::SourceBacked,
                    u64::try_from(count).unwrap() * 4,
                    [
                        gpu::GpuBufferUsage::Storage,
                        gpu::GpuBufferUsage::CopySource,
                    ],
                    gpu::GpuBufferInitialization::Zeroed,
                )
                .unwrap(),
            )
            .unwrap()
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum HostMapError {
        TooManyElements,
    }

    fn admitted_workgroups(len: usize) -> Result<Option<u32>, HostMapError> {
        if len == 0 {
            return Ok(None);
        }
        if len > 4097 {
            return Err(HostMapError::TooManyElements);
        }
        Ok(Some(
            u32::try_from(len.div_ceil(64)).expect("bounded input"),
        ))
    }

    fn graph(
        pipeline: gpu::GpuComputePipelineDescriptor,
        values: &[u32],
    ) -> Result<Option<(gpu::GpuPreparedWorkGraph, gpu::GpuReadbackId)>, HostMapError> {
        let Some(workgroups) = admitted_workgroups(values.len())? else {
            return Ok(None);
        };

        let mut scope = gpu::GpuResourceScope::new();
        let inputs = input_buffer(&mut scope, values);
        let outputs = output_buffer(&mut scope, values.len());
        let bindings = pipeline
            .runtime_bindings([
                gpu::GpuRuntimeBindingValue::whole_buffer(0, 0, &inputs),
                gpu::GpuRuntimeBindingValue::whole_buffer(0, 1, &outputs),
            ])
            .unwrap();
        let compute = gpu::GpuComputeOperation::new(
            pipeline,
            bindings,
            gpu::GpuDispatchIntent::direct(gpu::GpuDispatchSize::new(workgroups, 1, 1)),
        )
        .unwrap();
        let read = gpu::GpuReadbackOperation::ordinary(
            gpu::GpuBufferRegion::whole(&outputs).unwrap().into(),
        )
        .unwrap();
        let readback_id = read.id();
        let fragment =
            gpu::GpuWorkFragment::build("runen source-derived scalar-map proof", |work| {
                work.operation("runen selected source computation", compute)?;
                work.operation("observe Runen computed buffer", read)?;
                Ok(())
            })
            .unwrap();
        let graph = gpu::GpuPreparedWorkGraph::prepare(
            gpu::GpuResourceLabel::new("runen source-derived U32 scalar map").unwrap(),
            [fragment],
        )
        .unwrap();
        Ok(Some((graph, readback_id)))
    }

    fn inputs() -> Vec<u32> {
        let mut values = vec![0, 1, 65536, u32::MAX, 2, 3, 65535];
        for i in 0_u32..72 {
            values.push(i.wrapping_mul(1_103_515_245).wrapping_add(12345));
        }
        values
    }

    #[test]
    fn runen_gpu_admits_the_generated_source_and_its_compute_graph() {
        let kernel = generated("fn transform(value: U32) -> U32 { return value * value + 2; }");
        let pipeline = admitted_pipeline(&kernel);
        let (graph, _) = graph(pipeline, &inputs()).unwrap().unwrap();
        assert_eq!(graph.nodes().len(), 2);
        assert_eq!(graph.topological_order().len(), 2);
    }

    #[test]
    fn empty_host_map_does_not_dispatch_and_excess_size_rejects_before_allocating() {
        let kernel = generated("fn transform(value: U32) -> U32 { return value * value + 2; }");
        let pipeline = admitted_pipeline(&kernel);
        assert!(matches!(graph(pipeline.clone(), &[]), Ok(None)));
        assert!(matches!(
            graph(pipeline, &vec![0_u32; 4098]),
            Err(HostMapError::TooManyElements)
        ));
    }

    /// Run with software Vulkan in conformance CI. On a separate physical GPU
    /// executor, set RUNEN_GPU_REQUIRE_HARDWARE=1 to forbid fallback. Only the
    /// second mode can establish independently recorded hardware evidence.
    #[test]
    #[ignore = "requires native Vulkan execution and actual GPU API dispatch/readback"]
    fn native_runen_source_derived_u32_kernel_executes_and_reads_back() {
        use std::time::{Duration, Instant};
        let hardware_required = std::env::var("RUNEN_GPU_REQUIRE_HARDWARE")
            .ok()
            .as_deref()
            == Some("1");
        let mut reqs = gpu::GpuCapabilityRequirements::new();
        for feature in [
            gpu::GpuCapabilityFeature::Compute,
            gpu::GpuCapabilityFeature::Copy,
        ] {
            reqs.insert(gpu::GpuCapabilityRequirement::Required(feature))
                .unwrap();
        }
        let fallback_policy = if hardware_required {
            gpu::GpuSoftwareFallbackPolicy::Forbid
        } else {
            gpu::GpuSoftwareFallbackPolicy::Require
        };
        let descriptor = gpu::GpuContextDescriptor::new(reqs)
            .with_fallback_policy(fallback_policy)
            .with_allowed_backends([gpu::GpuBackendFamily::Vulkan])
            .with_label("Runen U32 source-to-device conformance");
        let context = pollster::block_on(gpu::GpuContext::request(descriptor))
            .expect("selected Vulkan conformance adapter must be available");
        println!("Runen U32 GPU conformance adapter: {:#?}", context.adapter_facts());
        if hardware_required {
            assert_eq!(
                context.adapter_facts().software(),
                gpu::GpuSoftwareStatus::Hardware,
                "hardware qualification must not accept an unknown/software adapter"
            );
            assert_eq!(
                context.adapter_facts().fallback(),
                gpu::GpuFallbackStatus::ConfirmedNotFallback
            );
        } else {
            assert_eq!(
                context.adapter_facts().fallback(),
                gpu::GpuFallbackStatus::ConfirmedFallback
            );
        }
        for (source, constant) in [
            (
                "fn transform(value: U32) -> U32 { return value * value + 2; }",
                2_u32,
            ),
            (
                "fn transform(value: U32) -> U32 { return value * value + 7; }",
                7_u32,
            ),
        ] {
            let values = inputs();
            let kernel = generated(source);
            let (graph, read_id) = graph(admitted_pipeline(&kernel), &values).unwrap().unwrap();
            let prepared = pollster::block_on(context.prepare_submission(graph))
                .expect("GPU graph must admit");
            let submission = context.submit_prepared(prepared).expect("GPU submission");
            let readback = submission.readback(read_id).unwrap().clone();
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                context.progress();
                match readback.status() {
                    gpu::GpuReadbackStatus::Ready(bytes)
                        if matches!(submission.status(), gpu::GpuSubmissionStatus::Completed) =>
                    {
                        let (chunks, remainder) = bytes.as_bytes().as_chunks::<4>();
                        let outputs = chunks
                            .iter()
                            .map(|chunk| u32::from_le_bytes(*chunk))
                            .collect::<Vec<_>>();
                        assert!(remainder.is_empty());
                        assert_eq!(outputs.len(), values.len());
                        for (input, output) in values.iter().zip(&outputs) {
                            assert_eq!(
                                *output,
                                input.wrapping_mul(*input).wrapping_add(constant),
                                "source-derived GPU result mismatch for {input}"
                            );
                        }
                        break;
                    }
                    gpu::GpuReadbackStatus::Failed(failure) => {
                        panic!("GPU readback failed: {failure:?}")
                    }
                    _ => {}
                }
                if let gpu::GpuSubmissionStatus::Failed(failure) = submission.status() {
                    panic!("GPU submission failed: {failure:?}");
                }
                assert!(Instant::now() < deadline, "GPU proof timed out");
                std::thread::yield_now();
            }
        }
    }
}
