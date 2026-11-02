//! Shared Set cardinalities must not rewrite witnessed Matrix axes.
use super::*;

fn exact_set(element: SchemaBody, cardinality: DimensionExpr) -> SchemaBody {
    SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Exact(cardinality),
    }
}
fn current_case(agree: bool) -> (SchemaTable, Value, SchemaId) {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let source_key = SchemaBody::Matrix {
        element: Box::new(matrix(matrix(scalar.clone(), 1, 4), 1, 6)),
        dimensions: vec![axis(0), axis(1)].into_boxed_slice(),
    };
    let source = schema(matrix(source_key, 1, 1), parameters(2));
    let concrete = matrix(
        matrix(matrix(matrix(scalar.clone(), 1, 4), 1, 6), 2, 6),
        1,
        1,
    );
    let shape = shape_for_schema_components(&source, &[(source.body(), concrete)], None).unwrap();
    let target_key = SchemaBody::Matrix {
        element: Box::new(exact_set(exact_set(scalar, axis(0)), axis(1))),
        dimensions: vec![axis(0), axis(1)].into_boxed_slice(),
    };
    let target = schema(
        exact_set(target_key, DimensionExpr::Constant(1)),
        parameters(2),
    );
    // SAME source dimensions/plan for both cases. Agree: six distinct
    // two-unique rows. Conflict: three distinct four-unique rows repeated.
    let group = ValueDataDraft::Matrix(
        (0u8..6)
            .map(|row| {
                let items = if agree {
                    let first = 2 * row;
                    [first, first + 1, first, first + 1]
                } else {
                    let first = 4 * (row % 3);
                    [first, first + 1, first + 2, first + 3]
                };
                key(&items)
            })
            .collect(),
    );
    let key = ValueDataDraft::Matrix(vec![group; 12].into_boxed_slice());
    fixture(
        source,
        shape,
        ValueDataDraft::Matrix(vec![key].into_boxed_slice()),
        target,
    )
}
fn assert_selected_path(plan: &ConversionPlan) {
    let ConversionStep::MatrixToSet(key) = &plan.step else {
        panic!("outer Set")
    };
    let ConversionStep::MatrixElements(group) = &key.step else {
        panic!("same key axes")
    };
    let ConversionStep::MatrixToSet(row) = &group.step else {
        panic!("nested Set")
    };
    let ConversionStep::MatrixToSet(scalar) = &row.step else {
        panic!("scalar Set")
    };
    assert!(matches!(&scalar.step, ConversionStep::Identity));
}
fn key_axes(schema: &Schema, shape: &ShapeInstance) -> Box<[DimensionExpr]> {
    let SchemaBody::Set { element, .. } = schema.closed_body(shape).unwrap() else {
        panic!("outer Set")
    };
    let SchemaBody::Matrix { dimensions, .. } = *element else {
        panic!("matrix key")
    };
    dimensions
}

#[test]
fn converted_set_cardinalities_cannot_silently_reshape_shared_matrix_axes() {
    let (table, source, target) = current_case(false);
    let selected = plan(&table, &source, target);
    let target = table.get(target).unwrap();
    assert_selected_path(&selected);
    let seed = conversion_output_shape(
        source.validate_against(&table).unwrap(),
        source.shape(),
        target,
        &selected,
    )
    .unwrap();
    assert_eq!(
        key_axes(target, &seed).as_ref(),
        &[DimensionExpr::Constant(2), DimensionExpr::Constant(6)]
    );
    let result = execute_conversion_draft_with_schema(
        source.canonical_data_draft().unwrap(),
        &selected,
        target,
        &seed,
        &SnapshotValidationContext::new(&table),
    )
    .and_then(|data| conversion_output_shape_for_data(target, &seed, &data));
    assert!(
        result.is_err(),
        "four/three cardinalities must not rewrite 2x6 axes to 4x3"
    );
    assert_eq!(
        key_axes(target, &seed).as_ref(),
        &[DimensionExpr::Constant(2), DimensionExpr::Constant(6)]
    );
}

#[test]
fn converted_set_cardinalities_may_share_matrix_parameters_when_they_agree() {
    let (table, source, target) = current_case(true);
    let selected = plan(&table, &source, target);
    assert_selected_path(&selected);
    let output = convert(&table, &source, target);
    assert_eq!(
        key_axes(table.get(target).unwrap(), output.shape()).as_ref(),
        &[DimensionExpr::Constant(2), DimensionExpr::Constant(6)]
    );
    let group = ValueDataDraft::Set(
        (0u8..6)
            .map(|row| {
                let first = 2 * row;
                ValueDataDraft::Set(
                    vec![ValueDataDraft::U8(first), ValueDataDraft::U8(first + 1)]
                        .into_boxed_slice(),
                )
            })
            .collect(),
    );
    let expected = ValueDataDraft::Set(
        vec![ValueDataDraft::Matrix(vec![group; 12].into_boxed_slice())].into_boxed_slice(),
    );
    assert_eq!(output.canonical_data_draft().unwrap(), expected);
}

fn partial_payload_case(enum_payload: bool, present: bool) {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let matrix = SchemaBody::Matrix {
        element: Box::new(scalar.clone()),
        dimensions: vec![axis(0), DimensionExpr::Constant(1)].into_boxed_slice(),
    };
    let payload = if enum_payload {
        SchemaBody::Enum {
            key: NominalKey::from_bytes([57; 32]),
            variants: vec![
                EnumVariantSchema {
                    name: "absent".into(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "rows".into(),
                    payload: Some(matrix),
                },
            ]
            .into_boxed_slice(),
        }
    } else {
        SchemaBody::Option(Box::new(matrix))
    };
    // Set occurs FIRST: a later actual Matrix must still compare its axes
    // with the immutable seed, not with the earlier cardinality assignment.
    let schema = schema(
        SchemaBody::Tuple(vec![exact_set(scalar, axis(0)), payload].into_boxed_slice()),
        parameters(1),
    );
    let seed = schema
        .instantiate_shape(vec![if present { 2 } else { 0 }].into_boxed_slice())
        .unwrap();
    let payload_data = present.then(|| Box::new(key(&[8, 9])));
    let payload = if enum_payload {
        ValueDataDraft::Enum(EnumDraft {
            ordinal: if present { 1 } else { 0 },
            payload: payload_data,
        })
    } else {
        ValueDataDraft::Option(mech_core::snapshot::OptionDraft {
            present,
            value: payload_data,
        })
    };
    let data = ValueDataDraft::Tuple(
        vec![
            ValueDataDraft::Set(
                vec![
                    ValueDataDraft::U8(1),
                    ValueDataDraft::U8(2),
                    ValueDataDraft::U8(3),
                ]
                .into_boxed_slice(),
            ),
            payload,
        ]
        .into_boxed_slice(),
    );
    let shape = shape_for_value_components(&schema, [(schema.body(), &data)], &seed);
    if present {
        assert!(
            shape.is_err(),
            "a present 2x1 Matrix cannot become 3x1 via a sibling Set"
        );
    } else {
        let shape = shape.unwrap();
        assert_eq!(
            shape.parameter_values(),
            &[3],
            "an absent Matrix payload does not constrain this current value"
        );
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let table = build.into_parts().0;
        let value = ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data: data.clone(),
        }
        .finalize(&SnapshotValidationContext::new(&table))
        .unwrap();
        assert_eq!(value.canonical_data_draft().unwrap(), data);
    }
    assert_eq!(seed.parameter_values(), &[if present { 2 } else { 0 }]);
}

#[test]
fn only_present_option_matrix_payloads_constrain_shared_cardinality_refresh() {
    partial_payload_case(false, false);
    partial_payload_case(false, true);
}

#[test]
fn only_selected_enum_matrix_payloads_constrain_shared_cardinality_refresh() {
    partial_payload_case(true, false);
    partial_payload_case(true, true);
}
