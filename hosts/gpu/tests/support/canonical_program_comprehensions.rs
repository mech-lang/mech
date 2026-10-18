//! Actual generated collection bodies, geometry, ownership and atomic failure.
use super::{artifacts, check, equal, inputs, limits, reference};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetDimension as Dimension, ResidentKernelError, Value,
    ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
    NativeProgramExecutionLimits,
};

fn numbers(values: &[u64]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect())
}

#[test]
fn unicode_string_and_matrix_generators_keep_exact_canonical_elements() {
    check(
        "+> text\ntext/join([letter | letter <- input<string>], \"|\")\n",
        &[("input", ValueDataDraft::String("éé🧑‍🚀🇺🇳🇺🇸\r\na".into()))],
    );
    check(
        "+> text\ntext/join([word | word <- values<[string]>], \"|\")\n",
        &[(
            "values",
            ValueDataDraft::Matrix(
                ["é", "", "é", "🧑‍🚀"]
                    .into_iter()
                    .map(|text| ValueDataDraft::String(text.to_owned()))
                    .collect(),
            ),
        )],
    );
}

#[test]
fn scalar_matrix_and_ordered_set_sources_preserve_exact_element_kinds() {
    for (kind, elements) in [
        (
            "bool",
            vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)],
        ),
        (
            "i64",
            [i64::MIN, 0, i64::MAX]
                .into_iter()
                .map(ValueDataDraft::I64)
                .collect(),
        ),
        (
            "f64",
            [
                0u64,
                1u64 << 63,
                0x7ff0_0000_0000_0000,
                0x7ff8_0000_0000_0001,
            ]
            .into_iter()
            .map(|bits| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_bits(bits)))
            .collect(),
        ),
    ] {
        for values in [elements, Vec::new()] {
            check(
                &format!("[value | value <- values<[{kind}]>]\n"),
                &[("values", ValueDataDraft::Matrix(values.into_boxed_slice()))],
            );
        }
    }
    // Transport a canonical Set, not an invalid draft with duplicate keys.
    for values in [vec!["A", "é", "🧑‍🚀"], Vec::new()] {
        check(
            "[value | value <- values<{string}>]\n",
            &[(
                "values",
                ValueDataDraft::Set(
                    values
                        .into_iter()
                        .map(|text| ValueDataDraft::String(text.to_owned()))
                        .collect(),
                ),
            )],
        );
    }
}

#[test]
fn generated_nested_generators_filters_wildcards_and_literal_loops_keep_their_scopes() {
    for source in [
        "[x + y | x <- values<[u64]>, y <- values, x < y]\n",
        "[1u64 | * <- values<[u64]>]\n",
        "inc(x<u64>) => <u64> := x + 1u64\n[x |2> inc | x <- values<[u64]>]\n",
        "+> sequence\n[sequence/length([x + y | y <- values]) | x <- values<[u64]>]\n",
    ] {
        check(source, &[("values", numbers(&[3, 1, 2]))]);
        check(source, &[("values", numbers(&[]))]);
    }
}

#[test]
fn string_iteration_retirement_preserves_captures_yields_and_nested_results() {
    for source in [
        "[left | left <- values<[string]>, * <- values]\n",
        "identity(word<string>) => <string>\n  | value => value.\n[identity(word) | word <- values<[string]>]\n",
        "identity(word<string>) => <string>\n  | value => value.\n[word |2> identity | word <- values<[string]>]\n",
        "+> text\n[text/join([inner | inner <- values], outer) | outer <- values<[string]>]\n",
    ] {
        for values in [vec!["é", "", "🧑‍🚀"], Vec::new()] {
            check(
                source,
                &[(
                    "values",
                    ValueDataDraft::Matrix(
                        values
                            .into_iter()
                            .map(|word| ValueDataDraft::String(word.into()))
                            .collect(),
                    ),
                )],
            );
        }
    }
}

fn shaped_input(
    artifact: &mech_engine::ProgramArtifact,
    extents: [u64; 2],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    assert_eq!(artifact.inputs().len(), 1);
    let input = &artifact.inputs()[0];
    let shape = mech_core::shape_for_resolved_extents(
        artifact.schemas().get(input.schema).unwrap(),
        &extents,
    )
    .unwrap();
    let count = extents[0].checked_mul(extents[1]).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: numbers(&(1..=count).collect::<Vec<_>>()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    vec![value.into_memory_budget(&mut reservation).unwrap()]
}

#[test]
fn one_compiled_pattern_lift_preserves_rows_columns_and_empty_axes_across_calls() {
    let source = "+> number\nshow(x<u64>) => <string>\n  | x => number/format(x + 1u64, 0u64).\nshow(values<[u64]>)\n";
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        for extents in [[3, 1], [1, 3], [2, 2], [0, 4], [4, 0], [0, 0], [1, 1]] {
            let args = shaped_input(&artifact, extents, &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            let value = output.last().unwrap().as_ref().unwrap();
            let owner = value.schemas().unwrap();
            let mech_core::SchemaBody::Matrix { dimensions, .. } =
                owner.get(value.schema()).unwrap().body()
            else {
                panic!("matrix output")
            };
            assert_eq!(
                dimensions
                    .iter()
                    .map(|axis| value.shape().resolve_dimension(axis).unwrap())
                    .collect::<Vec<_>>(),
                extents
            );
        }
    }
}

#[test]
fn late_checked_error_discards_partial_collection_and_recovers_on_the_same_jit() {
    let (artifacts, _) = artifacts("[100u64 / value | value <- values<[u64]>]\n");
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let valid = inputs(&artifact, &[("values", numbers(&[2, 4]))], &memory);
        let denied = inputs(&artifact, &[("values", numbers(&[2, 0]))], &memory);
        let recovered = inputs(&artifact, &[("values", numbers(&[5]))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&valid, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        assert!(matches!(
            jit.evaluate_into(&denied, &mut output, limits(), &memory),
            Err(NativeProgramExecutionError::Control(
                NativeIterationExecutionError::Arithmetic { .. }
            ))
        ));
        assert_eq!(
            memory.used_bytes(),
            before,
            "denied builder/source backing must retire"
        );
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&recovered, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &recovered));
    }
}

#[test]
fn collection_dimension_denials_release_workspace_preserve_output_and_recover() {
    let (artifacts, _) = artifacts("[value + 1u64 | value <- values<[u64]>]\n");
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(&artifact, &[("values", numbers(&[1, 2, 3]))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|value| value.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        for dimension in [
            Dimension::OutputElements,
            Dimension::OutputBytes,
            Dimension::TemporaryBytes,
            Dimension::ClonedBytes,
            Dimension::RetainedNodes,
            Dimension::ComputeWork,
        ] {
            let mut denied = limits();
            match dimension {
                Dimension::OutputElements => denied.library.max_output_elements = Some(1),
                Dimension::OutputBytes => denied.library.max_output_bytes = Some(0),
                Dimension::TemporaryBytes => {
                    denied.library.max_temporary_bytes = Some(report.transport_bytes + 1024)
                }
                Dimension::ClonedBytes => denied.library.max_cloned_bytes = Some(0),
                Dimension::RetainedNodes => denied.library.max_retained_nodes = Some(0),
                Dimension::ComputeWork => denied.library.max_compute_work = Some(0),
                _ => unreachable!(),
            }
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&args, &mut output, denied, &memory)
                .unwrap_err();
            assert!(
                matches!(&error, NativeProgramExecutionError::Collection {
                error: ResidentKernelError::ResourceLimit { dimension: actual, required, limit, .. }, ..
            } if *actual == dimension && required > limit),
                "{dimension:?}: {error:?}"
            );
            assert_eq!(
                memory.used_bytes(),
                before,
                "{dimension:?}: no leaked workspace"
            );
            equal(&artifact, &output, &prior);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &prior);
        }
    }
}

// Derive the exact admission boundary from typed denials, rather than relying
// on platform-specific metadata sizes. This changes only this invocation's
// deliberately restrictive test policy, never the production/default limits.
fn minimum_filtered_policy(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: usize,
    dimension: Dimension,
    memory: &ManagedMemoryBudget,
) -> NativeProgramExecutionLimits {
    let mut policy = limits();
    let ceiling = match dimension {
        Dimension::ComputeWork => policy.library.max_compute_work.unwrap(),
        Dimension::TemporaryBytes => policy.library.max_temporary_bytes.unwrap(),
        _ => unreachable!(),
    };
    let mut boundary = 0;
    loop {
        match dimension {
            Dimension::ComputeWork => policy.library.max_compute_work = Some(boundary),
            Dimension::TemporaryBytes => policy.library.max_temporary_bytes = Some(boundary),
            _ => unreachable!(),
        }
        let before = memory.used_bytes();
        let mut output = vec![None; outputs];
        let result = jit.evaluate_into(args, &mut output, policy, memory);
        if result.is_ok() {
            drop(output);
            assert_eq!(memory.used_bytes(), before);
            return policy;
        }
        assert!(output.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), before, "boundary denial leaked memory");
        match result.unwrap_err() {
            NativeProgramExecutionError::Collection {
                error:
                    ResidentKernelError::ResourceLimit {
                        dimension: actual,
                        required,
                        limit,
                        ..
                    },
                ..
            } if actual == dimension && limit == boundary && required > limit => {
                assert!(
                    required <= ceiling,
                    "fixture exceeds its finite test ceiling"
                );
                boundary = required;
            }
            error => panic!("unexpected {dimension:?} boundary error: {error:?}"),
        }
    }
}

#[test]
fn filtered_wildcard_visits_remain_cumulative_collection_compute() {
    let (artifacts, _) = artifacts("[1u64 | * <- values<[u64]>, keep<bool>]\n");
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let named = |values| {
            [
                ("values", numbers(values)),
                ("keep", ValueDataDraft::Bool(false)),
            ]
        };
        let small = inputs(&artifact, &named(&[1]), &memory);
        let many = inputs(&artifact, &named(&vec![1; 4096]), &memory);
        let policy = minimum_filtered_policy(
            &jit,
            &small,
            artifact.outputs().len(),
            Dimension::ComputeWork,
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let result = jit.evaluate_into(&many, &mut output, policy, &memory);
        assert!(
            matches!(result, Err(NativeProgramExecutionError::Collection {
            error: ResidentKernelError::ResourceLimit {
                dimension: Dimension::ComputeWork, required, limit, ..
            }, ..
        }) if required > limit),
            "filtered wildcard traversal bypassed its collection budget: {result:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &small));
        drop((small, many, prior, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn filtered_string_visits_admit_the_old_and_new_element_coexistence_peak() {
    let (artifacts, _) = artifacts("[word | word <- values<[string]>, keep<bool>]\n");
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let named = |count| {
            [
                (
                    "values",
                    ValueDataDraft::Matrix(
                        (0..count)
                            .map(|_| ValueDataDraft::String("x".repeat(8192)))
                            .collect(),
                    ),
                ),
                ("keep", ValueDataDraft::Bool(false)),
            ]
        };
        let small = inputs(&artifact, &named(1), &memory);
        let many = inputs(&artifact, &named(2), &memory);
        let policy = minimum_filtered_policy(
            &jit,
            &small,
            artifact.outputs().len(),
            Dimension::TemporaryBytes,
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let result = jit.evaluate_into(&many, &mut output, policy, &memory);
        assert!(
            matches!(result, Err(NativeProgramExecutionError::Collection {
            error: ResidentKernelError::ResourceLimit {
                dimension: Dimension::TemporaryBytes, required, limit, ..
            }, ..
        }) if required > limit),
            "retained generator element was omitted from the peak: {result:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &small));
        drop((small, many, prior, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
