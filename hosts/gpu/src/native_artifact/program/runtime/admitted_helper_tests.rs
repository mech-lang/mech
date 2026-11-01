//! Differential controls for composing the original two synchronous callbacks.
use super::*;

const SOURCE: &str = r#"+> sequence
walk(data<[u32]>, left<u64>, n<u64>, flag<bool>) => (out<{data<[u32]> n<u64> flag<bool>}>) :=
  out := ((left > 0u64)? | true => walk(data, left - 1u64, n + 1u64, flag == false) | false => {data: data, n: n, flag: flag}).
r := walk(data<[u32]>, 3u64, n<u64>, flag<bool>)
(r.n / divisor<u64>, r.data, r.flag)
"#;

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    n: u64,
    flag: bool,
    divisor: u64,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let (data, shape_values): (ValueDataDraft, Box<[u64]>) =
                match mech_engine::decode_source_input_name(&port.name)
                    .unwrap()
                    .as_str()
                {
                    "data" => (
                        ValueDataDraft::Matrix(vec![ValueDataDraft::U32(7)].into()),
                        mech_core::shape_for_resolved_extents(
                            artifact.schemas().get(port.schema).unwrap(),
                            &[1, 1],
                        )
                        .unwrap()
                        .parameter_values()
                        .to_vec()
                        .into_boxed_slice(),
                    ),
                    "n" => (ValueDataDraft::U64(n), Box::new([]) as Box<[u64]>),
                    "flag" => (ValueDataDraft::Bool(flag), Box::new([])),
                    "divisor" => (ValueDataDraft::U64(divisor), Box::new([])),
                    name => panic!("unexpected {name}"),
                };
            let value = ValueDraft {
                schema: port.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

fn snapshots(values: &[Option<Value>]) -> Vec<ValueDataDraft> {
    values
        .iter()
        .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}

#[test]
fn admitted_helpers_match_split_callbacks_original_decoded_failure_and_recovery() {
    let original = artifact_source(SOURCE);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let restored = restore_admitted_helper_descriptor(&jit);
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut fused_output = vec![None; artifact.outputs().len()];
            let mut split_output = fused_output.clone();
            for (n, flag, divisor) in [(8, false, 3), (12, true, 0), (20, true, 7), (2, false, 1)] {
                let args = arguments(&artifact, n, flag, divisor, &memory);
                let aliases = args.clone();
                let input_snapshots: Vec<_> = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect();
                let prior_fused = fused_output.clone();
                let prior_split = split_output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (fused, fused_rows) = execute_observed_admitted_helper(
                    &jit,
                    descriptor,
                    &args,
                    &mut fused_output,
                    &memory,
                    false,
                    0,
                );
                let (split, split_rows) = execute_observed_admitted_helper(
                    &jit,
                    descriptor,
                    &args,
                    &mut split_output,
                    &memory,
                    true,
                    0,
                );
                assert!(
                    !fused_rows.is_empty(),
                    "must exercise the composed callback"
                );
                assert_eq!(fused_rows, split_rows);
                match (fused, split) {
                    (Ok(a), Ok(b)) => {
                        assert_eq!(a.work, b.work);
                        assert_eq!(snapshots(&fused_output), snapshots(&split_output));
                        assert_eq!(
                            snapshots(&fused_output),
                            vec![ValueDataDraft::Tuple(
                                vec![
                                    ValueDataDraft::U64((n + 3) / divisor),
                                    ValueDataDraft::Matrix(vec![ValueDataDraft::U32(7)].into()),
                                    ValueDataDraft::Bool(!flag)
                                ]
                                .into()
                            )]
                        );
                    }
                    (Err(a), Err(b)) => {
                        assert_eq!(format!("{a:?}"), format!("{b:?}"));
                        assert!(
                            matches!(
                                &a,
                                NativeProgramExecutionError::Control(
                                    NativeIterationExecutionError::Arithmetic { .. }
                                )
                            ),
                            "{a:?}"
                        );
                        assert_eq!(
                            retained,
                            (memory.used_bytes(), memory.retained_snapshot_nodes())
                        );
                        for (actual, prior) in
                            [(&fused_output, &prior_fused), (&split_output, &prior_split)]
                        {
                            assert!(actual.iter().zip(prior).all(|(a, b)| {
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            }));
                        }
                    }
                    other => panic!("callback disagreement {other:?}"),
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    input_snapshots
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop((fused_output, split_output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn admitted_helpers_preserve_admission_before_bad_pointer_arguments_and_site_errors() {
    let artifact = artifact_source(SOURCE);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = arguments(&artifact, 8, false, 3, &memory);
    let mut outputs = vec![None; artifact.outputs().len()];
    for fault in 1..=5 {
        execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            false,
            0,
        )
        .0
        .unwrap();
        let prior = outputs.clone();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let (fused, fused_rows) = execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            false,
            fault,
        );
        let (split, split_rows) = execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            true,
            fault,
        );
        let failure = fused.unwrap_err();
        let error = format!("{failure:?}");
        assert_eq!(error, format!("{:?}", split.unwrap_err()));
        assert_eq!(fused_rows, split_rows);
        let failed = fused_rows.last().unwrap();
        assert_ne!(failed.status, STATUS_OK);
        if matches!(fault, 3 | 4) {
            assert!(
                matches!(
                    &failure,
                    NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Work { .. }
                    )
                ),
                "{error}"
            );
            assert_eq!(failed.work_before, failed.work_after);
        } else if fault == 1 {
            assert!(error.contains("helper result pointer mismatch"), "{error}");
            assert!(failed.work_after > failed.work_before);
        }
        assert_eq!(
            retained,
            (memory.used_bytes(), memory.retained_snapshot_nodes())
        );
        assert!(outputs.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            false,
            0,
        )
        .0
        .unwrap();
    }
    drop((outputs, args));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}
