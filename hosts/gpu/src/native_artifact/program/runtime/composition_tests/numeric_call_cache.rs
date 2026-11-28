//! Optional numeric metadata must fit alongside current payloads, not a policy
//! ceiling. Exercise genuinely selected maintained leaves and their old route.
use super::*;
use mech_core::snapshot::F64Bits;
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget, NativePreparedLibraryCallCache,
};

fn fixtures() -> Vec<(&'static str, Primitive)> {
    vec![
        ("input<[f64]>'\n", Primitive::MatrixTranspose),
        (
            "+> stats\nstats/sum/column(input<[f64]>)\n",
            Primitive::SumColumns,
        ),
        (
            "+> stats\nstats/sum/row(input<[f64]>)\n",
            Primitive::SumRows,
        ),
        ("+> math\nmath/sqrt(input<[f64]>)\n", Primitive::MatrixSqrt),
        ("input<[f64]> + peer<[f64]>\n", Primitive::MatrixAdd),
        ("input<[f64]> - peer<[f64]>\n", Primitive::MatrixSubtract),
        ("input<[f64]> * peer<[f64]>\n", Primitive::MatrixMultiply),
        ("input<[f64]> / peer<[f64]>\n", Primitive::MatrixDivide),
        ("input<[f64]> ** peer<[f64]>\n", Primitive::MatrixMatmul),
        (
            "+> math\nmath/pow(input<[f64]>,peer<f64>)\n",
            Primitive::F64Power,
        ),
        (
            "+> math\nmath/pow(input<f64>,peer<f64>)\n",
            Primitive::F64Power,
        ),
    ]
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    primitive: Primitive,
    shape: [u64; 2],
    offset: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let schema = artifact.schemas().get(input.schema).unwrap();
            let dims = if name == "peer" {
                if primitive == Primitive::MatrixMatmul {
                    [shape[1], shape[0]]
                } else {
                    [1, shape[1]]
                }
            } else {
                shape
            };
            let (data, shape_values) =
                if matches!(schema.body(), mech_core::SchemaBody::Matrix { .. }) {
                    let shape = mech_core::shape_for_resolved_extents(schema, &dims).unwrap();
                    (
                        Data::Matrix(
                            (0..dims[0] * dims[1])
                                .map(|i| {
                                    Data::F64(F64Bits::from_f64(((i + offset) % 11 + 1) as f64))
                                })
                                .collect(),
                        ),
                        shape.parameter_values().into(),
                    )
                } else {
                    (
                        Data::F64(F64Bits::from_f64(if name == "peer" {
                            2.0
                        } else {
                            offset as f64 + 1.0
                        })),
                        Box::new([]) as Box<[u64]>,
                    )
                };
            let raw = mech_core::snapshot::ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(raw.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            Some(raw.into_memory_budget(&mut reservation).unwrap())
        })
        .map(Option::unwrap)
        .collect()
}
fn draft(output: &Option<Value>) -> Data {
    output.as_ref().unwrap().canonical_data_draft().unwrap()
}

#[test]
fn numeric_call_cache_retains_real_bindings_with_full_ceiling_live_inputs_and_fresh_geometry() {
    for (source, primitive) in fixtures() {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let call = jit
                .descriptor()
                .metadata()
                .calls
                .iter()
                .flatten()
                .find(|call| call.template().primitive() == primitive)
                .unwrap();
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut policy = NativeProgramLimitMode::NoLimits.execution().library;
            policy.max_temporary_bytes = Some(memory.limit_bytes());
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            let mut ordinary = None;
            for (shape, offset) in [
                ([2, 3], 1),
                ([2, 3], 1),
                ([2, 3], 5),
                ([3, 2], 2),
                ([3, 2], 2),
                ([0, 3], 1),
                ([2, 3], 1),
            ] {
                let args = arguments(&artifact, &memory, primitive, shape, offset);
                let before = contents(&args);
                let values = args.iter().cloned().map(Some).collect::<Vec<_>>();
                let used = work.used();
                let a = call.invoke_current_cached(&mut cache, &values, &mut output, policy, &work);
                let delta = work.used().0 - used.0;
                let b = call.invoke_current(&values, &mut ordinary, policy, &work);
                assert_eq!(
                    a.is_ok(),
                    b.is_ok(),
                    "{primitive:?} {shape:?}: {a:?} / {b:?}"
                );
                if a.is_ok() {
                    assert_eq!(draft(&output), draft(&ordinary), "{primitive:?} {shape:?}");
                    let retained = cache.retained_metadata_demand(&work).unwrap();
                    assert!(
                        retained.turn_peak_bytes > 0,
                        "actual bound call cache {primitive:?}"
                    );
                    assert!(delta > 0, "fresh finite work on a cache hit");
                }
                assert_eq!(contents(&args), before);
            }
            let args = arguments(&artifact, &memory, primitive, [2, 3], 3);
            let values = args.iter().cloned().map(Some).collect::<Vec<_>>();
            call.invoke_current_cached(&mut cache, &values, &mut output, policy, &work)
                .unwrap();
            let prior = output.clone();
            let foreign = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            assert!(
                call.invoke_current_cached(&mut cache, &values, &mut output, policy, &foreign)
                    .is_err()
            );
            assert!(
                output
                    .as_ref()
                    .unwrap()
                    .shares_frozen_storage(prior.as_ref().unwrap())
            );
            call.invoke_current_cached(&mut cache, &values, &mut output, policy, &work)
                .unwrap();
            drop((prior, values, args, cache, output, ordinary, work, foreign));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_call_cache_cold_and_warm_declines_do_not_starve_a_viable_call() {
    for artifact in artifacts("input<[f64]>'\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let call = jit
            .descriptor()
            .metadata()
            .calls
            .iter()
            .flatten()
            .find(|call| call.template().primitive() == Primitive::MatrixTranspose)
            .unwrap();
        for dimension in ["bytes", "nodes"] {
            let memory =
                ManagedMemoryBudget::new(32 * 1024 * 1024).with_limits(32 * 1024 * 1024, 100_000);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut policy = NativeProgramLimitMode::NoLimits.execution().library;
            policy.max_temporary_bytes = Some(memory.limit_bytes());
            let args = arguments(&artifact, &memory, Primitive::MatrixTranspose, [2, 3], 1);
            let values = args.iter().cloned().map(Some).collect::<Vec<_>>();
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut published = None;
            call.invoke_current_cached(&mut cache, &values, &mut published, policy, &work)
                .unwrap();
            assert!(
                cache
                    .retained_metadata_demand(&work)
                    .unwrap()
                    .turn_peak_bytes
                    > 0
            );
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let available = if dimension == "bytes" {
                memory.limit_bytes() - memory.used_bytes()
            } else {
                memory.limit_nodes() - memory.retained_snapshot_nodes()
            };
            let trial = |allowance| {
                let bytes = (dimension == "bytes")
                    .then(|| memory.reserve_capacity(available - allowance).unwrap());
                let nodes = (dimension == "nodes")
                    .then(|| memory.reserve_nodes(available - allowance).unwrap());
                let mut out = published.clone();
                let pass = call
                    .invoke_current(&values, &mut out, policy, &work)
                    .is_ok();
                if pass {
                    assert_eq!(draft(&out), draft(&published));
                } else {
                    assert!(
                        out.as_ref()
                            .unwrap()
                            .shares_frozen_storage(published.as_ref().unwrap())
                    );
                }
                drop((out, bytes, nodes));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
                pass
            };
            let (mut low, mut high) = (0, available);
            assert!(trial(high));
            while low < high {
                let middle = low + (high - low) / 2;
                if trial(middle) {
                    high = middle;
                } else {
                    low = middle + 1;
                }
            }
            assert!(low > 0 && trial(low) && !trial(low - 1));
            let bytes =
                (dimension == "bytes").then(|| memory.reserve_capacity(available - low).unwrap());
            let nodes =
                (dimension == "nodes").then(|| memory.reserve_nodes(available - low).unwrap());
            let mut out = published.clone();
            call.invoke_current_cached(&mut cache, &values, &mut out, policy, &work)
                .unwrap();
            assert_eq!(draft(&out), draft(&published));
            drop(out);
            // A warm hit can need less additional capacity than an ordinary
            // rebind and legitimately keep its cache at this boundary.
            let mut cold_cache = NativePreparedLibraryCallCache::default();
            let mut fresh = published.clone();
            call.invoke_current_cached(&mut cold_cache, &values, &mut fresh, policy, &work)
                .unwrap();
            assert_eq!(draft(&fresh), draft(&published));
            drop((fresh, cold_cache));
            drop((bytes, nodes));
            call.invoke_current_cached(&mut cache, &values, &mut published, policy, &work)
                .unwrap();
            assert!(
                cache
                    .retained_metadata_demand(&work)
                    .unwrap()
                    .turn_peak_bytes
                    > 0
            );
            drop((values, args, published, cache, work));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_call_cache_keeps_finite_kernel_limits_atomic_outputs_and_retry() {
    for (source, primitive) in fixtures().into_iter().filter(|(_, p)| {
        matches!(
            p,
            Primitive::MatrixTranspose | Primitive::MatrixMatmul | Primitive::SumColumns
        )
    }) {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let call = jit
                .descriptor()
                .metadata()
                .calls
                .iter()
                .flatten()
                .find(|call| call.template().primitive() == primitive)
                .unwrap();
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut full = NativeProgramLimitMode::NoLimits.execution().library;
            full.max_temporary_bytes = Some(memory.limit_bytes());
            let args = arguments(&artifact, &memory, primitive, [2, 3], 1);
            let before = contents(&args);
            let values = args.iter().cloned().map(Some).collect::<Vec<_>>();
            let mut published = None;
            call.invoke_current(&values, &mut published, full, &work)
                .unwrap();
            let expected = draft(&published);
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for dimension in 0..7 {
                for cap in [0, 1, 5, 16, 255, 4095, 65535] {
                    let mut policy = full;
                    match dimension {
                        0 => policy.max_output_elements = Some(cap),
                        1 => policy.max_output_bytes = Some(cap),
                        2 => policy.max_temporary_bytes = Some(cap),
                        3 => policy.max_cloned_bytes = Some(cap),
                        4 => policy.max_retained_nodes = Some(cap),
                        5 => policy.max_compute_work = Some(cap),
                        _ => policy.max_comparison_work = Some(cap),
                    }
                    let mut cache = NativePreparedLibraryCallCache::default();
                    let mut actual = published.clone();
                    let mut old = published.clone();
                    let a =
                        call.invoke_current_cached(&mut cache, &values, &mut actual, policy, &work);
                    let b = call.invoke_current(&values, &mut old, policy, &work);
                    if dimension < 5 {
                        assert_eq!(
                            a.is_ok(),
                            b.is_ok(),
                            "{primitive:?} dimension {dimension} cap {cap}: {a:?} / {b:?}"
                        );
                    }
                    if a.is_ok() {
                        assert_eq!(draft(&actual), expected);
                    } else {
                        assert!(
                            actual
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(published.as_ref().unwrap())
                        );
                    }
                    if b.is_ok() {
                        assert_eq!(draft(&old), expected);
                    } else {
                        assert!(
                            old.as_ref()
                                .unwrap()
                                .shares_frozen_storage(published.as_ref().unwrap())
                        );
                    }
                    drop((cache, actual, old));
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
            }
            let mut cache = NativePreparedLibraryCallCache::default();
            call.invoke_current_cached(&mut cache, &values, &mut published, full, &work)
                .unwrap();
            assert_eq!(draft(&published), expected);
            drop((cache, published, values, args, work));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_call_cache_evicts_warm_metadata_for_a_grown_viable_payload() {
    for artifact in artifacts("input<[f64]>'\n") {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let call = jit
            .descriptor()
            .metadata()
            .calls
            .iter()
            .flatten()
            .find(|c| c.template().primitive() == Primitive::MatrixTranspose)
            .unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let mut policy = NativeProgramLimitMode::NoLimits.execution().library;
        policy.max_temporary_bytes = Some(memory.limit_bytes());
        let small = arguments(&artifact, &memory, Primitive::MatrixTranspose, [2, 3], 1);
        let tiny = small.iter().cloned().map(Some).collect::<Vec<_>>();
        let grown = arguments(
            &artifact,
            &memory,
            Primitive::MatrixTranspose,
            [256, 256],
            3,
        );
        let large = grown.iter().cloned().map(Some).collect::<Vec<_>>();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut published = None;
        call.invoke_current_cached(&mut cache, &tiny, &mut published, policy, &work)
            .unwrap();
        assert!(
            cache
                .retained_metadata_demand(&work)
                .unwrap()
                .turn_peak_bytes
                > 0
        );
        drop(cache);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let available = memory.limit_bytes() - memory.used_bytes();
        let trial = |allowance| {
            let padding = memory.reserve_capacity(available - allowance).unwrap();
            let mut out = published.clone();
            let pass = call.invoke_current(&large, &mut out, policy, &work).is_ok();
            drop((out, padding));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
            pass
        };
        let (mut low, mut high) = (0, available);
        assert!(trial(high));
        while low < high {
            let middle = low + (high - low) / 2;
            if trial(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(low > 0 && trial(low) && !trial(low - 1));
        let mut cache = NativePreparedLibraryCallCache::default();
        call.invoke_current_cached(&mut cache, &tiny, &mut published, policy, &work)
            .unwrap();
        let optional = memory.used_bytes() - baseline.0;
        assert!(
            optional > 0 && low >= optional,
            "grown call must have room for the existing cache: {low} / {optional}"
        );
        let padding = memory.reserve_capacity(available - low).unwrap();
        let mut actual = published.clone();
        call.invoke_current_cached(&mut cache, &large, &mut actual, policy, &work)
            .unwrap();
        assert_eq!(
            cache
                .retained_metadata_demand(&work)
                .unwrap()
                .turn_peak_bytes,
            0,
            "warm optional state must retire before original grown invocation"
        );
        drop((actual, padding, cache));
        let mut expected = published.clone();
        call.invoke_current(&large, &mut expected, policy, &work)
            .unwrap();
        drop((expected, published, large, grown, tiny, small, work));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
