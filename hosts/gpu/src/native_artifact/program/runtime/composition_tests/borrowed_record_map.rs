//! Real current canonical Record views, paired fallback, limits and retry.
use super::*;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use std::{cell::Cell, ffi::c_void};

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/retained-compound-map.mec");

thread_local! {
    static ORIGINAL_HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
    static FORCE_ORDINARY: Cell<bool> = const { Cell::new(false) };
    static REPLACE_BEFORE_READ: Cell<bool> = const { Cell::new(false) };
    static REPLACEMENTS: Cell<u64> = const { Cell::new(0) };
}

unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    op: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    if op == GlobalHelper::BorrowedRecordMapProject.raw() && FORCE_ORDINARY.with(Cell::get) {
        // SAFETY: the checked caller is suspended at the exact selected view
        // boundary. Forget only optional metadata; Values and work stay intact.
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        invocation.test_forget_borrowed_record_map_publication();
    }
    if op == GlobalHelper::BorrowedRecordMapRead.raw()
        && REPLACE_BEFORE_READ.with(|flag| flag.replace(false))
    {
        // Replace only the generation of the exact same admitted canonical
        // Record. Its immutable payload, schema and account stay identical.
        // A pointer/data-equality guard would miss this stale-view witness.
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        let id = invocation.descriptor.metadata().borrowed_record_maps.reads[site as usize]
            .expect("selected read");
        let source = invocation
            .descriptor
            .metadata()
            .borrowed_record_maps
            .regions[id]
            .source;
        let before = invocation
            .state
            .arena
            .current_handle(source, &mut invocation.state.control.work)
            .unwrap();
        let current = invocation
            .state
            .arena
            .copy_current(source, &mut invocation.state.control.work)
            .unwrap();
        let after = invocation
            .state
            .arena
            .put(source, &current, &mut invocation.state.control.work)
            .unwrap();
        assert_ne!(
            before, after,
            "test must actually replace the current generation"
        );
        REPLACEMENTS.with(|count| count.set(count.get() + 1));
    }
    // SAFETY: the same synchronous callback/context and all lanes are forwarded
    // exactly once. No current Value or helper result is substituted.
    unsafe { ORIGINAL_HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
}

fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    restored: bool,
    force_ordinary: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 4],
) {
    run_with_policy(
        jit,
        args,
        output,
        NativeProgramLimitMode::NoLimits.execution(),
        memory,
        restored,
        force_ordinary,
    )
}

fn run_with_policy(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
    force_ordinary: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 4],
) {
    use super::super::maps::owner::borrowed_records::take_observation;
    let decoded = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = decoded.as_ref().unwrap_or_else(|| jit.descriptor());
    take_observation();
    FORCE_ORDINARY.with(|value| value.set(force_ordinary));
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| {
            // SAFETY: execute retains the initialized frame and this exact JIT
            // owner. The original helper is restored before returning.
            unsafe {
                let original = (*raw).helper;
                ORIGINAL_HELPER.with(|value| value.set(Some(original)));
                (*raw).helper = helper;
                let status = entry(raw);
                (*raw).helper = original;
                ORIGINAL_HELPER.with(|value| value.set(None));
                status
            }
        },
        args,
        output,
        policy,
        memory,
    );
    FORCE_ORDINARY.with(|value| value.set(false));
    (result, take_observation())
}

#[test]
fn borrowed_record_map_reads_current_publication_and_keeps_exact_paired_fallback() {
    let catalog = mech_stdlib::source_catalog();
    let mut expected_errors = [None, None, None, None];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let metadata = jit.descriptor().metadata();
        let independent =
            crate::native_artifact::program::prepare::borrowed_record_map::Layout::derive(
                jit.descriptor().plan(),
                metadata.map_owner_words.as_ref().unwrap(),
            )
            .unwrap();
        assert_eq!(independent, metadata.borrowed_record_maps);
        assert_eq!(independent.regions.len(), 1, "fixture must select the pair");
        let proof = &independent.regions[0];
        assert_eq!(independent.projects[proof.project], Some(0));
        assert_eq!(independent.reads[proof.read], Some(0));
        assert_ne!(proof.source, proof.output);
        for restored in [false, true] {
            for force_ordinary in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (keys, steps, seed, old, allowed, divisor, query, expected, error) in [
                    (
                        vec!["a", "a", "é", "界"],
                        4,
                        3,
                        7,
                        true,
                        1,
                        "é",
                        Some(12),
                        None,
                    ),
                    (vec!["x"], 1, 10, 4, true, 1, "x", Some(23), None),
                    (vec![], 0, 8, 91, true, 1, "old", Some(101), None),
                    (vec!["old", "old"], 2, 3, 5, true, 1, "old", Some(10), None),
                    (vec!["a"], 1, 3, 7, true, 1, "missing", None, Some(0)),
                    (vec!["a"], 1, 3, 7, false, 0, "a", None, Some(1)),
                    (vec!["a"], 1, 3, 7, true, 0, "a", None, Some(2)),
                    (vec![], 1, 3, 7, true, 1, "a", None, Some(3)),
                    (vec!["a"], 1, 2, 9, true, 1, "a", Some(7), None),
                ] {
                    let args = retained_compound_map::arguments(
                        &artifact, &memory, &keys, steps, seed, old, allowed, divisor, query,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, routes) =
                        run(&jit, &args, &mut output, &memory, restored, force_ordinary);
                    if let Some(index) = error {
                        let actual = format!("{:?}", result.unwrap_err());
                        assert_eq!(
                            actual,
                            *expected_errors[index].get_or_insert(actual.clone()),
                            "selected/fallback/original/decoded/restored error origin differs"
                        );
                        for (current, old) in output.iter().zip(&previous) {
                            assert!(
                                current
                                    .as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            );
                        }
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        let report = result.unwrap();
                        assert!(report.work > 0);
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected.unwrap())
                        );
                        assert_eq!(
                            routes,
                            if force_ordinary {
                                [0, 1, 0, 1]
                            } else {
                                [1, 0, 1, 0]
                            },
                            "actual physical projection/read routing must match the current witness"
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn borrowed_record_map_finite_boundaries_keep_publication_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            for dimension in [
                "control",
                "library",
                "comparison",
                "memory",
                "nodes",
                "transport",
            ] {
                // Fresh retained tables have randomized probe work. Exact work
                // boundaries use one existing key. Distinct/new Unicode keys
                // retain the strict memory/node/transport boundaries, with
                // ample work limits; semantic tests cover both routes too.
                let fixed_work = matches!(dimension, "control" | "library" | "comparison");
                let memory = mode.memory_budget();
                let (keys, steps, query): (&[&str], u64, &str) = if fixed_work {
                    (&["old", "old", "old"], 3, "old")
                } else {
                    (&["a", "a", "é", "界"], 4, "é")
                };
                let args = retained_compound_map::arguments(
                    &artifact, &memory, keys, steps, 3, 7, true, 1, query,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut published = vec![None; artifact.outputs().len()];
                let (result, routes) = run(&jit, &args, &mut published, &memory, restored, false);
                result.unwrap();
                assert_eq!(routes, [1, 0, 1, 0]);
                let expected = contents(
                    &published
                        .iter()
                        .map(|v| v.as_ref().unwrap().clone())
                        .collect::<Vec<_>>(),
                );
                assert_eq!(expected.last(), Some(&Data::U64(12)));
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let trial = |allowance| {
                    let mut policy = mode.execution();
                    let mut budget = memory.clone();
                    match dimension {
                        "control" => policy.execution.max_work = allowance,
                        "library" => policy.library.max_compute_work = Some(allowance),
                        "comparison" => policy.library.max_comparison_work = Some(allowance),
                        "memory" => budget = memory.with_limits(allowance, u64::MAX),
                        "nodes" => budget = memory.with_limits(u64::MAX, allowance),
                        "transport" => policy.execution.max_transport_bytes = allowance,
                        _ => unreachable!(),
                    }
                    let mut output = published.clone();
                    let (result, _) =
                        run_with_policy(&jit, &args, &mut output, policy, &budget, restored, false);
                    if result.is_err() {
                        for (value, old) in output.iter().zip(&published) {
                            assert!(
                                value
                                    .as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            );
                        }
                    }
                    assert_eq!(
                        contents(
                            &output
                                .iter()
                                .map(|v| v.as_ref().unwrap().clone())
                                .collect::<Vec<_>>()
                        ),
                        expected
                    );
                    let pass = result.is_ok();
                    drop(output);
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                    pass
                };
                let (mut lo, mut hi) = (0, 1_000_000_000_000u64);
                assert!(trial(hi), "{dimension}");
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if trial(mid) {
                        hi = mid;
                    } else {
                        lo = mid + 1;
                    }
                }
                assert!(trial(lo), "{dimension} exact={lo}");
                if lo > 0 {
                    assert!(!trial(lo - 1), "{dimension} one-short={lo}");
                }
                assert!(trial(1_000_000_000_000));
                println!(
                    "{}",
                    serde_json::json!({"witness":"borrowed-record-map-finite-boundary","dimension":dimension,"minimum":lo,"fixed_work_witness":fixed_work,"restored":restored})
                );
                drop((published, args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn borrowed_record_map_rejects_a_replaced_generation_before_read_and_recovers() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(
            jit.descriptor()
                .metadata()
                .borrowed_record_maps
                .regions
                .len(),
            1
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args =
                retained_compound_map::arguments(&artifact, &memory, &["a"], 1, 3, 7, true, 1, "a");
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            let (result, routes) = run(&jit, &args, &mut output, &memory, restored, false);
            result.unwrap();
            assert_eq!(routes, [1, 0, 1, 0]);
            let published = output.clone();
            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
            REPLACEMENTS.with(|value| value.set(0));
            REPLACE_BEFORE_READ.with(|value| value.set(true));
            let (result, routes) = run(&jit, &args, &mut output, &memory, restored, false);
            REPLACE_BEFORE_READ
                .with(|value| assert!(!value.replace(false), "read fault hook must execute"));
            REPLACEMENTS.with(|value| assert_eq!(value.get(), 1));
            let error = format!("{:?}", result.unwrap_err());
            assert!(
                error.contains("stale or foreign borrowed Record Map root"),
                "{error}"
            );
            assert_eq!(
                routes[0], 1,
                "projection must acquire the real native view first"
            );
            for (current, old) in output.iter().zip(&published) {
                assert!(
                    current
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                );
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                held
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            let (result, routes) = run(&jit, &args, &mut output, &memory, restored, false);
            result.unwrap();
            assert_eq!(routes, [1, 0, 1, 0]);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::U64(9)
            );
            drop((published, output, args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn borrowed_record_map_declines_multiple_reads_public_escape_and_map_writes() {
    let body = "  out := words.index{key} + words.count + marker.";
    let variants = [
        ("two-reads", SOURCE.replace(body, "  index := words.index\n  out := index{key} + index{\"old\"} + words.count + marker.")),
        ("public-map", SOURCE.replace("consume(words<dictionary>, marker<u64>, key<string>) => (out<u64>)", "consume(words<dictionary>, marker<u64>, key<string>) => (out<({string:u64},u64)>)").replace(body, "  index := words.index\n  out := (index, index{key} + words.count + marker).")),
        ("map-write", SOURCE.replace(body, "  index := words.index\n  updated := map/insert(index, key, 99u64)\n  out := index{key} + updated{key} + words.count + marker.")),
    ];
    for (name, source) in variants {
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .borrowed_record_maps
                    .regions
                    .is_empty(),
                "{name}: complete-use proof must decline"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let args = retained_compound_map::arguments(
                    &artifact,
                    &memory,
                    &["a"],
                    1,
                    3,
                    7,
                    true,
                    1,
                    "a",
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut output = vec![None; artifact.outputs().len()];
                let (result, routes) = run(&jit, &args, &mut output, &memory, restored, false);
                result.unwrap();
                assert_eq!(routes, [0; 4]);
                let actual = output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap();
                match name {
                    "two-reads" => assert_eq!(actual, Data::U64(16)),
                    "map-write" => assert_eq!(actual, Data::U64(108)),
                    "public-map" => {
                        let Data::Tuple(values) = actual else {
                            panic!("canonical tuple");
                        };
                        assert_eq!(values[1], Data::U64(9));
                        let Data::Map(entries) = &values[0] else {
                            panic!("canonical Map");
                        };
                        let entries = entries
                            .iter()
                            .map(|entry| {
                                let [Data::String(key), Data::U64(value)] = entry.items.as_ref()
                                else {
                                    panic!("String/u64 pair");
                                };
                                (key.as_str(), *value)
                            })
                            .collect::<std::collections::BTreeMap<_, _>>();
                        assert_eq!(entries, [("a", 3), ("old", 7)].into_iter().collect());
                    }
                    _ => unreachable!(),
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                drop((output, args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
