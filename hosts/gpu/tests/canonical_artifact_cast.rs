//! Checked scalar cast qualification against actual maintained resident execution.
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

#[path = "support/canonical_cast_cases.rs"]
mod cases;
fn core_reference(
    cast: cases::Cast,
    input: NativeScalarValue,
) -> Result<NativeScalarValue, mech_core::ConversionExecutionError> {
    use mech_core::{BuiltinScalarKind as K, execute_scalar_conversion};
    let data = match input {
        NativeScalarValue::U64(v) => ValueDataDraft::U64(v),
        NativeScalarValue::I64(v) => ValueDataDraft::I64(v),
        NativeScalarValue::F64(v) => ValueDataDraft::F64(v),
        _ => panic!(),
    };
    let kind = |kind| match kind {
        NativeScalarKind::U64 => K::U64,
        NativeScalarKind::I64 => K::I64,
        NativeScalarKind::F64 => K::F64,
        _ => panic!(),
    };
    execute_scalar_conversion(data, kind(cast.input), kind(cast.output)).map(|value| match value {
        ValueDataDraft::U64(v) => NativeScalarValue::U64(v),
        ValueDataDraft::I64(v) => NativeScalarValue::I64(v),
        ValueDataDraft::F64(v) => NativeScalarValue::F64(v),
        _ => panic!(),
    })
}
#[test]
fn all_six_scalar_casts_match_actual_core_resident_and_jit_original_and_decoded() {
    for cast in cases::CASTS {
        let (artifacts, _) = artifacts(cast.source);
        for artifact in &artifacts {
            let plan =
                lower_pure_native_artifact(artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), 1);
            assert_eq!(plan.lets()[0].op(), cast.op);
            assert!(
                plan.outputs()
                    .iter()
                    .all(|output| output.ty().kind() == cast.output)
            );
            let jit = JitArtifact::compile(&plan).unwrap();
            for input in cases::samples(cast) {
                let args = inputs(artifact, &[("value", input)]);
                let core = core_reference(cast, input);
                let resident = reference(artifact, &args);
                let mut outputs = vec![NativeScalarValue::U64(0xfeed); plan.outputs().len()];
                let native =
                    jit.evaluate_into(&args, &mut outputs, NativeExecutionLimits::default(), None);
                match core {
                    Ok(expected) => {
                        let resident = resident.unwrap();
                        assert!(resident.iter().all(|value| *value == expected));
                        native.unwrap();
                        assert_eq!(outputs, resident, "{} {input:?}", cast.name);
                    }
                    Err(error) => {
                        assert!(resident.is_err(), "{} {input:?} {error:?}", cast.name);
                        assert!(
                            matches!(native,Err(NativeExecutionError::Arithmetic {node}) if node==plan.lets()[0].node()),
                            "{} {input:?} {native:?}",
                            cast.name
                        );
                        assert!(
                            outputs
                                .iter()
                                .all(|value| *value == NativeScalarValue::U64(0xfeed))
                        );
                    }
                }
            }
            // Same compiled instance must still succeed after all failures.
            let zero = match cast.input {
                NativeScalarKind::U64 => NativeScalarValue::U64(0),
                NativeScalarKind::I64 => NativeScalarValue::I64(0),
                NativeScalarKind::F64 => NativeScalarValue::F64(F64Bits::from_bits(0)),
                _ => panic!(),
            };
            let args = inputs(artifact, &[("value", zero)]);
            let mut output = vec![NativeScalarValue::U64(99); plan.outputs().len()];
            jit.evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None)
                .unwrap();
            assert_eq!(output, reference(artifact, &args).unwrap());

            // All three numeric domains share a 64-bit transport word. A zero
            // bit pattern must not allow one domain to impersonate another.
            let same_width_wrong_inputs = match cast.input {
                NativeScalarKind::U64 => [
                    NativeScalarValue::I64(0),
                    NativeScalarValue::F64(F64Bits::from_bits(0)),
                ],
                NativeScalarKind::I64 => [
                    NativeScalarValue::U64(0),
                    NativeScalarValue::F64(F64Bits::from_bits(0)),
                ],
                NativeScalarKind::F64 => [
                    NativeScalarValue::U64(0),
                    NativeScalarValue::I64(0),
                ],
                NativeScalarKind::Bool => unreachable!(),
            };
            for wrong in same_width_wrong_inputs {
                let args = inputs(artifact, &[("value", wrong)]);
                let mut output = vec![NativeScalarValue::U64(0xfeed); plan.outputs().len()];
                let before = output.clone();
                assert!(matches!(
                    jit.evaluate_into(&args, &mut output, NativeExecutionLimits::default(), None),
                    Err(NativeExecutionError::InputType { .. })
                ));
                assert_eq!(output, before);
            }
        }
    }
}
#[test]
fn cast_boundaries_truncate_before_range_and_never_saturate() {
    use NativeScalarValue::{F64, I64, U64};
    let casts = cases::CASTS;
    for (index, input, expected) in [
        (
            2,
            U64(u64::MAX),
            F64(F64Bits::from_bits(0x43f0_0000_0000_0000)),
        ),
        (
            3,
            I64(i64::MAX),
            F64(F64Bits::from_bits(0x43e0_0000_0000_0000)),
        ),
        (
            3,
            I64(i64::MIN),
            F64(F64Bits::from_bits(0xc3e0_0000_0000_0000)),
        ),
        (4, F64(F64Bits::from_f64(-0.5)), U64(0)),
        (4, F64(F64Bits::from_bits(0x8000_0000_0000_0000)), U64(0)),
        (
            5,
            F64(F64Bits::from_bits(0xc3e0_0000_0000_0000)),
            I64(i64::MIN),
        ),
    ] {
        assert_eq!(core_reference(casts[index], input).unwrap(), expected);
    }
    for (index, input, error) in [
        (
            0,
            U64(i64::MAX as u64 + 1),
            mech_core::ConversionExecutionError::ConversionOutOfRange,
        ),
        (
            1,
            I64(-1),
            mech_core::ConversionExecutionError::ConversionOutOfRange,
        ),
        (
            4,
            F64(F64Bits::from_f64(-1.0)),
            mech_core::ConversionExecutionError::ConversionOutOfRange,
        ),
        (
            4,
            F64(F64Bits::from_bits(0x43f0_0000_0000_0000)),
            mech_core::ConversionExecutionError::ConversionOutOfRange,
        ),
        (
            5,
            F64(F64Bits::from_bits(0x43e0_0000_0000_0000)),
            mech_core::ConversionExecutionError::ConversionOutOfRange,
        ),
        (
            4,
            F64(F64Bits::from_bits(0x7ff0_0000_0000_0001)),
            mech_core::ConversionExecutionError::ConversionNonFinite,
        ),
        (
            5,
            F64(F64Bits::from_bits(0xfff0_0000_0000_0000)),
            mech_core::ConversionExecutionError::ConversionNonFinite,
        ),
    ] {
        assert_eq!(core_reference(casts[index], input), Err(error));
    }
}
#[test]
fn cast_error_order_denials_atomic_outputs_and_recovery_are_preserved() {
    let source = "unused<u64> := value<f64>\nother<i64> := integer<u64>\nother\n";
    let (artifacts, _) = artifacts(source);
    for artifact in &artifacts {
        let plan = lower_pure_native_artifact(artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(plan.lets().len(), 2);
        let jit = JitArtifact::compile(&plan).unwrap();
        let setup = 2
            * (jit.descriptor().inputs().len()
                + jit.descriptor().lets().len()
                + jit.descriptor().outputs().len()) as u64
            + 8 * jit.descriptor().inputs().len() as u64
            + 64;
        let limits = NativeExecutionLimits {
            max_work: setup + 256,
            ..NativeExecutionLimits::default()
        };
        for (float, integer, first_arithmetic) in
            [(f64::NAN, u64::MAX, true), (0.5, u64::MAX, false)]
        {
            let args = inputs(
                artifact,
                &[
                    ("value", NativeScalarValue::F64(F64Bits::from_f64(float))),
                    ("integer", NativeScalarValue::U64(integer)),
                ],
            );
            let mut output = vec![NativeScalarValue::I64(71); plan.outputs().len()];
            let error = jit
                .evaluate_into(&args, &mut output, limits, None)
                .unwrap_err();
            if first_arithmetic {
                assert!(
                    matches!(error,NativeExecutionError::Arithmetic{node} if node==plan.lets()[0].node())
                );
            } else {
                assert!(
                    matches!(error,NativeExecutionError::WorkLimit{phase:NativeExecutionPhase::Node(node),..} if node==plan.lets()[1].node())
                );
            }
            assert!(
                output
                    .iter()
                    .all(|value| *value == NativeScalarValue::I64(71))
            );
        }
        let good = inputs(
            artifact,
            &[
                ("value", NativeScalarValue::F64(F64Bits::from_f64(-0.5))),
                ("integer", NativeScalarValue::U64(7)),
            ],
        );
        for (limits, budget) in [
            (
                NativeExecutionLimits {
                    max_work: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits {
                    max_work: setup + 512,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits {
                    max_transport_bytes: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits::default(),
                Some(mech_core::ManagedMemoryBudget::new(0)),
            ),
        ] {
            let mut output = vec![NativeScalarValue::I64(71); plan.outputs().len()];
            assert!(
                jit.evaluate_into(&good, &mut output, limits, budget.as_ref())
                    .is_err()
            );
            assert!(
                output
                    .iter()
                    .all(|value| *value == NativeScalarValue::I64(71))
            );
        }
        let mut wrong = good.clone();
        wrong[0] = NativeScalarValue::Bool(false);
        let mut output = vec![NativeScalarValue::I64(71); plan.outputs().len()];
        assert!(matches!(
            jit.evaluate_into(&wrong, &mut output, NativeExecutionLimits::default(), None),
            Err(NativeExecutionError::InputType { .. })
        ));
        assert!(
            output
                .iter()
                .all(|value| *value == NativeScalarValue::I64(71))
        );
        jit.evaluate_into(&good, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output, reference(artifact, &good).unwrap());
    }
}
#[test]
fn cast_object_metadata_has_new_typed_identity_but_is_not_execution() {
    for cast in cases::CASTS {
        let (artifact, _) = compile(cast.source);
        let plan = lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let object = emit_object(&plan).unwrap();
        assert!(!object.bytes().is_empty());
        assert_eq!(object.metadata()["linked"], false);
        assert_eq!(object.metadata()["executed"], false);
        assert_eq!(object.metadata()["timing_samples"], 0);
        assert_eq!(mech_gpu::native_artifact::NATIVE_TYPED_IR_VERSION, 4);
    }
}
