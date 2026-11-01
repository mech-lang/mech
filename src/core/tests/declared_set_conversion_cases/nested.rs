use super::*;

#[test]
fn matrix_elements_bind_outer_axes_and_refresh_only_converted_set_cardinalities() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let source = schema(matrix(matrix(scalar.clone(), 1, 3), 1, 2), Box::new([]));
    let shape = source.instantiate_shape(Box::new([])).unwrap();
    let target = schema(
        matrix(
            SchemaBody::Set {
                element: Box::new(scalar),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            1,
            2,
        ),
        Box::new([]),
    );
    let (table, source, target) = fixture(
        source,
        shape,
        ValueDataDraft::Matrix(vec![key(&[2, 1, 2]), key(&[4, 3, 4])].into_boxed_slice()),
        target,
    );
    let selected = plan(&table, &source, target);
    assert!(matches!(selected.step, ConversionStep::MatrixElements(_)));
    assert!(selected.step.contains_matrix_to_set());
    assert_eq!(
        convert(&table, &source, target)
            .canonical_data_draft()
            .unwrap(),
        ValueDataDraft::Matrix(
            vec![
                ValueDataDraft::Set(
                    vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice()
                ),
                ValueDataDraft::Set(
                    vec![ValueDataDraft::U8(3), ValueDataDraft::U8(4)].into_boxed_slice()
                ),
            ]
            .into_boxed_slice()
        )
    );
    let target_schema = table.get(target).unwrap();
    let candidate = conversion_output_shape(
        source.validate_against(&table).unwrap(),
        source.shape(),
        target_schema,
        &selected,
    )
    .unwrap();
    let wrong = execute_conversion_draft_with_schema(
        ValueDataDraft::Matrix(vec![key(&[1, 1, 1]), key(&[4, 3, 4])].into_boxed_slice()),
        &selected,
        target_schema,
        &candidate,
        &SnapshotValidationContext::new(&table),
    )
    .unwrap();
    assert_eq!(
        conversion_output_shape_for_data(target_schema, &candidate, &wrong),
        Err(ConversionExecutionError::ConversionShapeMismatch)
    );
}

#[test]
fn converted_set_valued_keys_are_finalized_with_their_actual_current_cardinality() {
    let scalar = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let source = schema(matrix(matrix(scalar.clone(), 1, 3), 1, 3), Box::new([]));
    let shape = source.instantiate_shape(Box::new([])).unwrap();
    let target = schema(
        set(SchemaBody::Set {
            element: Box::new(scalar),
            cardinality: CardinalitySpec::Exact(axis(0)),
        }),
        parameters(1),
    );
    let (table, source, target) = fixture(
        source,
        shape,
        ValueDataDraft::Matrix(
            vec![key(&[2, 1, 2]), key(&[4, 3, 4]), key(&[2, 1, 2])].into_boxed_slice(),
        ),
        target,
    );
    let selected = plan(&table, &source, target);
    assert!(matches!(&selected.step,ConversionStep::MatrixToSet(inner)
        if matches!(inner.step,ConversionStep::MatrixToSet(_))));
    let output = convert(&table, &source, target);
    assert_eq!(output.shape().parameter_values(), &[2]);
    assert_eq!(
        output.canonical_data_draft().unwrap(),
        ValueDataDraft::Set(
            vec![
                ValueDataDraft::Set(
                    vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice()
                ),
                ValueDataDraft::Set(
                    vec![ValueDataDraft::U8(3), ValueDataDraft::U8(4)].into_boxed_slice()
                ),
            ]
            .into_boxed_slice()
        )
    );
    let target = table.get(target).unwrap();
    let candidate = conversion_output_shape(
        source.validate_against(&table).unwrap(),
        source.shape(),
        target,
        &selected,
    )
    .unwrap();
    // A single target parameter constrains every Set key. The last key cannot
    // overwrite an earlier conflicting current-cardinality witness silently.
    assert!(
        execute_conversion_draft_with_schema(
            ValueDataDraft::Matrix(vec![key(&[1, 1, 1]), key(&[4, 3, 4])].into_boxed_slice()),
            &selected,
            target,
            &candidate,
            &SnapshotValidationContext::new(&table)
        )
        .is_err()
    );
}

#[test]
fn declared_normalization_keeps_the_supplied_canonicalization_limit() {
    let element = set(SchemaBody::UnsignedInteger(IntegerWidth::W8));
    let source = schema(matrix(element.clone(), 1, 2), Box::new([]));
    let shape = source.instantiate_shape(Box::new([])).unwrap();
    let inner =
        ValueDataDraft::Set(vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice());
    let (table, source, target) = fixture(
        source,
        shape,
        ValueDataDraft::Matrix(vec![inner.clone(), inner].into_boxed_slice()),
        schema(set(element), Box::new([])),
    );
    let plan = plan(&table, &source, target);
    let target = table.get(target).unwrap();
    let shape = conversion_output_shape(
        source.validate_against(&table).unwrap(),
        source.shape(),
        target,
        &plan,
    )
    .unwrap();
    let denied = SnapshotCanonicalizationBudget::new(0);
    assert!(
        execute_conversion_draft_with_schema(
            source.canonical_data_draft().unwrap(),
            &plan,
            target,
            &shape,
            &SnapshotValidationContext::new(&table).with_canonicalization_budget(&denied)
        )
        .is_err()
    );
    assert_eq!(denied.consumed(), 0);
    let paid = SnapshotCanonicalizationBudget::new(32);
    let data = execute_conversion_draft_with_schema(
        source.canonical_data_draft().unwrap(),
        &plan,
        target,
        &shape,
        &SnapshotValidationContext::new(&table).with_canonicalization_budget(&paid),
    )
    .unwrap();
    assert!(paid.consumed() > 0);
    assert!(matches!(data,ValueDataDraft::Set(values) if values.len()==1));
}
