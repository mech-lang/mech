//! A late failed visit must discard staged buffer versions and finite work.
use super::super::{artifacts, equal, inputs, limits};
use super::{Data, JitProgramArtifact, ManagedMemoryBudget, check_cases};

#[test]
fn recursive_invariant_failure_discards_prior_buffer_writes_and_recovers() {
    let source = "fill(xs<[u16]>, n<u64>) => (out<[u16]>) :=\n  valid! := n != 3u64\n  wide<u64> := xs[1u64]\n  ~next := xs\n  next[1u64] = wide + n\n  left := ((n > 0u64)? | true => n - 1u64 | false => 0u64)\n  out := ((n > 0u64)? | true => fill(next, left) | false => xs).\nfill(input<[u16]>, count<u64>)\n";
    let matrix = |values: &[u16]| Data::Matrix(values.iter().copied().map(Data::U16).collect());
    check_cases(
        source,
        &[
            (
                vec![("input", matrix(&[17, 17])), ("count", Data::U64(2))],
                Some(matrix(&[20, 17])),
            ),
            (
                vec![("input", matrix(&[17, 17, 17])), ("count", Data::U64(5))],
                None,
            ),
            (
                vec![("input", matrix(&[17])), ("count", Data::U64(1))],
                Some(matrix(&[18])),
            ),
            (
                vec![("input", matrix(&[17, 17])), ("count", Data::U64(0))],
                Some(matrix(&[17, 17])),
            ),
        ],
    );
}

#[test]
fn assertion_paths_keep_a_finite_work_limit_and_allow_retry() {
    use mech_gpu::native_artifact::{
        NativeExecutionError, NativeIterationExecutionError, NativeProgramExecutionError,
        NativeValueError,
    };
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n + 1u64.\nchecked(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let first = inputs(&artifact, &[("input", Data::U64(3))], &memory);
        let report = jit
            .evaluate_into(&first, &mut output, limits(), &memory)
            .unwrap();
        let args = inputs(&artifact, &[("input", Data::U64(4))], &memory);
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut policy = limits();
        policy.execution.max_work = report.work.checked_sub(1).unwrap();
        let error = jit
            .evaluate_into(&args, &mut output, policy, &memory)
            .unwrap_err();
        assert!(
            matches!(
                error,
                NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit { .. })
                    | NativeProgramExecutionError::Value(NativeValueError::Base(
                        NativeExecutionError::WorkLimit { .. }
                    ))
                    | NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Work { .. }
                    )
            ),
            "{error:?}"
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::U64(5)
        );
        drop((first, args, prior, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}
