//! Raw native reconciliation words are not canonical managed arguments.
use super::*;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};

#[test]
fn native_argument_bank_numeric_words_do_not_inflate_managed_values() {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(include_str!(
        "../../../../../tests/fixtures/native/numeric-nested-phase.mec"
    ));
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let restored_descriptor = restored.then(|| {
                crate::native_artifact::program::runtime::tests::restore_admitted_helper_descriptor(
                    &jit,
                )
            });
            let descriptor = restored_descriptor.as_ref().unwrap_or(jit.descriptor());
            let metadata = descriptor.metadata();
            assert!(metadata.compositions.parents.iter().all(|parent| {
                parent
                    .segments
                    .iter()
                    .all(|piece| piece.prepaid_region.is_none())
            }));
            assert_eq!(
                metadata
                    .closed_tails
                    .regions
                    .iter()
                    .filter(|r| r.tail.parent().is_none()
                        && r.numeric.as_ref().is_some_and(|p| p.phase.is_some()))
                    .count(),
                1
            );
            let maximum_values = metadata
                .plan
                .blocks()
                .iter()
                .filter_map(|b| match b.action() {
                    NativeIterationAction::LibraryCall { operands, .. } => Some(operands.len()),
                    _ => None,
                })
                .max()
                .unwrap();
            assert!(metadata.max_library_inputs > maximum_values);
            let maximum_values = maximum_values.max(metadata.compositions.argument_values());
            assert_eq!(metadata.max_call_values, maximum_values);
            let memory = mode.memory_budget();
            let prepared = Prepared::new(descriptor, mode.execution(), &memory).unwrap();
            assert_eq!(
                prepared.state.as_ref().unwrap().call_inputs.values().len(),
                maximum_values,
                "numeric reconciliation words must not enlarge the managed-value bank"
            );
            assert_eq!(
                prepared.state.as_ref().unwrap().call_inputs.funded_len(),
                metadata.max_library_inputs
            );
            drop(prepared);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn native_argument_bank_retains_full_admission_and_rejects_invalid_geometry() {
    use crate::native_artifact::program::runtime::call_inputs::CallInputs;
    let memory = ManagedMemoryBudget::new(64 << 20);
    for (funded, values) in [(0, 0), (75, 9), (239, 36)] {
        let bytes = NativeTypedStorage::<Option<Value>>::admission_bytes(funded).unwrap();
        let bank = CallInputs::new(funded, values, 64 << 20, &memory).unwrap();
        assert_eq!(bank.values().len(), values);
        assert_eq!(bank.funded_len(), funded);
        assert_eq!(memory.used_bytes(), bytes);
        drop(bank);
        assert_eq!(memory.used_bytes(), 0);
        if bytes > 0 {
            let tight = memory.with_limits(bytes - 1, u64::MAX);
            assert!(CallInputs::new(funded, values, 64 << 20, &tight).is_err());
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    assert!(CallInputs::new(9, 75, 64 << 20, &memory).is_err());
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn native_argument_bank_covers_last_current_owner_and_cleans_aliases() {
    use crate::native_artifact::program::runtime::call_inputs::CallInputs;
    let memory = ManagedMemoryBudget::new(64 << 20);
    let mut bank = CallInputs::new(239, 36, 64 << 20, &memory).unwrap();
    let backing = memory.used_bytes();
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source("17u64");
    let value = ValueDraft {
        schema: artifact.outputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(17),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
        .unwrap();
    let value = value.into_memory_budget(&mut reservation).unwrap();
    drop(reservation);
    let alias = value.clone();
    *bank.values_mut().last_mut().unwrap() = Some(value);
    assert!(bank.values().iter().any(Option::is_some));
    assert!(bank.values_mut().get_mut(36).is_none());
    assert!(
        bank.values()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .shares_frozen_storage(&alias)
    );
    bank.values_mut().fill(None);
    assert!(!bank.values().iter().any(Option::is_some));
    assert_eq!(
        alias.canonical_data_draft().unwrap(),
        ValueDataDraft::U64(17)
    );
    drop(alias);
    assert_eq!(memory.used_bytes(), backing);
    drop(bank);
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}
