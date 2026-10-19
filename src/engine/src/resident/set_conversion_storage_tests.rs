//! Representation facts only: no synthetic conversion plan or execution.
use super::*;

#[test]
fn packed_key_growth_uses_actual_carriers_not_an_extra_key_fee() {
    let h = size_of::<ValueData>() as u64;
    let bytes = [1u8, 2, 3];
    let strings: [Box<str>; 2] = ["é".into(), "👩‍🔬".into()];
    let compound = [ValueData::Option(Some(Box::new(ValueData::U8(1))))];
    for (values, expected_width) in [
        (SequenceView::U8(&bytes), Some(size_of::<u8>() as u64)),
        (
            SequenceView::String(&strings),
            Some(size_of::<Box<str>>() as u64),
        ),
        (SequenceView::Unit(128), Some(0)),
        (SequenceView::Values(&compound), None),
    ] {
        assert_eq!(packed_width(values), expected_width);
        let mut stats = Stats::default();
        if let Some(width) = packed_width(values) {
            stats
                .grow_storage(width, h, checked_u64(values.len()).unwrap())
                .unwrap();
        }
        assert_eq!(
            stats.storage_growth,
            expected_width.map_or(0, |w| values.len() as u64 * h.saturating_sub(w))
        );
        assert_eq!(
            stats.option_wrappers, 0,
            "new Option boxes are accounted separately"
        );
    }
}

#[test]
fn matrix_target_packing_has_same_facts_and_checked_growth() {
    let h = size_of::<ValueData>() as u64;
    assert_eq!(
        target_matrix_width(&Body::String),
        size_of::<Box<str>>() as u64
    );
    assert_eq!(
        target_matrix_width(&Body::Option(Box::new(Body::UnsignedInteger(
            IntegerWidth::W8
        )))),
        h
    );
    assert_eq!(
        target_matrix_width(&Body::FloatingPoint(FloatWidth::W64)),
        size_of::<mech_core::snapshot::F64Bits>() as u64
    );
    let mut stats = Stats::default();
    stats
        .grow_storage(1, target_matrix_width(&Body::String), 3)
        .unwrap();
    assert_eq!(stats.storage_growth, 3 * (size_of::<Box<str>>() as u64 - 1));
    stats.grow_storage(h, h, 128).unwrap();
    stats.grow_storage(h, 0, 128).unwrap();
    let before = stats.storage_growth;
    assert!(matches!(
        stats.grow_storage(0, h, u64::MAX),
        Err(Error::InvalidShape)
    ));
    assert_eq!(
        stats.storage_growth, before,
        "overflow does not partially install growth"
    );
}
