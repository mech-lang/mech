use super::{artifacts, limits};
use mech_core::ValueData;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits, NativeProgramLimitMode};

#[test]
fn no_limits_mode_admits_buffer_above_default_byte_budget_and_recovers() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts("+> sequence\nsequence/with-capacity(0u8, 16777217u64)\n").0 {
        let rejected = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeCompileLimits {
                max_definition_table_bytes: 0,
                ..mode.compilation()
            },
        )
        .err()
        .expect("explicit zero-byte compiler budget must reject");
        let reason = rejected.to_string();
        assert!(
            reason.contains("DefinitionTableLimit") && reason.contains("limit: 0"),
            "{reason}"
        );
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let mut output = vec![None; artifact.outputs().len()];
        for unrestricted in [false, true, false, true] {
            let old = output.clone();
            let retained = memory.used_bytes();
            let policy = if unrestricted {
                mode.execution()
            } else {
                limits()
            };
            let result = jit.evaluate_into(&[], &mut output, policy, &memory);
            if unrestricted {
                result.unwrap();
                let ValueData::Matrix(matrix) = output.last().unwrap().as_ref().unwrap().data()
                else {
                    panic!("reserved byte buffer")
                };
                assert_eq!(matrix.elements().len(), 0);
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(16777217));
            } else {
                let error = result.unwrap_err().to_string();
                assert!(error.contains("TargetLimitExceeded"), "{error}");
                assert_eq!(memory.used_bytes(), retained);
                assert!(old.iter().zip(&output).all(|(before, after)| {
                    match (before, after) {
                        (Some(before), Some(after)) => before.shares_frozen_storage(after),
                        (None, None) => true,
                        _ => false,
                    }
                }));
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
