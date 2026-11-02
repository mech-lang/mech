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
            assert!(metadata.compositions.parents.is_empty());
            assert_eq!(
                metadata
                    .closed_tails
                    .regions
                    .iter()
                    .filter(|r| r.numeric.as_ref().is_some_and(|p| p.phase.is_some()))
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
            let memory = mode.memory_budget();
            let prepared = Prepared::new(descriptor, mode.execution(), &memory).unwrap();
            assert_eq!(
                prepared.state.as_ref().unwrap().call_inputs.values().len(),
                maximum_values,
                "numeric reconciliation words must not enlarge the managed-value bank"
            );
            drop(prepared);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
