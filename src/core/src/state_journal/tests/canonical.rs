#[cfg(feature = "functions")]
use crate::Ref;
use crate::{CanonicalStateJournal, ValueCell, ValueData, ValueDataDraft};

fn index_cell(value: usize) -> ValueCell {
    ValueCell::from_exact(value).unwrap()
}

fn read_index(cell: &ValueCell) -> usize {
    let snapshot = cell.snapshot().unwrap();
    let ValueData::Index(value) = snapshot.data() else {
        panic!("index cell changed representation")
    };
    usize::try_from(*value).unwrap()
}

fn replace_index(cell: &ValueCell, value: usize) {
    let replacement = cell
        .rebuild_data_draft(ValueDataDraft::Index(value as u64))
        .unwrap();
    cell.replace(&replacement).unwrap();
}

#[test]
fn canonical_cells_restore_contents_without_replacing_identity() {
    let cell = index_cell(1);
    let alias = cell.clone();
    let mut journal = CanonicalStateJournal::new();

    journal.capture_value_cell(&cell).unwrap();
    replace_index(&cell, 2);
    journal.restore_before().unwrap();

    assert!(cell.same_cell(&alias));
    assert_eq!(read_index(&cell), 1);
}

#[test]
fn canonical_cells_are_deduplicated_by_identity() {
    let first = index_cell(1);
    let second = index_cell(1);
    let mut journal = CanonicalStateJournal::new();

    journal.capture_value_cell(&first).unwrap();
    journal.capture_value_cell(&first).unwrap();
    journal.capture_value_cell(&second).unwrap();

    assert_eq!(journal.cell_count(), 2);
}

#[test]
#[cfg(feature = "functions")]
fn exact_cells_restore_contents_without_replacing_handles() {
    let cell = Ref::new(vec![1usize, 2]);
    let alias = cell.clone();
    let mut journal = CanonicalStateJournal::new();

    journal.capture_exact_ref(&cell).unwrap();
    *cell.borrow_mut() = vec![3, 4, 5];
    journal.restore_before().unwrap();

    assert!(cell.same_handle(&alias));
    assert_eq!(cell.borrow().as_slice(), &[1, 2]);
}

#[test]
#[cfg(feature = "functions")]
fn committed_delta_rewinds_and_replays_canonical_and_exact_state() {
    let canonical = index_cell(1);
    let exact = Ref::new(vec![1usize]);
    let mut journal = CanonicalStateJournal::new();
    journal.capture_value_cell(&canonical).unwrap();
    journal.capture_exact_ref(&exact).unwrap();

    replace_index(&canonical, 2);
    *exact.borrow_mut() = vec![2, 3];
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    assert_eq!(read_index(&canonical), 1);
    assert_eq!(exact.borrow().as_slice(), &[1]);

    delta.replay().unwrap();
    assert_eq!(read_index(&canonical), 2);
    assert_eq!(exact.borrow().as_slice(), &[2, 3]);
}

#[test]
fn record_after_seals_the_journal() {
    let first = index_cell(1);
    let second = index_cell(2);
    let mut journal = CanonicalStateJournal::new();
    journal.capture_value_cell(&first).unwrap();
    journal.record_after().unwrap();

    let error = journal.capture_value_cell(&second).unwrap_err();
    assert_eq!(error.kind_name(), "ValueStateJournalSealed");
}
