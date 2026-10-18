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

const ROOT: &str = "+> text\n+> sequence\n+> number\n+> map\nwords := text/lines(input<string>)\njoined := text/join(words, \",\")\nsize := sequence/length(joined)\nformatted := number/format(size, 0u64)\ncounts := map/histogram(words)\nkeys := map/keys(counts)\nvalues := map/values(counts)\nmore := size + 1u64\n";
const LEXICAL: &str = "+> text\n+> sequence\n+> number\n+> map\nstep(x<string>) => (next<string>) :=\n  words := text/lines(x)\n  joined := text/join(words, \",\")\n  size := sequence/length(joined)\n  counts := map/histogram(words)\n  keys := map/keys(counts)\n  values := map/values(counts)\n  formatted := number/format(size + sequence/length(values), 0u64)\n  next := text/join(keys, formatted).\nseed<string> |2> step\n";
fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f47), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
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
    let mut instance = activate(
        ReactiveInstanceId::new(0x50524f47, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
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
        .unwrap()
        .publish()
        .unwrap();
    (0..artifact.outputs().len())
        .map(|index| instance.copied_output(index).unwrap())
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
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
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
