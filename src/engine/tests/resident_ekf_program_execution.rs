#![cfg(feature = "resident-artifact")]

use mech_core::{InstanceEpoch, MResult, ReactiveInstanceId, ResidentValueRef};
use mech_engine::__gate_b_resident::ResidentEkfBatch;
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, FrozenEkfCompilationServices, ReactiveInstance,
    ResidentExecutionError, ResidentStorageClass, ResidentTurnSummary, ResidentValueBorrow,
    activate, compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
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

fn state(instance: &ReactiveInstance) -> [f64; 12] {
    let mut result = [0.0; 12];
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let ResidentValueBorrow::F64 { values, .. } =
            instance.state_borrow(slot.artifact_id).unwrap()
        else {
            panic!("EKF state is f64")
        };
        match values.len() {
            3 => result[..3].copy_from_slice(values),
            9 => result[3..].copy_from_slice(values),
            _ => panic!("unexpected EKF state shape"),
        }
    }
    result
}

fn execute_turn(
    instance: &mut ReactiveInstance,
    frame: &[f64; 4],
) -> Result<ResidentTurnSummary, ResidentExecutionError> {
    let input = CapturedSignalInput {
        slot: instance.plan.inputs[0].slot,
        value: ResidentValueRef::F64(frame),
    };
    instance.turn(&[input])
}

fn instance(id: u32) -> MResult<ReactiveInstance> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    Ok(activate(
        ReactiveInstanceId::new(id, 0),
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap())
}

#[test]
fn source_and_bytecode_artifacts_execute_the_complete_frozen_trace() -> MResult<()> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    let mut source = activate(
        ReactiveInstanceId::new(0, 0),
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let mut decoded = activate(
        ReactiveInstanceId::new(1, 0),
        &compilation.decoded_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let mut control = ResidentEkfBatch::new(1);
    let mut trajectory = Vec::with_capacity(TURNS);

    for (turn, frame) in frames().enumerate() {
        let source_receipt = execute_turn(&mut source, &frame).expect("source artifact turn");
        let decoded_receipt = execute_turn(&mut decoded, &frame).expect("bytecode artifact turn");
        control.turn(frame).expect("Gate B control turn");
        assert_eq!(state(&source), state(&decoded), "turn {turn}");
        let control_state = control.state(0);
        assert_eq!(&state(&source)[..3], &control_state.state, "turn {turn}");
        assert_eq!(
            &state(&source)[3..],
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
fn abort_and_integrity_failure_leave_publication_unchanged() -> MResult<()> {
    let mut instance = instance(0)?;
    let frame = frames().next().unwrap();
    execute_turn(&mut instance, &frame).unwrap();
    let published = state(&instance);
    let epoch = instance.published_epoch();

    let input = CapturedSignalInput {
        slot: instance.plan.inputs[0].slot,
        value: ResidentValueRef::F64(&frame),
    };
    instance.prepare_turn(&[input]).unwrap().abort();
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(state(&instance), published);

    let mut invalid = frame;
    invalid[0] = f64::NAN;
    assert!(matches!(
        execute_turn(&mut instance, &invalid),
        Err(ResidentExecutionError::Integrity { .. })
    ));
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(state(&instance), published);
    let probe = instance.structural_probe();
    assert_eq!(probe.candidate_seed_bytes, 0);
    assert_eq!(probe.candidate_materialized_bytes, 96);
    assert_eq!(probe.published_buffer_copy_bytes, 0);
    assert_eq!(probe.publication_store_count, 1);
    Ok(())
}

#[test]
fn maximum_epoch_publishes_once_then_exhausts() -> MResult<()> {
    let mut instance = instance(0)?;
    instance.set_next_epoch_for_test(u64::MAX);
    let frame = frames().next().unwrap();
    let receipt = execute_turn(&mut instance, &frame).unwrap();
    assert_eq!(receipt.after_epoch, InstanceEpoch::new(u64::MAX));
    assert_eq!(instance.published_epoch(), InstanceEpoch::new(u64::MAX));
    assert_eq!(
        execute_turn(&mut instance, &frame),
        Err(ResidentExecutionError::EpochExhausted)
    );
    Ok(())
}

#[test]
fn odd_and_even_publications_follow_the_two_sparse_buffers() -> MResult<()> {
    let mut instance = instance(0)?;
    let mut control = ResidentEkfBatch::new(1);
    for (turn, frame) in frames().take(2).enumerate() {
        execute_turn(&mut instance, &frame).unwrap();
        control.turn(frame).unwrap();
        let expected = control.state(0);
        assert_eq!(&state(&instance)[..3], &expected.state, "turn {}", turn + 1);
        assert_eq!(
            &state(&instance)[3..],
            &expected.covariance,
            "turn {}",
            turn + 1
        );
    }
    Ok(())
}
