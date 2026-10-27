//! Deterministic capacity responses and actual original/decoded JIT recovery.
use crate::native_artifact::program::runtime::collections::histogram::finish::allocation::{
    HistogramAllocationSite, observation,
};
use crate::native_artifact::program::runtime::collections::histogram::finish::{allocation, *};
use crate::native_artifact::program::runtime::tests::{artifact_source, limits};
use crate::native_artifact::{JitProgramArtifact, NativeExecutionError};
use mech_core::{ManagedMemoryBudget, Value, ValueDataDraft as Data};
use mech_engine::native_artifact::NativeCollectionResult;
use std::cell::Cell;

#[test]
fn finish_buffers_reject_short_excess_and_nonempty_before_any_draft_fill() {
    for mode in ["exact", "short", "excess", "nonempty"] {
        let mut values = Vec::<u64>::with_capacity(3);
        let actual = values.capacity();
        let count = match mode {
            "short" => actual + 1,
            "excess" => actual - 1,
            _ => actual,
        };
        if mode == "nonempty" {
            values.push(7);
        }
        let result = allocation::vector_with(
            count,
            HistogramAllocationSite::Entries,
            "test vector",
            |_| Ok(values),
        );
        if mode == "exact" {
            let values = result.unwrap();
            assert!(values.is_empty());
            assert_eq!(values.capacity(), count);
        } else {
            assert!(result.is_err());
        }
    }
    assert!(
        allocation::vector::<u64>(0, HistogramAllocationSite::Entries, "empty entries")
            .unwrap()
            .is_empty()
    );
    for source in ["", "a\0é🧑‍🚀"] {
        assert_eq!(allocation::key(source).unwrap(), source);
        let calls = Cell::new(0);
        let result = allocation::key_with(source, |count| {
            calls.set(calls.get() + 1);
            Ok(String::with_capacity(count + 1))
        });
        assert!(result.is_err());
        assert_eq!(calls.get(), 1, "no retry or shrink after excess response");
    }
    assert!(allocation::key_with("ab", |_| Ok(String::with_capacity(1))).is_err());
    assert!(allocation::key_with("ab", |_| Ok(String::from("ab"))).is_err());
}

#[test]
fn finish_capacity_failure_drops_owners_under_claim_and_never_refunds_work() {
    struct Probe<'a>(&'a Cell<u64>, ManagedMemoryBudget);
    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            self.0.set(self.1.used_bytes());
        }
    }
    let memory = ManagedMemoryBudget::new(64 * 1024);
    let work = mech_engine::native_artifact::NativeLibraryWorkBudget::new(10_000, 10_000, &memory)
        .unwrap();
    let baseline = memory.used_bytes();
    let reservation = memory.reserve_capacity(128).unwrap();
    let before = work.used();
    work.charge(64, 7).unwrap();
    let observed = Cell::new(0);
    let mut values = Vec::with_capacity(2);
    let count = values.capacity() - 1;
    values.push(Probe(&observed, memory.clone()));
    assert!(
        allocation::vector_with(
            count,
            HistogramAllocationSite::Items,
            "test excess items",
            |_| Ok(values)
        )
        .is_err()
    );
    assert_eq!(observed.get(), baseline + 128);
    assert_eq!(memory.used_bytes(), baseline + 128);
    assert_eq!(work.used(), (before.0 + 64, before.1 + 7));
    drop(reservation);
    assert_eq!(memory.used_bytes(), baseline);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn finish_claim_exact_one_short_checks_before_allocator_and_without_retry() {
    for (limit, passes) in [(128, true), (127, false)] {
        let memory = ManagedMemoryBudget::new(limit);
        let allocations = Cell::new(0);
        let result = memory.reserve_capacity(128).map(|reservation| {
            let values = allocation::vector_with::<u64>(
                0,
                HistogramAllocationSite::Entries,
                "test empty",
                |_| {
                    allocations.set(allocations.get() + 1);
                    Ok(Vec::new())
                },
            )
            .unwrap();
            assert_eq!(memory.used_bytes(), 128);
            drop(values);
            drop(reservation);
        });
        assert_eq!(result.is_ok(), passes);
        assert_eq!(allocations.get(), usize::from(passes));
        assert_eq!(memory.used_bytes(), 0);
    }
}

fn variants() -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source("+> map\nmap/histogram([part | part <- source<[string]>])\n");
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn input(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Value {
    let input = &artifact.inputs()[0];
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape = mech_core::shape_for_resolved_extents(schema, &[1, 5]).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: Data::Matrix(
            ["", "é", "\0x", "é", "🧑‍🚀"]
                .into_iter()
                .map(|key| Data::String(key.into()))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn snapshots(artifact: &mech_engine::ProgramArtifact, output: &[Option<Value>]) -> Vec<Box<[u8]>> {
    output
        .iter()
        .map(|value| {
            value
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        })
        .collect()
}

#[test]
fn actual_histogram_finish_excess_at_every_owned_buffer_is_atomic_and_recovers() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| matches!(
                    collection.result(),
                    NativeCollectionResult::Histogram(_)
                ))
                .count(),
            1
        );
        let args = [input(&artifact, &memory)];
        let mut output = vec![None; artifact.outputs().len()];
        let (result, exact) = observation::with(None, || {
            jit.evaluate_into(&args, &mut output, limits(), &memory)
        });
        result.unwrap();
        assert_eq!(exact.attempts, [1, 4, 4]);
        assert_eq!(exact.accepted, exact.attempts);
        let Data::Map(entries) = output[0].as_ref().unwrap().canonical_data_draft().unwrap() else {
            panic!("Map")
        };
        assert_eq!(entries.len(), 4);
        let prior = snapshots(&artifact, &output);
        let mut owners = output
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let baseline = memory.used_bytes();
        for (kind, ordinal, attempts, accepted, reason) in [
            (
                HistogramAllocationSite::Entries,
                1,
                [1, 0, 0],
                [0, 0, 0],
                "Histogram result draft allocation",
            ),
            (
                HistogramAllocationSite::Items,
                1,
                [1, 1, 0],
                [1, 0, 0],
                "Histogram result item allocation",
            ),
            (
                HistogramAllocationSite::Key,
                1,
                [1, 1, 1],
                [1, 1, 0],
                "Histogram result key allocation",
            ),
            (
                HistogramAllocationSite::Items,
                2,
                [1, 2, 1],
                [1, 1, 1],
                "Histogram result item allocation",
            ),
            (
                HistogramAllocationSite::Key,
                2,
                [1, 2, 2],
                [1, 2, 1],
                "Histogram result key allocation",
            ),
        ] {
            State::test_histogram_clear_observation();
            let (result, rejected) = observation::with(Some((kind, ordinal)), || {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
            });
            assert!(
                matches!(result, Err(Error::Base(NativeExecutionError::Descriptor { reason: actual })) if actual == reason)
            );
            assert_eq!(rejected.attempts, attempts);
            assert_eq!(rejected.accepted, accepted);
            assert_eq!(rejected.rejected, 1);
            let demand = State::test_histogram_take_observation().unwrap();
            assert_eq!(demand.stage, crate::native_artifact::program::runtime::collections::histogram::HistogramStage::Finish);
            assert_eq!(rejected.envelope, demand.reservation_bytes);
            assert!(rejected.memory_on_reject >= baseline + rejected.envelope);
            assert!(rejected.work_at_admission.0 >= demand.compute_debit);
            assert!(rejected.work_after.0 >= rejected.work_at_admission.0);
            assert!(
                rejected.work_after.1 >= rejected.work_at_admission.1,
                "same account never refunds finish prepayment"
            );
            assert_eq!(snapshots(&artifact, &output), prior);
            for (value, old) in output.iter().zip(&owners) {
                assert!(value.as_ref().unwrap().shares_frozen_storage(old));
            }
            assert_eq!(memory.used_bytes(), baseline);
            let (recovery, exact) = observation::with(None, || {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
            });
            recovery.unwrap();
            assert_eq!(exact.attempts, [1, 4, 4]);
            assert_eq!(exact.accepted, exact.attempts);
            assert_eq!(exact.rejected, 0);
            assert_eq!(snapshots(&artifact, &output), prior);
            owners = output.iter().map(|v| v.as_ref().unwrap().clone()).collect();
            assert_eq!(memory.used_bytes(), baseline);
        }
        drop((owners, output, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}
