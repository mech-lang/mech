//! Signed scalar canonical artifact semantics through the shared native emitter.
#![cfg(feature = "canonical-native")]
use mech_core::{
    ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::native_artifact::{
    NativeScalarKind, NativeScalarValue, lower_pure_native_artifact,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_gpu::native_artifact::{
    JitArtifact, NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, emit_object,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
fn compile(source: &str) -> (ProgramArtifact, usize) {
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
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
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
    let (original, output) = compile(source);
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

fn successful(source: &str, left: i64, right: i64, expected: NativeScalarValue) {
    let (artifacts, last) = artifacts(source);
    for artifact in artifacts {
        let args = inputs(
            &artifact,
            &[
                ("left", NativeScalarValue::I64(left)),
                ("right", NativeScalarValue::I64(right)),
            ],
        );
        let resident = reference(&artifact, &args).unwrap();
        assert_eq!(resident[last], expected);
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitArtifact::compile(&plan).unwrap();
        let mut output = vec![NativeScalarValue::U64(0xfeed); plan.outputs().len()];
        jit.evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output, resident);
    }
}
#[test]
fn all_signed_scalar_operations_match_resident_original_and_decoded() {
    use NativeScalarValue::{Bool, I64};
    for (operator, left, right, expected) in [
        ("+", i64::MIN, 0, i64::MIN),
        ("+", i64::MAX, 0, i64::MAX),
        ("+", -(1i64 << 53) - 1, 7, -(1i64 << 53) + 6),
        ("-", i64::MIN, -1, i64::MIN + 1),
        ("-", 7, 10, -3),
        ("*", -(1i64 << 53) - 1, 2, -(1i64 << 54) - 2),
        ("/", -7, 3, -2),
        ("/", 7, -3, -2),
        ("/", i64::MIN, 1, i64::MIN),
        ("%", -7, 3, -1),
        ("%", 7, -3, 1),
        ("%", i64::MIN, 3, i64::MIN % 3),
    ] {
        successful(
            &format!("left<i64> {operator} right<i64>\n"),
            left,
            right,
            I64(expected),
        );
    }
    for (left, right, expected) in [
        (i64::MIN, i64::MAX, [false, true, true, true, false, false]),
        (i64::MAX, i64::MIN, [false, true, false, false, true, true]),
        (i64::MIN, i64::MIN, [true, false, false, true, false, true]),
    ] {
        for (operator, expected) in ["==", "!=", "<", "<=", ">", ">="].into_iter().zip(expected) {
            successful(
                &format!("left<i64> {operator} right<i64>\n"),
                left,
                right,
                Bool(expected),
            );
        }
    }
    successful("-left<i64>\n", 7, 0, I64(-7));
    successful("-left<i64>\n", i64::MIN + 1, 0, I64(i64::MAX));
}
#[test]
fn signed_overflow_zero_division_and_minimum_remainder_fail_before_publication() {
    for (operator, left, right) in [
        ("+", i64::MAX, 1),
        ("+", i64::MIN, -1),
        ("-", i64::MIN, 1),
        ("*", i64::MAX, 2),
        ("*", i64::MIN, -1),
        ("/", 1, 0),
        ("/", i64::MIN, -1),
        ("%", 1, 0),
        ("%", i64::MIN, -1),
        ("neg", i64::MIN, 0),
    ] {
        let source = if operator == "neg" {
            "-left<i64>\n".into()
        } else {
            format!("left<i64> {operator} right<i64>\n")
        };
        let (artifacts, _) = artifacts(&source);
        for artifact in artifacts {
            let args = inputs(
                &artifact,
                &[
                    ("left", NativeScalarValue::I64(left)),
                    ("right", NativeScalarValue::I64(right)),
                ],
            );
            assert!(
                reference(&artifact, &args)
                    .unwrap_err()
                    .contains("Arithmetic")
            );
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            let mut output = vec![NativeScalarValue::I64(17); plan.outputs().len()];
            let before = output.clone();
            assert!(
                matches!(jit.evaluate_into(&args,&mut output,NativeExecutionLimits::default(),None),Err(NativeExecutionError::Arithmetic{node}) if node==plan.lets()[0].node())
            );
            assert_eq!(output, before);
            let recovery = inputs(
                &artifact,
                &[
                    ("left", NativeScalarValue::I64(7)),
                    ("right", NativeScalarValue::I64(2)),
                ],
            );
            jit.evaluate_into(
                &recovery,
                &mut output,
                NativeExecutionLimits::default(),
                None,
            )
            .unwrap();
            assert_eq!(output, reference(&artifact, &recovery).unwrap());
        }
    }
}
#[test]
fn signed_denials_preserve_type_transport_and_first_error_order() {
    let (artifact, _) = compile("unused := left<i64> / divisor<i64>\n-left<i64>\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    assert_eq!(plan.lets().len(), 2);
    let jit = JitArtifact::compile(&plan).unwrap();
    let mut output = vec![NativeScalarValue::I64(55); plan.outputs().len()];
    let before = output.clone();
    let limits = NativeExecutionLimits {
        max_work: 400,
        ..NativeExecutionLimits::default()
    };
    let bad_first = inputs(
        &artifact,
        &[
            ("left", NativeScalarValue::I64(i64::MIN)),
            ("divisor", NativeScalarValue::I64(0)),
        ],
    );
    assert!(
        matches!(jit.evaluate_into(&bad_first,&mut output,limits,None),Err(NativeExecutionError::Arithmetic{node}) if node==plan.lets()[0].node())
    );
    assert_eq!(output, before);
    let bad_later = inputs(
        &artifact,
        &[
            ("left", NativeScalarValue::I64(i64::MIN)),
            ("divisor", NativeScalarValue::I64(1)),
        ],
    );
    assert!(
        matches!(jit.evaluate_into(&bad_later,&mut output,limits,None),Err(NativeExecutionError::WorkLimit{phase:NativeExecutionPhase::Node(node),..}) if node==plan.lets()[1].node())
    );
    assert_eq!(output, before);
    let mut wrong = bad_later.clone();
    wrong[0] = NativeScalarValue::U64(0);
    assert!(matches!(
        jit.evaluate_into(&wrong, &mut output, NativeExecutionLimits::default(), None),
        Err(NativeExecutionError::InputType {
            expected: NativeScalarKind::I64,
            ..
        })
    ));
    assert_eq!(output, before);
    assert!(matches!(
        jit.evaluate_into(
            &wrong,
            &mut output,
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
    assert_eq!(output, before);
}
#[test]
fn signed_object_identity_is_typed_and_emission_is_not_execution() {
    let (artifact, _) = compile("left<i64> + right<i64>\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let object = emit_object(&plan).unwrap();
    assert!(!object.bytes().is_empty());
    assert!(
        object
            .descriptor()
            .inputs()
            .iter()
            .all(|value| value.kind() == NativeScalarKind::I64)
    );
    assert_eq!(object.metadata()["executed"], false);
    assert_eq!(object.metadata()["linked"], false);
    assert_eq!(object.metadata()["timing_samples"], 0);
}
