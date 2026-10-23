use super::*;

#[test]
fn fixed_matrix_set_keys_use_declared_axes_before_sort_and_deduplication() {
    let element = matrix(SchemaBody::UnsignedInteger(IntegerWidth::W8), 2, 2);
    let source = schema(matrix(element.clone(), 1, 3), Box::new([]));
    let shape = source.instantiate_shape(Box::new([])).unwrap();
    let (table, source, target) = fixture(
        source,
        shape,
        ValueDataDraft::Matrix(
            vec![key(&[4, 3, 2, 1]), key(&[1, 2, 3, 4]), key(&[4, 3, 2, 1])].into_boxed_slice(),
        ),
        schema(set(element), Box::new([])),
    );
    let expected =
        ValueDataDraft::Set(vec![key(&[1, 2, 3, 4]), key(&[4, 3, 2, 1])].into_boxed_slice());
    assert_eq!(
        convert(&table, &source, target)
            .canonical_data_draft()
            .unwrap(),
        expected
    );
    // The metadata-free API also handles genuinely fixed declared axes.
    assert_eq!(
        execute_conversion_draft(
            source.canonical_data_draft().unwrap(),
            &plan(&table, &source, target).step
        )
        .unwrap(),
        expected
    );
    let plan = plan(&table, &source, target);
    let target_schema = table.get(target).unwrap();
    let candidate = conversion_output_shape(
        source.validate_against(&table).unwrap(),
        source.shape(),
        target_schema,
        &plan,
    )
    .unwrap();
    assert!(
        execute_conversion_draft_with_schema(
            ValueDataDraft::Matrix(vec![key(&[1, 2, 3])].into_boxed_slice()),
            &plan,
            target_schema,
            &candidate,
            &SnapshotValidationContext::new(&table)
        )
        .is_err()
    );
}

#[test]
fn current_matrix_key_axes_are_rebound_not_copied_between_schema_environments() {
    for (rows, columns) in [(2, 2), (1, 4)] {
        let source = schema(
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    dimensions: vec![axis(2), axis(3)].into_boxed_slice(),
                }),
                dimensions: vec![axis(0), axis(1)].into_boxed_slice(),
            },
            parameters(4),
        );
        // Canonical schema construction may reorder parameter IDs. Establish
        // the actual source geometry, not an assumed declaration tape order.
        let concrete = matrix(
            matrix(SchemaBody::UnsignedInteger(IntegerWidth::W8), rows, columns),
            1,
            3,
        );
        let source_shape =
            shape_for_schema_components(&source, &[(source.body(), concrete)], None).unwrap();
        assert_eq!(source_shape.parameter_values().len(), 4);
        let target = schema(
            set(SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: vec![axis(0), axis(1)].into_boxed_slice(),
            }),
            parameters(2),
        );
        let (table, source, target) = fixture(
            source,
            source_shape,
            ValueDataDraft::Matrix(
                vec![key(&[1, 2, 3, 4]), key(&[4, 3, 2, 1]), key(&[1, 2, 3, 4])].into_boxed_slice(),
            ),
            target,
        );
        let output = convert(&table, &source, target);
        assert_eq!(output.shape().parameter_values(), &[rows, columns]);
        let plan = plan(&table, &source, target);
        assert_eq!(
            execute_conversion_draft_with_schema(
                source.canonical_data_draft().unwrap(),
                &plan,
                table.get(target).unwrap(),
                source.shape(),
                &SnapshotValidationContext::new(&table)
            ),
            Err(ConversionExecutionError::ConversionShapeMismatch),
            "a four-parameter source tape cannot be reused as the two-parameter target tape"
        );
    }
}
