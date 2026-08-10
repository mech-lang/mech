use super::super::ValueStateKey;
use super::support::{scalar, scalar_value};
use crate::{
    LegacyValue, ReactiveCellId, Ref, ValueKind, ValueStateEntryTypeMismatch, ValueStateJournal,
};
use core::any::type_name;

#[test]
fn state_journal_scalar_restore_preserves_address_and_reactive_identity() {
    let cell = scalar(1.0);
    let value = scalar_value(&cell);
    let address = cell.addr();
    let identity = ReactiveCellId::new(cell.id());

    let mut journal = ValueStateJournal::new();
    assert!(journal.is_empty());
    journal.capture_value(&value).unwrap();
    assert_eq!(journal.cell_count(), 1);

    *cell.borrow_mut() = 9.0;
    journal.restore_before().unwrap();

    assert_eq!(*cell.borrow(), 1.0);
    assert_eq!(cell.addr(), address);
    assert_eq!(value.reactive_root_cell_ids(), vec![identity]);
}

#[test]
fn state_journal_non_cell_roots_remain_empty() {
    let roots = vec![
        LegacyValue::Id(1),
        LegacyValue::Kind(ValueKind::F64),
        LegacyValue::IndexAll,
        LegacyValue::EmptyKind(ValueKind::F64),
        LegacyValue::Empty,
        LegacyValue::Typed(Box::new(LegacyValue::Empty), ValueKind::F64),
    ];
    let mut journal = ValueStateJournal::new();
    for root in roots {
        journal.capture_value(&root).unwrap();
    }
    assert!(journal.is_empty());
    journal.record_after().unwrap();
    assert_eq!(journal.into_delta().unwrap().cell_count(), 0);
}

#[test]
fn state_journal_erased_type_mismatch_is_structured() {
    let index = Ref::new(1usize);
    let float = scalar(1.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&LegacyValue::Index(index)).unwrap();

    let float_key = ValueStateKey::of(&float);
    journal.entry_indices.insert(float_key, 0);
    let error = journal.capture_value(&scalar_value(&float)).unwrap_err();
    let mismatch = error.kind_as::<ValueStateEntryTypeMismatch>().unwrap();
    assert_eq!(mismatch.type_name, type_name::<f64>());
}
