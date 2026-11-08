//! Native fixed key-set storage, independent seed values, and canonical fallback.
use super::dictionary_seed_frames::{arguments, data, source};
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::{
    GlobalHelper,
    abi::{Frame, Helper},
};
use std::{cell::Cell, ffi::c_void};
thread_local! {
    static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
    static COUNTS:Cell<[u64;3]>=const{Cell::new([0;3])};
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
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) };
    if status == 0
        && matches!(
            GlobalHelper::decode(op),
            GlobalHelper::MapOwnerBegin | GlobalHelper::MapOwnerBeginBorrowedValues
        )
    {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        if let Some(proof) = ctx
            .descriptor
            .metadata()
            .borrowed_dictionary_values
            .initializer(site as usize)
        {
            let borrowed = ctx.state.map_owners.has_borrowed_dictionary(proof.owner);
            COUNTS.with(|v| {
                let mut n = v.get();
                n[0] += u64::from(borrowed);
                n[1] +=
                    u64::from(!borrowed && ctx.state.map_owners.has_retained_binding(proof.owner));
                n[2] += u64::from(op == GlobalHelper::MapOwnerBeginBorrowedValues.raw());
                v.set(n);
            });
        }
    }
    status
}
pub(in crate::native_artifact::program::runtime) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    m: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 3],
) {
    let image = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = image.as_ref().unwrap_or_else(|| jit.descriptor());
    let entry = jit.test_checked_entry();
    COUNTS.with(|v| v.set([0; 3]));
    let result = execute(
        descriptor,
        |raw: *mut Frame| unsafe {
            let old = (*raw).helper;
            ORIGINAL.with(|v| v.set(Some(old)));
            (*raw).helper = observed;
            let r = entry(raw);
            (*raw).helper = old;
            ORIGINAL.with(|v| v.set(None));
            r
        },
        args,
        out,
        NativeProgramLimitMode::NoLimits.execution(),
        m,
    );
    (result, COUNTS.with(Cell::get))
}
#[test]
fn borrowed_dictionary_values_execute_original_restored_empty_unicode_long_and_aliases() {
    for alias in [false, true] {
        for artifact in artifacts(&source(alias)) {
            let cat = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &cat).unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &cat,
                OptimizationPass::BorrowedDictionaryValues,
            )
            .unwrap();
            assert_eq!(
                jit.descriptor()
                    .metadata()
                    .borrowed_dictionary_values
                    .regions
                    .len(),
                1
            );
            let long: Vec<_> = (0..73)
                .map(|i| format!("{i}:{}", "東京\0é".repeat(20)))
                .collect();
            for restored in [false, true] {
                for keys in [
                    vec![],
                    vec!["é", "a\0b", "", "東京", "é"],
                    long.iter().map(String::as_str).collect(),
                ] {
                    let m = NativeProgramLimitMode::NoLimits.memory_budget();
                    let args = arguments(&artifact, &m, &keys, &[]);
                    let aliases = args.clone();
                    let before = (m.used_bytes(), m.retained_nodes());
                    let mut expected = vec![None; artifact.outputs().len()];
                    let (r, n) = run(&ordinary, &args, &mut expected, &m, restored);
                    r.unwrap();
                    assert_eq!(n, [0, 1, 0]);
                    let mut actual = vec![None; artifact.outputs().len()];
                    let (r, n) = run(&jit, &args, &mut actual, &m, restored);
                    r.unwrap_or_else(|e| panic!("{alias}/{restored}/{}: {e:?}", keys.len()));
                    assert_eq!(n, [1, 0, 1]);
                    assert_eq!(data(&actual), data(&expected));
                    assert!(
                        args.iter()
                            .zip(&aliases)
                            .all(|(a, b)| a.shares_frozen_storage(b))
                    );
                    drop((actual, expected));
                    assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                    drop((args, aliases));
                    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
                }
            }
        }
    }
}
#[test]
fn borrowed_dictionary_values_images_rederive_and_public_dictionary_escapes_decline() {
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    for artifact in artifacts(&source(false)) {
        let cat = mech_stdlib::source_catalog();
        let object = emit_program_object(&artifact, &cat).unwrap();
        let expected = &object.descriptor().metadata().borrowed_dictionary_values;
        assert_eq!(expected.regions.len(), 1);
        let independent = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(object.native_metadata(), &cat)
        }
        .unwrap();
        assert_eq!(&independent.metadata().borrowed_dictionary_values, expected);
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                object.native_metadata(),
                &cat,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        assert_eq!(&bound.metadata().borrowed_dictionary_values, expected);
    }
    let text = source(false).trim_end_matches("ids\n").to_string() + "(ids,index)\n";
    for artifact in artifacts(&text) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .borrowed_dictionary_values
                .regions
                .is_empty()
        );
    }
}
#[test]
fn borrowed_dictionary_values_preserve_distinct_seed_values_and_decline_different_key_sets() {
    for (key, borrowed) in [("b", true), ("z", false)] {
        let text=source(true).replace("index := renumber(keys,0u64,end,histogram)",
            &format!("seed := map/insert(histogram,\"{key}\",900u64)\nindex := renumber(keys,0u64,1u64,seed)"));
        for artifact in artifacts(&text) {
            let cat = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &cat).unwrap();
            assert_eq!(
                jit.descriptor()
                    .metadata()
                    .borrowed_dictionary_values
                    .regions
                    .len(),
                1
            );
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &cat,
                OptimizationPass::BorrowedDictionaryValues,
            )
            .unwrap();
            for restored in [false, true] {
                let m = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = arguments(&artifact, &m, &["b", "a", "b"], &[]);
                let baseline = (m.used_bytes(), m.retained_nodes());
                let mut expected = vec![None; artifact.outputs().len()];
                ordinary
                    .evaluate_into(
                        &args,
                        &mut expected,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &m,
                    )
                    .unwrap();
                let mut actual = vec![None; artifact.outputs().len()];
                let (r, n) = run(&jit, &args, &mut actual, &m, restored);
                r.unwrap();
                assert_eq!(n, [u64::from(borrowed), u64::from(!borrowed), 1]);
                assert_eq!(data(&actual), data(&expected));
                drop((actual, expected));
                assert_eq!((m.used_bytes(), m.retained_nodes()), baseline);
                drop(args);
                assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
            }
        }
    }
}
#[test]
fn borrowed_dictionary_values_late_canonical_fallback_keeps_utf8_payload_and_cleanup() {
    use super::generic_dictionary::{Fault, run as dictionary_run};
    for artifact in artifacts(&source(true)) {
        let cat = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &cat).unwrap();
        assert_eq!(
            jit.descriptor()
                .metadata()
                .borrowed_dictionary_values
                .regions
                .len(),
            1
        );
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &cat,
            OptimizationPass::BorrowedDictionaryValues,
        )
        .unwrap();
        for restored in [false, true] {
            for tags in [vec![], vec!["東京\0é", "", "long-key", "東京\0é"]] {
                let m = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = arguments(&artifact, &m, &tags, &[]);
                let before = (m.used_bytes(), m.retained_nodes());
                let mut expected = vec![None; artifact.outputs().len()];
                ordinary
                    .evaluate_into(
                        &args,
                        &mut expected,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &m,
                    )
                    .unwrap();
                for fault in [Fault::RefusePrepare, Fault::RefuseFinish] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let (r, n) = dictionary_run(&jit, &args, &mut actual, &m, restored, fault);
                    r.unwrap_or_else(|e| panic!("{fault:?}/{restored}: {e:?}"));
                    assert_eq!(n[0], 1);
                    assert!(n[3] >= 1);
                    assert_eq!(data(&actual), data(&expected));
                    drop(actual);
                }
                drop(expected);
                assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                drop(args);
                assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
            }
        }
    }
}
