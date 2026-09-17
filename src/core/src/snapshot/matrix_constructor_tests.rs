use super::*;
use crate::{DimensionExpr, IntegerWidth, SchemaBody, SchemaDraft, SchemaTableBuilder};
use std::sync::Arc;

#[test]
fn matrix_projection_preserves_canonical_cells_and_rejects_invalid_inputs() {
    let mut builder = SchemaTableBuilder::new();
    let mut handles = Vec::new();
    for (width, rows, columns) in [
        (IntegerWidth::W8, 2, 2),
        (IntegerWidth::W8, 4, 1),
        (IntegerWidth::W16, 2, 2),
        (IntegerWidth::W16, 4, 1),
    ] {
        handles.push(
            builder
                .insert(
                    SchemaDraft {
                        body: SchemaBody::Matrix {
                            element: Box::new(SchemaBody::UnsignedInteger(width)),
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
    let source = ValueDraft {
        schema: ids[0],
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![1, 2, 3, 4]
                .into_iter()
                .map(ValueDataDraft::U8)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let constructor = MatrixSnapshotConstructor::bind(
        ids[1],
        shape(ids[1]),
        (ids[0], shape(ids[0])),
        Arc::clone(&schemas),
    )
    .unwrap();
    let selected = constructor.construct(&source, [0, 2, 1, 3]).unwrap();
    let draft = selected.canonical_data_draft().unwrap();
    assert_eq!(
        draft,
        ValueDataDraft::Matrix(
            vec![1, 3, 2, 4]
                .into_iter()
                .map(ValueDataDraft::U8)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        )
    );
    let restored = ValueDraft {
        schema: ids[1],
        shape_values: Box::new([]),
        data: draft,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    assert!(selected.language_eq(&schemas, &restored, &schemas).unwrap());
    for positions in [vec![0, 1, 2], vec![0, 1, 2, 3, 0], vec![0, 1, 2, 4]] {
        assert!(constructor.construct(&source, positions).is_err());
    }
    assert!(
        MatrixSnapshotConstructor::bind(
            ids[3],
            shape(ids[3]),
            (ids[0], shape(ids[0])),
            Arc::clone(&schemas)
        )
        .is_err()
    );
    let wrong = ValueDraft {
        schema: ids[2],
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![1, 2, 3, 4]
                .into_iter()
                .map(ValueDataDraft::U16)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    assert!(constructor.construct(&wrong, [0, 1, 2, 3]).is_err());
}
