//! Ordinary recursive functions keep their real arguments and local values.
use super::super::{artifacts, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn ordinary_tail_recursion_can_carry_and_update_dynamic_arrays() {
    let source = "bump(xs<[u64]>, n<u64>) => (out<[u64]>) :=\n  ~next := xs\n  next[1u64] = xs[1u64] + n\n  out := next.\nfill(xs<[u64]>, n<u64>) => <[u64]>\n  | (xs, 0u64) => xs\n  | (xs, n) => fill(bump(xs, n), n - 1u64).\nfill(input<[u64]>, count<u64>)\n";
    for artifact in artifacts(source).0 {
        assert!(
            !artifact
                .slots()
                .iter()
                .any(|slot| slot.role == mech_engine::SlotRole::State)
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this unchanged descriptor comes from the exact checked local
        // build and is not paired with a foreign executable.
        let restored = unsafe {
            mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                &bytes,
                &mech_stdlib::source_catalog(),
            )
        }
        .unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (size, count) in [(0, 0), (3, 1), (3, 4), (0, 1), (2, 2), (0, 0)] {
            let initial = Data::Matrix(vec![Data::U64(17); size].into());
            let args = inputs(
                &artifact,
                &[("input", initial.clone()), ("count", Data::U64(count))],
                &memory,
            );
            if size == 0 && count != 0 {
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(matches!(error,
                    mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                        error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::IndexOutOfRange { index: 1, upper_bound: 0 }
                        ), ..
                    }), "{error:?}");
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                let mut expected = vec![Data::U64(17); size];
                if size != 0 {
                    expected[0] = Data::U64(17 + count * (count + 1) / 2);
                }
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into())
                );
            }
            let index = artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == "input"
                })
                .unwrap();
            assert_eq!(args[index].canonical_data_draft().unwrap(), initial);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn ordinary_tail_recursion_uses_scalar_argument_tuples_and_keeps_overflow_errors() {
    let source = "sum(n<u64>, total<u64>) => <u64>\n  | (0u64, total) => total\n  | (n, total) => sum(n - 1u64, total + n).\nsum(count<u64>, seed<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (count, seed) in [(0, 17), (8, 2), (1, u64::MAX), (3, 7)] {
            let args = inputs(
                &artifact,
                &[("count", Data::U64(count)), ("seed", Data::U64(seed))],
                &memory,
            );
            if seed == u64::MAX {
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                assert!(
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .is_err()
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                let ordinary = reference(&artifact, &args);
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &ordinary);
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(seed + count * (count + 1) / 2)
                );
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn function_local_mutation_is_fresh_for_repeated_calls_and_broadcast_visits() {
    let source = "step(n<u64>) => (out<u64>) :=\n  ~next := n\n  next += 3u64\n  out := next.\nstep(values<[u64]>)\n";
    for artifact in artifacts(source).0 {
        assert!(
            !artifact
                .slots()
                .iter()
                .any(|slot| slot.role == mech_engine::SlotRole::State)
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for values in [vec![1, 7, 1], vec![], vec![0, 1], vec![1, 7, 1]] {
            let args = inputs(
                &artifact,
                &[(
                    "values",
                    Data::Matrix(values.iter().copied().map(Data::U64).collect()),
                )],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix(values.iter().map(|value| Data::U64(value + 3)).collect())
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn non_tail_and_partial_recursion_keep_explicit_native_rejections() {
    for source in [
        "sum(n<u64>) => <u64>\n  | 0u64 => 0u64\n  | n => n + sum(n - 1u64).\nsum(input<u64>)\n",
        "again(n<u64>, tag<u64>) => <u64>\n  | (n, 1u64) => again(n - 1u64, 1u64).\nagain(input<u64>, 1u64)\n",
    ] {
        for artifact in artifacts(source).0 {
            assert!(
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err()
            );
        }
    }
}
