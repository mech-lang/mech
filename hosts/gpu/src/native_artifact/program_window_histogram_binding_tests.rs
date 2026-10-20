//! Observe real admitted buffers in original/decoded generated invocations.
use super::super::super::super::tests::{artifact_source, limits};
use crate::native_artifact::JitProgramArtifact;
use mech_core::{
    ManagedMemoryBudget, Value, ValueDataDraft as Data,
    snapshot::{SnapshotValidationContext, ValueDraft},
};
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
struct Record {
    cells: usize,
    bytes: u64,
    work: u64,
}
thread_local! {
    static LAST: Cell<Option<Record>> = const { Cell::new(None) };
}
pub(super) fn record(cells: usize, bytes: u64, work: u64) {
    LAST.with(|slot| slot.set(Some(Record { cells, bytes, work })));
}
fn take() -> Record {
    LAST.with(|slot| slot.take())
        .expect("actual compiled scratch admission")
}
fn variants() -> [mech_engine::ProgramArtifact; 2] {
    let original = artifact_source(include_str!(
        "../../tests/fixtures/native/window-histogram.mec"
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
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "input" => Data::String("\0".repeat(length)),
                "width" => Data::U64(1),
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
    assert_eq!(entries.len(), usize::from(length != 0));
    if length != 0 {
        assert_eq!(
            entries[0].items.as_ref(),
            &[Data::String("\0".into()), Data::U64(length as u64)]
        );
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
        assert_eq!((expanded.cells, expanded.bytes), (512, 4096));
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
            assert_eq!(
                (record.cells, record.bytes),
                (expected, (expected * 8) as u64)
            );
            drop((outputs, args));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
