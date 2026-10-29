#![cfg(feature = "canonical-native-program")]
//! Real generated mixed CFG execution; no scalar or resident fallback in JIT.
use mech_core::{
    ManagedMemoryBudget, ReactiveInstanceId, Value, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::{
    CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind,
    resident::{ActivationFacts, CapturedValueInput, activate},
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits, emit_program_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[path = "../../../src/core/tests/support/allocation_probe.rs"]
pub mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

#[path = "support/canonical_program_binding_pressure.rs"]
mod binding_pressure_qualification;

#[path = "support/canonical_program_argument_transport.rs"]
mod argument_transport_qualification;

#[path = "support/canonical_program_shared_transport.rs"]
mod shared_transport_qualification;

#[path = "support/canonical_program_uniform_words.rs"]
mod uniform_words_qualification;

#[path = "support/canonical_program_sequence_capacity.rs"]
mod sequence_capacity_qualification;

#[path = "support/canonical_program_buffer_frames.rs"]
mod buffer_frame_qualification;

#[path = "support/canonical_program_numeric_tail.rs"]
mod numeric_tail_qualification;

#[path = "support/canonical_program_numeric_writes.rs"]
mod numeric_write_qualification;

#[path = "support/canonical_program_numeric_results.rs"]
mod numeric_result_qualification;

#[path = "support/canonical_program_scalar_state.rs"]
mod scalar_state_qualification;

#[path = "support/canonical_program_record_buffers.rs"]
mod record_buffer_qualification;

#[path = "support/canonical_program_proof_storage.rs"]
mod proof_storage_qualification;

#[path = "support/canonical_program_optimal_path.rs"]
mod optimal_path_qualification;

#[path = "support/canonical_program_linked.rs"]
mod linked_qualification;

#[path = "support/canonical_program_comprehensions.rs"]
mod collection_qualification;

#[path = "support/canonical_program_windows.rs"]
mod window_qualification;

#[path = "support/canonical_program_ranges.rs"]
mod range_qualification;

#[path = "support/canonical_program_range_cursor.rs"]
mod range_cursor_qualification;

#[path = "support/canonical_program_text_join_sink.rs"]
mod text_join_sink_qualification;

#[path = "support/canonical_program_selected_join.rs"]
mod selected_join_qualification;

#[path = "support/canonical_program_text_line_matrix.rs"]
mod text_line_matrix_qualification;

#[path = "support/canonical_program_line_reads.rs"]
mod line_read_qualification;

#[path = "support/canonical_program_histogram_query.rs"]
mod histogram_query_qualification;

#[path = "support/canonical_program_window_round.rs"]
mod window_round_qualification;

#[path = "support/canonical_program_histogram_sink.rs"]
mod histogram_sink_qualification;

#[path = "support/canonical_program_string_map_entry.rs"]
mod string_map_entry_qualification;
#[path = "support/canonical_program_string_span.rs"]
mod string_span_qualification;

#[path = "support/canonical_program_access.rs"]
mod access_qualification;

#[path = "support/canonical_program_huffman.rs"]
mod huffman_qualification;

#[path = "support/canonical_program_level12_huffman.rs"]
mod level12_huffman_qualification;

#[path = "support/canonical_program_tally.rs"]
mod tally_qualification;

#[path = "support/canonical_program_costs.rs"]
mod costs_qualification;

#[path = "support/canonical_program_default_costs.rs"]
mod default_costs_qualification;

#[path = "support/canonical_program_header.rs"]
mod header_qualification;

#[path = "support/canonical_program_block_choice.rs"]
mod block_choice_qualification;
#[path = "support/canonical_program_block_header_writer.rs"]
mod block_header_writer_qualification;
#[path = "support/canonical_program_stored_writer.rs"]
mod stored_writer_qualification;

#[path = "support/canonical_program_literal_choice.rs"]
mod literal_choice_qualification;

#[path = "support/canonical_program_optimization.rs"]
mod optimization_qualification;

#[path = "support/canonical_program_path_writer.rs"]
mod path_writer_qualification;
#[path = "support/canonical_program_sequence_writer.rs"]
mod sequence_writer_qualification;
#[path = "support/canonical_program_writer_tables.rs"]
mod writer_tables_qualification;

#[path = "support/canonical_program_function_axes.rs"]
mod function_axes_qualification;

#[path = "support/canonical_program_word_reads.rs"]
mod word_read_qualification;

#[path = "support/canonical_program_record_words.rs"]
mod record_word_qualification;

#[path = "support/canonical_program_unsigned_lengths.rs"]
mod unsigned_length_qualification;

#[path = "support/canonical_program_narrow_words.rs"]
mod narrow_word_qualification;

#[path = "support/canonical_program_matchfinder.rs"]
mod matchfinder_qualification;

#[path = "support/canonical_program_function_broadcast.rs"]
mod function_broadcast_qualification;

#[path = "support/canonical_program_local_buffers.rs"]
mod local_buffer_qualification;

#[path = "support/canonical_program_dynamic_buffers.rs"]
mod dynamic_buffer_qualification;

#[path = "support/canonical_program_dynamic_projections.rs"]
mod dynamic_projection_qualification;

#[path = "support/canonical_program_packed_fields.rs"]
mod packed_field_qualification;

#[path = "support/canonical_program_packed_updates.rs"]
mod packed_update_qualification;

#[path = "support/canonical_program_nested_tail.rs"]
mod nested_tail_qualification;

#[path = "support/canonical_program_binary_tree.rs"]
mod binary_tree_qualification;

#[path = "support/canonical_program_collector.rs"]
mod collector_qualification;

#[path = "support/canonical_program_block_statistics.rs"]
mod block_statistics_qualification;

#[path = "support/canonical_program_full_block.rs"]
mod full_block_qualification;

#[path = "support/canonical_program_recursive_statements.rs"]
mod recursive_statement_qualification;

#[path = "support/canonical_program_lexical_invariants.rs"]
mod lexical_invariant_qualification;

#[path = "support/canonical_program_unsigned_conversion.rs"]
mod unsigned_conversion_qualification;

#[path = "support/canonical_program_nested_conversion.rs"]
mod nested_conversion_qualification;

#[path = "support/canonical_program_borrowed_strings.rs"]
mod borrowed_string_qualification;
#[path = "support/canonical_program_tail.rs"]
mod tail_qualification;
#[path = "support/canonical_program_text_length.rs"]
mod text_length_qualification;

#[path = "support/canonical_program_activation.rs"]
mod activation_qualification;

#[path = "support/canonical_program_retirement.rs"]
mod retirement_qualification;

const ROOT: &str = "+> text\n+> sequence\n+> number\n+> map\nwords := text/lines(input<string>)\njoined := text/join(words, \",\")\nsize := sequence/length(joined)\nformatted := number/format(size, 0u64)\ncounts := map/histogram(words)\nkeys := map/keys(counts)\nvalues := map/values(counts)\nmore := size + 1u64\n";
const LEXICAL: &str = "+> text\n+> sequence\n+> number\n+> map\nstep(x<string>) => (next<string>) :=\n  words := text/lines(x)\n  joined := text/join(words, \",\")\n  size := sequence/length(joined)\n  counts := map/histogram(words)\n  keys := map/keys(counts)\n  values := map/values(counts)\n  formatted := number/format(size + sequence/length(values), 0u64)\n  next := text/join(keys, formatted).\nseed<string> |2> step\n";
fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    artifacts_mode(source, false)
}
fn artifacts_mode(source: &str, interactive: bool) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let product = if interactive {
        CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    } else {
        CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
    }
    .unwrap();
    let last = product
        .document_outputs()
        .iter()
        .rev()
        .find(|out| out.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    ([original, decoded], last)
}
fn limits() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    // Explicit finite qualification policy, NOT a default-budget/performance claim.
    library.max_compute_work = Some(134_217_728);
    library.max_comparison_work = Some(1_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, ValueDataDraft)],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = named
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1
                .clone();
            let shape_values = match &data {
                ValueDataDraft::Matrix(_)
                    if artifact
                        .schemas()
                        .get(input.schema)
                        .unwrap()
                        .dimension_parameters()
                        .is_empty() =>
                {
                    Box::new([])
                }
                ValueDataDraft::Matrix(values) => mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
                _ => Box::new([]),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}
fn reference(artifact: &ProgramArtifact, args: &[Value]) -> Vec<Value> {
    reference_result(artifact, args).unwrap()
}
fn reference_result(artifact: &ProgramArtifact, args: &[Value]) -> Result<Vec<Value>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x50524f47, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| CapturedValueInput {
            slot: input.slot,
            value: &args[artifact
                .inputs()
                .iter()
                .position(|v| v.slot == input.artifact_slot)
                .unwrap()],
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .map_err(|error| format!("{error:?}"))?
        .publish()
        .map_err(|error| format!("{error:?}"))?;
    (0..artifact.outputs().len())
        .map(|index| {
            instance
                .copied_output(index)
                .map_err(|error| format!("{error:?}"))
        })
        .collect()
}
fn equal(artifact: &ProgramArtifact, left: &[Option<Value>], right: &[Value]) {
    assert_eq!(left.len(), right.len());
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        assert!(
            left.as_ref()
                .unwrap()
                .snapshot_eq(artifact.schemas(), right, artifact.schemas())
                .unwrap(),
            "output {index}: {left:?} != {right:?}"
        );
    }
}
fn check(source: &str, named: &[(&str, ValueDataDraft)]) {
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, named, &memory);
        let ordinary = reference(&artifact, &args);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
            .unwrap_or_else(|error| {
                panic!(
                    "native compilation failed for {source}: {error:?}\nnodes: {:#?}",
                    artifact.nodes()
                )
            });
        let mut output = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        assert!(report.visits > 0);
        equal(&artifact, &output, &ordinary);
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(!object.bytes().is_empty());
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        assert_eq!(object.identity().bytes().len(), 192);
    }
}
#[test]
fn generated_matrix_comprehensions_execute_each_native_body_and_filter() {
    for source in [
        "[value + 1u64 | value <- values<[u64]>]\n",
        "[value * 2u64 | value <- values<[u64]>, value > 1u64]\n",
        "+> text\n+> number\ntext/join([number/format(value + 1u64, 0u64) | value <- values<[u64]>], \",\")\n",
    ] {
        for values in [vec![1u64, 2, 3], vec![], vec![7]] {
            check(
                source,
                &[(
                    "values",
                    ValueDataDraft::Matrix(values.into_iter().map(ValueDataDraft::U64).collect()),
                )],
            );
        }
    }
}

#[test]
fn repeated_native_leaf_visits_reuse_only_binding_and_drop_it_with_the_invocation() {
    let mut costs = Vec::new();
    for count in [1, 4] {
        let source = format!(
            "step(s<string>) => (out<string>) := out := s + \"x\".\nseed<string> |{count}> step\n"
        );
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[("seed", ValueDataDraft::String("é".into()))],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(
                object.descriptor().typed_ir_sha256(),
                jit.descriptor().typed_ir_sha256()
            );
            let mut output = vec![None; artifact.outputs().len()];
            // Warm the prior output so denial/recovery comparisons have the
            // same caller-visible retirement obligations as the measured call.
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::String(format!("é{}", "x".repeat(count)))
            );
            let prior = output
                .iter()
                .map(|value| value.clone().unwrap())
                .collect::<Vec<_>>();
            let retained = memory.used_bytes();
            let mut denied = limits();
            denied.execution.max_work = report.work - 1;
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), retained);
            // Release the comparison's old publication before replacing it;
            // otherwise this test itself retains a second successful root.
            drop(prior);
            let recovered = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            assert_eq!(recovered.work, report.work);
            equal(&artifact, &output, &ordinary);
            assert_eq!(memory.used_bytes(), retained);
            costs.push((count, report.work));
            drop((args, output));
            // No cache survives a successful, denied, or recovered invocation.
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    for decoded in 0..2 {
        let once = costs[decoded].1;
        let repeated = costs[decoded + 2].1;
        assert!(
            repeated < once * 2,
            "immutable binding was rebuilt per visit: once={once}, repeated={repeated}"
        );
    }
}

#[test]
fn repeated_native_parameterized_leaf_visits_reuse_exact_shapes_and_recompute_values() {
    let mut costs = Vec::new();
    for count in [1, 4] {
        let source = format!(
            "+> sequence\nstep(x<u64>) => (next<u64>) :=\n  values := 1u64..4u64\n  next := x + sequence/length(values).\nseed<u64> |{count}> step\n"
        );
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &[("seed", ValueDataDraft::U64(5))], &memory);
            let ordinary = reference(&artifact, &args);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(
                jit.descriptor().typed_ir_sha256(),
                object.descriptor().typed_ir_sha256()
            );
            let mut output = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::U64(5 + 3 * count)
            );
            costs.push(report.work);
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    for decoded in 0..2 {
        assert!(
            costs[decoded + 2] < costs[decoded] * 2,
            "parameterized leaf binding rebuilt per visit: {:?}",
            costs
        );
    }
}

#[test]
fn native_unselected_leaf_does_not_bind_and_selected_denial_recovers_lazily() {
    let source = "pick(s<string>, take<bool>) => (out<string>) := out := (take? | true => s + \"x\" | false => s).\npick(input<string>, flag<bool>)\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let no = inputs(
            &artifact,
            &[
                ("input", ValueDataDraft::String("untouched".into())),
                ("flag", ValueDataDraft::Bool(false)),
            ],
            &memory,
        );
        let yes = inputs(
            &artifact,
            &[
                ("input", ValueDataDraft::String("untouched".into())),
                ("flag", ValueDataDraft::Bool(true)),
            ],
            &memory,
        );
        let ordinary = reference(&artifact, &no);
        let mut policy = limits();
        // Positive finite allowance permits invocation setup, but cannot fund
        // even lookup/binding of the actual selected concat call.
        policy.library.max_compute_work = Some(1);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&no, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &ordinary);
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let retained = memory.used_bytes();
        assert!(matches!(
            jit.evaluate_into(&yes, &mut output, policy, &memory),
            Err(
                mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                    error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                        mech_core::ResidentKernelError::ResourceLimit {
                            dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                            ..
                        }
                    ),
                    ..
                }
            )
        ));
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), retained);
        jit.evaluate_into(&no, &mut output, policy, &memory)
            .unwrap();
        equal(&artifact, &output, &ordinary);
        assert_eq!(memory.used_bytes(), retained);
        drop((no, yes, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn generated_root_calls_execute_all_seven_families_and_scalar_add() {
    check(
        ROOT,
        &[("input", ValueDataDraft::String("a\nbb\na".into()))],
    );
}

#[test]
fn generated_dynamic_matrix_format_uses_live_shape_in_root_and_match_calls() {
    for columns in [3, 0, 1] {
        let values = ValueDataDraft::Matrix(
            (1..=columns)
                .map(ValueDataDraft::I64)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        check(
            "+> number\nnumber/format(values<[i64]>, 0u64)\n",
            &[("values", values.clone())],
        );
        for flag in [false, true] {
            check(
                "+> number\nresult := (choose<bool>? | true => number/format(values<[i64]>, 0u64) | false => number/format(values, 0u64))\nresult\n",
                &[
                    ("values", values.clone()),
                    ("choose", ValueDataDraft::Bool(flag)),
                ],
            );
        }
    }
}
#[test]
fn generated_lexical_calls_execute_all_seven_families_with_live_carriers() {
    check(
        LEXICAL,
        &[("seed", ValueDataDraft::String("a\nbb\na".into()))],
    );
}
#[test]
fn generated_managed_loops_and_match_choose_both_arms_at_zero_and_nonzero_counts() {
    for count in [0, 1, 3] {
        for choose in [false, true] {
            let source = format!(
                "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := (choose? | true => number/format(sequence/length(x), 0u64) | false => x).\nchoose := flag<bool>\nseed<string> |{count}> step\n"
            );
            check(
                &source,
                &[
                    ("flag", ValueDataDraft::Bool(choose)),
                    ("seed", ValueDataDraft::String("abc".into())),
                ],
            );
        }
    }
}
#[test]
fn generated_denials_preserve_every_prior_output_and_allow_recovery() {
    let (artifacts, _) = artifacts(ROOT);
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("input", ValueDataDraft::String("a\nbb\na".into()))],
            &memory,
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        // Public immutable constants can be shared into later foreign budgets
        // before native admission. Keep these imports alive through invocation
        // and recovery; the raw-constant import must use the bounded path.
        let _foreign_constants = jit
            .descriptor()
            .plan()
            .constants()
            .iter()
            .flat_map(|constant| {
                (0..16).map(move |_| {
                    let account = ManagedMemoryBudget::new(1_000_000);
                    let raw = constant.constant();
                    let mut capacity = account
                        .reserve_capacity(raw.memory_budget_admission_bytes(&account).unwrap())
                        .unwrap();
                    let imported = raw.clone().into_memory_budget(&mut capacity).unwrap();
                    (account, imported)
                })
            })
            .collect::<Vec<_>>();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let retained = memory.used_bytes();
        for policy in [
            NativeProgramExecutionLimits {
                execution: NativeExecutionLimits {
                    max_work: 0,
                    ..limits().execution
                },
                ..limits()
            },
            NativeProgramExecutionLimits {
                execution: NativeExecutionLimits {
                    max_transport_bytes: 0,
                    ..limits().execution
                },
                ..limits()
            },
            NativeProgramExecutionLimits {
                library: mech_core::MemoryBudgetLimits {
                    max_compute_work: Some(1),
                    ..limits().library
                },
                ..limits()
            },
            NativeProgramExecutionLimits {
                library: mech_core::MemoryBudgetLimits {
                    max_temporary_bytes: Some(0),
                    ..limits().library
                },
                ..limits()
            },
            NativeProgramExecutionLimits {
                library: mech_core::MemoryBudgetLimits {
                    max_comparison_work: None,
                    ..limits().library
                },
                ..limits()
            },
        ] {
            assert!(
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .is_err()
            );
            equal(&artifact, &output, &prior);
            assert_eq!(memory.used_bytes(), retained);
        }
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}
#[test]
fn generated_program_rejects_foreign_input_ownership_without_publication() {
    let (artifacts, _) = artifacts(ROOT);
    for artifact in artifacts {
        let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("input", ValueDataDraft::String("a\na".into()))],
            &foreign,
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        assert!(
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .is_err()
        );
        assert!(output.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn generated_nested_managed_carriers_rebind_scalar_captures_each_visit() {
    let source = "+> sequence\n+> number\nstep(x<string>) => (next<string>) := next := number/format(sequence/length(x) + delta, 0u64).\nouter(x<string>) => (next<string>) := next := x |3> step.\ndelta := amount<u64> + 0u64\nseed<string> |2> outer\n";
    for amount in [0, 7] {
        check(
            source,
            &[
                ("amount", ValueDataDraft::U64(amount)),
                ("seed", ValueDataDraft::String("abcd".into())),
            ],
        );
    }
}

#[test]
fn generated_arithmetic_denial_after_a_library_call_keeps_prior_typed_roots() {
    let source = "+> sequence\nsize := sequence/length(input<string>)\nsize / divisor<u64>\n";
    let (artifacts, _) = artifacts(source);
    for artifact in artifacts {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("input", ValueDataDraft::String("abcd".into())),
                ("divisor", ValueDataDraft::U64(2)),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        let prior = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let denied = inputs(
            &artifact,
            &[
                ("input", ValueDataDraft::String("changed".into())),
                ("divisor", ValueDataDraft::U64(0)),
            ],
            &memory,
        );
        assert!(matches!(
            jit.evaluate_into(&denied, &mut output, limits(), &memory),
            Err(
                mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                    mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                )
            )
        ));
        equal(&artifact, &output, &prior);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &prior);
    }
}
#[path = "support/canonical_program_closed_tail.rs"]
mod closed_tail_qualification;
#[path = "support/canonical_program_string_matrix.rs"]
mod string_matrix_qualification;

#[path = "support/canonical_program_selected_matrix.rs"]
mod selected_matrix_qualification;

#[path = "support/canonical_program_flush_block.rs"]
mod flush_block_qualification;

#[path = "support/canonical_program_selection.rs"]
mod selection_qualification;

#[path = "support/canonical_program_selected_record_axes.rs"]
mod selected_record_axes_qualification;

#[path = "support/canonical_program_compress.rs"]
mod compress_qualification;

#[path = "support/canonical_program_no_limits.rs"]
mod no_limits_qualification;
