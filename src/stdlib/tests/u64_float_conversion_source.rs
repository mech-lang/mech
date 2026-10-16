//! Qualify the numeric conversion boundaries used by ordinary frequency tables.
#![cfg(feature = "full_compiler")]

use mech_core::{
    DimensionExpr, IntegerWidth, ReactiveInstanceId, SchemaBody, Value, ValueCell, ValueData,
    snapshot::{SequenceView, ValueDataDraft},
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x55363446), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let artifact = program.compile_artifact().unwrap();
    assert!(
        artifact
            .operation_references()
            .iter()
            .any(|operation| operation.canonical_name() == "convert/kind"),
        "the real conversion must survive source lowering"
    );
    (artifact, output)
}

fn counts(rows: u64, columns: u64, values: &[u64]) -> Value {
    assert_eq!((rows * columns) as usize, values.len());
    ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: Box::new([
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]),
        },
        ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect()),
    )
    .unwrap()
    .snapshot()
    .unwrap()
}

fn check_live(source: &str, inputs: &[Value], expected: impl Fn(usize, &Value)) {
    let (original, output) = compile(source);
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x55363446, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let input = instance.plan.inputs[0].clone();
        for (turn, value) in inputs.iter().enumerate() {
            let admitted = value
                .rebind(input.schema, &input.shape, artifact.schemas())
                .unwrap();
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &admitted,
                }])
                .and_then(|prepared| prepared.publish())
                .unwrap();
            expected(turn, &instance.copied_output(output).unwrap());
        }
    }
}

#[test]
fn live_u64_matrix_to_f64_annotation_preserves_geometry_and_converts_each_value() {
    let cases: [&[u64]; 2] = [
        &[0, 1, 2, 3, 4, 5],
        &[u64::MAX, 9_007_199_254_740_993, 17, 7, 11, 13],
    ];
    for (rows, columns) in [(2u64, 3u64), (3, 2), (1, 6), (6, 1)] {
        let inputs = cases
            .iter()
            .map(|values| counts(rows, columns, values))
            .collect::<Vec<_>>();
        for target in [format!("[f64]:{rows},{columns}"), "[f64]".into()] {
            let source = format!("values<{target}> := counts<[u64]:{rows},{columns}>\nvalues\n");
            check_live(&source, &inputs, |turn, value| {
                assert!(
                    matches!(value.schemas().unwrap().get(value.schema()).unwrap()
                    .closed_body(value.shape()).unwrap(),
                    SchemaBody::Matrix {dimensions, element}
                        if dimensions.as_ref() == [DimensionExpr::Constant(rows), DimensionExpr::Constant(columns)]
                        && matches!(element.as_ref(), SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)))
                );
                let ValueData::Matrix(matrix) = value.data() else {
                    panic!("{value:?}")
                };
                let SequenceView::F64(elements) = matrix.elements() else {
                    panic!("{value:?}")
                };
                assert_eq!(
                    elements
                        .iter()
                        .map(|value| value.to_f64())
                        .collect::<Vec<_>>(),
                    cases[turn]
                        .iter()
                        .map(|value| *value as f64)
                        .collect::<Vec<_>>(),
                    "{source}"
                );
            });
        }
    }
}

#[test]
fn live_u64_row_sum_selects_the_total_before_scalar_f64_conversion() {
    check_live(
        "+> stats\ntotal<f64> := stats/sum/column(counts<[u64]:1,3>)[1u64]\ntotal\n",
        &[counts(1, 3, &[2, 3, 7]), counts(1, 3, &[13, 17, 19])],
        |turn, value| {
            assert!(
                matches!(value.data(), ValueData::F64(value)
            if value.to_f64() == [12.0, 49.0][turn]),
                "{value:?}"
            )
        },
    );
}
