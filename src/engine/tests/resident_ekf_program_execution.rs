#![cfg(feature = "resident-artifact")]

use mech_core::{
    BoundResidentKernel, ChangeDetectionPolicy, InstanceEpoch, MResult, ReactiveInstanceId,
    ResidentKernelError, ResidentKernelInputs, ResidentValueMut, ResidentValueRef,
};
use mech_engine::__gate_b_resident::ResidentEkfBatch;
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, FrozenEkfCompilationServices, ReactiveInstance,
    ResidentActivationOptions, ResidentExecutionError, ResidentIntegrityMode, ResidentStorageClass,
    ResidentTurnSummary, ResidentValueBorrow, activate, activate_with_options,
    compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
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
    instance_with_integrity(id, ResidentIntegrityMode::Checked)
}

fn instance_with_integrity(id: u32, integrity: ResidentIntegrityMode) -> MResult<ReactiveInstance> {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
    let catalog = frozen_ekf_compiler_catalog()?;
    Ok(activate_with_options(
        ReactiveInstanceId::new(id, 0),
        &compilation.source_artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions { integrity },
    )
    .unwrap())
}

#[test]
fn unchecked_integrity_is_explicit_and_omits_constraint_only_nodes() -> MResult<()> {
    let mut checked = instance_with_integrity(20, ResidentIntegrityMode::Checked)?;
    let mut unchecked = instance_with_integrity(21, ResidentIntegrityMode::Unchecked)?;
    let frame = frames().next().unwrap();

    let checked_summary = execute_turn(&mut checked, &frame).unwrap();
    let unchecked_summary = execute_turn(&mut unchecked, &frame).unwrap();
    assert_eq!(state(&checked), state(&unchecked));
    assert_eq!(checked_summary.dirty_nodes, 20);
    assert_eq!(unchecked_summary.dirty_nodes, 17);

    let mut invalid = frame;
    invalid[0] = f64::NAN;
    assert!(matches!(
        execute_turn(&mut checked, &invalid),
        Err(ResidentExecutionError::Integrity { .. })
    ));
    assert!(execute_turn(&mut unchecked, &invalid).is_ok());
    assert!(state(&unchecked).iter().any(|value| !value.is_finite()));
    Ok(())
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

#[test]
fn dropped_prepare_and_identical_retry_match_a_fresh_instance() -> MResult<()> {
    let frame = frames().next().unwrap();
    let mut retried = instance(7)?;
    let mut fresh = instance(7)?;
    let input = CapturedSignalInput {
        slot: retried.plan.inputs[0].slot,
        value: ResidentValueRef::F64(&frame),
    };

    drop(retried.prepare_turn(&[input]).unwrap());
    let actual = execute_turn(&mut retried, &frame).unwrap();
    let expected = execute_turn(&mut fresh, &frame).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(state(&retried), state(&fresh));
    Ok(())
}

fn partial_write_then_fail(
    _kernel: &BoundResidentKernel,
    _inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::F64(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    output[0] = 123_456.0;
    Err(ResidentKernelError::Arithmetic)
}

#[test]
fn partial_kernel_failure_and_retry_match_a_fresh_instance() -> MResult<()> {
    let frame = frames().next().unwrap();
    let mut retried = instance(8)?;
    let mut fresh = instance(8)?;
    let original = retried.plan.nodes[0].kernel.clone();
    retried.plan.nodes[0].kernel = BoundResidentKernel::new(partial_write_then_fail, Box::new([]));

    assert!(matches!(
        execute_turn(&mut retried, &frame),
        Err(ResidentExecutionError::Kernel {
            error: ResidentKernelError::Arithmetic,
            ..
        })
    ));
    retried.plan.nodes[0].kernel = original;
    let actual = execute_turn(&mut retried, &frame).unwrap();
    let expected = execute_turn(&mut fresh, &frame).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(state(&retried), state(&fresh));
    Ok(())
}

#[test]
fn integrity_failure_and_valid_retry_match_a_fresh_instance() -> MResult<()> {
    let frame = frames().next().unwrap();
    let mut retried = instance(9)?;
    let mut fresh = instance(9)?;
    let mut invalid = frame;
    invalid[0] = f64::NAN;

    assert!(matches!(
        execute_turn(&mut retried, &invalid),
        Err(ResidentExecutionError::Integrity { .. })
    ));
    let actual = execute_turn(&mut retried, &frame).unwrap();
    let expected = execute_turn(&mut fresh, &frame).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(state(&retried), state(&fresh));
    Ok(())
}

#[test]
fn forgotten_prepare_blocks_a_second_candidate() -> MResult<()> {
    let frame = frames().next().unwrap();
    let mut instance = instance(10)?;
    let input = CapturedSignalInput {
        slot: instance.plan.inputs[0].slot,
        value: ResidentValueRef::F64(&frame),
    };
    let prepared = instance.prepare_turn(&[input]).unwrap();
    core::mem::forget(prepared);

    assert!(matches!(
        instance.prepare_turn(&[input]),
        Err(ResidentExecutionError::ActiveCandidate)
    ));
    Ok(())
}

fn report_unchanged(
    _kernel: &BoundResidentKernel,
    _inputs: &dyn ResidentKernelInputs,
    _output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    Ok(false)
}

#[test]
fn always_changed_policy_propagates_when_the_kernel_reports_unchanged() -> MResult<()> {
    let frame = frames().next().unwrap();
    let mut always = instance(11)?;
    let mut reported = instance(12)?;
    execute_turn(&mut always, &frame).unwrap();
    execute_turn(&mut reported, &frame).unwrap();
    for instance in [&mut always, &mut reported] {
        instance.plan.nodes[0].kernel = BoundResidentKernel::new(report_unchanged, Box::new([]));
    }
    always.plan.nodes[0].change_detection = ChangeDetectionPolicy::AlwaysChanged;
    reported.plan.nodes[0].change_detection = ChangeDetectionPolicy::KernelReported;

    let always_summary = execute_turn(&mut always, &frame).unwrap();
    let reported_summary = execute_turn(&mut reported, &frame).unwrap();
    assert!(always_summary.dirty_nodes > reported_summary.dirty_nodes);
    Ok(())
}
