//! Strict positive witness for recovery finding G27. This is intentionally red
//! until the resident Index range prerequisite is implemented by its own owner.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    ReactiveInstanceId, SchemaBody, SchemaDraft, SchemaTableBuilder, Value, ValueDataDraft,
    ValueDraft,
};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_runtime::SourceDocument;
use mech_syntax::document::{ParseConfig, Revision};

fn index(value: u64) -> Value {
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Index,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    ValueDraft {
        schema: build.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: ValueDataDraft::Index(value),
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap()
}

fn exact_range(artifact: &ProgramArtifact, count: usize) {
    let mut instance = activate(
        ReactiveInstanceId::new(0x58c, 8),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap();
    for _ in 0..2 {
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        assert_eq!(
            output.canonical_data_draft().unwrap(),
            ValueDataDraft::Matrix((1..=count as u64).map(ValueDataDraft::Index).collect())
        );
        let body = output
            .schemas()
            .unwrap()
            .get(output.schema())
            .unwrap()
            .closed_body(output.shape())
            .unwrap();
        assert!(
            matches!(body, SchemaBody::Matrix { element, dimensions } if element.as_ref() == &SchemaBody::Index && dimensions.as_ref() == [mech_core::DimensionExpr::Constant(1), mech_core::DimensionExpr::Constant(count as u64)])
        );
    }
}

#[test]
fn bound_index_range_has_exact_source_and_bytecode_values() {
    for (operator, count) in [("..", 2), ("..=", 3)] {
        let source = format!("answer := lo<ix>{operator}hi<ix>\nanswer\n");
        let document = SourceDocument::parse_resolved(
            "index-range.mec",
            Revision(0),
            source.as_str(),
            ParseConfig::default(),
        )
        .unwrap();
        assert!(document.is_strictly_clean());
        let program = CanonicalSourceFrontend
            .compile_document_with_catalog(&document.document(), mech_stdlib::source_catalog())
            .unwrap();
        let bindings = program
            .program()
            .inputs
            .iter()
            .enumerate()
            .map(|(i, input)| (i as u32, index(if input.name == "lo" { 1 } else { 3 })))
            .collect::<Vec<_>>();
        let artifact = program
            .bind_input_constants(&bindings)
            .unwrap()
            .compile_artifact()
            .unwrap();
        let bytecode = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        exact_range(&artifact, count);
        exact_range(&bytecode, count);
    }
}
