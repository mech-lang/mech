use crate::{DMatrix, LegacyValue, Matrix2, Ref, ValueCell, ValueStateJournal};

#[test]
fn exact_scalar_roots_restore_deduplicate_and_preserve_handles() {
    let first = Ref::new(1.0_f64);
    let first_alias = first.clone();
    let equal_but_distinct = Ref::new(1.0_f64);
    let mut journal = ValueStateJournal::new();

    journal.capture_exact_ref(&first).unwrap();
    journal.capture_exact_ref(&first).unwrap();
    journal.capture_exact_ref(&equal_but_distinct).unwrap();
    assert_eq!(journal.cell_count(), 2);

    *first.borrow_mut() = 10.0;
    *equal_but_distinct.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    assert!(first.same_handle(&first_alias));
    assert_eq!((*first.borrow(), *equal_but_distinct.borrow()), (1.0, 1.0));
}

#[test]
fn exact_scalar_roots_record_after_rewind_and_replay() {
    let target = Ref::new(2.0_f64);
    let mut journal = ValueStateJournal::new();
    journal.capture_exact_ref(&target).unwrap();
    *target.borrow_mut() = 7.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    assert_eq!(*target.borrow(), 2.0);
    delta.replay().unwrap();
    assert_eq!(*target.borrow(), 7.0);
}

#[test]
fn exact_root_borrow_conflicts_are_structured_and_atomic() {
    let target = Ref::new(3.0_f64);
    let held_write = target.borrow_mut();
    let mut journal = ValueStateJournal::new();
    let error = journal.capture_exact_ref(&target).unwrap_err();
    assert_eq!(error.kind_name(), "ValueStateBorrowConflict");
    assert_eq!(
        error.kind_message(),
        "Cannot borrow f64 cell during capture-before."
    );
    assert!(journal.is_empty());
    drop(held_write);

    journal.capture_exact_ref(&target).unwrap();
    *target.borrow_mut() = 9.0;
    let held_read = target.borrow();
    let error = journal.restore_before().unwrap_err();
    assert_eq!(error.kind_name(), "ValueStateBorrowConflict");
    assert_eq!(*held_read, 9.0);
    drop(held_read);
    journal.restore_before().unwrap();
    assert_eq!(*target.borrow(), 3.0);
}

#[test]
fn exact_fixed_and_dynamic_matrix_roots_restore_shape_and_storage() {
    let fixed = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
    let dynamic = Ref::new(DMatrix::from_vec(
        2,
        3,
        vec![1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
    ));
    let mut journal = ValueStateJournal::new();
    journal.capture_exact_ref(&fixed).unwrap();
    journal.capture_exact_ref(&dynamic).unwrap();

    *fixed.borrow_mut() = Matrix2::new(9.0, 8.0, 7.0, 6.0);
    *dynamic.borrow_mut() = DMatrix::from_vec(1, 2, vec![9.0, 8.0]);
    journal.restore_before().unwrap();

    assert_eq!(*fixed.borrow(), Matrix2::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(dynamic.borrow().shape(), (2, 3));
    assert_eq!(dynamic.borrow().as_slice(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn exact_legacy_and_value_cell_roots_coexist_and_share_deduplication() {
    let exact = Ref::new(4.0_f64);
    let legacy_index = Ref::new(5usize);
    let cell = ValueCell::new(LegacyValue::Index(legacy_index.clone()));
    let mut journal = ValueStateJournal::new();

    journal.capture_exact_ref(&exact).unwrap();
    journal
        .capture_value(&LegacyValue::F64(exact.clone()))
        .unwrap();
    journal.capture_value_cell(&cell).unwrap();
    journal
        .capture_value(&LegacyValue::Index(legacy_index.clone()))
        .unwrap();

    assert_eq!(journal.cell_count(), 3);
    *exact.borrow_mut() = 40.0;
    *legacy_index.borrow_mut() = 50;
    *cell.borrow_mut() = LegacyValue::Empty;
    journal.restore_before().unwrap();

    assert_eq!(*exact.borrow(), 4.0);
    assert_eq!(*legacy_index.borrow(), 5);
    let LegacyValue::Index(restored) = &*cell.borrow() else {
        panic!("expected restored legacy index")
    };
    assert!(restored.same_handle(&legacy_index));
}
