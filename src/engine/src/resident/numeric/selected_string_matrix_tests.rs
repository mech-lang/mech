use super::*;

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
