#![cfg(feature = "canonical-native")]
//! Canonical JIT tests; the Object case is emission metadata, not linked execution.
use mech_core::{
    ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::native_artifact::{NativeScalarValue, lower_pure_native_artifact};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitArtifact, NativeCompileLimits, NativeExecutionError, NativeExecutionLimits,
    NativeExecutionPhase, emit_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    compile_mode(source, false)
}

fn compile_mode(source: &str, interactive: bool) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434e4154), Revision(1), source).unwrap(),
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
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|v| v.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (product.compile_artifact().unwrap(), output)
}
fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    artifacts_mode(source, false)
}

fn artifacts_mode(source: &str, interactive: bool) -> ([ProgramArtifact; 2], usize) {
    let (original, output) = compile_mode(source, interactive);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    ([original, decoded], output)
}
fn inputs(
    artifact: &ProgramArtifact,
    named: &[(&str, NativeScalarValue)],
) -> Vec<NativeScalarValue> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            named
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap_or_else(|| panic!("missing input {name}"))
                .1
        })
        .collect()
}
fn reference(
    artifact: &ProgramArtifact,
    args: &[NativeScalarValue],
) -> Result<Vec<NativeScalarValue>, String> {
    let mut instance = activate(
        ReactiveInstanceId::new(0x434e4154, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let values = artifact
        .inputs()
        .iter()
        .zip(args)
        .map(|(input, value)| {
            ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data: match value {
                    NativeScalarValue::Bool(v) => ValueDataDraft::Bool(*v),
                    NativeScalarValue::U64(v) => ValueDataDraft::U64(*v),
                    NativeScalarValue::I64(v) => ValueDataDraft::I64(*v),
                    NativeScalarValue::F64(v) => ValueDataDraft::F64(*v),
                },
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect::<Vec<_>>();
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|v| v.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &values[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map_err(|e| format!("{e:?}"))?;
    Ok((0..artifact.outputs().len())
        .map(
            |index| match instance.copied_output(index).unwrap().data() {
                ValueData::Bool(v) => NativeScalarValue::Bool(*v),
                ValueData::U64(v) => NativeScalarValue::U64(*v),
                ValueData::I64(v) => NativeScalarValue::I64(*v),
                ValueData::F64(v) => NativeScalarValue::F64(*v),
                v => panic!("unexpected scalar {v:?}"),
            },
        )
        .collect())
}
fn check(source: &str, named: &[(&str, NativeScalarValue)], expected: NativeScalarValue) {
    let (artifacts, last) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(&artifact, named);
        let resident = reference(&artifact, &args).unwrap();
        assert_eq!(resident[last], expected);
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitArtifact::compile(&plan).unwrap();
        let mut output = vec![NativeScalarValue::U64(0xdead); plan.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output, resident);
        assert_eq!(report.completed_nodes, artifact.nodes().len());
        assert_eq!(jit.descriptor().revision(), artifact.revision());
    }
}
#[test]
fn all_certified_scalar_primitives_match_ordinary_resident_original_and_decoded() {
    use NativeScalarValue::{Bool, U64};
    for (op, a, b, result) in [
        ("+", (1u64 << 53) + 1, 6, (1u64 << 53) + 7),
        ("+", u64::MAX - 1, 1, u64::MAX),
        ("-", u64::MAX, 1, u64::MAX - 1),
        ("*", (1u64 << 53) + 1, 2, (1u64 << 54) + 2),
        ("/", u64::MAX, 3, u64::MAX / 3),
        ("%", u64::MAX, 7, u64::MAX % 7),
    ] {
        check(
            &format!("left<u64> {op} right<u64>\n"),
            &[("left", U64(a)), ("right", U64(b))],
            U64(result),
        );
    }
    for (op, result) in [
        ("==", false),
        ("!=", true),
        ("<", false),
        ("<=", false),
        (">", true),
        (">=", true),
    ] {
        check(
            &format!("left<u64> {op} right<u64>\n"),
            &[("left", U64(u64::MAX)), ("right", U64((1u64 << 53) + 1))],
            Bool(result),
        );
    }
    for (op, result) in [("&&", false), ("||", true), ("^^", true)] {
        check(
            &format!("left<bool> {op} right<bool>\n"),
            &[("left", Bool(true)), ("right", Bool(false))],
            Bool(result),
        );
    }
    check("!left<bool>\n", &[("left", Bool(false))], Bool(true));
    check(
        "left<bool> == right<bool>\n",
        &[("left", Bool(true)), ("right", Bool(true))],
        Bool(true),
    );
    check(
        "left<bool> != right<bool>\n",
        &[("left", Bool(false)), ("right", Bool(true))],
        Bool(true),
    );
}
#[test]
fn direct_constants_and_live_identity_need_no_computed_activation_or_local() {
    check(
        "true\n18446744073709551615u64\nleft<u64>\n",
        &[("left", NativeScalarValue::U64(123))],
        NativeScalarValue::U64(123),
    );
    let (artifacts, _) = artifacts_mode(
        "first := true\nsecond := 18446744073709551615u64\nsecond\n",
        true,
    );
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(plan.lets().is_empty());
        assert!(plan.outputs().len() >= 3);
        assert!(plan.outputs().iter().any(|output| {
            output
                .interactive_binding()
                .is_some_and(|binding| binding.lexical_name == "first")
        }));
        assert!(plan.outputs().iter().any(|output| {
            output
                .interactive_binding()
                .is_some_and(|binding| binding.lexical_name == "second")
        }));
        let jit = JitArtifact::compile(&plan).unwrap();
        let mut outputs = vec![NativeScalarValue::Bool(false); plan.outputs().len()];
        let report = jit
            .evaluate_into(&[], &mut outputs, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(report.completed_nodes, 0);
        assert_eq!(outputs, reference(&artifact, &[]).unwrap());
        assert!(outputs.contains(&NativeScalarValue::Bool(true)));
        assert!(outputs.contains(&NativeScalarValue::U64(u64::MAX)));
    }
}

#[test]
fn checked_u64_failure_matches_resident_and_never_publishes_partial_outputs() {
    use NativeScalarValue::U64;
    for (op, a, b) in [
        ("+", u64::MAX, 1),
        ("-", 0, 1),
        ("*", u64::MAX, 2),
        ("/", 7, 0),
        ("%", 7, 0),
    ] {
        let (artifacts, _) = artifacts(&format!("left<u64> {op} right<u64>\n"));
        for artifact in artifacts {
            let args = inputs(&artifact, &[("left", U64(a)), ("right", U64(b))]);
            let resident_error = reference(&artifact, &args).unwrap_err();
            assert!(resident_error.contains("Arithmetic"), "{resident_error}");
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            let mut output = vec![U64(0xcafe); plan.outputs().len()];
            let before = output.clone();
            assert!(
                matches!(jit.evaluate_into(&args,&mut output,NativeExecutionLimits::default(),None),Err(NativeExecutionError::Arithmetic{node}) if node==plan.lets()[0].node())
            );
            assert_eq!(output, before);
        }
    }
}
#[test]
fn shared_local_single_evaluation_and_all_declared_outputs_remain_ordered() {
    check(
        "shared := left<u64> + 1u64\nshared + shared\n",
        &[("left", NativeScalarValue::U64(19))],
        NativeScalarValue::U64(40),
    );
    let (artifacts, _) = artifacts_mode("shared := left<u64> + 1u64\nshared + shared\n", true);
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), 2);
        assert!(plan.outputs().len() >= 2);
        let jit = JitArtifact::compile(&plan).unwrap();
        let mut output = vec![NativeScalarValue::U64(0); plan.outputs().len()];
        let arguments = [NativeScalarValue::U64(19)];
        let report = jit
            .evaluate_into(
                &arguments,
                &mut output,
                NativeExecutionLimits::default(),
                None,
            )
            .unwrap();
        assert_eq!(report.completed_nodes, 2);
        assert_eq!(output, reference(&artifact, &arguments).unwrap());
    }
}
#[test]
fn first_arithmetic_error_precedes_later_work_denial_and_denial_skips_later_bad_code() {
    let (artifact, _) = compile("unused := left<u64> / divisor<u64>\nleft<u64> + 1u64\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    assert_eq!(plan.lets().len(), 2);
    let jit = JitArtifact::compile(&plan).unwrap();
    let limits = NativeExecutionLimits {
        max_work: 400,
        ..NativeExecutionLimits::default()
    };
    let mut output = vec![NativeScalarValue::U64(99); plan.outputs().len()];
    let before = output.clone();
    let args = inputs(
        &artifact,
        &[
            ("left", NativeScalarValue::U64(3)),
            ("divisor", NativeScalarValue::U64(0)),
        ],
    );
    assert!(
        matches!(jit.evaluate_into(&args,&mut output,limits,None),Err(NativeExecutionError::Arithmetic{node}) if node==plan.lets()[0].node())
    );
    assert_eq!(output, before);
    let args = inputs(
        &artifact,
        &[
            ("left", NativeScalarValue::U64(u64::MAX)),
            ("divisor", NativeScalarValue::U64(1)),
        ],
    );
    assert!(
        matches!(jit.evaluate_into(&args,&mut output,limits,None),Err(NativeExecutionError::WorkLimit{phase:NativeExecutionPhase::Node(node),..}) if node==plan.lets()[1].node())
    );
    assert_eq!(output, before);
}
#[test]
fn finite_denials_and_completion_failure_are_atomic_and_recover() {
    let (artifact, _) = compile("left<u64> + 1u64\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitArtifact::compile(&plan).unwrap();
    let args = [NativeScalarValue::U64(4)];
    let mut output = vec![NativeScalarValue::U64(99); plan.outputs().len()];
    let before = output.clone();
    for limits in [
        NativeExecutionLimits {
            max_work: 0,
            ..NativeExecutionLimits::default()
        },
        NativeExecutionLimits {
            max_transport_bytes: 0,
            ..NativeExecutionLimits::default()
        },
        NativeExecutionLimits {
            max_native_stack_bytes: 0,
            ..NativeExecutionLimits::default()
        },
    ] {
        assert!(jit.evaluate_into(&args, &mut output, limits, None).is_err());
        assert_eq!(output, before);
    }
    assert!(matches!(
        jit.evaluate_into(&[], &mut output, NativeExecutionLimits::default(), None),
        Err(NativeExecutionError::InvalidInputs { .. })
    ));
    assert!(matches!(
        jit.evaluate_into(
            &[NativeScalarValue::Bool(true)],
            &mut output,
            NativeExecutionLimits::default(),
            None
        ),
        Err(NativeExecutionError::InputType { .. })
    ));
    assert!(matches!(
        jit.evaluate_into(&args, &mut [], NativeExecutionLimits::default(), None),
        Err(NativeExecutionError::InvalidOutputs { .. })
    ));
    let denied = mech_core::ManagedMemoryBudget::new(0);
    assert!(matches!(
        jit.evaluate_into(
            &args,
            &mut output,
            NativeExecutionLimits::default(),
            Some(&denied)
        ),
        Err(NativeExecutionError::Memory { .. })
    ));
    assert_eq!(denied.used_bytes(), 0);
    assert_eq!(output, before);
    let budget = mech_core::ManagedMemoryBudget::new(1024 * 1024);
    let report = jit
        .evaluate_into(
            &args,
            &mut output,
            NativeExecutionLimits::default(),
            Some(&budget),
        )
        .unwrap();
    assert_eq!(budget.used_bytes(), 0);
    output.fill(NativeScalarValue::U64(99));
    let before = output.clone();
    assert!(matches!(
        jit.evaluate_into(
            &args,
            &mut output,
            NativeExecutionLimits {
                max_work: report.work - 1,
                ..NativeExecutionLimits::default()
            },
            Some(&budget)
        ),
        Err(NativeExecutionError::WorkLimit {
            phase: NativeExecutionPhase::Completion,
            ..
        })
    ));
    assert_eq!(output, before);
    assert_eq!(budget.used_bytes(), 0);
    for _ in 0..3 {
        jit.evaluate_into(
            &args,
            &mut output,
            NativeExecutionLimits::default(),
            Some(&budget),
        )
        .unwrap();
        assert_eq!(output.last(), Some(&NativeScalarValue::U64(5)));
        assert_eq!(budget.used_bytes(), 0);
    }
}
#[test]
fn setup_and_memory_admission_precede_even_input_type_traversal() {
    let (artifact, _) = compile("left<u64> + 1u64\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let jit = JitArtifact::compile(&plan).unwrap();
    let wrong_type = [NativeScalarValue::Bool(true)];
    let mut outputs = vec![NativeScalarValue::U64(99); plan.outputs().len()];
    let before = outputs.clone();
    assert!(matches!(
        jit.evaluate_into(
            &wrong_type,
            &mut outputs,
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            None
        ),
        Err(NativeExecutionError::WorkLimit {
            phase: NativeExecutionPhase::Setup,
            ..
        })
    ));
    assert!(matches!(
        jit.evaluate_into(
            &wrong_type,
            &mut outputs,
            NativeExecutionLimits {
                max_transport_bytes: 0,
                ..NativeExecutionLimits::default()
            },
            None
        ),
        Err(NativeExecutionError::TransportLimit { .. })
    ));
    let memory = mech_core::ManagedMemoryBudget::new(0);
    assert!(matches!(
        jit.evaluate_into(
            &wrong_type,
            &mut outputs,
            NativeExecutionLimits::default(),
            Some(&memory)
        ),
        Err(NativeExecutionError::Memory { .. })
    ));
    assert_eq!(memory.used_bytes(), 0);
    assert_eq!(outputs, before);
    // With admission funded, the same argument reaches the actual type check.
    assert!(matches!(
        jit.evaluate_into(
            &wrong_type,
            &mut outputs,
            NativeExecutionLimits::default(),
            None
        ),
        Err(NativeExecutionError::InputType { .. })
    ));
    assert_eq!(outputs, before);
}

#[test]
fn object_emission_and_same_definition_metadata_are_not_execution_evidence() {
    let (artifacts, _) = artifacts("left<u64> * 3u64\n");
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitArtifact::compile(&plan).unwrap();
        let object = emit_object(&plan).unwrap();
        assert!(!object.bytes().is_empty());
        assert_eq!(object.descriptor().revision(), jit.descriptor().revision());
        assert_eq!(object.descriptor().lets(), jit.descriptor().lets());
        assert_eq!(
            object.descriptor().stack_bytes(),
            jit.descriptor().stack_bytes()
        );
        assert_eq!(object.metadata()["linked"], false);
        assert_eq!(object.metadata()["executed"], false);
        assert_eq!(object.metadata()["timing_samples"], 0);
        assert!(
            JitArtifact::compile_with_limits(
                &plan,
                NativeCompileLimits {
                    max_stack_bytes: 0,
                    ..NativeCompileLimits::default()
                }
            )
            .is_err()
        );
    }
}
