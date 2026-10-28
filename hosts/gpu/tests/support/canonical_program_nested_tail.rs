//! Selected tail calls below nested matches return to their exact lexical target.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

#[test]
fn nested_tail_calls_keep_a_scalar_carrier_and_recover_after_work_denial() {
    let source = "count(n<u64>) => <u64>\n  | n => ((n > 0u64)? | true => count(n - 1u64) | false => 7u64).\ncount(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 7, 1, 0] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
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
                Data::U64(7)
            );
        }
        let args = inputs(&artifact, &[("input", Data::U64(100))], &memory);
        let prior = output
            .iter()
            .map(|value| value.clone().unwrap())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        let mut policy = limits();
        policy.execution.max_work = 1;
        assert!(
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .is_err()
        );
        equal(&artifact, &output, &prior);
        assert_eq!(memory.used_bytes(), before);
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
        drop((args, output, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn ordinary_tail_calls_under_nested_matches_keep_the_outer_argument_tuple() {
    let source = "count(n<u64>, total<u64>) => <u64>\n  | (n, total) => ((n > 0u64)?\n      | true => ((n > 2u64)? | true => count(n - 2u64, total + n) | false => count(n - 1u64, total + n))\n      | false => total).\ncount(input<u64>, seed<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 1, 4, 7, 2, 0] {
            let args = inputs(
                &artifact,
                &[("input", Data::U64(n)), ("seed", Data::U64(19))],
                &memory,
            );
            let ordinary = reference(&artifact, &args);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_tail_function_helpers_keep_their_own_loop_targets() {
    let source = "inner(n<u64>, total<u64>) => <u64>\n  | (n, total) => ((n > 0u64)? | true => inner(n - 1u64, total + 1u64) | false => total).\nouter(n<u64>, total<u64>) => <u64>\n  | (n, total) => ((n > 0u64)? | true => outer(n - 1u64, inner(n, total)) | false => total).\nouter(input<u64>, seed<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 1, 4, 2, 0] {
            let args = inputs(
                &artifact,
                &[("input", Data::U64(n)), ("seed", Data::U64(11))],
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
                Data::U64(11 + n * (n + 1) / 2)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn scalar_tail_carriers_retire_managed_string_locals_between_visits() {
    let source = "+> number\n+> sequence\nstep(n<u64>) => (out<u64>) :=\n  word := number/format(n, 0u64)\n  out := n - sequence/length(word).\ncount(n<u64>) => <u64>\n  | n => ((n > 0u64)? | true => count(step(n)) | false => 7u64).\ncount(input<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = jit.descriptor().plan();
        let (iteration, _) = plan
            .iterations()
            .iter()
            .enumerate()
            .find(|(_, control)| {
                matches!(control,
            mech_engine::native_artifact::NativeControlLoop::Tail(tail)
                if plan.values()[tail.carrier().get() as usize].scalar_kind().is_some())
            })
            .unwrap();
        assert!(
            plan.values()
                .iter()
                .zip(plan.value_scopes())
                .any(
                    |(ty, scope)| ty.definition().body() == &mech_core::SchemaBody::String
                        && *scope
                            == mech_engine::native_artifact::NativeIterationValueScope::Immutable {
                                iteration: Some(iteration as u32)
                            }
                )
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 1, 19, 4, 0] {
            let args = inputs(&artifact, &[("input", Data::U64(n))], &memory);
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &reference(&artifact, &args));
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(7)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_tail_array_mutation_preserves_input_and_discards_late_failure() {
    use super::reference_result;
    let source = "bump(xs<[u64]>, n<u64>) => (out<[u64]>) :=\n  unused := 10u64 / (n % 3u64)\n  ~next := xs\n  next[1u64] = xs[1u64] + n\n  out := next.\nfill(xs<[u64]>, n<u64>) => <[u64]>\n  | (xs, n) => ((n > 0u64)? | true => fill(bump(xs, n), n - 1u64) | false => xs).\nfill(input<[u64]>, count<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for n in [0, 2, 5, 1] {
            let initial = Data::Matrix(vec![Data::U64(17); 3].into());
            let args = inputs(
                &artifact,
                &[("input", initial.clone()), ("count", Data::U64(n))],
                &memory,
            );
            if n == 5 {
                assert!(reference_result(&artifact, &args).is_err());
                let prior = output
                    .iter()
                    .map(|value| value.clone().unwrap())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(
                    matches!(
                        error,
                        mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                        )
                    ),
                    "{error:?}"
                );
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(
                        vec![
                            Data::U64(17 + n * (n + 1) / 2),
                            Data::U64(17),
                            Data::U64(17)
                        ]
                        .into()
                    )
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
fn nested_calls_cannot_discard_work_or_claim_a_partial_child_is_total() {
    for source in [
        "count(n<u64>) => <u64>\n  | n => ((n > 0u64)? | true => count(n - 1u64) | false => 7u64) + 1u64.\ncount(input<u64>)\n",
        "count(n<u64>) => <u64>\n  | 0u64 => 0u64\n  | n => (count(n - 1u64), 7u64).2.\ncount(input<u64>)\n",
        "pick(n<u64>) => <u64>\n  | 1u64 => 7u64.\ncount(n<u64>) => <u64>\n  | n => ((n > 0u64)? | true => count(n - 1u64) | false => pick(n)).\ncount(input<u64>)\n",
    ] {
        for artifact in artifacts(source).0 {
            assert!(
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).is_err()
            );
        }
    }
}
