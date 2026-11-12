//! End-to-end virtual keys, real snapshot retirement, late fallback and source lifetime.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::{
    GlobalHelper,
    abi::{Frame, Helper},
};
use std::{cell::Cell, ffi::c_void};
const SOURCE: &str = r#"
+> map
+> sequence
renumber(keys<[string]>, at<u64>, end<u64>, index<{string:u64}>) => (out<{string:u64}>) :=
  more := at < end
  next := (more? | true => map/insert(index, keys[at + 1u64], at) | false => index)
  out := (more? | true => renumber(keys, at + 1u64, end, next) | false => index).
build(tags<[string]>, first<u64>, last<u64>) => (out<{string:u64}>) :=
  histogram := map/histogram(tags)
  keys := map/keys(histogram)
  out := renumber(keys, first, last, histogram).
build(tags<[string]>, first<u64>, last<u64>)
"#;
#[derive(Clone, Copy, Default)]
pub(in crate::native_artifact::program::runtime) enum Fault {
    #[default]
    None,
    RefuseOwner,
    StaleVirtual,
    ProducerCloned,
    ConsumerOutput,
    Token,
}
thread_local! {
    static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
    // views, canonical matrices at producer, borrowed cells, retired original handles
    static COUNTS:Cell<[u64;4]>=const{Cell::new([0;4])};
    static FAULT:Cell<Fault>=const{Cell::new(Fault::None)};
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
    let metadata = ctx.descriptor.metadata();
    let proof = &metadata.borrowed_map_keys.regions[0];
    let (_, cell) = metadata.matrix_map_keys.producer(proof.cells[0]).unwrap();
    let refuse = matches!(
        GlobalHelper::decode(op),
        GlobalHelper::MapOwnerBegin | GlobalHelper::MapOwnerBeginBorrowedValues
    ) && matches!(FAULT.with(Cell::get), Fault::RefuseOwner);
    if refuse {
        super::super::maps::owner::set_borrowed_entry_refusal(true);
    }
    if site as usize == cell.consumer
        && op == GlobalHelper::MapOwnerCall.raw()
        && matches!(FAULT.with(Cell::get), Fault::StaleVirtual)
    {
        let ty = metadata.types[proof.output.get() as usize];
        ctx.state
            .arena
            .define_virtual(
                proof.output.get(),
                ty.schema,
                ty.key,
                &mut ctx.state.control.work,
            )
            .unwrap();
    }
    if site as usize == cell.producer
        && op == GlobalHelper::MapMatrixKey.raw()
        && matches!(FAULT.with(Cell::get), Fault::ProducerCloned)
    {
        ctx.limits.max_cloned_bytes = Some(0);
    }
    if site as usize == cell.consumer && op == GlobalHelper::MapOwnerCall.raw() {
        match FAULT.with(Cell::get) {
            Fault::ConsumerOutput => ctx.limits.max_output_bytes = Some(0),
            Fault::Token => unsafe {
                *(a as *mut u64) = (*(a as *mut u64)).wrapping_add(1);
            },
            _ => {}
        }
    }
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) };
    if refuse {
        super::super::maps::owner::set_borrowed_entry_refusal(false);
    }
    if status == 0 {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        COUNTS.with(|v| {
            let mut n = v.get();
            if site as usize == proof.producer {
                if ctx
                    .state
                    .map_owners
                    .borrowed_keys(proof.output.get())
                    .is_some()
                {
                    n[0] += 1;
                }
                if ctx
                    .state
                    .arena
                    .copy_current_prepaid(proof.output.get(), &ctx.state.control.work)
                    .unwrap()
                    .is_some()
                {
                    n[1] += 1;
                }
            }
            if site as usize == cell.producer && op == GlobalHelper::MapMatrixKey.raw() {
                assert!(
                    ctx.state
                        .arena
                        .copy_current_prepaid(cell.output.get(), &ctx.state.control.work)
                        .unwrap()
                        .is_none()
                );
                n[2] += 1;
            }
            for f in &proof.private_frames {
                if f.pack == site as usize && f.retires_snapshot_handle {
                    assert!(
                        ctx.state
                            .arena
                            .copy_current_prepaid(proof.snapshot.get(), &ctx.state.control.work)
                            .unwrap()
                            .is_none()
                    );
                    assert!(
                        ctx.state
                            .map_owners
                            .borrowed_keys(proof.output.get())
                            .is_some()
                    );
                    n[3] += 1;
                }
            }
            v.set(n);
        });
    }
    status
}
pub(in crate::native_artifact::program::runtime) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    restored: bool,
    fault: Fault,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 4],
) {
    let image = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = image.as_ref().unwrap_or_else(|| jit.descriptor());
    FAULT.with(|v| v.set(fault));
    COUNTS.with(|v| v.set([0; 4]));
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| unsafe {
            let old = (*raw).helper;
            ORIGINAL.with(|v| v.set(Some(old)));
            (*raw).helper = observed;
            let result = entry(raw);
            (*raw).helper = old;
            ORIGINAL.with(|v| v.set(None));
            result
        },
        args,
        output,
        NativeProgramLimitMode::NoLimits.execution(),
        memory,
    );
    FAULT.with(|v| v.set(Fault::None));
    (result, COUNTS.with(Cell::get))
}
#[test]
fn borrowed_map_keys_execute_and_materialize_after_later_owner_refusal() {
    for a in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &catalog).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &catalog, OptimizationPass::BorrowedMapKeys)
                .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .borrowed_map_keys
                .regions
                .len(),
            1
        );
        for restored in [false, true] {
            for keys in [vec![], vec!["z", "é", "a\0b", "", "東京", "é"]] {
                let unique = keys
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len() as u64;
                let m = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = super::matrix_map_keys::arguments(&a, &m, &keys, 0, unique);
                let before = (m.used_bytes(), m.retained_snapshot_nodes());
                let mut expected = vec![None; a.outputs().len()];
                ordinary
                    .evaluate_into(
                        &args,
                        &mut expected,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &m,
                    )
                    .unwrap();
                let expected_data: Vec<_> = expected
                    .iter()
                    .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                    .collect();
                drop(expected);
                for fault in [Fault::None, Fault::RefuseOwner] {
                    let mut output = vec![None; a.outputs().len()];
                    let (result, counts) = run(&native, &args, &mut output, &m, restored, fault);
                    result.unwrap_or_else(|e| {
                        panic!(
                            "restored={restored} refused={}: {e:?}",
                            matches!(fault, Fault::RefuseOwner)
                        )
                    });
                    assert_eq!(
                        output
                            .iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>(),
                        expected_data
                    );
                    assert_eq!(counts[0], 1);
                    assert_eq!(counts[1], 0);
                    assert_eq!(counts[3], 1);
                    if matches!(fault, Fault::None) {
                        assert_eq!(counts[2], unique)
                    } else {
                        assert_eq!(counts[2], 0)
                    }
                    drop(output);
                    assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), before);
                }
                if unique > 0 {
                    let mut output = vec![None; a.outputs().len()];
                    let (result, _) = run(
                        &native,
                        &args,
                        &mut output,
                        &m,
                        restored,
                        Fault::StaleVirtual,
                    );
                    assert!(result.is_err());
                    assert!(output.iter().all(Option::is_none));
                    drop(output);
                    assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), before);
                    let mut output = vec![None; a.outputs().len()];
                    run(&native, &args, &mut output, &m, restored, Fault::None)
                        .0
                        .unwrap();
                    drop(output);
                }
                drop(args);
                assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
            }
        }
    }
}
#[test]
fn borrowed_map_keys_later_length_survives_insertion_owner_publication() {
    let declarations = SOURCE.split("\nbuild(").next().unwrap();
    let source = format!(
        "{declarations}\ntags := tags<[string]>\nfirst := sequence/length(tags) % 2u64\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,first,end,histogram)\nlast := sequence/length(keys)\n(index,last)\n"
    );
    for a in artifacts(&source) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &catalog).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &catalog, OptimizationPass::BorrowedMapKeys)
                .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .borrowed_map_keys
                .regions
                .len(),
            1
        );
        assert_eq!(
            native.descriptor().metadata().borrowed_map_keys.regions[0]
                .lengths
                .len(),
            2
        );
        let m = NativeProgramLimitMode::NoLimits.memory_budget();
        let args = super::matrix_map_keys::arguments(&a, &m, &["z", "a", "é", "z"], 0, 3);
        let mut expected = vec![None; a.outputs().len()];
        ordinary
            .evaluate_into(
                &args,
                &mut expected,
                NativeProgramLimitMode::NoLimits.execution(),
                &m,
            )
            .unwrap();
        let data: Vec<_> = expected
            .iter()
            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
            .collect();
        drop(expected);
        for restored in [false, true] {
            let mut out = vec![None; a.outputs().len()];
            let (result, n) = run(&native, &args, &mut out, &m, restored, Fault::None);
            result.unwrap();
            assert_eq!(
                out.iter()
                    .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                    .collect::<Vec<_>>(),
                data
            );
            assert_eq!(n[0], 1);
            assert!(n[2] > 0);
            drop(out);
        }
        drop(args);
        assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
    }
}

#[test]
fn borrowed_map_keys_errors_caps_and_fresh_invocations_preserve_publication_and_aliases() {
    for a in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &catalog).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &catalog, OptimizationPass::BorrowedMapKeys)
                .unwrap();
        for restored in [false, true] {
            let m = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = super::matrix_map_keys::arguments(&a, &m, &["b", "a", "é", "a\0b"], 0, 4);
            let aliases = args.clone();
            let mut out = vec![None; a.outputs().len()];
            run(&native, &args, &mut out, &m, restored, Fault::None)
                .0
                .unwrap();
            for fault in [
                Fault::StaleVirtual,
                Fault::ProducerCloned,
                Fault::ConsumerOutput,
                Fault::Token,
            ] {
                let old = out.clone();
                let before = (m.used_bytes(), m.retained_snapshot_nodes());
                let (r, _) = run(&native, &args, &mut out, &m, restored, fault);
                assert!(r.is_err());
                if matches!(fault, Fault::StaleVirtual) {
                    assert!(format!("{:?}", r.unwrap_err()).contains("StaleHandle"));
                }
                assert!(out.iter().zip(&old).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), before);
                run(&native, &args, &mut out, &m, restored, Fault::None)
                    .0
                    .unwrap();
            }
            for (first, end) in [(1, 4), (3, 3), (2, 7), (u64::MAX, u64::MAX)] {
                let next =
                    super::matrix_map_keys::arguments(&a, &m, &["z", "a", "é", "z"], first, end);
                let before = out.clone();
                let mut expected = before.clone();
                let x = run(&native, &next, &mut out, &m, restored, Fault::None).0;
                let y = ordinary.evaluate_into(
                    &next,
                    &mut expected,
                    NativeProgramLimitMode::NoLimits.execution(),
                    &m,
                );
                assert_eq!(x.is_ok(), y.is_ok(), "borrowed={x:?} ordinary={y:?}");
                if let (Err(x), Err(y)) = (&x, &y) {
                    assert_eq!(format!("{x:?}"), format!("{y:?}"));
                }
                let data = |v: &[Option<Value>]| {
                    v.iter()
                        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                        .collect::<Vec<_>>()
                };
                assert_eq!(data(&out), data(&expected));
                if x.is_err() {
                    assert!(out.iter().zip(&before).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((args, aliases, out));
            assert_eq!((m.used_bytes(), m.retained_snapshot_nodes()), (0, 0));
        }
    }
}
