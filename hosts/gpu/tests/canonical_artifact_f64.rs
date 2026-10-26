//! Bitwise F64 qualification of the actual dense canonical scalar semantics.
#![cfg(feature = "canonical-native")]
use mech_core::{
    ReactiveInstanceId, ValueData, ValueDataDraft,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
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
fn source_artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
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

#[path = "support/canonical_f64_cases.rs"]
mod cases;
fn fp(bits: u64) -> NativeScalarValue {
    NativeScalarValue::F64(F64Bits::from_bits(bits))
}
fn evaluate(jit: &JitArtifact, arguments: &[NativeScalarValue]) -> Vec<NativeScalarValue> {
    let mut output = vec![NativeScalarValue::U64(0xfeed); jit.descriptor().outputs().len()];
    jit.evaluate_into(
        arguments,
        &mut output,
        NativeExecutionLimits::default(),
        None,
    )
    .unwrap();
    output
}

#[test]
fn dense_f64_arithmetic_and_platform_sqrt_match_resident_bits() {
    for operator in ["+", "-", "*", "/"] {
        let (artifacts, _) = source_artifacts(&format!("left<f64> {operator} right<f64>\n"));
        for artifact in artifacts {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            for &(label, left, right) in cases::PAIRS {
                let arguments = inputs(&artifact, &[("left", fp(left)), ("right", fp(right))]);
                assert_eq!(
                    evaluate(&jit, &arguments),
                    reference(&artifact, &arguments).unwrap(),
                    "{operator}/{label}: left={left:016x} right={right:016x}"
                );
            }
        }
    }
    for (source, negate) in [
        ("-left<f64>\n", true),
        ("+> math\nmath/sqrt(left<f64>)\n", false),
    ] {
        let (artifacts, result) = source_artifacts(source);
        for artifact in artifacts {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            for &bits in cases::UNARY_BITS {
                let arguments = inputs(&artifact, &[("left", fp(bits))]);
                let expected = reference(&artifact, &arguments).unwrap();
                if negate {
                    assert_eq!(expected[result], fp(bits ^ (1u64 << 63)));
                }
                assert_eq!(
                    evaluate(&jit, &arguments),
                    expected,
                    "{source}: input={bits:016x}"
                );
            }
        }
    }
}

#[test]
fn f64_comparisons_are_numeric_for_nan_and_signed_zero() {
    for operator in ["==", "!=", "<", "<=", ">", ">="] {
        let (artifacts, result) = source_artifacts(&format!("left<f64> {operator} right<f64>\n"));
        for artifact in artifacts {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            for &(label, left, right) in cases::PAIRS {
                let arguments = inputs(&artifact, &[("left", fp(left)), ("right", fp(right))]);
                let expected = reference(&artifact, &arguments).unwrap();
                let (left, right) = (f64::from_bits(left), f64::from_bits(right));
                let boolean = match operator {
                    "==" => left == right,
                    "!=" => left != right,
                    "<" => left < right,
                    "<=" => left <= right,
                    ">" => left > right,
                    _ => left >= right,
                };
                assert_eq!(expected[result], NativeScalarValue::Bool(boolean));
                assert_eq!(evaluate(&jit, &arguments), expected, "{operator}/{label}");
            }
        }
    }
}

#[test]
fn f64_ordered_nodes_are_not_fused_reassociated_or_dropped() {
    for (source, left, right, adjust, expected_bits) in [
        (
            "product := left<f64> * right<f64>\nproduct + adjust<f64>\n",
            0x3ff0000002000000,
            0x3feffffffc000000,
            (-1.0f64).to_bits(),
            0u64,
        ),
        (
            "sum := left<f64> + right<f64>\nsum + adjust<f64>\n",
            (1e16f64).to_bits(),
            (-1e16f64).to_bits(),
            1.0f64.to_bits(),
            1.0f64.to_bits(),
        ),
    ] {
        let (artifacts, result) = source_artifacts(source);
        for artifact in artifacts {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), 2);
            let jit = JitArtifact::compile(&plan).unwrap();
            let arguments = inputs(
                &artifact,
                &[
                    ("left", fp(left)),
                    ("right", fp(right)),
                    ("adjust", fp(adjust)),
                ],
            );
            let expected = reference(&artifact, &arguments).unwrap();
            assert_eq!(expected[result], fp(expected_bits));
            assert_eq!(evaluate(&jit, &arguments), expected);
        }
    }
    let (artifacts, _) = source_artifacts("unused := value<f64> / 0.0\n-value<f64>\n");
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), 2);
        let jit = JitArtifact::compile(&plan).unwrap();
        let arguments = inputs(&artifact, &[("value", fp(1.0f64.to_bits()))]);
        assert_eq!(
            evaluate(&jit, &arguments),
            reference(&artifact, &arguments).unwrap()
        );
        let mut output = vec![fp(0x8000000000000000); plan.outputs().len()];
        let before = output.clone();
        assert!(
            matches!(jit.evaluate_into(&arguments, &mut output, NativeExecutionLimits { max_work: 400, ..NativeExecutionLimits::default() }, None), Err(NativeExecutionError::WorkLimit { phase: NativeExecutionPhase::Node(node), .. }) if node == plan.lets()[1].node())
        );
        assert_eq!(output, before);
    }
    let (artifacts, _) =
        source_artifacts("+> math\nunused := integer<u64> / 0u64\nmath/sqrt(value<f64>)\n");
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let jit = JitArtifact::compile(&plan).unwrap();
        let arguments = inputs(
            &artifact,
            &[
                ("integer", NativeScalarValue::U64(1)),
                ("value", fp(0x7ff0000000000042)),
            ],
        );
        assert!(
            reference(&artifact, &arguments)
                .unwrap_err()
                .contains("Arithmetic")
        );
        let mut output = vec![fp(17); plan.outputs().len()];
        let before = output.clone();
        assert!(
            matches!(jit.evaluate_into(&arguments, &mut output, NativeExecutionLimits { max_work: 400, ..NativeExecutionLimits::default() }, None), Err(NativeExecutionError::Arithmetic { node }) if node == plan.lets()[0].node())
        );
        assert_eq!(output, before);
    }
}

#[test]
fn f64_setup_completion_memory_and_type_denials_are_atomic_and_recover() {
    let (artifacts, _) = source_artifacts("+> math\nmath/sqrt(value<f64>)\n");
    for artifact in artifacts {
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.inputs().len(), 1);
        assert_eq!(plan.lets().len(), 1);
        let jit = JitArtifact::compile(&plan).unwrap();
        let arguments = inputs(&artifact, &[("value", fp(0x7ff0000000000042))]);
        let mut output = vec![fp(0x8000000000000000); plan.outputs().len()];
        let before = output.clone();
        let words = (plan.inputs().len() + plan.lets().len() + plan.outputs().len()) as u64;
        let setup = words * 2 + plan.inputs().len() as u64 * 8 + 64;
        let completion_limit = setup + plan.lets().len() as u64 * 256;
        assert!(matches!(
            jit.evaluate_into(
                &arguments,
                &mut output,
                NativeExecutionLimits {
                    max_work: completion_limit,
                    ..NativeExecutionLimits::default()
                },
                None
            ),
            Err(NativeExecutionError::WorkLimit {
                phase: NativeExecutionPhase::Completion,
                ..
            })
        ));
        assert_eq!(output, before);
        for limits in [
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            NativeExecutionLimits {
                max_transport_bytes: 0,
                ..NativeExecutionLimits::default()
            },
        ] {
            assert!(
                jit.evaluate_into(&arguments, &mut output, limits, None)
                    .is_err()
            );
            assert_eq!(output, before);
        }
        let budget = mech_core::ManagedMemoryBudget::new(1);
        assert!(
            jit.evaluate_into(
                &arguments,
                &mut output,
                NativeExecutionLimits::default(),
                Some(&budget)
            )
            .is_err()
        );
        assert_eq!(budget.used_bytes(), 0);
        assert_eq!(output, before);
        let wrong = vec![NativeScalarValue::U64(0)];
        assert!(matches!(
            jit.evaluate_into(&wrong, &mut output, NativeExecutionLimits::default(), None),
            Err(NativeExecutionError::InputType {
                expected: NativeScalarKind::F64,
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
        jit.evaluate_into(
            &arguments,
            &mut output,
            NativeExecutionLimits::default(),
            None,
        )
        .unwrap();
        assert_eq!(output, reference(&artifact, &arguments).unwrap());
    }
}

#[test]
fn f64_object_identity_is_typed_emission_without_execution() {
    let (artifact, _) = compile("left<f64> + right<f64>\n");
    let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let object = emit_object(&plan).unwrap();
    assert!(
        object
            .descriptor()
            .inputs()
            .iter()
            .all(|ty| ty.kind() == NativeScalarKind::F64)
    );
    assert_eq!(object.metadata()["linked"], false);
    assert_eq!(object.metadata()["executed"], false);
    assert_eq!(object.metadata()["timing_samples"], 0);
}
