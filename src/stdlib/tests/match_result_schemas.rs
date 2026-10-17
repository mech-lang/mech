//! Match arm identity follows finalized schemas, including selected scalars.
#![cfg(feature = "full_compiler")]

use mech_core::{
    IntegerWidth, ReactiveInstanceId, ResidentValueRef, SchemaBody, Value, ValueData,
    ValueDataDraft, shape_for_resolved_extents,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use mech_engine::resident::{
    ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentExecutionError, activate,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn syntax(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d525343), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = product.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    values: &[u64],
    flag: bool,
) -> Result<(), ResidentExecutionError> {
    assert_eq!(instance.plan.inputs.len(), 2);
    let owned = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let schema = artifact.schemas().get(input.schema).unwrap();
            let (shape_values, data) = match schema.body() {
                SchemaBody::Bool => (Box::new([]) as Box<[u64]>, ValueDataDraft::Bool(flag)),
                SchemaBody::Matrix { element, .. }
                    if element.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64) =>
                {
                    let shape =
                        shape_for_resolved_extents(schema, &[1, values.len() as u64]).unwrap();
                    (
                        shape.parameter_values().to_vec().into_boxed_slice(),
                        ValueDataDraft::Matrix(
                            values.iter().copied().map(ValueDataDraft::U64).collect(),
                        ),
                    )
                }
                other => panic!("unexpected input schema: {other:?}"),
            };
            ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap()
        })
        .collect::<Vec<_>>();
    let snapshots = owned
        .into_iter()
        .map(|value| [Some(value)])
        .collect::<Vec<_>>();
    let flag_bytes = [u8::from(flag)];
    let captured = instance
        .plan
        .inputs
        .iter()
        .zip(&snapshots)
        .map(|(input, value)| CapturedSignalInput {
            slot: input.slot,
            value: match artifact.schemas().get(input.schema).unwrap().body() {
                SchemaBody::Bool => ResidentValueRef::Bool(&flag_bytes),
                SchemaBody::Matrix { .. } => ResidentValueRef::Snapshot(value),
                other => panic!("unexpected input schema: {other:?}"),
            },
        })
        .collect::<Vec<_>>();
    instance.turn(&captured).map(|_| ())
}

fn assert_scalar(value: &Value, expected: u64) {
    assert!(
        matches!(value.data(), ValueData::U64(actual) if *actual == expected),
        "{value:?}"
    );
    let schemas = value.schemas().unwrap();
    let schema = schemas.get(value.schema()).unwrap();
    assert_eq!(
        schema.body(),
        &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    );
    assert!(schema.dimension_parameters().is_empty());
    assert!(value.shape().parameter_values().is_empty());
}

#[test]
fn selected_live_u64_and_literal_arms_have_one_scalar_schema_in_either_order() {
    for (selected_on_true, source) in [
        (
            true,
            "values := input<[u64]>\n(flag<bool>? | true => values[1u64] | false => 0u64)\n",
        ),
        (
            false,
            "values := input<[u64]>\n(flag<bool>? | true => 0u64 | false => values[1u64])\n",
        ),
    ] {
        let (artifacts, output) = artifacts(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = activate(
                ReactiveInstanceId::new(0x4d525343, ordinal as u32),
                artifact,
                &mech_stdlib::source_catalog(),
                &ActivationFacts::default(),
            )
            .unwrap();
            for (values, selected, expected) in [
                (&[][..], false, 0),
                (&[u64::MAX, 9, 4][..], true, u64::MAX),
                (&[9][..], true, 9),
                (&[11][..], true, 11),
            ] {
                turn(
                    &mut instance,
                    artifact,
                    values,
                    selected == selected_on_true,
                )
                .unwrap();
                assert_scalar(&instance.copied_output(output).unwrap(), expected);
            }
            // Only the selected arm may perform the bounds check. A failed
            // selected access keeps the prior output; a later lazy arm recovers.
            assert!(turn(&mut instance, artifact, &[], selected_on_true).is_err());
            assert_scalar(&instance.copied_output(output).unwrap(), 11);
            turn(&mut instance, artifact, &[], !selected_on_true).unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), 0);
            turn(&mut instance, artifact, &[13, 21], selected_on_true).unwrap();
            assert_scalar(&instance.copied_output(output).unwrap(), 13);
        }
    }
}

#[test]
fn selected_u64_still_rejects_a_floating_point_result_arm() {
    for source in [
        "values := input<[u64]>\n(flag<bool>? | true => values[1u64] | false => 0.0<f64>)\n",
        "values := input<[u64]>\n(flag<bool>? | true => 0.0<f64> | false => values[1u64])\n",
    ] {
        let error = CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
            .err()
            .expect("different scalar schemas must remain incompatible");
        assert_eq!(
            error.code,
            "source-semantics/incompatible-match-result-kind"
        );
    }
}

#[test]
fn equal_element_count_does_not_make_distinct_fixed_result_geometries_equal() {
    for source in [
        "(flag<bool>? | true => [1u64 2u64] | false => [3u64; 4u64])\n",
        "(flag<bool>? | true => [3u64; 4u64] | false => [1u64 2u64])\n",
    ] {
        let error = CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax(source), mech_stdlib::source_catalog())
            .err()
            .expect("different matrix schemas must remain incompatible");
        assert_eq!(
            error.code,
            "source-semantics/incompatible-match-result-kind"
        );
    }
}
