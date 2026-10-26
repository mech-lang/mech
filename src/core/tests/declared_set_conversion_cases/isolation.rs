use super::*;

#[test]
fn conversion_cardinality_traversal_does_not_change_ordinary_shape_inference() {
    let body = set(matrix(
        SchemaBody::Set {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            cardinality: CardinalitySpec::Exact(axis(0)),
        },
        1,
        1,
    ));
    let schema = schema(body, parameters(1));
    let seed = shape_for_declared_lower_bounds(&schema).unwrap();
    let data = ValueDataDraft::Set(
        vec![ValueDataDraft::Matrix(
            vec![ValueDataDraft::Set(
                vec![ValueDataDraft::U8(1)].into_boxed_slice(),
            )]
            .into_boxed_slice(),
        )]
        .into_boxed_slice(),
    );
    let ordinary = shape_for_value_data(&schema, &data, &[], Some(&seed)).unwrap();
    assert_eq!(ordinary.parameter_values(), &[0]);
    let converted = shape_for_value_components(&schema, [(schema.body(), &data)], &seed).unwrap();
    assert_eq!(converted.parameter_values(), &[1]);
    assert_eq!(seed.parameter_values(), &[0]);
}
