use super::*;

#[test]
fn final_range_summary_skips_geometry_cell_callbacks_without_a_cache() {
    let cells = ["a", "猫", "e\u{301}", ""];
    let selector = Selector::UnitRange { start: 2, count: 3 };
    let reads = core::cell::Cell::new(0);
    let ordinary = cell_geometry(
        (1, cells.len()),
        cells.len(),
        selector,
        |index| {
            reads.set(reads.get() + 1);
            cells.get(index).copied()
        },
        |_, _| None,
    )
    .unwrap();
    assert_eq!(reads.get(), 3);
    let summaries = core::cell::Cell::new(0);
    for _ in 0..2 {
        reads.set(0);
        let summarized = cell_geometry(
            (1, cells.len()),
            cells.len(),
            selector,
            |index| {
                reads.set(reads.get() + 1);
                cells.get(index).copied()
            },
            |first, end| {
                summaries.set(summaries.get() + 1);
                assert_eq!((first, end), (1, 4));
                Some(6)
            },
        )
        .unwrap();
        assert_eq!(reads.get(), 0);
        assert_eq!(summarized.count, ordinary.count);
        assert_eq!(summarized.payload, ordinary.payload);
        assert_eq!(summarized.contiguous_range(), ordinary.contiguous_range());
    }
    assert_eq!(
        summaries.get(),
        2,
        "each visit uses its immutable owner facts"
    );
}

#[test]
fn range_summary_still_checks_fixed_geometry_and_the_output_schema() {
    use mech_core::{DimensionExpr, SchemaDraft, SchemaTableBuilder};
    for (schema_columns, planned_columns, dynamic, succeeds) in [
        (3, 3, false, true),
        (3, 2, false, false),
        (2, 3, true, false),
    ] {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Matrix {
                        element: Box::new(SchemaBody::String),
                        dimensions: vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Constant(schema_columns),
                        ]
                        .into_boxed_slice(),
                    },
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let schema_id = built.resolve(handle).unwrap();
        let schema = built.table.get(schema_id).unwrap();
        let shape = schema.instantiate_shape(Box::new([])).unwrap();
        // Metadata-only fixture: it grants no allocation or execution permit.
        // The real original/decoded prepared-factory tests cover those gates.
        let plan = SnapshotAccessPlan {
            dense_source: None,
            matrix_constructor: None,
            selectors: vec![SnapshotAccessSelectorLayout {
                schema: schema_id,
                shape: shape.clone(),
                resident_shape: ResidentShape::SCALAR,
            }]
            .into_boxed_slice(),
            matrix_mode: Some(ResolvedSelectionMode::LinearGather),
            source_dimensions: Some((1, 4)),
            output_dimensions: Some((1, planned_columns)),
            output_dimensions_dynamic: dynamic,
            linear_gather_row: true,
            output_geometry: ResolvedAccessGeometry {
                logical_output_rows: 1,
                logical_output_columns: planned_columns,
            },
            aggregate_ordinal: None,
            output_schema: schema_id,
        };
        let output = ResidentSnapshotOutput {
            schema: schema_id,
            schema_key: schema.key(),
            shape,
            exact_cardinality: None,
            maximum_cardinality: None,
        };
        let kernel = BoundResidentKernel::new(|_, _, _| panic!("no execution"), Box::new([]))
            .with_snapshot_output(output)
            .with_snapshot_schemas(built.table)
            .with_retained_state(Arc::new(plan));
        let summaries = core::cell::Cell::new(0);
        let result = prepare_cells(
            &kernel,
            (1, 4),
            4,
            Selector::UnitRange { start: 2, count: 3 },
            |_| panic!("eligible summary must skip geometry cell reads"),
            |first, end| {
                assert_eq!((first, end), (1, 4));
                summaries.set(summaries.get() + 1);
                Some(6)
            },
        );
        assert_eq!(summaries.get(), 1);
        if succeeds {
            assert_eq!(result.unwrap().count, 3);
        } else {
            assert_eq!(result.unwrap_err(), ResidentKernelError::InvalidShape);
        }
    }
}

#[test]
fn range_summary_uncertainty_preserves_ordered_cell_errors_and_matrix_geometry() {
    let cells = ["a", "猫", "c", "d", "é", "f"];
    let mut seen = Vec::new();
    let matrix = cell_geometry(
        (2, 3),
        cells.len(),
        Selector::UnitRange { start: 2, count: 3 },
        |index| {
            seen.push(index);
            cells.get(index).copied()
        },
        |_, _| panic!("non-row matrices must retain column-major conversion"),
    )
    .unwrap();
    assert_eq!(seen, [3, 1, 4]);
    assert_eq!(matrix.contiguous_range(), None);

    for (start, count, expected) in [(0, 1, vec![]), (6, 2, vec![5]), (1, 0, vec![])] {
        let mut seen = Vec::new();
        let result = cell_geometry(
            (1, cells.len()),
            cells.len(),
            Selector::UnitRange { start, count },
            |index| {
                seen.push(index);
                cells.get(index).copied()
            },
            |_, _| panic!("invalid or empty endpoints cannot authorize a summary"),
        );
        assert_eq!(seen, expected);
        if count == 0 {
            assert_eq!(result.unwrap().count, 0);
        } else {
            assert_eq!(result.unwrap_err(), ResidentKernelError::InvalidInput);
        }
    }
    let mut seen = Vec::new();
    assert_eq!(
        cell_geometry(
            (1, cells.len()),
            cells.len(),
            Selector::UnitRange { start: 2, count: 3 },
            |index| {
                seen.push(index);
                (index != 2).then(|| cells[index])
            },
            |_, _| None,
        )
        .unwrap_err(),
        ResidentKernelError::InvalidInput
    );
    assert_eq!(
        seen,
        [1, 2],
        "missing retained cells keep their original order"
    );
    assert_eq!(
        cell_geometry(
            (2, 2),
            cells.len(),
            Selector::UnitRange { start: 1, count: 1 },
            |_| panic!("shape validation precedes cell reads"),
            |_, _| panic!("shape validation precedes summaries"),
        )
        .unwrap_err(),
        ResidentKernelError::InvalidShape
    );
}

#[test]
#[cfg(target_pointer_width = "64")]
fn range_summary_keeps_portable_endpoint_fallback() {
    let maximum = mech_core::PORTABLE_SELECTOR_INDEX_MAX;
    let mut seen = Vec::new();
    let result = cell_geometry(
        (1, usize::MAX),
        usize::MAX,
        Selector::UnitRange {
            start: maximum,
            count: 2,
        },
        |index| {
            seen.push(index);
            Some("")
        },
        |_, _| panic!("an invalid terminal endpoint must use the original walk"),
    );
    assert_eq!(result.unwrap_err(), ResidentKernelError::InvalidInput);
    assert_eq!(seen, [maximum as usize - 1]);
}

#[test]
fn unit_range_uses_the_maintained_matrix_selector_errors() {
    for (start, count, upper) in [(1, 4, 4), (2, 2, 4), (0, 1, 4), (4, 2, 4), (1, 1, 0)] {
        let mut actual = Vec::new();
        let result = Selector::UnitRange { start, count }.for_each_index(upper, |index| {
            actual.push(index);
            Ok(())
        });
        let mut expected = Vec::new();
        let materialized = (start..start + count).collect::<Vec<_>>();
        let oracle =
            sequence_for_each_access_index(SequenceView::U64(&materialized), upper, |index| {
                expected.push(index);
                Ok::<_, ResidentKernelError>(())
            });
        assert_eq!(format!("{result:?}"), format!("{oracle:?}"));
        assert_eq!(actual, expected);
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn range_endpoints_preserve_portable_sequence_limits_without_wrapping() {
    let maximum = mech_core::PORTABLE_SELECTOR_INDEX_MAX;
    let mut seen = Vec::new();
    Selector::UnitRange {
        start: maximum - 1,
        count: 2,
    }
    .for_each_index(usize::MAX, |index| {
        seen.push(index);
        Ok(())
    })
    .unwrap();
    assert_eq!(seen, [maximum as usize - 2, maximum as usize - 1]);
    // An endpoint beyond the portable index bound fails at the same element
    // and with the same error as a genuine packed U64 selector.
    for (start, count, values) in [
        (maximum, 2, vec![maximum, maximum + 1]),
        (u64::MAX, 1, vec![u64::MAX]),
    ] {
        let mut actual = Vec::new();
        let result = Selector::UnitRange { start, count }.for_each_index(usize::MAX, |index| {
            actual.push(index);
            Ok(())
        });
        let mut expected = Vec::new();
        let oracle =
            sequence_for_each_access_index(SequenceView::U64(&values), usize::MAX, |index| {
                expected.push(index);
                Ok(())
            });
        assert_eq!(result, oracle);
        assert_eq!(actual, expected);
    }
    // Even internal impossible geometry fails closed before arithmetic wraps.
    assert!(
        Selector::UnitRange {
            start: u64::MAX,
            count: 2
        }
        .for_each_index(usize::MAX, |_| Ok(()))
        .is_err()
    );
}

#[test]
fn compact_geometry_keeps_column_major_selection_of_row_major_cells() {
    let cells = ["a", "猫", "c", "d", "é", "f"];
    let geometry = Geometry {
        rows: 2,
        columns: 3,
        count: 3,
        payload: 0,
        contiguous: None,
    };
    let selector = Selector::UnitRange { start: 2, count: 3 };
    let mut output = Vec::new();
    geometry
        .for_each_cells(
            cells.len(),
            selector,
            |index| cells.get(index).copied(),
            |ordinal, text| {
                output.push((ordinal, text.to_owned()));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(
        output,
        [
            (0, "d".to_owned()),
            (1, "猫".to_owned()),
            (2, "é".to_owned())
        ]
    );
    assert!(
        geometry
            .for_each_cells(
                cells.len(),
                Selector::UnitRange { start: 2, count: 2 },
                |index| cells.get(index).copied(),
                |_, _| Ok(())
            )
            .is_err()
    );
    assert!(
        geometry
            .for_each_cells(
                cells.len() - 1,
                selector,
                |index| cells.get(index).copied(),
                |_, _| Ok(())
            )
            .is_err()
    );
}
