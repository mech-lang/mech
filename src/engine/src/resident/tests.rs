use mech_core::InstanceEpoch;

use super::*;

#[test]
fn candidate_reads_observe_own_writes_before_publication() {
    let mut instance = ReactiveInstance::frozen_ekf_batch(1);
    let published = instance.state.published_state(InstanceEpoch(0), 0);
    let mut candidate = instance.begin_candidate([1.0, 0.1, 24.0, -0.6]).unwrap();
    execute_ekf_candidate(&mut candidate).unwrap();
    assert_ne!(
        candidate
            .instance
            .state
            .candidate_state(candidate.candidate_buffer, 0),
        published
    );
    assert!(node_executed(
        &candidate,
        candidate.instance.plan.nodes.len() - 1
    ));
    candidate.abort();
    assert_eq!(
        instance.state.published_state(InstanceEpoch(0), 0),
        published
    );
}

#[test]
fn aborts_reuse_the_same_two_buffers() {
    let mut instance = ReactiveInstance::frozen_ekf_batch(1);
    let addresses = instance.state.buffer_addresses();
    for _ in 0..10_000 {
        let mut candidate = instance.begin_candidate([1.0, 0.1, 24.0, -0.6]).unwrap();
        execute_ekf_candidate(&mut candidate).unwrap();
        candidate.abort();
    }
    assert_eq!(instance.state.buffer_addresses(), addresses);
    assert_eq!(instance.state.states.epochs.iter().flatten().count(), 1);
    assert_eq!(
        instance.state.covariances.epochs.iter().flatten().count(),
        1
    );
}

#[test]
fn maximum_epoch_is_legal_once_and_never_reused() {
    let mut instance = ReactiveInstance::frozen_ekf_batch(1);
    instance.next_epoch = Some(InstanceEpoch(u64::MAX));
    let mut candidate = instance.begin_candidate([1.0, 0.1, 24.0, -0.6]).unwrap();
    assert_eq!(candidate.working_epoch, InstanceEpoch(u64::MAX));
    execute_ekf_candidate(&mut candidate).unwrap();
    candidate.publish();
    assert_eq!(instance.published_epoch(), InstanceEpoch(u64::MAX));
    assert_eq!(instance.next_epoch, None);
    assert!(matches!(
        instance.begin_candidate([1.0, 0.1, 24.0, -0.6]),
        Err(ResidentExecutionError::EpochExhausted)
    ));
}

#[test]
fn accepted_candidate_tracks_touched_invalidated_and_changed_outputs() {
    let mut instance = ReactiveInstance::frozen_ekf_batch(8);
    let mut candidate = instance.begin_candidate([1.0, 0.1, 24.0, -0.6]).unwrap();
    execute_ekf_candidate(&mut candidate).unwrap();
    assert_eq!(candidate.instance.workspace.touched_slots.len(), 16);
    assert_eq!(candidate.instance.workspace.invalidated_slots.len(), 16);
    assert_eq!(candidate.instance.workspace.changed_slots.len(), 16);
    candidate.publish();
}

#[test]
fn aborted_and_accepted_epochs_are_never_reused() {
    let input = [1.0, 0.1, 24.0, -0.6];
    let mut instance = ReactiveInstance::frozen_ekf_batch(1);
    let first = instance.begin_candidate(input).unwrap();
    let rejected_epoch = first.working_epoch;
    first.abort();
    let mut second = instance.begin_candidate(input).unwrap();
    let accepted_epoch = second.working_epoch;
    assert!(accepted_epoch.0 > rejected_epoch.0);
    execute_ekf_candidate(&mut second).unwrap();
    second.publish();
    let third = instance.begin_candidate(input).unwrap();
    assert!(third.working_epoch.0 > accepted_epoch.0);
    third.abort();
    assert_eq!(instance.published_epoch(), accepted_epoch);
}
