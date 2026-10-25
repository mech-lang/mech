//! Real call-lane construction and original/decoded generated invocation.
use super::*;

fn limits() -> NativeProgramExecutionLimits {
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library: policy(),
    }
}

#[test]
fn shared_call_lane_node_ceiling_denies_before_backing_allocation() {
    let [artifact, _] = artifacts("+> sequence\nsequence/length(seed<string>)\n");
    let jit = super::super::super::super::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    let bytes = CallCacheLane::metadata_bytes(descriptor.plan.blocks().len()).unwrap();
    let memory = ManagedMemoryBudget::new(bytes * 4).with_limits(bytes * 4, bytes * 2 - 1);
    let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
    let first = CallCacheLane::new(descriptor, bytes, &memory, &mut work).unwrap();
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    assert_eq!(baseline, (bytes, bytes));
    let (denied, stats) = call_cache_allocation::with_extra(0, &memory, || {
        CallCacheLane::new(descriptor, bytes, &memory, &mut work)
    });
    assert!(
        matches!(denied, Err(NativeProgramExecutionError::Base(NativeExecutionError::Memory {
        error: mech_core::MemoryRuntimeError::BudgetExceeded { operation: Some(operation), .. }
    })) if operation == "retained node admission")
    );
    assert_eq!(
        stats.0, 0,
        "node refusal must precede allocation and entry initialization"
    );
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    drop(first);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    let recovery = CallCacheLane::new(descriptor, bytes, &memory, &mut work).unwrap();
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    drop(recovery);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn actual_lane_exact_and_one_short_keep_setup_error_order_and_debit() {
    for source in [
        "+> sequence\nsequence/length(seed<string>)\n",
        "+> sequence\ntext := payload<string>\nstep(x<u64>) => (next<u64>) := next := x + sequence/length(text).\nseed<u64> |4> step\n",
    ] {
        for artifact in artifacts(source) {
            let jit = super::super::super::super::JitProgramArtifact::compile(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            let descriptor = jit.descriptor();
            let count = descriptor.plan.blocks().len();
            let bytes = CallCacheLane::metadata_bytes(count).unwrap();
            let debit = bytes * 8 + 128;
            // These are actual lane-constructor boundaries, NOT whole-turn peaks.
            for mode in 0..5 {
                let memory = ManagedMemoryBudget::new(if mode == 1 { bytes - 1 } else { bytes });
                let mut work =
                    NativeWorkMeter::new(if mode == 2 { debit - 1 } else { debit }).unwrap();
                let cap = if mode == 3 { bytes - 1 } else { bytes };
                let extra = usize::from(mode == 4);
                let (result, stats) = call_cache_allocation::with_extra(extra, &memory, || {
                    CallCacheLane::new(descriptor, cap, &memory, &mut work)
                });
                match mode {
                    0 => {
                        let lane = result.unwrap();
                        assert_eq!((stats.0, stats.1, stats.2), (1, count, 0));
                        assert_eq!(memory.used_bytes(), bytes);
                        assert!(
                            lane.entries
                                .iter()
                                .zip(descriptor.plan.blocks())
                                .all(|(entry, block)| matches!(entry, CallCacheEntry::Once)
                                    == block.iteration().is_none())
                        );
                        drop(lane);
                        assert_eq!(work.used(), debit);
                    }
                    1 => {
                        assert!(matches!(
                            result,
                            Err(NativeProgramExecutionError::Base(
                                NativeExecutionError::Memory {
                                    error: mech_core::MemoryRuntimeError::BudgetExceeded { .. }
                                }
                            ))
                        ));
                        assert_eq!(work.used(), debit);
                        assert_eq!(stats.0, 0);
                    }
                    2 => {
                        assert!(matches!(result, Err(NativeProgramExecutionError::Base(
                        NativeExecutionError::WorkLimit { phase: NativeExecutionPhase::Setup, required, limit }
                    )) if required == debit && limit == debit - 1));
                        assert_eq!(work.used(), 0);
                        assert_eq!(stats.0, 0);
                    }
                    3 => {
                        assert!(matches!(result, Err(NativeProgramExecutionError::Base(
                        NativeExecutionError::TransportLimit { required, limit }
                    )) if required == bytes && limit == bytes - 1));
                        assert_eq!(work.used(), 0);
                        assert_eq!(stats.0, 0);
                    }
                    _ => {
                        assert!(matches!(
                            result,
                            Err(NativeProgramExecutionError::Base(
                                NativeExecutionError::Descriptor {
                                    reason: "call cache backing exceeds admitted capacity"
                                }
                            ))
                        ));
                        assert_eq!(stats, (1, 0, 1, bytes));
                        assert_eq!(work.used(), debit);
                    }
                }
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn actual_jit_excess_lane_preserves_prior_output_and_same_account_recovery() {
    for artifact in artifacts("+> sequence\nsequence/length(seed<string>)\n") {
        let jit = super::super::super::super::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let descriptor = jit.descriptor();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [text(
            descriptor,
            only_leaf(descriptor),
            "e\u{301}A",
            &memory,
        )];
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs[0].as_ref().unwrap().clone();
        assert!(matches!(prior.data(), ValueData::U64(2)));
        let baseline = memory.used_bytes();
        let bytes = CallCacheLane::metadata_bytes(descriptor.plan.blocks().len()).unwrap();
        let (result, stats) = call_cache_allocation::with_extra(1, &memory, || {
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
        });
        assert!(matches!(
            result,
            Err(NativeProgramExecutionError::Base(
                NativeExecutionError::Descriptor {
                    reason: "call cache backing exceeds admitted capacity"
                }
            ))
        ));
        assert_eq!((stats.0, stats.1, stats.2), (1, 0, 1));
        assert!(stats.3 >= baseline + bytes);
        assert!(outputs[0].as_ref().unwrap().shares_frozen_storage(&prior));
        assert_eq!(memory.used_bytes(), baseline);
        let (result, stats) = call_cache_allocation::with_extra(0, &memory, || {
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
        });
        result.unwrap();
        assert_eq!(
            (stats.0, stats.1, stats.2),
            (1, descriptor.plan.blocks().len(), 0)
        );
        assert!(matches!(
            outputs[0].as_ref().unwrap().data(),
            ValueData::U64(2)
        ));
        drop((outputs, prior, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}
