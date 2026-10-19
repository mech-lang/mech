use super::*;
use crate::{DimensionExpr, FloatWidth, IntegerWidth, SchemaBody, SchemaDraft, SchemaTableBuilder};

#[test]
fn prospective_packed_matrix_nodes_equal_actual_canonical_footprints() {
    for (element, item) in [
        (SchemaBody::Bool, ValueDataDraft::Bool(true)),
        (
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            ValueDataDraft::U64(7),
        ),
        (
            SchemaBody::SignedInteger(IntegerWidth::W64),
            ValueDataDraft::I64(-7),
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W64),
            ValueDataDraft::F64(F64Bits::from_f64(-0.0)),
        ),
        (SchemaBody::String, ValueDataDraft::String("é🧑‍🚀".into())),
    ] {
        for count in [0usize, 1, 3, 128] {
            let mut builder = SchemaTableBuilder::new();
            let handle = builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body: SchemaBody::Matrix {
                            element: Box::new(element.clone()),
                            dimensions: vec![
                                DimensionExpr::Constant(1),
                                DimensionExpr::Constant(count as u64),
                            ]
                            .into_boxed_slice(),
                        },
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let build = builder.finish().unwrap();
            let schema = build.resolve(handle).unwrap();
            let schemas = build.into_parts().0;
            let value = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(vec![item.clone(); count].into_boxed_slice()),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap();
            assert_eq!(
                prospective_packed_matrix_node_count(&element, count as u64),
                Some(value.retained_footprint(&schemas).unwrap().node_count)
            );
        }
    }
    assert_eq!(
        prospective_packed_matrix_node_count(&SchemaBody::String, u64::MAX),
        None
    );
    assert_eq!(
        prospective_packed_matrix_node_count(&SchemaBody::Tuple(Box::new([])), 3),
        None
    );
}
