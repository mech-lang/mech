//! Original-width block counters merge through checked native U32 addition.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramExecutionError as Error};
use std::time::Instant;

const SOURCE: &str = concat!(
    "+> sequence\n",
    include_str!("../fixtures/native/deflate-block-collector.mec"),
);

#[test]
fn block_frequency_merging_retains_all_259_counters_and_recovers() {
    let source = format!("{SOURCE}\noptimal-merge-frequencies(total<[u32]>, fresh<[u32]>)\n");
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        policy.execution.max_work = 10_000_000_000;
        for overflow in [false, true, false] {
            let mut total = vec![Data::U32(0); 259];
            let mut fresh = vec![Data::U32(0); 259];
            total[3] = Data::U32(5);
            fresh[3] = Data::U32(7);
            total[258] = Data::U32(if overflow { u32::MAX } else { 100 });
            fresh[258] = Data::U32(1);
            let args = inputs(
                &artifact,
                &[
                    ("total", Data::Matrix(total.clone().into())),
                    ("fresh", Data::Matrix(fresh.into())),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let start = Instant::now();
            let (result, allocations, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"259-counter-frequency-merge","artifact":form,"overflow":overflow,"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":allocations,"requested_allocation_bytes":bytes,"result":format!("{result:?}")})
            );
            if overflow {
                assert!(
                    matches!(
                        result,
                        Err(Error::Control(
                            mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
                        ))
                    ),
                    "{result:?}"
                );
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                let Data::Record(fields) = output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap()
                else {
                    panic!("merged counters")
                };
                total[3] = Data::U32(12);
                total[258] = Data::U32(101);
                for (name, expected) in [
                    ("total", Data::Matrix(total.into())),
                    ("fresh", Data::Matrix(vec![Data::U32(0); 259].into())),
                ] {
                    assert_eq!(
                        fields
                            .iter()
                            .find(|field| field.name == name)
                            .unwrap()
                            .value,
                        expected
                    );
                }
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn block_observation_merging_keeps_offsets_checked_counts_and_recovery() {
    let source = format!(
        "{SOURCE}\noptimal-merge-observations(fresh<[u32]>, base<u64>, total<[u32]>, 512u64, count<u64>)\n"
    );
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (base, word_overflow, count) in [
            (0, false, 1024),
            (2, true, u64::MAX),
            (0, false, u64::MAX),
            (2, false, 1024),
        ] {
            let mut total = (0..10).map(Data::U32).collect::<Vec<_>>();
            let mut fresh = vec![Data::U32(77); base + 10];
            for index in 0..10 {
                fresh[base + index] = Data::U32(1);
            }
            if word_overflow {
                total[9] = Data::U32(u32::MAX);
            }
            let args = inputs(
                &artifact,
                &[
                    ("fresh", Data::Matrix(fresh.clone().into())),
                    ("base", Data::U64(base as u64)),
                    ("total", Data::Matrix(total.clone().into())),
                    ("count", Data::U64(count)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            if word_overflow || count == u64::MAX {
                let Err(Error::Control(
                    mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { origin },
                )) = result
                else {
                    panic!("{result:?}")
                };
                let op = jit
                    .descriptor()
                    .plan()
                    .blocks()
                    .iter()
                    .find(|block| block.origin() == origin)
                    .map(|block| block.action());
                assert!(
                    matches!(op, Some(mech_engine::native_artifact::NativeIterationAction::Scalar(mech_engine::native_artifact::NativeMatchAction::Primitive { op, .. })) if *op == if word_overflow { mech_engine::native_artifact::NativeScalarOp::U32Add } else { mech_engine::native_artifact::NativeScalarOp::U64Add }),
                    "{op:?}"
                );
                assert_eq!(memory.used_bytes(), retained);
                assert!(output.iter().zip(&prior).all(|(value, old)| {
                    value
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                let Data::Record(fields) = output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap()
                else {
                    panic!("merged observations")
                };
                for index in 0..10 {
                    total[index] = Data::U32(index as u32 + 1);
                    fresh[base + index] = Data::U32(0);
                }
                for (name, expected) in [
                    ("observations", Data::Matrix(total.into())),
                    ("new-observations", Data::Matrix(fresh.into())),
                    ("num-new-observations", Data::U64(0)),
                    ("num-observations", Data::U64(1536)),
                ] {
                    assert_eq!(
                        fields
                            .iter()
                            .find(|field| field.name == name)
                            .unwrap()
                            .value,
                        expected
                    );
                }
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}
