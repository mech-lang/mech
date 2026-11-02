//! Observe real admitted buffers in original/decoded generated invocations.
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program::runtime::tests::{artifact_source, limits as previous_limits};
use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data};
use std::cell::Cell;

// These tests target storage thresholds and atomic memory denial. Real table
// lookups now consume comparison work as well as the original bounded scan.
// Keep both finite and preserve every exact/one-short memory assertion.
fn limits() -> crate::native_artifact::NativeProgramExecutionLimits {
    let mut limits = previous_limits();
    limits.library.max_compute_work = Some(1_000_000_000);
    limits.library.max_comparison_work = Some(1_000_000_000);
    limits
}

#[derive(Clone, Copy, Debug)]
struct Record {
    cells: usize,
    reserved_bytes: u64,
    work: u64,
}
thread_local! {
    static LAST: Cell<Option<Record>> = const { Cell::new(None) };
}
pub(in crate::native_artifact::program::runtime::collections::histogram::window) fn record(
    cells: usize,
    reserved_bytes: u64,
    work: u64,
) {
    LAST.with(|slot| {
        slot.set(Some(Record {
            cells,
            reserved_bytes,
            work,
        }))
    });
}
fn take() -> Record {
    LAST.with(|slot| slot.take())
        .expect("actual compiled scratch admission")
}
fn variants() -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(include_str!(
        "../../../../../../../tests/fixtures/native/window-histogram.mec"
    ));
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    length: usize,
) -> Vec<Value> {
    inputs_width(artifact, memory, length, 1)
}
fn inputs_width(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    length: usize,
    width: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "input" => Data::String("\0".repeat(length)),
                "width" => Data::U64(width),
                _ => panic!("unexpected input {name}"),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut claim = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut claim).unwrap()
        })
        .collect()
}
fn exact(outputs: &[Option<Value>], length: usize) {
    exact_width(outputs, length, 1);
}
fn exact_width(outputs: &[Option<Value>], length: usize, width: usize) {
    let Data::Map(entries) = outputs
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
    else {
        panic!("canonical Map");
    };
    let count = length.checked_sub(width).map_or(0, |last| last + 1);
    assert_eq!(entries.len(), usize::from(count != 0));
    if count != 0 {
        assert_eq!(
            entries[0].items.as_ref(),
            &[Data::String("\0".repeat(width)), Data::U64(count as u64)]
        );
    }
}

#[test]
fn original_and_decoded_pair_banks_keep_thresholds_denial_cleanup_and_recovery() {
    let logical = crate::native_artifact::program::prepare::window_histogram::PAIR_CELLS;
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for length in [logical * 8, logical * 8 + 1] {
            let args = inputs_width(&artifact, &memory, length, 2);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            exact_width(&outputs, length, 2);
            let actual = take();
            assert_eq!(
                actual.cells,
                if length == logical * 8 {
                    logical
                } else {
                    logical * 4
                }
            );
            assert_eq!(
                actual.reserved_bytes,
                if actual.cells == logical {
                    (logical * 8) as u64
                } else {
                    crate::native_artifact::program::prepare::window_histogram::banked_peak(2, size_of::<crate::native_artifact::program::runtime::collections::histogram::window::Entry>() as u64).unwrap()
                }
            );
            let previous = outputs.clone();
            let before = memory.used_bytes();
            let mut denied = limits();
            denied.execution.max_work = actual.work - 1;
            let error = jit
                .evaluate_into(&args, &mut outputs, denied, &memory)
                .unwrap_err();
            assert!(format!("{error:?}").contains("ComputeWork"), "{error:?}");
            assert!(LAST.with(|slot| slot.get()).is_none());
            assert_eq!(memory.used_bytes(), before);
            for (actual, old) in outputs.iter().zip(&previous) {
                assert!(
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                );
            }
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            exact_width(&outputs, length, 2);
            assert_eq!(take().cells, actual.cells);
            drop((outputs, previous, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn actual_banks_keep_results_and_finite_denial_is_atomic_with_owner_cleanup() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(&artifact, &memory, 1031);
        let baseline = memory.used_bytes();
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        exact(&outputs, 1031);
        let expanded = take();
        assert_eq!(
            (expanded.cells, expanded.reserved_bytes),
            (
                512,
                crate::native_artifact::program::prepare::window_histogram::banked_peak(1, size_of::<crate::native_artifact::program::runtime::collections::histogram::window::Entry>() as u64).unwrap()
            )
        );
        let previous = outputs.clone();
        let before = memory.used_bytes();
        let mut denied = limits();
        denied.execution.max_work = expanded.work - 1;
        let error = jit
            .evaluate_into(&args, &mut outputs, denied, &memory)
            .unwrap_err();
        assert!(format!("{error:?}").contains("ComputeWork"), "{error:?}");
        assert!(
            LAST.with(|slot| slot.get()).is_none(),
            "deny before allocating scratch"
        );
        assert_eq!(memory.used_bytes(), before);
        for (actual, old) in outputs.iter().zip(&previous) {
            assert!(
                actual
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(old.as_ref().unwrap())
            );
        }
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        exact(&outputs, 1031);
        assert_eq!(take().cells, 512);
        drop((previous, outputs));
        assert_eq!(memory.used_bytes(), baseline);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_scratch_extents_follow_threshold_and_keep_the_same_logical_map() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for length in [0, 7, 8, 1023, 1024, 1031, 4097] {
            let args = inputs(&artifact, &memory, length);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            exact(&outputs, length);
            let record = take();
            let expected = if length == 0 {
                0
            } else if length < 1024 {
                128
            } else {
                512
            };
            assert_eq!(record.cells, expected);
            assert_eq!(
                record.reserved_bytes,
                if expected == 512 {
                    crate::native_artifact::program::prepare::window_histogram::banked_peak(1, size_of::<crate::native_artifact::program::runtime::collections::histogram::window::Entry>() as u64).unwrap()
                } else {
                    (expected * 8) as u64
                }
            );
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn bank_admission_losing_shared_headroom_retries_serial_without_leaking() {
    let entry = size_of::<
        crate::native_artifact::program::runtime::collections::histogram::window::Entry,
    >() as u64;
    let peak =
        crate::native_artifact::program::prepare::window_histogram::banked_peak(1, entry).unwrap();
    for remaining in [1023, 1024, peak - 1] {
        let memory = ManagedMemoryBudget::new(peak);
        let preferred = crate::native_artifact::program::prepare::window_histogram::prefer_banks(
            1, 1024, 0, None, peak, entry,
        );
        assert!(preferred);
        // Deterministic competing execution between the preference snapshot and
        // actual reservation. It shares the genuine account, not a mock quota.
        let competitor = memory.clone().reserve_capacity(peak - remaining).unwrap();
        let result = crate::native_artifact::program::runtime::collections::histogram::window::reserve_layout(&memory, 128, 1, preferred);
        if remaining < 1024 {
            assert!(matches!(
                result,
                Err(
                    crate::native_artifact::program::runtime::NativeProgramExecutionError::Base(
                        crate::native_artifact::NativeExecutionError::Memory {
                            error: mech_core::MemoryRuntimeError::BudgetExceeded { .. }
                        }
                    )
                )
            ));
            assert_eq!(memory.used_bytes(), peak - remaining);
        } else {
            let (cells, reservation) = result.unwrap();
            assert_eq!((cells, reservation.capacity_bytes()), (128, 1024));
            assert_eq!(memory.used_bytes(), peak - remaining + 1024);
            drop(reservation);
            assert_eq!(memory.used_bytes(), peak - remaining);
        }
        drop(competitor);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn banked_result_transfers_reserved_peak_even_when_shared_account_is_full() {
    let entry = size_of::<
        crate::native_artifact::program::runtime::collections::histogram::window::Entry,
    >() as u64;
    let peak =
        crate::native_artifact::program::prepare::window_histogram::banked_peak(1, entry).unwrap();
    for distinct in [1, 13, 128] {
        let memory = ManagedMemoryBudget::new(peak + 17);
        let (cells, reservation) = crate::native_artifact::program::runtime::collections::histogram::window::reserve_layout(&memory, 128, 1, true).unwrap();
        assert_eq!((cells, reservation.capacity_bytes()), (512, peak));
        let mut bins =
            crate::native_artifact::program::runtime::collections::histogram::window::Bins {
                values: vec![0; cells],
                width: 1,
                _reservation: reservation,
            };
        let competitor = memory.clone().reserve_capacity(17).unwrap();
        assert!(memory.reserve_capacity(1).is_err());
        let result_bytes = distinct * (entry + 1);
        let result = bins.reserve_result(&memory, result_bytes).unwrap();
        assert_eq!(memory.used_bytes(), peak + 17);
        assert_eq!(result.capacity_bytes(), result_bytes);
        assert_eq!(bins._reservation.capacity_bytes(), peak - result_bytes);
        drop(bins);
        assert_eq!(memory.used_bytes(), result_bytes + 17);
        drop(result);
        drop(competitor);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn pair_bank_peak_transfers_keys_and_lost_headroom_keeps_serial_fallback() {
    let entry = size_of::<
        crate::native_artifact::program::runtime::collections::histogram::window::Entry,
    >() as u64;
    let logical = crate::native_artifact::program::prepare::window_histogram::PAIR_CELLS;
    let serial = (logical * size_of::<u64>()) as u64;
    let peak =
        crate::native_artifact::program::prepare::window_histogram::banked_peak(2, entry).unwrap();
    let memory = ManagedMemoryBudget::new(peak);
    let competitor = memory.reserve_capacity(peak - serial).unwrap();
    let (cells, reservation) =
        crate::native_artifact::program::runtime::collections::histogram::window::reserve_layout(
            &memory, logical, 2, true,
        )
        .unwrap();
    assert_eq!((cells, reservation.capacity_bytes()), (logical, serial));
    drop(reservation);
    drop(competitor);
    assert_eq!(memory.used_bytes(), 0);

    let (cells, reservation) =
        crate::native_artifact::program::runtime::collections::histogram::window::reserve_layout(
            &memory, logical, 2, true,
        )
        .unwrap();
    assert_eq!(cells, logical * 4);
    assert_eq!(reservation.capacity_bytes(), peak);
    let mut bins = crate::native_artifact::program::runtime::collections::histogram::window::Bins {
        values: vec![0; cells],
        width: 2,
        _reservation: reservation,
    };
    assert!(memory.reserve_capacity(1).is_err());
    let keys = logical as u64 * (entry + 2);
    let result = bins.reserve_result(&memory, keys).unwrap();
    assert_eq!(result.capacity_bytes(), keys);
    drop(bins);
    assert_eq!(memory.used_bytes(), keys);
    drop(result);
    assert_eq!(memory.used_bytes(), 0);
}
