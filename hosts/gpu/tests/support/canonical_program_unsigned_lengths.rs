//! The maintained length operation reads current admitted matrix metadata.
use super::{allocation_probe, artifacts, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as InvokeError, NativeCollectionPrimitive as Primitive,
    NativeIterationAction as Action, NativeLibraryWorkBudget, lower_native_program_artifact,
    prepare_native_program_library_call,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError as Error,
};

#[test]
fn selected_unsigned_lengths_keep_call_limits_admission_and_zero_allocations() {
    use mech_core::{MemoryBudgetDimension as Dimension, ResourceDemand};
    for (width, word) in [
        (8, Data::U8(17)),
        (16, Data::U16(17)),
        (32, Data::U32(17)),
        (64, Data::U64(17)),
    ] {
        for artifact in artifacts(&format!(
            "+> sequence\nsequence/length(values<[u{width}]>)\n"
        ))
        .0
        {
            let catalog = mech_stdlib::source_catalog();
            let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
            let template = plan
                .blocks()
                .iter()
                .find_map(|b| match b.action() {
                    Action::LibraryCall { template, .. }
                        if template.primitive() == Primitive::SequenceLength =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
            assert!(call.supports_unsigned_matrix_length());
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(1 << 32, 1 << 32, &memory).unwrap();
            for count in [0, 3, 31] {
                let args = inputs(
                    &artifact,
                    &[("values", Data::Matrix(vec![word.clone(); count].into()))],
                    &memory,
                );
                let source = &args[0];
                let before = memory.used_bytes();
                let mut policy = limits().library;
                policy.max_cloned_bytes = Some(0);
                policy.max_output_elements = Some(1);
                policy.max_output_bytes = Some(8);
                policy.max_temporary_bytes =
                    Some(source.memory_budget_retained_bytes(&memory).unwrap() + 8);
                policy.max_retained_nodes =
                    Some(source.memory_budget_retained_nodes(&memory).unwrap() + 1);
                let start = work.used();
                let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                    call.unsigned_matrix_length(source, policy, &work, ResourceDemand::default())
                });
                assert_eq!(result.unwrap(), count as u64);
                assert_eq!((allocations, bytes), (0, 0));
                let cost = work.used().0 - start.0;
                assert!(cost > 0);
                assert_eq!(work.used().1 - start.1, 1);
                policy.max_compute_work = Some(cost);
                policy.max_comparison_work = Some(1);
                for dimension in [
                    Dimension::ComputeWork,
                    Dimension::ComparisonWork,
                    Dimension::OutputBytes,
                    Dimension::OutputElements,
                    Dimension::TemporaryBytes,
                    Dimension::RetainedNodes,
                ] {
                    let mut denied = policy;
                    let (required, cap) = match dimension {
                        Dimension::ComputeWork => {
                            denied.max_compute_work = Some(cost - 1);
                            (cost, cost - 1)
                        }
                        Dimension::ComparisonWork => {
                            denied.max_comparison_work = Some(0);
                            (1, 0)
                        }
                        Dimension::OutputBytes => {
                            denied.max_output_bytes = Some(7);
                            (8, 7)
                        }
                        Dimension::OutputElements => {
                            denied.max_output_elements = Some(0);
                            (1, 0)
                        }
                        Dimension::TemporaryBytes => {
                            let n = policy.max_temporary_bytes.unwrap();
                            denied.max_temporary_bytes = Some(n - 1);
                            (n, n - 1)
                        }
                        Dimension::RetainedNodes => {
                            let n = policy.max_retained_nodes.unwrap();
                            denied.max_retained_nodes = Some(n - 1);
                            (n, n - 1)
                        }
                        _ => unreachable!(),
                    };
                    let error = call
                        .unsigned_matrix_length(source, denied, &work, ResourceDemand::default())
                        .unwrap_err();
                    let actual = match error {
                        InvokeError::Kernel(mech_core::ResidentKernelError::ResourceLimit {
                            dimension,
                            required,
                            limit,
                            ..
                        }) => (dimension, required, limit),
                        InvokeError::Plan(mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation,
                        }) => (violation.dimension, violation.required, violation.limit),
                        error => panic!("{dimension:?}: {error:?}"),
                    };
                    assert_eq!(actual, (dimension, required, cap));
                    assert_eq!(memory.used_bytes(), before);
                    assert_eq!(
                        call.unsigned_matrix_length(
                            source,
                            policy,
                            &work,
                            ResourceDemand::default()
                        )
                        .unwrap(),
                        count as u64
                    );
                }
                let foreign = ManagedMemoryBudget::new(32 * 1024 * 1024);
                let foreign_work =
                    NativeLibraryWorkBudget::new(1 << 32, 1 << 32, &foreign).unwrap();
                assert!(matches!(
                    call.unsigned_matrix_length(
                        source,
                        limits().library,
                        &foreign_work,
                        ResourceDemand::default()
                    ),
                    Err(InvokeError::UnadmittedValue)
                ));
            }
            drop(work);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn native_unsigned_length_reads_fresh_buffers_preserves_assertions_and_recovers() {
    let source = "+> sequence\nsize(xs<[u32]>, allowed<bool>, divisor<u64>) => (out<u64>) :=\n  enabled! := allowed\n  count := sequence/length(xs)\n  out := count + 12u64 / divisor.\nsize(values<[u32]>, allowed<bool>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, allowed, divisor) in [
            (3, true, 3),
            (1_500_513, false, 0),
            (0, true, 0),
            (1_500_513, true, 2),
            (2, true, 4),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("values", Data::Matrix(vec![Data::U32(17); count].into())),
                    ("allowed", Data::Bool(allowed)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let (result, _, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
            });
            if count == 1_500_513 {
                assert!(
                    bytes < count * 4,
                    "a length query must not copy its payload: {bytes}"
                );
            }
            if !allowed || divisor == 0 {
                assert!(
                    matches!(
                        (allowed, &result),
                        (
                            false,
                            Err(Error::Control(
                                NativeIterationExecutionError::Integrity { .. }
                            ))
                        ) | (
                            true,
                            Err(Error::Control(
                                NativeIterationExecutionError::Arithmetic { .. }
                            ))
                        )
                    ),
                    "{result:?}"
                );
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), before);
                if count <= 3 {
                    assert!(reference_result(&artifact, &args).is_err());
                }
            } else {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(count as u64 + 12 / divisor)
                );
                if count <= 3 {
                    super::equal(
                        &artifact,
                        &output,
                        &reference_result(&artifact, &args).unwrap(),
                    );
                }
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn unsigned_length_requires_its_factory_and_keeps_string_graphemes() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    for (key, entry) in mech_stdlib::source_catalog().resident_factories() {
        if key.module_path.as_ref() != ["sequence"] || key.operation_name != "length" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    builder
        .insert_resident_factory(
            ["sequence"],
            "length",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("replacement must be rejected before binding"),
        )
        .unwrap();
    let replacement = builder.build().unwrap();
    for artifact in artifacts("+> sequence\nsequence/length(values<[u32]>)\n").0 {
        assert!(JitProgramArtifact::compile(&artifact, &replacement).is_err());
    }
    for artifact in artifacts("+> sequence\nsequence/length(value<string>)\n").0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for text in ["", "a\r\n", "e\u{301}👩‍💻"] {
            let args = inputs(&artifact, &[("value", Data::String(text.into()))], &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &output,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
