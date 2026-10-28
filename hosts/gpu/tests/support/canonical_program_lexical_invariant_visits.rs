//! A late failed visit must discard staged buffer versions and finite work.
use super::super::{artifacts, equal, inputs, limits};
use super::{Data, JitProgramArtifact, ManagedMemoryBudget, check_cases};

#[test]
fn nested_unused_checked_call_precedes_later_closed_failure() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(n<u64>) => (out<u64>) :=\n  ignored := checked(n)\n  out := 1u64 / 0u64.\nouter(input<u64>)\n";
    check_cases(source, &[(vec![("input", Data::U64(0))], None)]);
}

#[test]
fn nested_counted_checked_call_precedes_later_closed_failure() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(n<u64>) => (out<u64>) :=\n  ignored := n |2> checked\n  out := 1u64 / 0u64.\nouter(input<u64>)\n";
    check_cases(source, &[(vec![("input", Data::U64(0))], None)]);
}

#[test]
fn nested_piped_checked_call_precedes_later_closed_failure() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(n<u64>) => (out<u64>) :=\n  ignored := n |> checked\n  out := 1u64 / 0u64.\nouter(input<u64>)\n";
    check_cases(source, &[(vec![("input", Data::U64(0))], None)]);
}

#[test]
fn expression_caller_keeps_a_checked_left_operand_before_a_closed_failure() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(n<u64>) => <u64> := checked(n) + 1u64 / 0u64.\nouter(input<u64>)\n";
    check_cases(source, &[(vec![("input", Data::U64(0))], None)]);
}

#[test]
fn unused_checked_result_is_observable_in_nested_invocations() {
    let source = "checked(n<u64>) => (out<u64>) :=\n  positive! := n > 0u64\n  out := n.\nouter(n<u64>) => (out<u64>) :=\n  ignored := checked(n)\n  out := 17u64.\nouter(input<u64>)\n";
    check_cases(
        source,
        &[
            (vec![("input", Data::U64(1))], Some(Data::U64(17))),
            (vec![("input", Data::U64(0))], None),
            (vec![("input", Data::U64(2))], Some(Data::U64(17))),
        ],
    );
}

#[test]
fn resident_success_failure_retry_preserves_publication_aliases_and_memory() {
    use mech_core::ReactiveInstanceId;
    use mech_engine::resident::{
        ActivationFacts, CapturedValueInput, ResidentActivationOptions, activate_with_options,
    };
    let source = "write-checked(xs<[u16]>, n<u64>) => (out<[u16]>) :=\n  ~next := xs\n  next[1u64] = n\n  positive! := n > 0u64\n  out := next.\nwrite-checked(input<[u16]>, count<u64>)\n";
    check_cases(
        source,
        &[
            (
                vec![
                    ("input", Data::Matrix(vec![Data::U16(7); 3].into())),
                    ("count", Data::U64(3)),
                ],
                Some(Data::Matrix(
                    vec![Data::U16(3), Data::U16(7), Data::U16(7)].into(),
                )),
            ),
            (
                vec![
                    ("input", Data::Matrix(vec![Data::U16(7); 3].into())),
                    ("count", Data::U64(0)),
                ],
                None,
            ),
            (
                vec![
                    ("input", Data::Matrix(vec![Data::U16(7); 3].into())),
                    ("count", Data::U64(5)),
                ],
                Some(Data::Matrix(
                    vec![Data::U16(5), Data::U16(7), Data::U16(7)].into(),
                )),
            ),
        ],
    );
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut instance = activate_with_options(
            ReactiveInstanceId::new(0xA55E27, 0),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(memory.clone()),
                ..ResidentActivationOptions::default()
            },
        )
        .unwrap();
        let last = artifact.outputs().len() - 1;
        let mut alias = None;
        for n in [3, 0, 5] {
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::Matrix(vec![Data::U16(7); 3].into())),
                    ("count", Data::U64(n)),
                ],
                &memory,
            );
            let captured = instance
                .plan
                .inputs
                .iter()
                .map(|input| CapturedValueInput {
                    slot: input.slot,
                    value: &args[artifact
                        .inputs()
                        .iter()
                        .position(|declaration| declaration.slot == input.artifact_slot)
                        .unwrap()],
                })
                .collect::<Vec<_>>();
            let before = memory.used_bytes();
            let result = instance
                .prepare_turn_values(&captured)
                .map_err(|error| format!("{error:?}"))
                .and_then(|turn| turn.publish().map_err(|error| format!("{error:?}")));
            if n == 0 {
                assert!(result.unwrap_err().contains("LexicalIntegrity"));
                // Aborting may also retire scratch from the previous turn.
                // It must not retain new candidate allocations.
                assert!(memory.used_bytes() <= before);
                let previous = instance.copied_output(last).unwrap();
                assert!(
                    previous
                        .snapshot_eq(
                            artifact.schemas(),
                            alias.as_ref().unwrap(),
                            artifact.schemas()
                        )
                        .unwrap()
                );
            } else {
                result.unwrap();
                let output = instance.copied_output(last).unwrap();
                assert_eq!(
                    output.canonical_data_draft().unwrap(),
                    Data::Matrix(vec![Data::U16(n as u16), Data::U16(7), Data::U16(7)].into())
                );
                if let Some(old) = &alias {
                    assert_eq!(
                        old.canonical_data_draft().unwrap(),
                        Data::Matrix(vec![Data::U16(3), Data::U16(7), Data::U16(7)].into())
                    );
                }
                alias = Some(output);
            }
            for arg in &args {
                if matches!(arg.data(), mech_core::ValueData::Matrix(_)) {
                    assert_eq!(
                        arg.canonical_data_draft().unwrap(),
                        Data::Matrix(vec![Data::U16(7); 3].into())
                    );
                }
            }
        }
        drop((alias, instance));
        assert_eq!(memory.used_bytes(), 0);
    }
}

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
