//! Exact source/decoded acceptance for recovery finding G27's resident Index
//! range cardinality, physical binding, execution, and rejection boundaries.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    ReactiveInstanceId, ResidentKernelBindError, SchemaBody, SchemaDraft, SchemaTableBuilder,
    Value, ValueDataDraft, ValueDraft,
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

fn exact_range(artifact: &ProgramArtifact, expected: &[u64]) {
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
            ValueDataDraft::Matrix(
                expected
                    .iter()
                    .copied()
                    .map(ValueDataDraft::Index)
                    .collect()
            )
        );
        let body = output
            .schemas()
            .unwrap()
            .get(output.schema())
            .unwrap()
            .closed_body(output.shape())
            .unwrap();
        assert!(
            matches!(body, SchemaBody::Matrix { element, dimensions } if element.as_ref() == &SchemaBody::Index && dimensions.as_ref() == [mech_core::DimensionExpr::Constant(1), mech_core::DimensionExpr::Constant(expected.len() as u64)])
        );
    }
}

fn compile(source: &str, bindings: &[(u32, Value)]) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "index-range.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    CanonicalSourceFrontend
        .compile_document_with_catalog(&document.document(), mech_stdlib::source_catalog())
        .unwrap()
        .bind_input_constants(bindings)
        .unwrap()
        .compile_artifact()
        .unwrap()
}

fn source_and_decoded(source: &str, bindings: &[(u32, Value)], expected: &[u64]) {
    let artifact = compile(source, bindings);
    let bytecode = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    exact_range(&artifact, expected);
    exact_range(&bytecode, expected);
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
        let expected = (1..=count as u64).collect::<Vec<_>>();
        source_and_decoded(source.as_str(), &bindings, &expected);
    }
}

#[test]
fn index_range_increments_and_literal_endpoints_are_exact() {
    for (source, expected) in [
        ("answer := 1<ix>..2<ix>..6<ix>\nanswer\n", &[1, 3, 5][..]),
        ("answer := 1<ix>..2<ix>..=5<ix>\nanswer\n", &[1, 3, 5][..]),
        (
            "answer := 18446744073709551614<ix>..=18446744073709551615<ix>\nanswer\n",
            &[u64::MAX - 1, u64::MAX][..],
        ),
        (
            "answer := 18446744073709551614<ix>..2<ix>..=18446744073709551615<ix>\nanswer\n",
            &[u64::MAX - 1][..],
        ),
    ] {
        source_and_decoded(source, &[], expected);
    }
}

#[test]
fn index_range_rejects_unrepresentable_cardinality_and_live_endpoints() {
    let too_large = SourceDocument::parse_resolved(
        "index-range-overflow.mec",
        Revision(0),
        "answer := 1<ix>..=18446744073709551615<ix>\nanswer\n",
        ParseConfig::default(),
    )
    .unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&too_large.document(), mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    assert!(matches!(
        activate(
            ReactiveInstanceId::new(0x58c, 9),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        ),
        Err(mech_engine::resident::ResidentActivationError::InvalidDependency { .. })
            | Err(mech_engine::resident::ResidentActivationError::RegionSizeOverflow)
    ));

    let live = SourceDocument::parse_resolved(
        "index-range-live.mec",
        Revision(0),
        "answer := lo<ix>..hi<ix>\nanswer\n",
        ParseConfig::default(),
    )
    .unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&live.document(), mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    assert!(matches!(
        activate(
            ReactiveInstanceId::new(0x58c, 10),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        ),
        Err(mech_engine::resident::ResidentActivationError::KernelBind {
            error: ResidentKernelBindError::UnsupportedLayout,
            ..
        })
    ));
}
