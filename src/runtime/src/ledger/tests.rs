use core::num::NonZeroU64;

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
