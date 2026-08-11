#![cfg(feature = "resident-ekf-artifact")]

use mech_core::{InstanceEpoch, MResult, ReactiveInstanceId};
use mech_engine::__gate_b_resident::ResidentEkfBatch;
use mech_engine::__gate_d::{
    ActivatedNodeKind, FrozenEkfCompilationServices, ReactiveInstance, ResidentExecutionError,
    ResidentTurnSummary, activate, compile_frozen_ekf_source,
};
use sha2::{Digest, Sha256};

const SOURCE: &str =
    include_str!("../../../tests/architecture/resident-activation/ekf-source-v1.mec");
const TRACE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/runtime/gate-b/ekf-input-v1.bin"
));
const TURNS: usize = 4_096;
const EXPECTED_HASH: &str = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758";

fn frames() -> impl Iterator<Item = [f64; 4]> {
    assert_eq!(TRACE.len(), TURNS * 32);
    TRACE.chunks_exact(32).map(|row| {
        let value = |offset| f64::from_le_bytes(row[offset..offset + 8].try_into().unwrap());
        [value(0), value(8), value(16), value(24)]
    })
}

fn quantized_hash(states: impl IntoIterator<Item = [f64; 12]>) -> String {
    let mut hash = Sha256::new();
    for state in states {
        for value in state {
            hash.update(((value / 1.0e-10).round() as i64).to_le_bytes());
        }
    }
    format!("{:x}", hash.finalize())
}

fn state(instance: &mech_engine::__gate_d::ReactiveInstance) -> [f64; 12] {
    let mut state = [0.0; 12];
    state[..3].copy_from_slice(instance.estimate());
    state[3..].copy_from_slice(instance.covariance());
    state
}

fn execute_turn(
    instance: &mut ReactiveInstance,
    frame: [f64; 4],
) -> Result<ResidentTurnSummary, ResidentExecutionError> {
    Ok(instance.prepare_turn(frame)?.publish())
}

#[test]
fn source_and_bytecode_artifacts_execute_the_complete_frozen_trace() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut source = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let mut decoded = activate(
        ReactiveInstanceId::new(1, 0),
        &compilation.decoded_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let mut control = ResidentEkfBatch::new(1);
    let mut trajectory = Vec::with_capacity(TURNS);

    for (turn, frame) in frames().enumerate() {
        let source_receipt = execute_turn(&mut source, frame).expect("source artifact turn");
        let decoded_receipt = execute_turn(&mut decoded, frame).expect("bytecode artifact turn");
        control.turn(frame).expect("Gate B control turn");
        assert_eq!(state(&source), state(&decoded), "turn {turn}");
        let control_state = control.state(0);
        assert_eq!(source.estimate(), &control_state.state, "turn {turn}");
        assert_eq!(
            source.covariance(),
            &control_state.covariance,
            "turn {turn}"
        );
        assert_eq!(source_receipt.before_epoch, decoded_receipt.before_epoch);
        assert_eq!(source_receipt.after_epoch, decoded_receipt.after_epoch);
        assert_eq!(source_receipt.state_hash, decoded_receipt.state_hash);
        assert_eq!(source_receipt.touched_slots, 2);
        assert_eq!(source_receipt.dirty_nodes, 20);
        trajectory.push(state(&source));
    }

    assert_eq!(quantized_hash(trajectory), EXPECTED_HASH);
    assert_eq!(source.published_epoch(), InstanceEpoch::new(TURNS as u64));
    assert_eq!(decoded.published_epoch(), source.published_epoch());
    Ok(())
}

#[test]
fn abort_and_integrity_failure_leave_publication_unchanged_and_reuse_storage() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let frame = frames().next().unwrap();
    execute_turn(&mut instance, frame).unwrap();
    let published = state(&instance);
    let epoch = instance.published_epoch();

    let prepared = instance.prepare_turn(frame).unwrap();
    assert_eq!(prepared.published_estimate(), &published[..3]);
    prepared.abort();
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(state(&instance), published);

    let mut invalid = frame;
    invalid[0] = f64::NAN;
    assert_eq!(
        execute_turn(&mut instance, invalid),
        Err(ResidentExecutionError::NonFiniteState)
    );
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(state(&instance), published);
    assert_eq!(instance.structural_probe().candidate_seed_bytes, 0);
    assert_eq!(instance.structural_probe().candidate_written_bytes, 96);
    assert_eq!(instance.structural_probe().published_buffer_copy_bytes, 0);
    assert_eq!(instance.structural_probe().publication_store_count, 1);
    Ok(())
}

#[test]
fn maximum_epoch_publishes_once_then_exhausts() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    instance.set_next_epoch_for_d1_test(u64::MAX);
    let receipt = execute_turn(&mut instance, frames().next().unwrap()).unwrap();
    assert_eq!(receipt.after_epoch, InstanceEpoch::new(u64::MAX));
    assert_eq!(instance.published_epoch(), InstanceEpoch::new(u64::MAX));
    assert_eq!(
        execute_turn(&mut instance, frames().next().unwrap()),
        Err(ResidentExecutionError::EpochExhausted)
    );
    assert_eq!(instance.published_epoch(), InstanceEpoch::new(u64::MAX));
    Ok(())
}

#[test]
fn independently_activated_instances_keep_separate_epochs_and_state() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut left = activate(
        ReactiveInstanceId::new(7, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let mut right = activate(
        ReactiveInstanceId::new(8, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();

    execute_turn(&mut left, [1.0, 0.01, 20.0, 0.1]).unwrap();
    execute_turn(&mut right, [3.0, -0.02, 18.0, -0.1]).unwrap();
    assert_eq!(left.published_epoch(), InstanceEpoch::new(1));
    assert_eq!(right.published_epoch(), InstanceEpoch::new(1));
    assert_ne!(left.estimate(), right.estimate());
    assert_ne!(left.covariance(), right.covariance());
    Ok(())
}

#[test]
fn repeated_identical_aborted_frames_skip_numeric_work_but_complete_the_candidate() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let frame = frames().next().unwrap();
    let first = instance.execute_then_abort(frame).unwrap();
    let second = instance.execute_then_abort(frame).unwrap();
    assert_eq!(first.dirty_nodes, 20);
    assert!(second.dirty_nodes < first.dirty_nodes);
    assert!(second.dirty_nodes >= 5);
    assert_eq!(second.touched_slots, 2);
    assert!(
        instance
            .workspace
            .predicate_values()
            .iter()
            .all(|value| *value)
    );
    assert_eq!(instance.published_epoch(), InstanceEpoch::ZERO);
    Ok(())
}

#[test]
fn published_readers_follow_both_candidate_buffers() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let mut control = ResidentEkfBatch::new(1);
    for (turn, frame) in frames().take(2).enumerate() {
        execute_turn(&mut instance, frame).unwrap();
        control.turn(frame).unwrap();
        let expected = control.state(0);
        assert_eq!(instance.estimate(), &expected.state, "turn {}", turn + 1);
        assert_eq!(
            instance.covariance(),
            &expected.covariance,
            "turn {}",
            turn + 1
        );
        assert_eq!(
            instance.published_epoch(),
            InstanceEpoch::new((turn + 1) as u64)
        );
    }
    Ok(())
}

#[test]
fn malformed_plan_cannot_publish_an_incomplete_candidate() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &compilation.resource_request,
    )
    .unwrap();
    let omitted = instance
        .plan
        .nodes
        .iter()
        .position(|node| matches!(node.kind, ActivatedNodeKind::StateCopy { .. }))
        .unwrap() as u32;
    instance.plan.topology.linear_node_order = instance
        .plan
        .topology
        .linear_node_order
        .iter()
        .copied()
        .filter(|node| node.get() != omitted)
        .collect();
    let before = state(&instance);
    assert_eq!(
        execute_turn(&mut instance, frames().next().unwrap()),
        Err(ResidentExecutionError::IncompleteCandidate)
    );
    assert_eq!(instance.published_epoch(), InstanceEpoch::ZERO);
    assert_eq!(state(&instance), before);
    Ok(())
}
