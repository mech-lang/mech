//! Actual generated collection bodies, geometry, ownership and atomic failure.
use super::{artifacts, check, equal, inputs, limits, reference, reference_result};
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
fn large_unsigned_comprehension_initializes_a_real_buffer_and_recovers_after_denial() {
    let source = "repeat(value<u32>, count<u64>) => (out<[u32]>) :=\n  out := [value | i <- 0u64..=count, i < count].\nrepeat(word<u32>, count<u64>)\n";
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut policy = limits();
        policy.library.max_output_elements = Some(1_500_513);
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        policy.execution.max_work = 10_000_000_000;
        let mut output = vec![None; artifact.outputs().len()];
        for count in [0, 3, 1_500_513, 5] {
            let args = inputs(
                &artifact,
                &[
                    ("word", ValueDataDraft::U32(0xf0123456)),
                    ("count", ValueDataDraft::U64(count)),
                ],
                &memory,
            );
            let retained = memory.used_bytes();
            let start = std::time::Instant::now();
            let (result, allocations, bytes) = super::allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            let elapsed = start.elapsed().as_secs_f64();
            if count == 1_500_513 {
                println!(
                    "{}",
                    serde_json::json!({"witness":"large-u32-comprehension",
                    "artifact":form, "count":count, "elapsed_seconds":elapsed,
                    "allocation_requests":allocations, "requested_allocation_bytes":bytes,
                    "result":format!("{result:?}")})
                );
            }
            if result.is_err() {
                assert_eq!(memory.used_bytes(), retained);
            }
            result.unwrap();
            let mech_core::ValueData::Matrix(matrix) =
                output.last().unwrap().as_ref().unwrap().data()
            else {
                panic!()
            };
            let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                panic!("packed U32 output")
            };
            assert_eq!(words.len(), count as usize);
            assert!(words.iter().all(|word| *word == 0xf0123456));
            if count <= 3 {
                equal(&artifact, &output, &reference(&artifact, &args));
            }
            if count == 1_500_513 {
                let prior = output.clone();
                let retained = memory.used_bytes();
                let mut denied = policy;
                denied.library.max_output_bytes = Some(1);
                assert!(
                    jit.evaluate_into(&args, &mut output, denied, &memory)
                        .is_err()
                );
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn packed_word_collection_keeps_copy_limits_and_current_output_on_rejection() {
    let source = "[7u32 | i <- 0u64..=count<u64>, i < count]\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("count", ValueDataDraft::U64(3))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        // The cursor computes its words; the three output writes consume
        // twelve bytes. Finalization transfers the packed buffer to its Value.
        policy.library.max_cloned_bytes = Some(12);
        jit.evaluate_into(&args, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
        let prior = output.clone();
        let before = memory.used_bytes();
        policy.library.max_cloned_bytes = Some(11);
        let error = jit
            .evaluate_into(&args, &mut output, policy, &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                NativeProgramExecutionError::Collection {
                    error: ResidentKernelError::ResourceLimit {
                        dimension: Dimension::ClonedBytes,
                        required: 12,
                        limit: 11,
                        ..
                    },
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        assert!(output.iter().zip(&prior).all(|(value, old)| {
            value
                .as_ref()
                .unwrap()
                .shares_frozen_storage(old.as_ref().unwrap())
        }));
        policy.library.max_cloned_bytes = Some(12);
        jit.evaluate_into(&args, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nonuniform_packed_collections_retain_admitted_capacity_and_recover() {
    use mech_core::{ValueData, snapshot::SequenceView};
    for (kind, width) in [("u8", 1), ("u16", 2), ("u32", 4), ("u64", 8)] {
        let source = format!("[i | i <- input<[{kind}]>]\n");
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for count in [3usize, 5] {
                let word = |i: usize| match kind {
                    "u8" => ValueDataDraft::U8(i as u8),
                    "u16" => ValueDataDraft::U16(i as u16),
                    "u32" => ValueDataDraft::U32(i as u32),
                    "u64" => ValueDataDraft::U64(i as u64),
                    _ => unreachable!(),
                };
                let args = inputs(
                    &artifact,
                    &[(
                        "input",
                        ValueDataDraft::Matrix((1..=count).map(word).collect()),
                    )],
                    &memory,
                );
                let capacity = count.next_power_of_two();
                let schema = artifact
                    .schemas()
                    .get(artifact.outputs().last().unwrap().schema)
                    .unwrap();
                let retained_bytes = (capacity * width
                    + core::mem::size_of::<Value>()
                    + core::mem::size_of::<ValueData>()
                    + schema.dimension_parameters().len() * 8)
                    as u64;
                let mut policy = limits();
                // Each source read and each output write has its existing debit.
                policy.library.max_cloned_bytes = Some((2 * count * width) as u64);
                policy.library.max_output_bytes = Some(retained_bytes);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!("packed matrix");
                };
                assert_eq!(
                    matrix.unsigned_buffer_capacity(),
                    Some(capacity),
                    "{kind}/{count}"
                );
                let actual = match matrix.elements() {
                    SequenceView::U8(v) => v.iter().map(|x| u64::from(*x)).collect::<Vec<_>>(),
                    SequenceView::U16(v) => v.iter().map(|x| u64::from(*x)).collect(),
                    SequenceView::U32(v) => v.iter().map(|x| u64::from(*x)).collect(),
                    SequenceView::U64(v) => v.to_vec(),
                    _ => panic!("exact unsigned storage kind"),
                };
                assert_eq!(actual, (1..=count as u64).collect::<Vec<_>>());
                equal(&artifact, &output, &reference(&artifact, &args));
                let prior = output.clone();
                let retained = memory.used_bytes();
                for dimension in [Dimension::ClonedBytes, Dimension::OutputBytes] {
                    let mut denied = policy;
                    let required = match dimension {
                        Dimension::ClonedBytes => (2 * count * width) as u64,
                        Dimension::OutputBytes => retained_bytes,
                        _ => unreachable!(),
                    };
                    match dimension {
                        Dimension::ClonedBytes => {
                            denied.library.max_cloned_bytes = Some(required - 1)
                        }
                        Dimension::OutputBytes => {
                            denied.library.max_output_bytes = Some(required - 1)
                        }
                        _ => unreachable!(),
                    }
                    let error = jit
                        .evaluate_into(&args, &mut output, denied, &memory)
                        .unwrap_err();
                    assert!(
                        matches!(error, NativeProgramExecutionError::Collection {
                        error: ResidentKernelError::ResourceLimit { dimension: actual, required: amount, limit, .. }, ..
                    } if actual == dimension && amount == required && limit == required - 1),
                        "{kind}/{count}: {error:?}"
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    assert!(output.iter().zip(&prior).all(|(value, old)| {
                        value
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                        .unwrap();
                    equal(&artifact, &output, &reference(&artifact, &args));
                    // The successful retry may replace the output allocation.
                    output.clone_from(&prior);
                    assert_eq!(memory.used_bytes(), retained);
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn completed_packed_collection_is_discarded_when_later_publication_fails() {
    let source =
        "+> sequence\nwords := [i | i <- input<[u8]>]\nsequence/length(words) / divisor<u64>\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for count in [3usize, 5] {
            for divisor in [1, 0, 1] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            ValueDataDraft::Matrix(
                                (1..=count).map(|i| ValueDataDraft::U8(i as u8)).collect(),
                            ),
                        ),
                        ("divisor", ValueDataDraft::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let retained = memory.used_bytes();
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                if divisor == 0 {
                    assert!(
                        matches!(
                            result,
                            Err(NativeProgramExecutionError::Control(
                                NativeIterationExecutionError::Arithmetic { .. }
                            ))
                        ),
                        "{result:?}"
                    );
                    assert_eq!(memory.used_bytes(), retained);
                    assert!(output.iter().zip(&prior).all(|(value, old)| {
                        value
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                } else {
                    result.unwrap();
                    equal(&artifact, &output, &reference(&artifact, &args));
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        ValueDataDraft::U64(count as u64)
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn generated_dynamic_ranges_feed_ordinary_comprehensions() {
    let (variants, _) = artifacts("[i + 1u64 | i <- 0u64..count<u64>]\n");
    for artifact in variants {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output: Vec<Option<Value>> = vec![None; artifact.outputs().len()];
        for count in [1u64, 0, 5] {
            let args = inputs(&artifact, &[("count", ValueDataDraft::U64(count))], &memory);
            if count == 0 {
                // Existing canonical ranges reject empty output. Keep that
                // error, prior publication and next-invocation recovery too.
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|v| v.as_ref().unwrap().clone())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                assert!(matches!(
                    jit.evaluate_into(&args, &mut output, limits(), &memory),
                    Err(NativeProgramExecutionError::Library { .. })
                ));
                assert_eq!(memory.used_bytes(), before);
                equal(&artifact, &output, &prior);
            } else {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn general_mech_window_function_compiles_without_a_window_helper() {
    // The inclusive candidate range remains nonempty even for n=0. Positive
    // width and the fit filter reject the final candidate before any slice.
    let source = format!(
        "{}windows(input<string>, width<u64>)\n",
        super::window_qualification::WINDOWS_FUNCTION
    );
    let (variants, _) = artifacts(&source);
    for artifact in &variants {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for (text, width, expected) in [
            ("ABBA", 2, vec!["AB", "BB", "BA"]),
            ("ABBA", 0, vec![]),
            ("ABBA", 5, vec![]),
            ("", 2, vec![]),
            ("é🧑‍🚀é", 2, vec!["é🧑‍🚀", "🧑‍🚀é"]),
        ] {
            let args = inputs(
                artifact,
                &[
                    ("input", ValueDataDraft::String(text.into())),
                    ("width", ValueDataDraft::U64(width)),
                ],
                &memory,
            );
            let ordinary = reference(artifact, &args);
            let ValueDataDraft::Matrix(values) =
                ordinary.last().unwrap().canonical_data_draft().unwrap()
            else {
                panic!("window output must be a String matrix");
            };
            let actual = values
                .iter()
                .map(|value| match value {
                    ValueDataDraft::String(text) => text.as_str(),
                    _ => panic!("window element must be a String"),
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
        assert_eq!(memory.used_bytes(), 0);
    }
    for (text, width) in [("ABBA", 2), ("ABBA", 0), ("ABBA", 5), ("", 2), ("é🧑‍🚀é", 2)] {
        check(
            &source,
            &[
                ("input", ValueDataDraft::String(text.into())),
                ("width", ValueDataDraft::U64(width)),
            ],
        );
    }
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

#[test]
fn collection_publication_counts_a_body_string_while_its_yield_draft_is_copied() {
    let source = "+> text\n+> number\n[text/join(words<[string]>, number/format(1u64 / divisor, 0u64)) | divisor <- divisors<[u64]>]\n";
    // The final zero stops after16 successful pushes, before finalization can
    // hide the push peak behind its larger construction envelope. The former
    // check admitted exactly this backing but omitted the live body String.
    const BYTES: usize = 256 * 1024;
    const YIELDS: usize = 16;
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let named = |word: String, divisors: &[u64]| {
            [
                (
                    "words",
                    ValueDataDraft::Matrix(vec![ValueDataDraft::String(word)].into_boxed_slice()),
                ),
                ("divisors", numbers(divisors)),
            ]
        };
        let small = inputs(&artifact, &named("x".into(), &[1]), &memory);
        let mut divisors = vec![1; YIELDS];
        divisors.push(0);
        let large = inputs(&artifact, &named("x".repeat(BYTES), &divisors), &memory);
        let mut policy = limits();
        policy.library.max_temporary_bytes =
            Some((YIELDS * (BYTES + core::mem::size_of::<ValueDataDraft>()) + 8192) as u64);
        policy.library.max_output_bytes = Some(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let result = jit.evaluate_into(&large, &mut output, policy, &memory);
        assert!(
            matches!(result, Err(NativeProgramExecutionError::Collection {
            error: ResidentKernelError::ResourceLimit { dimension: Dimension::TemporaryBytes, required, limit, .. }, ..
        }) if required > limit),
            "body String was omitted before yield cloning: {result:?}"
        );
        assert_eq!(memory.used_bytes(), before);
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&small, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &small));
        drop((output, prior, small, large));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn collection_publication_retained_nodes_match_the_canonical_matrix_footprint() {
    for (kind, item) in [
        ("bool", ValueDataDraft::Bool(true)),
        ("u64", ValueDataDraft::U64(7)),
        ("i64", ValueDataDraft::I64(-7)),
        (
            "f64",
            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(-0.0)),
        ),
        ("string", ValueDataDraft::String("é🧑‍🚀".into())),
    ] {
        let source = format!("[value | value <- values<[{kind}]>]\n");
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut output = vec![None; artifact.outputs().len()];
            for count in [1usize, 0, 3] {
                let args = inputs(
                    &artifact,
                    &[(
                        "values",
                        ValueDataDraft::Matrix(vec![item.clone(); count].into_boxed_slice()),
                    )],
                    &memory,
                );
                let expected = reference(&artifact, &args);
                let nodes = expected
                    .last()
                    .unwrap()
                    .retained_footprint(artifact.schemas())
                    .unwrap()
                    .node_count;
                assert_eq!(
                    nodes,
                    if kind == "string" {
                        count as u64 + 3
                    } else {
                        3
                    }
                );
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                let prior = output
                    .iter()
                    .map(|v| v.as_ref().unwrap().clone())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let mut policy = limits();
                policy.library.max_retained_nodes = Some(nodes - 1);
                let result = jit.evaluate_into(&args, &mut output, policy, &memory);
                assert!(
                    matches!(result, Err(NativeProgramExecutionError::Collection {
                    error: ResidentKernelError::ResourceLimit { dimension: Dimension::RetainedNodes, required, limit, .. }, ..
                }) if required == nodes && limit == nodes - 1),
                    "{kind}/{count}: {result:?}"
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
                policy.library.max_retained_nodes = Some(nodes);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                equal(&artifact, &output, &expected);
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
