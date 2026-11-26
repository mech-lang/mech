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

const FILTERED_RANGE: &str = "[value | value <- 1u64..=n<u64>, flag<bool>]\n";

#[test]
fn admitted_helpers_collection_range_and_value_sources_match_split_callbacks() {
    use mech_engine::native_artifact::NativeIterationAction as A;
    let mut coverage = [0; 6];
    for source in [
        FILTERED_RANGE,
        "[value | value <- data<[u32]>, flag<bool>]\n",
    ] {
        let original = artifact_source(source);
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
                let mut fast = vec![None; artifact.outputs().len()];
                let mut split = fast.clone();
                for (n, flag) in [(3, true), (0, false), (1, true), (7, false), (5, true)] {
                    let args = arguments(&artifact, n, flag, 1, &memory);
                    let before = args
                        .iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>();
                    let prior_fast = fast.clone();
                    let prior_split = split.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (a, arows) = execute_observed_admitted_helper(
                        &jit, descriptor, &args, &mut fast, &memory, false, 0,
                    );
                    let (b, brows) = execute_observed_admitted_helper(
                        &jit, descriptor, &args, &mut split, &memory, true, 0,
                    );
                    assert_eq!(
                        arows, brows,
                        "complete work/previous/result/consumption transitions"
                    );
                    let collections = arows
                        .iter()
                        .filter(|r| r.operation == GlobalHelper::Collection.raw())
                        .collect::<Vec<_>>();
                    assert!(
                        !collections.is_empty(),
                        "actual composed collection callbacks"
                    );
                    for row in collections {
                        let k =
                            match descriptor.metadata().plan.blocks()[row.site as usize].action() {
                                A::PrepareU64Range { .. } => 0,
                                A::CollectionBegin { .. } => 1,
                                A::Initialize { .. } => 2,
                                A::CollectionElement { .. } => 3,
                                A::CollectionPush { .. } => 4,
                                A::CollectionFinish { .. } => 5,
                                other => panic!("unexpected admitted collection {other:?}"),
                            };
                        coverage[k] += 1;
                    }
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            assert_eq!(a, b);
                            assert_eq!(snapshots(&fast), snapshots(&split));
                        }
                        (Err(a), Err(b)) => {
                            assert_eq!(format!("{a:?}"), format!("{b:?}"));
                            assert_eq!(
                                retained,
                                (memory.used_bytes(), memory.retained_snapshot_nodes())
                            );
                            for (actual, prior) in [(&fast, &prior_fast), (&split, &prior_split)] {
                                assert!(actual.iter().zip(prior).all(|(a, b)| match (a, b) {
                                    (Some(a), Some(b)) => a.shares_frozen_storage(b),
                                    (None, None) => true,
                                    _ => false,
                                }));
                            }
                        }
                        other => panic!("composed/split disagreement {other:?}"),
                    }
                    assert_eq!(
                        args.iter()
                            .map(|v| v.canonical_data_draft().unwrap())
                            .collect::<Vec<_>>(),
                        before
                    );
                }
                drop((fast, split));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
    assert!(
        coverage.iter().all(|n| *n > 0),
        "range/begin/init/element/push/finish {coverage:?}"
    );
}

#[test]
fn admitted_helpers_collection_finite_limits_preserve_exact_denial_atomicity_and_retry() {
    let original = artifact_source(FILTERED_RANGE);
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
            let args = arguments(&artifact, 3, true, 1, &memory);
            let mut fast = vec![None; artifact.outputs().len()];
            let mut split = fast.clone();
            execute_observed_admitted_helper(&jit, descriptor, &args, &mut fast, &memory, false, 0)
                .0
                .unwrap();
            execute_observed_admitted_helper(&jit, descriptor, &args, &mut split, &memory, true, 0)
                .0
                .unwrap();
            for dimension in 0..9 {
                for cap in [0, 1, 16, 128, 4096, 65535, 1_000_000] {
                    let mut policy = limits();
                    match dimension {
                        0 => policy.library.max_output_elements = Some(cap),
                        1 => policy.library.max_output_bytes = Some(cap),
                        2 => policy.library.max_temporary_bytes = Some(cap),
                        3 => policy.library.max_cloned_bytes = Some(cap),
                        4 => policy.library.max_retained_nodes = Some(cap),
                        5 => policy.library.max_compute_work = Some(cap),
                        6 => policy.library.max_comparison_work = Some(cap),
                        7 => policy.library.max_storage_buffer_bytes = Some(cap),
                        _ => policy.execution.max_work = cap,
                    }
                    let prior_fast = fast.clone();
                    let prior_split = split.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (a, arows) = execute_observed_admitted_helper_with_limits(
                        &jit, descriptor, &args, &mut fast, &memory, false, 0, policy,
                    );
                    let (b, brows) = execute_observed_admitted_helper_with_limits(
                        &jit, descriptor, &args, &mut split, &memory, true, 0, policy,
                    );
                    assert_eq!(arows, brows, "dimension {dimension} cap {cap}");
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            assert_eq!(a, b);
                            assert_eq!(snapshots(&fast), snapshots(&split));
                        }
                        (Err(a), Err(b)) => {
                            assert_eq!(
                                format!("{a:?}"),
                                format!("{b:?}"),
                                "dimension {dimension} cap {cap}"
                            );
                            assert_eq!(
                                retained,
                                (memory.used_bytes(), memory.retained_snapshot_nodes())
                            );
                            for (actual, prior) in [(&fast, &prior_fast), (&split, &prior_split)] {
                                assert!(actual.iter().zip(prior).all(|(a, b)| {
                                    a.as_ref()
                                        .unwrap()
                                        .shares_frozen_storage(b.as_ref().unwrap())
                                }));
                            }
                        }
                        other => panic!("limit mismatch {other:?}"),
                    }
                }
            }
            execute_observed_admitted_helper(&jit, descriptor, &args, &mut fast, &memory, false, 0)
                .0
                .unwrap();
            execute_observed_admitted_helper(&jit, descriptor, &args, &mut split, &memory, true, 0)
                .0
                .unwrap();
            assert_eq!(snapshots(&fast), snapshots(&split));
            drop((fast, split, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn admitted_helpers_collection_admission_precedes_bad_pointer_and_invalid_site() {
    let artifact = artifact_source(FILTERED_RANGE);
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = arguments(&artifact, 3, true, 1, &memory);
    let mut outputs = vec![None; artifact.outputs().len()];
    for fault in [1, 3, 5] {
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
        let (a, arows) = execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            false,
            fault,
        );
        let (b, brows) = execute_observed_admitted_helper(
            &jit,
            jit.descriptor(),
            &args,
            &mut outputs,
            &memory,
            true,
            fault,
        );
        assert_eq!(arows, brows);
        let a = a.unwrap_err();
        assert_eq!(format!("{a:?}"), format!("{:?}", b.unwrap_err()));
        let failed = arows.last().unwrap();
        assert_eq!(failed.operation, GlobalHelper::Collection.raw());
        if fault == 3 {
            assert_eq!(failed.work_before, failed.work_after);
            assert!(matches!(
                a,
                NativeProgramExecutionError::Control(NativeIterationExecutionError::Work { .. })
            ));
        }
        if fault == 1 {
            assert!(format!("{a:?}").contains("helper result pointer mismatch"));
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
    }
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
    drop((outputs, args));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn admitted_helpers_managed_tail_initializers_and_latches_use_composed_callbacks() {
    // A String-bearing carrier retains ordinary managed transitions. Numeric
    // scalar/aggregate/phase specializations must keep their existing routes.
    let source = r#"walk(text<string>, left<u64>, n<u64>, flag<bool>) => (out<{text<string> n<u64> flag<bool>}>) :=
      out := ((left > 0u64)? | true => walk(text, left - 1u64, n + 1u64, flag == false) | false => {text: text, n: n, flag: flag}).
    r := walk("kept", 3u64, n<u64>, flag<bool>)
    (r.n / divisor<u64>, r.text, r.flag)
    "#;
    let original = artifact_source(source);
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
            let mut coverage = [0; 2];
            let mut fast = vec![None; artifact.outputs().len()];
            let mut split = fast.clone();
            for (n, flag, divisor) in [(8, false, 3), (12, true, 0), (20, true, 7), (2, false, 1)] {
                let args = arguments(&artifact, n, flag, divisor, &memory);
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior_fast = fast.clone();
                let prior_split = split.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (a, arows) = execute_observed_admitted_helper(
                    &jit, descriptor, &args, &mut fast, &memory, false, 0,
                );
                let (b, brows) = execute_observed_admitted_helper(
                    &jit, descriptor, &args, &mut split, &memory, true, 0,
                );
                assert_eq!(arows, brows);
                for row in &arows {
                    if row.operation == GlobalHelper::Move.raw() {
                        use mech_engine::native_artifact::{
                            NativeControlLoop as L, NativeIterationAction as A,
                        };
                        match descriptor.metadata().plan.blocks()[row.site as usize].action() {
                            A::Initialize { iteration }
                                if matches!(
                                    descriptor.metadata().plan.iterations()[*iteration as usize],
                                    L::Tail(_)
                                ) =>
                            {
                                coverage[0] += 1
                            }
                            A::Latch { iteration }
                                if matches!(
                                    descriptor.metadata().plan.iterations()[*iteration as usize],
                                    L::Tail(_)
                                ) =>
                            {
                                coverage[1] += 1
                            }
                            _ => {}
                        }
                    }
                }
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        assert_eq!(a, b);
                        assert_eq!(snapshots(&fast), snapshots(&split));
                        assert_eq!(
                            snapshots(&fast),
                            vec![ValueDataDraft::Tuple(
                                vec![
                                    ValueDataDraft::U64((n + 3) / divisor),
                                    ValueDataDraft::String("kept".into()),
                                    ValueDataDraft::Bool(!flag)
                                ]
                                .into()
                            )]
                        );
                    }
                    (Err(a), Err(b)) => {
                        assert_eq!(format!("{a:?}"), format!("{b:?}"));
                        assert_eq!(
                            retained,
                            (memory.used_bytes(), memory.retained_snapshot_nodes())
                        );
                        for (actual, prior) in [(&fast, &prior_fast), (&split, &prior_split)] {
                            assert!(actual.iter().zip(prior).all(|(a, b)| {
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(b.as_ref().unwrap())
                            }));
                        }
                    }
                    other => panic!("managed transition mismatch {other:?}"),
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before
                );
            }
            assert!(
                coverage.iter().all(|n| *n > 0),
                "actual Tail init/latch callback composition {coverage:?}"
            );
            drop((fast, split));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
