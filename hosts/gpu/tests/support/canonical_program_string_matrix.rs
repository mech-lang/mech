use mech_core::snapshot::*;
use mech_core::{DimensionExpr, IntegerWidth, SchemaBody, SchemaDraft, SchemaTableBuilder};
use std::sync::Arc;

#[test]
fn compact_string_matrices_match_validated_drafts_and_move_the_payloads() {
    let mut builder = SchemaTableBuilder::new();
    let mut handles = Vec::new();
    for (element, rows, columns) in [
        (SchemaBody::String, 2, 2),
        (SchemaBody::String, 1, 4),
        (SchemaBody::String, 0, 4),
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), 2, 2),
    ] {
        handles.push(
            builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Matrix {
                            element: Box::new(element),
                            dimensions: vec![
                                DimensionExpr::Constant(rows),
                                DimensionExpr::Constant(columns),
                            ]
                            .into_boxed_slice(),
                        },
                        dimension_parameters: Box::new([]),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap(),
        );
    }
    let built = builder.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|h| built.resolve(h).unwrap())
        .collect::<Vec<_>>();
    let (schemas, _) = built.into_parts();
    let schemas = Arc::new(schemas);
    let shape = |id| {
        schemas
            .get(id)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap()
    };
    for id in &ids[..3] {
        let text: &[&str] = if *id == ids[2] {
            &[]
        } else {
            &["", "\0\r\n", "é", "e\u{301}🧑‍🚀"]
        };
        let strings = text
            .iter()
            .map(|s| Box::<str>::from(*s))
            .collect::<Vec<_>>();
        let pointer = strings.get(2).map(|s| s.as_ptr());
        let value = StringMatrixSnapshotConstructor::bind(*id, shape(*id), Arc::clone(&schemas))
            .unwrap()
            .construct(strings.into_boxed_slice())
            .unwrap();
        let ordinary = ValueDraft {
            schema: *id,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                text.iter()
                    .map(|s| ValueDataDraft::String((*s).into()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap();
        assert_eq!(value.schema_key(), ordinary.schema_key());
        assert_eq!(value.shape(), ordinary.shape());
        assert!(value.snapshot_eq(&schemas, &ordinary, &schemas).unwrap());
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            ordinary.canonical_data_draft().unwrap()
        );
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("matrix")
        };
        let SequenceView::String(strings) = matrix.elements() else {
            panic!("packed strings")
        };
        assert_eq!(
            strings.get(2).map(|s| s.as_ptr()),
            pointer,
            "construction must transfer existing String payload owners"
        );
    }
    for count in [0, 3, 5] {
        let strings = (0..count)
            .map(|_| Box::<str>::from("cell"))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        assert!(
            StringMatrixSnapshotConstructor::bind(ids[0], shape(ids[0]), Arc::clone(&schemas))
                .unwrap()
                .construct(strings)
                .is_err()
        );
    }
    assert!(
        StringMatrixSnapshotConstructor::bind(ids[3], shape(ids[3]), Arc::clone(&schemas)).is_err()
    );
}
