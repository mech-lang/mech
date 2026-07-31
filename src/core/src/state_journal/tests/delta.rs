use super::support::{scalar, scalar_value};
use crate::{
    ValueStateAfterAlreadyRecorded, ValueStateAfterNotRecorded, ValueStateJournal,
    ValueStateJournalSealed,
};

#[test]
fn state_journal_delta_rewinds_and_replays_repeatedly() {
    let cell = scalar(1.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&cell)).unwrap();
    *cell.borrow_mut() = 2.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    assert_eq!(*cell.borrow(), 1.0);
    delta.replay().unwrap();
    assert_eq!(*cell.borrow(), 2.0);
    delta.rewind().unwrap();
    assert_eq!(*cell.borrow(), 1.0);

    *cell.borrow_mut() = 99.0;
    delta.replay().unwrap();
    assert_eq!(*cell.borrow(), 2.0);
}

#[test]
fn state_journal_lifecycle_errors_are_structured() {
    let cell = scalar(1.0);
    let mut unrecorded = ValueStateJournal::new();
    unrecorded.capture_value(&scalar_value(&cell)).unwrap();
    let error = match unrecorded.into_delta() {
        Ok(_) => panic!("unrecorded journal unexpectedly produced a delta"),
        Err(error) => error,
    };
    assert!(error.kind_as::<ValueStateAfterNotRecorded>().is_some());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&cell)).unwrap();
    journal.record_after().unwrap();
    assert!(
        journal
            .record_after()
            .unwrap_err()
            .kind_as::<ValueStateAfterAlreadyRecorded>()
            .is_some()
    );
    assert!(
        journal
            .capture_value(&scalar_value(&cell))
            .unwrap_err()
            .kind_as::<ValueStateJournalSealed>()
            .is_some()
    );
}
