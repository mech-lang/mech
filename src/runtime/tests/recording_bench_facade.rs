use mech_runtime::__gate_a_recording::{
    AccountedRecord, OwnedTurnRecordQueue, RecordEstimate, RetainedTurnLedger, prepare_retained,
    reserve_retained,
};

#[test]
fn hidden_probe_facade_exposes_the_minimum_recording_surface() {
    let mut ledger = RetainedTurnLedger::new(1, 4).unwrap();
    let record = vec![1_u8, 2, 3, 4].into_boxed_slice();
    let permit = reserve_retained(
        &ledger,
        RecordEstimate {
            records: 1,
            bytes: record.retained_bytes(),
        },
    )
    .unwrap();
    let prepared = prepare_retained(&mut ledger, permit, record).unwrap();
    assert_eq!(prepared.append().get(), 1);

    let queue = OwnedTurnRecordQueue::<Box<[u8]>>::new(1, 4).unwrap();
    assert!(queue.is_empty());
}

#[test]
fn hidden_probe_facade_rejects_unapproved_external_operations() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/recording_bench/accounted_record_external.rs");
    tests.compile_fail("tests/ui/recording_bench/cross_destination_append.rs");
}
