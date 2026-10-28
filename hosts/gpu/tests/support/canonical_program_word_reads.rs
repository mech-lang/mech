//! Exact unsigned-buffer reads without transient selector/result snapshots.
use super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, ValueDataDraft as Data};
use mech_engine::native_artifact::NativeCollectionInvocationError;
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramDescriptor, NativeProgramExecutionError,
};

fn matrix(values: &[u64]) -> Data {
    Data::Matrix(
        values
            .iter()
            .copied()
            .map(Data::U64)
            .collect::<Vec<_>>()
            .into(),
    )
}

#[test]
fn u64_read_selection_requires_the_maintained_factory() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    for (key, entry) in mech_stdlib::source_catalog().resident_factories() {
        if key.module_path.as_ref() != ["access"] || key.operation_name != "scalar" {
            builder
                .insert_resident_factory_entry(entry.clone())
                .unwrap();
        }
    }
    builder
        .insert_resident_factory(
            ["access"],
            "scalar",
            mech_core::ImplementationMemoryClass::CanonicalFinalize,
            |_| panic!("native validation must not run the replacement factory"),
        )
        .unwrap();
    let replacement = builder.build().unwrap();
    for artifact in artifacts("xs := values<[u64]>\nxs[position<u64>]\n").0 {
        assert!(matches!(
            mech_engine::native_artifact::lower_native_program_artifact(&artifact, &replacement),
            Err(
                mech_engine::native_artifact::NativeArtifactLoweringError::MissingSemanticWitness { .. }
            )
        ));
    }
}

#[test]
fn selected_u64_read_keeps_owner_admission_exact_call_limits_and_no_allocation() {
    use mech_core::{
        MemoryBudgetDimension as Dimension, ResourceDemand, ValueDraft,
        snapshot::SnapshotValidationContext,
    };
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as Primitive, NativeIterationAction, NativeLibraryWorkBudget,
        lower_native_program_artifact, prepare_native_program_library_call,
    };
    for artifact in artifacts("xs := values<[u64]>\nxs[position<u64>]\n").0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let template = plan
            .blocks()
            .iter()
            .find_map(|block| match block.action() {
                NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == Primitive::MatrixScalarAccess =>
                {
                    Some(template)
                }
                _ => None,
            })
            .unwrap();
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        assert!(call.supports_u64_matrix_element());
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let named = [
            ("values", matrix(&[17, u64::MAX])),
            ("position", Data::U64(2)),
        ];
        let args = inputs(&artifact, &named, &memory);
        let source = &args[artifact
            .inputs()
            .iter()
            .position(|input| {
                mech_engine::decode_source_input_name(&input.name).unwrap() == "values"
            })
            .unwrap()];
        let work = NativeLibraryWorkBudget::new(1 << 40, 1 << 32, &memory).unwrap();
        let before = memory.used_bytes();
        let mut policy = limits().library;
        policy.max_cloned_bytes = Some(0);
        policy.max_output_elements = Some(1);
        policy.max_output_bytes = Some(8);
        policy.max_temporary_bytes =
            Some(source.memory_budget_retained_bytes(&memory).unwrap() + 16);
        policy.max_retained_nodes = Some(source.memory_budget_retained_nodes(&memory).unwrap() + 2);
        let start = work.used();
        assert_eq!(
            call.u64_matrix_element(source, 2, policy, &work, ResourceDemand::default())
                .unwrap(),
            u64::MAX
        );
        let cost = work.used().0 - start.0;
        assert!(cost > 0);
        assert_eq!(work.used().1, start.1);
        assert_eq!(memory.used_bytes(), before);
        policy.max_compute_work = Some(cost);
        assert_eq!(
            call.u64_matrix_element(source, 1, policy, &work, ResourceDemand::default())
                .unwrap(),
            17
        );
        for dimension in [
            Dimension::TemporaryBytes,
            Dimension::RetainedNodes,
            Dimension::ComputeWork,
            Dimension::OutputBytes,
            Dimension::OutputElements,
        ] {
            let mut denied = policy;
            let (required, limit) = match dimension {
                Dimension::TemporaryBytes => {
                    let required = policy.max_temporary_bytes.unwrap();
                    denied.max_temporary_bytes = Some(required - 1);
                    (required, required - 1)
                }
                Dimension::RetainedNodes => {
                    let required = policy.max_retained_nodes.unwrap();
                    denied.max_retained_nodes = Some(required - 1);
                    (required, required - 1)
                }
                Dimension::ComputeWork => {
                    denied.max_compute_work = Some(cost - 1);
                    (cost, cost - 1)
                }
                Dimension::OutputBytes => {
                    denied.max_output_bytes = Some(7);
                    (8, 7)
                }
                Dimension::OutputElements => {
                    denied.max_output_elements = Some(0);
                    (1, 0)
                }
                _ => unreachable!(),
            };
            let error = call
                .u64_matrix_element(source, 2, denied, &work, ResourceDemand::default())
                .unwrap_err();
            match error {
                NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                    dimension: actual,
                    required: need,
                    limit: cap,
                    ..
                }) => {
                    assert_eq!((actual, need, cap), (dimension, required, limit));
                }
                NativeCollectionInvocationError::Plan(
                    mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                ) => {
                    assert_eq!(
                        (violation.dimension, violation.required, violation.limit),
                        (dimension, required, limit)
                    );
                }
                error => panic!("{dimension:?}: {error:?}"),
            }
            assert_eq!(memory.used_bytes(), before);
            assert_eq!(
                call.u64_matrix_element(source, 2, policy, &work, ResourceDemand::default())
                    .unwrap(),
                u64::MAX
            );
        }
        let unadmitted = ValueDraft {
            schema: source.schema(),
            shape_values: source
                .shape()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
            data: source.canonical_data_draft().unwrap(),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        assert!(matches!(
            call.u64_matrix_element(&unadmitted, 1, policy, &work, ResourceDemand::default()),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        ));
        let foreign_memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let foreign = inputs(&artifact, &named, &foreign_memory);
        let index = artifact
            .inputs()
            .iter()
            .position(|input| {
                mech_engine::decode_source_input_name(&input.name).unwrap() == "values"
            })
            .unwrap();
        assert!(matches!(
            call.u64_matrix_element(&foreign[index], 1, policy, &work, ResourceDemand::default()),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        ));
        let wrong = args
            .iter()
            .find(|value| value.schema() != source.schema())
            .unwrap();
        assert!(matches!(
            call.u64_matrix_element(wrong, 1, policy, &work, ResourceDemand::default()),
            Err(NativeCollectionInvocationError::InvalidInput)
        ));
    }
}

#[test]
fn u64_scalar_reads_keep_wide_words_live_extents_errors_and_recovery() {
    for source in [
        "xs := values<[u64]>\nxs[position<u64>]\n",
        "pick(xs<[u64]>, at<u64>) => (out<u64>) :=\n  out := xs[at].\npick(values<[u64]>, position<u64>)\n",
    ] {
        let (artifacts, last) = artifacts(source);
        for artifact in artifacts {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let image = jit.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: unchanged image from this exact checked build; this
            // test inspects metadata without pairing it with a foreign object.
            let restored =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), image);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (values, position) in [
                (vec![u64::MAX, 1 << 63, 9_007_199_254_740_993], 1),
                (vec![u64::MAX, 1 << 63, 9_007_199_254_740_993], 3),
                (vec![], 1),
                (vec![17], 0),
                (vec![17], 2),
                (vec![17], u64::MAX),
                (vec![23], 1),
                (vec![0, 1, 2, u64::MAX], 4),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("values", matrix(&values)),
                        ("position", Data::U64(position)),
                    ],
                    &memory,
                );
                if position == 0 || position > values.len() as u64 {
                    let ordinary_error = reference_result(&artifact, &args).unwrap_err();
                    let prior = output
                        .iter()
                        .map(|value| value.clone().unwrap())
                        .collect::<Vec<_>>();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    let expected = if position == 0 || position > u32::MAX as u64 {
                        ResidentKernelError::InvalidInput
                    } else {
                        ResidentKernelError::IndexOutOfRange {
                            index: position,
                            upper_bound: values.len() as u64,
                        }
                    };
                    assert!(
                        ordinary_error.contains(&format!("{expected:?}")),
                        "{ordinary_error}"
                    );
                    assert!(
                        matches!(error, NativeProgramExecutionError::Library {
                        error: NativeCollectionInvocationError::Kernel(ref actual), ..
                    } if actual == &expected),
                        "{error:?}"
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                } else {
                    let expected = reference(&artifact, &args);
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                    assert_eq!(
                        output[last]
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(values[position as usize - 1])
                    );
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn u64_reads_use_linear_column_major_order_and_already_admitted_scalar_storage() {
    let (artifacts, last) = artifacts("xs := values<[u64]:2,3>\nxs[position<u64>]\n");
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let values = [u64::MAX, 2, 1 << 63, 4, 5, 6];
        let mut policy = limits();
        // The original read executes and still funds its complete source and
        // native frame. It allocates/clones no selector or result Value.
        policy.library.max_cloned_bytes = Some(0);
        policy.library.max_output_bytes = Some(8);
        for position in [6, 1, 3, 2, 5, 4] {
            let args = artifact
                .inputs()
                .iter()
                .map(|input| {
                    let name = mech_engine::decode_source_input_name(&input.name).unwrap();
                    let data = if name == "values" {
                        matrix(&values)
                    } else {
                        Data::U64(position)
                    };
                    let value = mech_core::ValueDraft {
                        schema: input.schema,
                        shape_values: Box::new([]),
                        data,
                    }
                    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                        artifact.schemas(),
                    ))
                    .unwrap();
                    let mut reservation = memory
                        .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                        .unwrap();
                    value.into_memory_budget(&mut reservation).unwrap()
                })
                .collect::<Vec<_>>();
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .unwrap();
            assert_eq!(
                output[last]
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(values[position as usize - 1])
            );
        }
    }
}

#[test]
fn u64_read_budget_denials_preserve_previous_publication_and_recover() {
    let (artifacts, last) = artifacts("xs := values<[u64]>\nxs[position<u64>]\n");
    for artifact in artifacts {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("values", matrix(&[u64::MAX, 17])),
                ("position", Data::U64(1)),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        for dimension in 0..5 {
            let mut policy = limits();
            match dimension {
                0 => policy.library.max_compute_work = Some(0),
                1 => policy.library.max_temporary_bytes = Some(0),
                2 => policy.library.max_retained_nodes = Some(0),
                3 => policy.library.max_output_elements = Some(0),
                _ => policy.library.max_output_bytes = Some(7),
            }
            let before = memory.used_bytes();
            assert!(
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(
                output[last]
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(u64::MAX)
            );
        }
    }
}
