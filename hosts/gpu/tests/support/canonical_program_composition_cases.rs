use super::super::{artifacts, equal, inputs, limits, reference_result};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

const ROOTS: &str = r#"put(xs<[u32]>, at<u64>, word<u32>) => (out<[u32]>) :=
  ~next := xs
  next[at] = word
  out := next.
walk(xs<[u32]>, i<u64>, left<u64>) => <([u32],u64)>
  | (xs, i, left) => ((left > 0u64)? | true => walk(put(xs, i + 1u64, xs[i + 1u64] + 1u32), i + 1u64, left - 1u64) | false => (xs, i)).
parent(xs<[u32]>, old<[u32]>, i<u64>, again<u64>, flag<bool>) => (out<([u32],u32)>) :=
  left := (flag? | true => 1u64 | false => 0u64)
  (next, at) := walk(xs, i, left)
  out := ((again > 0u64)? | true => parent(next, old, at, again - 1u64, flag == false) | false => (next, old[1u64])).
parent(data<[u32]>, old<[u32]>, 0u64, 2u64, true)
"#;

#[test]
fn composition_keeps_distinct_logical_roots_with_shared_backing_and_fresh_predicates() {
    for artifact in artifacts(ROOTS).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata()["compiled_numeric_segments"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let matrix = Data::Matrix([10, 20, 30].map(Data::U32).into());
        let mut args = inputs(
            &artifact,
            &[("data", matrix.clone()), ("old", matrix)],
            &memory,
        );
        let root = args[0].clone();
        let alias = root
            .rebind(args[1].schema(), args[1].shape(), artifact.schemas())
            .unwrap();
        let mut reserve = memory
            .reserve_capacity(alias.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        args[1] = alias.into_memory_budget(&mut reserve).unwrap();
        assert!(args[0].shares_frozen_storage(&args[1]));
        let before = args
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(
            &artifact,
            &output,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(
                vec![
                    Data::Matrix([11, 21, 30].map(Data::U32).into()),
                    Data::U32(10)
                ]
                .into()
            )
        );
        assert_eq!(
            args.iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert!(args[0].shares_frozen_storage(&root) && args[1].shares_frozen_storage(&root));
        drop((args, root, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn composition_preserves_errors_before_child_and_after_completed_child() {
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    for before_child in [true, false] {
        let source = if before_child {
            source.replace(
                "  more := width >= 1u64",
                "  current := width / divisor<u64>\n  more := current >= 1u64",
            )
        } else {
            source.replace(
                "  (next, next-i) := step",
                "  (next, child-i) := step\n  next-i := child-i / divisor<u64>",
            )
        };
        let source = source
            .replace(
                "counts<[u64]>, width<u64>, i<u64>)",
                "counts<[u64]>, width<u64>, i<u64>, divisor<u64>)",
            )
            .replace(
                "next, counts, width - 1u64, next-i)",
                "next, counts, width - 1u64, next-i, divisor)",
            )
            .replace(
                "width<u64>, start<u64>)",
                "width<u64>, start<u64>, divisor<u64>)",
            );
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                !emit_program_object(&artifact, &mech_stdlib::source_catalog())
                    .unwrap()
                    .metadata()["compiled_numeric_segments"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let mut output = vec![None; artifact.outputs().len()];
            for (divisor, symbols, valid) in [
                (1, vec![2, 0, 3, 1], true),
                (
                    0,
                    if before_child {
                        vec![0, 3, 2, 1]
                    } else {
                        vec![2, 0, 3, 1]
                    },
                    false,
                ),
                (1, vec![2, 0, 3, 1], true),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "a",
                            Data::Matrix(symbols.into_iter().map(Data::U32).collect()),
                        ),
                        ("lens", Data::Matrix([9, 9, 9, 9].map(Data::U8).into())),
                        ("counts", Data::Matrix([0, 1, 1, 2].map(Data::U64).into())),
                        ("width", Data::U64(3)),
                        (
                            "start",
                            Data::U64(if before_child && !valid { u64::MAX } else { 0 }),
                        ),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let original = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior = output.clone();
                let published = prior
                    .iter()
                    .map(|v: &Option<mech_core::Value>| {
                        v.as_ref().map(|v| v.canonical_data_draft().unwrap())
                    })
                    .collect::<Vec<_>>();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
                let resident = reference_result(&artifact, &args);
                if valid {
                    native.unwrap();
                    equal(&artifact, &output, &resident.unwrap());
                } else {
                    let error = native.unwrap_err();
                    let mech_gpu::native_artifact::NativeProgramExecutionError::Control(
                        mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic {
                            origin,
                        },
                    ) = error
                    else {
                        panic!("{error:?}")
                    };
                    let divisions = jit
                        .descriptor()
                        .plan()
                        .blocks()
                        .iter()
                        .filter_map(|block| match block.action() {
                            mech_engine::native_artifact::NativeIterationAction::Scalar(
                                mech_engine::native_artifact::NativeMatchAction::Primitive {
                                    op: mech_engine::native_artifact::NativeScalarOp::U64Div,
                                    ..
                                },
                            ) => Some(block.origin()),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(divisions, [origin]);
                    let resident = resident.unwrap_err();
                    assert!(resident.contains("Arithmetic"), "{resident}");
                    assert_eq!(
                        output
                            .iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>(),
                        published
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    original
                );
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn composition_handoff_preserves_the_existing_payload_copy_slope() {
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut observed = Vec::new();
        for length in [288, 8192] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("a", Data::Matrix([2, 0, 3, 1].map(Data::U32).into())),
                    ("lens", Data::Matrix(vec![Data::U8(9); length].into())),
                    ("counts", Data::Matrix([0, 1, 1, 2].map(Data::U64).into())),
                    ("width", Data::U64(3)),
                    ("start", Data::U64(0)),
                ],
                &memory,
            );
            let alias = args.clone();
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            output.fill(None);
            let (run, requests, bytes) =
                super::super::allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                });
            run.unwrap();
            let mut expected = vec![Data::U8(9); length];
            for (slot, value) in [3, 1, 3, 2].into_iter().enumerate() {
                expected[slot] = Data::U8(value);
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
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
            assert!(
                args.iter()
                    .zip(&alias)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            observed.push((length, requests, bytes));
            println!(
                "{}",
                serde_json::json!({"witness":"parent-child-payload-allocation", "length":length,"requests":requests,"bytes":bytes,"timing_observations":0})
            );
            drop((output, args, alias));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
        assert_eq!(observed[1].1, observed[0].1);
        // The retained compiler also allocates three payloads for these
        // successive calls. Composition may improve that inherited slope,
        // but must not add a fourth backing through a stale parent alias.
        assert!(
            (observed[1].2 - observed[0].2) <= 3 * (observed[1].0 - observed[0].0),
            "additional payload copy at parent handoff: {observed:?}"
        );
    }
}
