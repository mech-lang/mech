#![cfg(feature = "runtime_bench_gate_b")]

use mech_engine::__gate_b_resident::{
    PreparedResidentTurn, ResidentEkfBatch, ResidentExecutionError, ResidentFullWrite,
    ResidentTurnSummary,
};
use mech_runtime::__gate_b_recording::{LedgerSequence, ResidentTurnRecorder, TurnFailurePhase};

const INPUT: [f64; 4] = [1.0, 0.1, 24.0, -0.6];

fn commit_prepared(
    resident: &mut ResidentEkfBatch,
    recorder: &mut ResidentTurnRecorder,
    turn: usize,
    input: [f64; 4],
) -> (LedgerSequence, ResidentTurnSummary) {
    let permit = recorder.take_admission_permit(turn).unwrap();
    let prepared = resident.prepare_scheduled_turn(input).unwrap();
    let summary = prepared.summary();
    let sequence = recorder.prepare_commit(permit, prepared).unwrap().commit();
    (sequence, summary)
}

fn finish_prepared(
    prepared: PreparedResidentTurn<'_>,
    recorder: &mut ResidentTurnRecorder,
    permit: mech_runtime::__gate_b_recording::LedgerPermit,
) -> LedgerSequence {
    recorder.prepare_commit(permit, prepared).unwrap().commit()
}

#[test]
fn accepted_commit_is_invisible_until_one_publication_then_retains_owned_receipt() {
    let mut resident = ResidentEkfBatch::new(1);
    let mut recorder = ResidentTurnRecorder::new(2, 0).unwrap();
    let initial = resident.state(0);
    let permit = recorder.take_admission_permit(0).unwrap();
    let prepared = resident.prepare_scheduled_turn(INPUT).unwrap();
    let summary = prepared.summary();

    assert_eq!(prepared.published_epoch(), 0);
    assert_eq!(prepared.published_state(0), initial);
    assert_eq!(summary.before_epoch, 0);
    assert_eq!(summary.after_epoch, 1);
    assert_eq!(summary.touched_slots, 2);
    assert_eq!(summary.changed_slots, 2);
    assert_eq!(summary.dirty_nodes, 15);

    let sequence = finish_prepared(prepared, &mut recorder, permit);
    assert_eq!(sequence.get(), 1);
    assert_eq!(resident.published_epoch(), 1);
    assert_ne!(resident.state(0), initial);
    assert_eq!(recorder.recorded_ledger_len(), 1);

    let first_body = {
        let record = recorder.inspect_last().unwrap();
        assert!(record.accepted);
        assert_eq!(record.turn_id, 1);
        assert_eq!(record.transaction_id, 1);
        assert_eq!(record.input_first, 1);
        assert_eq!(record.input_last, 1);
        assert_eq!(record.failure_kind, None);
        assert_eq!(record.failure_phase, None);
        assert_eq!(record.body.before_epoch(), 0);
        assert_eq!(record.body.after_epoch(), 1);
        assert_eq!(record.body.state_hash(), summary.state_hash);
        assert_eq!(record.body.touched_slots(), 2);
        assert_eq!(record.body.changed_slots(), 2);
        assert_eq!(record.body.dirty_nodes(), 15);
        assert!(record.body.is_accepted());
        assert_eq!(record.body.version(), 0);
        record.body
    };

    commit_prepared(&mut resident, &mut recorder, 1, INPUT);
    assert_eq!(first_body.state_hash(), summary.state_hash);
    let record = recorder.inspect_last().unwrap();
    assert_eq!(record.sequence, 2);
    assert_eq!(record.turn_id, 2);
    assert_eq!(record.transaction_id, 2);
    assert_eq!(record.input_first, 2);
}

#[test]
fn rejected_candidate_records_failure_without_publication_and_epoch_is_not_reused() {
    let mut resident = ResidentEkfBatch::new(1);
    let mut recorder = ResidentTurnRecorder::new(2, 0).unwrap();
    let initial = resident.state(0);
    let permit = recorder.take_admission_permit(0).unwrap();
    let mut invalid = INPUT;
    invalid[2] = f64::NAN;
    let failure = match resident.prepare_scheduled_turn(invalid) {
        Ok(prepared) => {
            prepared.abort();
            panic!("invalid resident input unexpectedly prepared")
        }
        Err(failure) => failure,
    };
    let rejected = recorder
        .prepare_rejected(permit, resident.published_epoch(), failure)
        .unwrap();
    assert_eq!(rejected.append().get(), 1);

    assert_eq!(resident.published_epoch(), 0);
    assert_eq!(resident.state(0), initial);
    let record = recorder.inspect_last().unwrap();
    assert!(!record.accepted);
    assert!(record.failure_kind.is_some());
    assert!(!record.body.is_accepted());
    assert_eq!(record.body.after_epoch(), 0);

    let (_, summary) = commit_prepared(&mut resident, &mut recorder, 1, INPUT);
    assert_eq!(summary.after_epoch, 2);
    assert_eq!(resident.published_epoch(), 2);
}

#[test]
fn record_preparation_failure_automatically_aborts_before_publication() {
    let mut resident = ResidentEkfBatch::new(1);
    let mut recorder = ResidentTurnRecorder::new(2, 0).unwrap();
    let initial = resident.state(0);
    let permit = recorder.take_admission_permit(0).unwrap();
    let prepared = resident.prepare_scheduled_turn(INPUT).unwrap();
    let failed_epoch = prepared.summary().after_epoch;
    recorder.fail_next_preparation_for_test();
    assert!(recorder.prepare_commit(permit, prepared).is_err());
    assert_eq!(resident.published_epoch(), 0);
    assert_eq!(resident.state(0), initial);
    assert!(!resident.candidate_epoch_is_active_for_gate_b(failed_epoch));
    assert_eq!(recorder.recorded_ledger_len(), 0);
    drop(recorder.reserve_additional_permit_for_test().unwrap());

    let permit = recorder.take_admission_permit(1).unwrap();
    let prepared = resident.prepare_scheduled_turn(INPUT).unwrap();
    assert_eq!(prepared.summary().after_epoch, failed_epoch + 1);
    recorder.prepare_commit(permit, prepared).unwrap().commit();
    assert_eq!(resident.published_epoch(), failed_epoch + 1);
    assert_ne!(resident.state(0), initial);
    assert_eq!(recorder.recorded_ledger_len(), 1);
}

#[test]
fn admission_exhaustion_precedes_candidate_execution() {
    let resident = ResidentEkfBatch::new(1);
    let mut recorder = ResidentTurnRecorder::new(1, 0).unwrap();
    drop(recorder.take_admission_permit(0).unwrap());
    assert!(recorder.take_admission_permit(0).is_err());
    assert_eq!(resident.published_epoch(), 0);
    assert_eq!(resident.state(0), ResidentEkfBatch::new(1).state(0));
}

#[test]
fn retained_history_does_not_add_turn_work_or_record_iteration() {
    for history in [0, 1_000, 100_000] {
        let mut resident = ResidentEkfBatch::new(1);
        let mut recorder = ResidentTurnRecorder::new(1, history).unwrap();
        assert_eq!(recorder.records_inspected(), 0);
        let (sequence, summary) = commit_prepared(&mut resident, &mut recorder, 0, INPUT);
        assert_eq!(summary.dirty_nodes, 15);
        assert_eq!(recorder.recorded_ledger_len(), history + 1);
        assert_eq!(recorder.records_inspected(), 0);
        assert_eq!(sequence.get() as usize, history + 1);
        assert_eq!(
            recorder.inspect_last().unwrap().sequence as usize,
            history + 1
        );
        assert_eq!(recorder.records_inspected(), 1);
    }
}

#[test]
fn high_epoch_executes_the_same_trajectory_and_work() {
    let mut low = ResidentEkfBatch::new(1);
    let mut high = ResidentEkfBatch::new(1);
    high.set_next_epoch_for_gate_b(1_000_000_001);
    let low_prepared = low.prepare_scheduled_turn(INPUT).unwrap();
    let high_prepared = high.prepare_scheduled_turn(INPUT).unwrap();
    let low_summary = low_prepared.summary();
    let high_summary = high_prepared.summary();
    assert_eq!(low_summary.state_hash, high_summary.state_hash);
    assert_eq!(low_summary.touched_slots, high_summary.touched_slots);
    assert_eq!(low_summary.changed_slots, high_summary.changed_slots);
    assert_eq!(low_summary.dirty_nodes, high_summary.dirty_nodes);
    low_prepared.publish();
    high_prepared.publish();
    assert_eq!(low.state(0), high.state(0));
    assert_eq!(low.published_epoch(), 1);
    assert_eq!(high.published_epoch(), 1_000_000_001);
}

#[test]
fn rejection_failure_kinds_and_phases_are_stable_and_bounded() {
    let failures = [
        (
            ResidentExecutionError::EpochExhausted,
            TurnFailurePhase::Execution,
            "ResidentEpochExhausted",
        ),
        (
            ResidentExecutionError::LandmarkDistance,
            TurnFailurePhase::Integrity,
            "ResidentLandmarkDistance",
        ),
        (
            ResidentExecutionError::InnovationDeterminant,
            TurnFailurePhase::Integrity,
            "ResidentInnovationDeterminant",
        ),
        (
            ResidentExecutionError::NonFiniteState,
            TurnFailurePhase::Integrity,
            "ResidentNonFiniteState",
        ),
        (
            ResidentExecutionError::CovarianceDiagonal,
            TurnFailurePhase::Integrity,
            "ResidentCovarianceDiagonal",
        ),
        (
            ResidentExecutionError::CovarianceSymmetry,
            TurnFailurePhase::Integrity,
            "ResidentCovarianceSymmetry",
        ),
    ];
    let mut recorder = ResidentTurnRecorder::new(failures.len(), 0).unwrap();
    for (turn, (failure, phase, kind)) in failures.into_iter().enumerate() {
        let permit = recorder.take_admission_permit(turn).unwrap();
        recorder
            .prepare_rejected(permit, 0, failure)
            .unwrap()
            .append();
        let record = recorder.inspect_last().unwrap();
        assert_eq!(record.failure_phase, Some(phase));
        assert_eq!(record.failure_kind, Some(kind));
        assert!(!record.body.is_accepted());
    }
    assert_eq!(recorder.recorded_ledger_len(), 6);
}

#[test]
fn final_turn_identity_is_issued_once_without_wrap_or_reuse() {
    let mut recorder = ResidentTurnRecorder::new(2, 0).unwrap();
    recorder.set_next_turn_identity_for_test(u64::MAX);

    let permit = recorder.take_admission_permit(0).unwrap();
    recorder
        .prepare_rejected(permit, 0, ResidentExecutionError::EpochExhausted)
        .unwrap()
        .append();
    let record = recorder.inspect_last().unwrap();
    assert_eq!(record.turn_id, u64::MAX);
    assert_eq!(record.input_first, u64::MAX);
    assert_eq!(record.transaction_id, u128::from(u64::MAX));

    let permit = recorder.take_admission_permit(1).unwrap();
    assert!(
        recorder
            .prepare_rejected(permit, 0, ResidentExecutionError::EpochExhausted)
            .is_err()
    );
    assert_eq!(recorder.recorded_ledger_len(), 1);
}

#[test]
fn complete_full_write_prepares_receipt_before_one_publication() {
    let mut resident = ResidentFullWrite::new();
    let mut recorder = ResidentTurnRecorder::new(1, 0).unwrap();
    let before_epoch = resident.published_epoch();
    let permit = recorder.take_admission_permit(0).unwrap();
    let prepared = resident.prepare_turn(1.0).unwrap();
    let summary = prepared.summary();
    assert_eq!(summary.before_epoch, before_epoch);
    assert_eq!(summary.after_epoch, 1);
    assert_eq!(summary.touched_slots, 1);
    assert_eq!(summary.changed_slots, 1);
    assert_eq!(summary.dirty_nodes, 1);
    recorder
        .prepare_full_write_commit(permit, prepared)
        .unwrap()
        .commit();
    assert_eq!(resident.published_epoch(), 1);
    assert_eq!(recorder.recorded_ledger_len(), 1);
    let record = recorder.inspect_last().unwrap();
    assert!(record.accepted);
    assert_eq!(record.body.state_hash(), summary.state_hash);
}
