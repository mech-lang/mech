use core::num::NonZeroU64;
use std::sync::{Arc, Barrier};

use super::*;

fn capacity(records: usize, bytes: usize) -> CapacityController {
    CapacityController::new(records, bytes).unwrap()
}

#[test]
fn reservation_assigns_a_sequence_and_drop_returns_capacity() {
    let controller = capacity(1, 8);
    let permit = reserve(
        &controller,
        RecordEstimate {
            records: 1,
            bytes: 8,
        },
    )
    .unwrap();
    assert_eq!(permit.sequence().get(), 1);
    assert_eq!(controller.reserved().records, 1);
    drop(permit);
    assert_eq!(controller.reserved(), RecordEstimate::default());
}

#[test]
fn prepare_binds_actual_bytes_and_returns_overestimate() {
    let controller = capacity(2, 32);
    let permit = reserve(
        &controller,
        RecordEstimate {
            records: 2,
            bytes: 32,
        },
    )
    .unwrap();
    let prepared = prepare(&controller, permit, vec![1_u8; 8]).unwrap();
    assert_eq!(prepared.retained_bytes(), 8);
    assert_eq!(controller.reserved().records, 1);
    assert_eq!(controller.reserved().bytes, 8);
    drop(prepared);
    assert_eq!(controller.reserved(), RecordEstimate::default());
}

#[test]
fn underestimated_and_wrong_ledger_permits_release_capacity() {
    let first = capacity(1, 8);
    let second = capacity(1, 8);
    let underestimated = reserve(
        &first,
        RecordEstimate {
            records: 1,
            bytes: 4,
        },
    )
    .unwrap();
    assert_eq!(
        prepare(&first, underestimated, vec![0_u8; 8])
            .unwrap_err()
            .kind_name(),
        "LedgerCapacityExceeded"
    );
    assert_eq!(first.reserved(), RecordEstimate::default());

    let wrong_ledger = reserve(
        &first,
        RecordEstimate {
            records: 1,
            bytes: 8,
        },
    )
    .unwrap();
    assert_eq!(
        prepare(&second, wrong_ledger, vec![0_u8; 8])
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );
    assert_eq!(first.reserved(), RecordEstimate::default());
}

#[test]
fn generation_and_sequence_exhaustion_are_checked() {
    let controller = capacity(1, 8);
    let stale = reserve(
        &controller,
        RecordEstimate {
            records: 1,
            bytes: 8,
        },
    )
    .unwrap();
    controller.force_generation_for_test(NonZeroU64::new(2).unwrap());
    assert_eq!(
        prepare(&controller, stale, vec![0_u8; 8])
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );

    let exhausted = capacity(1, 8);
    exhausted.set_sequence_for_test(NonZeroU64::MAX);
    let last = reserve(
        &exhausted,
        RecordEstimate {
            records: 1,
            bytes: 8,
        },
    )
    .unwrap();
    assert_eq!(last.sequence().get(), u64::MAX);
    drop(last);
    assert_eq!(
        reserve(
            &exhausted,
            RecordEstimate {
                records: 1,
                bytes: 8,
            },
        )
        .unwrap_err()
        .kind_name(),
        "SequenceExhausted"
    );
}

fn boxed(bytes: usize) -> Box<[u8]> {
    vec![0_u8; bytes].into_boxed_slice()
}

fn append_retained(ledger: &mut RetainedTurnLedger<Box<[u8]>>, bytes: usize) -> LedgerSequence {
    let permit = ledger
        .reserve(RecordEstimate { records: 1, bytes })
        .unwrap();
    let prepared = ledger.prepare_append(permit, boxed(bytes)).unwrap();
    ledger.append(prepared)
}

#[test]
fn retained_ledger_is_fifo_exact_and_never_silently_evicts() {
    let mut ledger = RetainedTurnLedger::new(2, 5).unwrap();
    let first = append_retained(&mut ledger, 2);
    let second = append_retained(&mut ledger, 3);
    assert_eq!(ledger.len(), 2);
    assert_eq!(ledger.retained_bytes(), 5);
    assert_eq!(
        ledger
            .reserve(RecordEstimate {
                records: 1,
                bytes: 1,
            })
            .unwrap_err()
            .kind_name(),
        "LedgerCapacityExceeded"
    );
    assert_eq!(
        ledger.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(ledger.pop_front().unwrap().0, first);
    assert_eq!(ledger.retained_bytes(), 3);
    let drained = ledger.drain().collect::<Vec<_>>();
    assert_eq!(drained[0].0, second);
    assert!(ledger.is_empty());
    assert_eq!(ledger.retained_bytes(), 0);
    assert!(
        ledger
            .reserve(RecordEstimate {
                records: 2,
                bytes: 5,
            })
            .is_ok()
    );
}

#[test]
fn queued_record_crosses_threads_and_drain_returns_capacity() {
    let queue = OwnedTurnRecordQueue::new(2, 8).unwrap();
    let permit = queue
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = queue.prepare_append(permit, boxed(4)).unwrap();
    let sequence = queue.append(prepared);

    let consumer = queue.clone();
    let drained = std::thread::spawn(move || consumer.drain()).join().unwrap();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].0, sequence);
    assert_eq!(&*drained[0].1, &[0_u8; 4]);
    assert!(queue.is_empty());
    assert_eq!(queue.retained_bytes(), 0);
    assert!(
        queue
            .reserve(RecordEstimate {
                records: 2,
                bytes: 8,
            })
            .is_ok()
    );
}

#[test]
fn prepared_queue_append_survives_health_change_and_poison_recovery() {
    let queue = OwnedTurnRecordQueue::new(1, 4).unwrap();
    let permit = queue
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = queue.prepare_append(permit, boxed(4)).unwrap();
    queue.mark_writer_unhealthy();
    assert!(!queue.writer_is_healthy());
    assert_eq!(
        queue
            .reserve(RecordEstimate {
                records: 1,
                bytes: 1,
            })
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );
    queue.poison_mutex_for_test();
    queue.append(prepared);
    assert_eq!(queue.len(), 1);
}

#[test]
fn multiple_unused_permits_are_allowed_but_only_one_append_may_be_prepared() {
    let mut ledger = RetainedTurnLedger::new(2, 8).unwrap();
    let first = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let second = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    assert_eq!(first.sequence().get(), 1);
    assert_eq!(second.sequence().get(), 2);

    let prepared_first = ledger.prepare_append(first, boxed(4)).unwrap();
    let error = ledger.prepare_append(second, boxed(4)).unwrap_err();
    assert_eq!(error.kind_name(), "LedgerPermitInvalid");
    assert_eq!(ledger.append(prepared_first).get(), 1);
    assert_eq!(
        ledger
            .iter()
            .map(|(sequence, _)| sequence.get())
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn wrong_ledger_preparation_is_rejected_before_append() {
    let first = RetainedTurnLedger::<Box<[u8]>>::new(1, 4).unwrap();
    let second = RetainedTurnLedger::<Box<[u8]>>::new(1, 4).unwrap();
    let permit = first
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();

    assert_eq!(
        second
            .prepare_append(permit, boxed(4))
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );
    assert!(
        first
            .reserve(RecordEstimate {
                records: 1,
                bytes: 4,
            })
            .is_ok()
    );
}

#[test]
fn invalid_owned_turn_record_is_rejected_during_preparation() {
    let ledger = RetainedTurnLedger::new(1, 4).unwrap();
    let permit = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let invalid = crate::turn_record::OwnedTurnRecord {
        header: crate::turn_record::TurnRecordHeader {
            turn_id: crate::turn_record::TurnId::new(1).unwrap(),
            transaction_id: crate::TransactionId::ZERO,
            input_range: None,
            status: crate::turn_record::TurnRecordStatus::Accepted,
            failure: None,
        },
        body: boxed(4),
    };

    assert_eq!(
        ledger
            .prepare_append(permit, invalid)
            .unwrap_err()
            .kind_name(),
        "InvalidTurnRecord"
    );
    assert!(ledger.is_empty());
    assert!(
        ledger
            .reserve(RecordEstimate {
                records: 1,
                bytes: 4,
            })
            .is_ok()
    );
}

#[test]
fn internal_wrong_destination_assertion_preserves_origin_capacity() {
    let first = OwnedTurnRecordQueue::new(1, 4).unwrap();
    let second = OwnedTurnRecordQueue::new(1, 4).unwrap();
    let permit = first
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = first.prepare_append(permit, boxed(4)).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        second.append(prepared);
    }));
    assert!(result.is_err());
    assert!(first.is_empty());
    assert!(second.is_empty());
    assert!(
        first
            .reserve(RecordEstimate {
                records: 1,
                bytes: 4,
            })
            .is_ok()
    );
}

#[test]
fn dropping_prepared_record_releases_lease_and_bound_capacity() {
    let mut ledger = RetainedTurnLedger::new(1, 4).unwrap();
    let permit = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = ledger.prepare_append(permit, boxed(4)).unwrap();
    drop(prepared);

    let retry = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = ledger.prepare_append(retry, boxed(4)).unwrap();
    assert_eq!(ledger.append(prepared).get(), 2);
}

#[test]
fn older_and_duplicate_sequences_are_rejected_after_drain_but_newer_appends() {
    let mut ledger = RetainedTurnLedger::new(2, 8).unwrap();
    let older = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let newer = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = ledger.prepare_append(newer, boxed(4)).unwrap();
    assert_eq!(ledger.append(prepared).get(), 2);
    assert_eq!(ledger.drain().count(), 1);

    assert_eq!(
        ledger
            .prepare_append(older, boxed(4))
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );

    ledger.set_sequence_for_test(NonZeroU64::new(2).unwrap());
    let duplicate = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    assert_eq!(
        ledger
            .prepare_append(duplicate, boxed(4))
            .unwrap_err()
            .kind_name(),
        "LedgerPermitInvalid"
    );

    let next = ledger
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let prepared = ledger.prepare_append(next, boxed(4)).unwrap();
    assert_eq!(ledger.append(prepared).get(), 3);
}

#[test]
fn cloned_queue_producers_race_for_one_preparation_lease() {
    let queue = OwnedTurnRecordQueue::new(2, 8).unwrap();
    let first = queue
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let second = queue
        .reserve(RecordEstimate {
            records: 1,
            bytes: 4,
        })
        .unwrap();
    let start = Arc::new(Barrier::new(2));
    let finish = Arc::new(Barrier::new(2));

    let handles = [(queue.clone(), first), (queue.clone(), second)]
        .into_iter()
        .map(|(producer, permit)| {
            let start = Arc::clone(&start);
            let finish = Arc::clone(&finish);
            std::thread::spawn(move || {
                start.wait();
                let prepared = producer.prepare_append(permit, boxed(4));
                finish.wait();
                prepared.is_ok()
            })
        })
        .collect::<Vec<_>>();

    let successes = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    assert!(queue.is_empty());
    assert!(
        queue
            .reserve(RecordEstimate {
                records: 2,
                bytes: 8,
            })
            .is_ok()
    );
}

#[test]
fn pool_segments_are_exclusive_bounded_and_reused() {
    let pool = RecordBufferPool::new(1, 8, 8).unwrap();
    let mut segment = pool.acquire(8).unwrap();
    segment.try_extend_from_slice(&[1, 2, 3, 4]).unwrap();
    assert_eq!(segment.as_slice(), &[1, 2, 3, 4]);
    assert_eq!(
        pool.acquire(1).unwrap_err().kind_name(),
        "RecordBufferPoolExhausted"
    );
    drop(segment);
    let recycled = pool.acquire(4).unwrap();
    assert!(recycled.is_empty());
    assert_eq!(pool.stats().allocations, 1);
    assert_eq!(pool.stats().reuses, 1);
}

#[test]
fn oversized_pool_segments_are_dropped_instead_of_retained() {
    let pool = RecordBufferPool::new(1, 16, 4).unwrap();
    let segment = pool.acquire(8).unwrap();
    assert_eq!(pool.stats().total_capacity, 8);
    drop(segment);
    assert_eq!(pool.stats().available_segments, 0);
    assert_eq!(pool.stats().total_capacity, 0);
    assert_eq!(pool.stats().dropped_oversized, 1);
}

#[test]
fn pool_replaces_the_largest_undersized_available_segment() {
    let pool = RecordBufferPool::new(2, 9, 9).unwrap();
    let small = pool.acquire(2).unwrap();
    let large = pool.acquire(6).unwrap();
    drop(small);
    drop(large);

    let replacement = pool.acquire(7).unwrap();
    assert_eq!(replacement.capacity(), 7);
    assert_eq!(pool.stats().total_capacity, 9);
}

#[test]
fn pool_replaces_an_available_segment_when_bytes_are_exhausted_first() {
    let pool = RecordBufferPool::new(3, 9, 9).unwrap();
    let available = pool.acquire(6).unwrap();
    drop(available);

    let replacement = pool.acquire(7).unwrap();
    assert_eq!(replacement.capacity(), 7);
    assert_eq!(pool.stats().total_capacity, 7);
}

#[test]
fn pool_fill_never_grows_a_segment_implicitly() {
    let pool = RecordBufferPool::new(1, 4, 4).unwrap();
    let mut segment = pool.acquire(4).unwrap();
    segment.try_extend_from_slice(&[1, 2, 3, 4]).unwrap();
    assert_eq!(
        segment.try_extend_from_slice(&[5]).unwrap_err().kind_name(),
        "RecordBufferCapacityExceeded"
    );
    assert_eq!(segment.capacity(), 4);
}
