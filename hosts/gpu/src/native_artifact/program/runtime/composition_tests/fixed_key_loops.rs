//! Differential native execution, genuine current loans, and cold failures.
use super::dictionary_seed_frames::{arguments, data, source};
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::{
    GlobalHelper,
    abi::{Frame, Helper},
};
use crate::native_artifact::program::runtime::maps::fixed_keys;
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
};

thread_local! {
    static ORIGINAL: Cell<Option<Helper>> = const { Cell::new(None) };
    static HELPERS: Cell<[u64;2]> = const { Cell::new([0;2]) };
    static PEER_BYTES: RefCell<Option<mech_core::ManagedMemoryReservation>> = const { RefCell::new(None) };
    static INJECT_BYTES: Cell<bool> = const { Cell::new(false) };
    static PEER_SOURCE: RefCell<Option<Value>> = const { RefCell::new(None) };
    static PEER_SNAPSHOT: RefCell<Option<Value>> = const { RefCell::new(None) };
}
unsafe extern "C" fn observed(
    raw: *mut c_void,
    site: u64,
    op: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    let keys = &ctx.descriptor.metadata().fixed_key_loops;
    let producer = keys.regions.iter().any(|r| r.cell == site as usize)
        && op == GlobalHelper::MapMatrixKey.raw();
    let writer = keys.regions.iter().any(|r| r.insert == site as usize)
        && op == GlobalHelper::MapOwnerCall.raw();
    HELPERS.with(|n| {
        let mut v = n.get();
        v[0] += u64::from(producer);
        v[1] += u64::from(writer);
        n.set(v);
    });
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) };
    if status == 0
        && op == GlobalHelper::ClosedTailBegin.raw()
        && fixed_keys::observations(false)[0] != 0
        && INJECT_BYTES.with(Cell::get)
    {
        INJECT_BYTES.with(|v| v.set(false));
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        let amount = ctx.limits.max_temporary_bytes.unwrap();
        let memory = ctx.state.library.memory_budget();
        // Model another worker reserving in the same account after entry.
        // This clone's local limit may differ; the generated source call must
        // still observe the shared counter against its own original cap.
        let claim = std::thread::spawn(move || {
            memory
                .with_limits(u64::MAX, u64::MAX)
                .reserve_capacity(amount)
                .unwrap()
        })
        .join()
        .unwrap();
        PEER_BYTES.with(|p| *p.borrow_mut() = Some(claim));
    }
    if status == 0
        && op == GlobalHelper::ClosedTailBegin.raw()
        && fixed_keys::observations(false)[0] != 0
        && let Some(value) = PEER_SOURCE.with(|p| p.borrow_mut().take())
    {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        let memory = ctx.state.library.memory_budget();
        let snapshot = std::thread::spawn(move || {
            let memory = memory.with_limits(u64::MAX, u64::MAX);
            let mut claim = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut claim).unwrap()
        })
        .join()
        .unwrap();
        PEER_SNAPSHOT.with(|p| *p.borrow_mut() = Some(snapshot));
    }
    status
}
pub(in crate::native_artifact::program::runtime) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    restored: bool,
    limits: NativeProgramExecutionLimits,
    inject: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 4],
    [u64; 2],
) {
    let image = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = image.as_ref().unwrap_or_else(|| jit.descriptor());
    let entry = jit.test_checked_entry();
    fixed_keys::observations(true);
    HELPERS.with(|v| v.set([0; 2]));
    INJECT_BYTES.with(|v| v.set(inject));
    let result = execute(
        descriptor,
        |frame: *mut Frame| unsafe {
            let original = (*frame).helper;
            ORIGINAL.with(|v| v.set(Some(original)));
            (*frame).helper = observed;
            let status = entry(frame);
            (*frame).helper = original;
            ORIGINAL.with(|v| v.set(None));
            status
        },
        args,
        out,
        limits,
        memory,
    );
    PEER_BYTES.with(|p| drop(p.borrow_mut().take()));
    INJECT_BYTES.with(|v| v.set(false));
    PEER_SNAPSHOT.with(|p| drop(p.borrow_mut().take()));
    (
        result,
        fixed_keys::observations(true),
        HELPERS.with(Cell::get),
    )
}
fn finite() -> NativeProgramExecutionLimits {
    let mut limits = NativeProgramLimitMode::NoLimits.execution();
    limits.library.max_temporary_bytes = Some(64 * 1024 * 1024);
    limits.library.max_retained_nodes = Some(1_000_000);
    limits.library.max_compute_work = Some(1 << 48);
    limits.library.max_comparison_work = Some(1 << 48);
    limits
}
#[test]
fn fixed_key_native_loops_execute_without_per_key_helpers_and_preserve_aliases() {
    for alias in [false, true] {
        for artifact in artifacts(&source(alias)) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            assert_eq!(jit.descriptor().metadata().fixed_key_loops.regions.len(), 1);
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::FixedKeyMapLoops,
            )
            .unwrap();
            let long: Vec<_> = (0..73)
                .map(|i| format!("{i}:{}", "東京\0é".repeat(20)))
                .collect();
            for restored in [false, true] {
                for tags in [
                    vec![],
                    vec!["é", "a\0b", "", "東京", "é"],
                    long.iter().map(String::as_str).collect(),
                ] {
                    let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                    let args = arguments(&artifact, &memory, &tags, &[]);
                    let aliases = args.clone();
                    let before = (memory.used_bytes(), memory.retained_nodes());
                    let mut expected = vec![None; artifact.outputs().len()];
                    let (r, n, h) = run(
                        &ordinary,
                        &args,
                        &mut expected,
                        &memory,
                        restored,
                        finite(),
                        false,
                    );
                    r.unwrap();
                    assert_eq!(n, [0; 4]);
                    let unique = tags
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len() as u64;
                    assert_eq!(h, [unique, unique]);
                    let mut actual = vec![None; artifact.outputs().len()];
                    let (r, n, h) =
                        run(&jit, &args, &mut actual, &memory, restored, finite(), false);
                    r.unwrap_or_else(|e| panic!("{alias}/{restored}/{unique}: {e:?}"));
                    assert_eq!(n, [1, unique, 1, 0]);
                    assert_eq!(h, [0; 2]);
                    assert_eq!(data(&actual), data(&expected));
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    drop((actual, expected));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
                    drop((args, aliases));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
            }
        }
    }
}
#[test]
fn fixed_key_native_loops_preserve_bounds_origin_atomic_output_and_retry() {
    for selector in [
        "0u64",
        "at + 2u64",
        "4294967296u64",
        "18446744073709551615u64",
    ] {
        let text = source(true).replace("keys[at + 1u64]", &format!("keys[{selector}]"));
        for artifact in artifacts(&text) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            assert_eq!(jit.descriptor().metadata().fixed_key_loops.regions.len(), 1);
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::FixedKeyMapLoops,
            )
            .unwrap();
            for restored in [false, true] {
                let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = arguments(&artifact, &memory, &["a", "b"], &[]);
                let before = (memory.used_bytes(), memory.retained_nodes());
                let mut expected = vec![None; artifact.outputs().len()];
                let error = run(
                    &ordinary,
                    &args,
                    &mut expected,
                    &memory,
                    restored,
                    finite(),
                    false,
                )
                .0
                .unwrap_err();
                for _ in 0..2 {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let (result, n, h) =
                        run(&jit, &args, &mut actual, &memory, restored, finite(), false);
                    assert_eq!(format!("{:?}", result.unwrap_err()), format!("{error:?}"));
                    assert_eq!(n[0], 1);
                    assert_eq!(n[3], 1);
                    assert_eq!(h, [0; 2]);
                    assert!(actual.iter().all(Option::is_none));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
                }
                drop(args);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
    }
}
#[test]
fn fixed_key_native_loops_observe_peer_bytes_after_entry_and_recover() {
    for artifact in artifacts(&source(true)) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &["a", "b"], &[]);
            let before = (memory.used_bytes(), memory.retained_nodes());
            let mut out = vec![None; artifact.outputs().len()];
            let (r, n, h) = run(&jit, &args, &mut out, &memory, restored, finite(), true);
            let error = format!("{:?}", r.unwrap_err());
            assert!(
                error.contains("ResourceLimit") && error.contains("Admission"),
                "{error}"
            );
            assert_eq!(n, [1, 0, 1, 1]);
            assert_eq!(h, [0; 2]);
            assert!(out.iter().all(Option::is_none));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            let (r, n, h) = run(&jit, &args, &mut out, &memory, restored, finite(), false);
            r.unwrap();
            assert_eq!(n, [1, 2, 1, 0]);
            assert_eq!(h, [0; 2]);
            drop(out);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn fixed_key_native_loops_keep_finite_refills_and_decline_tight_call_caps() {
    for artifact in artifacts(&source(true)) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::FixedKeyMapLoops,
        )
        .unwrap();
        let tags: Vec<_> = (0..1100).map(|i| format!("key-{i:04}")).collect();
        let tags: Vec<_> = tags.iter().map(String::as_str).collect();
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &tags, &[]);
            let before = (memory.used_bytes(), memory.retained_nodes());
            let mut expected = vec![None; artifact.outputs().len()];
            run(
                &ordinary,
                &args,
                &mut expected,
                &memory,
                restored,
                finite(),
                false,
            )
            .0
            .unwrap();
            let mut actual = vec![None; artifact.outputs().len()];
            let (r, n, h) = run(&jit, &args, &mut actual, &memory, restored, finite(), false);
            r.unwrap();
            assert_eq!(n, [1, 1100, 2, 0]);
            assert_eq!(h, [0; 2]);
            assert_eq!(data(&actual), data(&expected));
            drop((actual, expected));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            for cap in [0, 1, 127, 511] {
                let mut limits = finite();
                limits.library.max_cloned_bytes = Some(cap);
                let mut expected = vec![None; artifact.outputs().len()];
                let e = run(
                    &ordinary,
                    &args,
                    &mut expected,
                    &memory,
                    restored,
                    limits,
                    false,
                )
                .0
                .unwrap_err();
                let mut actual = vec![None; artifact.outputs().len()];
                let (r, n, _) = run(&jit, &args, &mut actual, &memory, restored, limits, false);
                assert_eq!(format!("{:?}", r.unwrap_err()), format!("{e:?}"));
                assert_eq!(n[0], 0);
                assert!(actual.iter().all(Option::is_none));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            }
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn fixed_key_native_loops_observe_peer_snapshot_nodes_after_entry() {
    for artifact in artifacts(&source(true)) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &["a", "b"], &[]);
            let before = (memory.used_bytes(), memory.retained_nodes());
            let port = &artifact.inputs()[0];
            let shape = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(port.schema).unwrap(),
                &[1, 65536],
            )
            .unwrap();
            let peer = ValueDraft {
                schema: port.schema,
                shape_values: shape.parameter_values().into(),
                data: Data::Matrix((0..65536).map(|_| Data::String("p".into())).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            PEER_SOURCE.with(|p| *p.borrow_mut() = Some(peer));
            let mut limits = finite();
            limits.library.max_retained_nodes = Some(65536);
            let mut out = vec![None; artifact.outputs().len()];
            let (r, n, h) = run(&jit, &args, &mut out, &memory, restored, limits, false);
            let error = format!("{:?}", r.unwrap_err());
            assert!(
                error.contains("ResourceLimit") && error.contains("RetainedNodes"),
                "{error}"
            );
            assert_eq!(n, [1, 0, 1, 1]);
            assert_eq!(h, [0; 2]);
            assert!(out.iter().all(Option::is_none));
            assert!(PEER_SOURCE.with(|p| p.borrow().is_none()));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            run(&jit, &args, &mut out, &memory, restored, finite(), false)
                .0
                .unwrap();
            drop(out);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
