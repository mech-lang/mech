//! Observe real ownership selection and both ordinary-fallback continuations.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::{
    GlobalHelper,
    abi::{Frame, Helper},
};
use std::{cell::Cell, ffi::c_void};
const DECLARATIONS: &str = r#"
+> map
+> sequence
renumber(keys<[string]>,at<u64>,end<u64>,index<{string:u64}>) => (out<{string:u64}>) :=
  more := at < end
  next := (more? | true => map/insert(index,keys[at + 1u64],at) | false => index)
  out := (more? | true => renumber(keys,at + 1u64,end,next) | false => index).
tags := tags<[string]>
histogram := map/histogram(tags)
keys := map/keys(histogram)
end := sequence/length(keys)
index := renumber(keys,0u64,end,histogram)
"#;
fn source(queries: bool) -> String {
    format!(
        "{DECLARATIONS}{}\nids := [index{{tag}} | tag <- {}]\nids\n",
        if queries {
            "queries := queries<[string]>"
        } else {
            ""
        },
        if queries { "queries" } else { "tags" }
    )
}
#[derive(Clone, Copy, Default, Debug)]
pub(in crate::native_artifact::program::runtime) enum Fault {
    #[default]
    None,
    RefusePrepare,
    RefuseFinish,
    StalePrepare,
    StaleFinish,
    OutputCap,
}
thread_local! {
 static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
 static FAULT:Cell<Fault>=const{Cell::new(Fault::None)};
 // native publications, canonical Values at publication, owned lookup loans, late Values
 static COUNTS:Cell<[u64;4]>=const{Cell::new([0;4])};
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
    let proof = &metadata.generic_dictionary.regions[0];
    let fault = FAULT.with(Cell::get);
    if site as u32 == proof.exit
        && op == GlobalHelper::MapDictionaryEnd.raw()
        && matches!(fault, Fault::OutputCap)
    {
        ctx.limits.max_output_bytes = Some(0)
    }
    let prepare = site as u32 == proof.lookup_header && op == GlobalHelper::Utf8ReadPrepare.raw();
    let finish = site as u32 == proof.lookup_header && op == GlobalHelper::Utf8ReadFinish.raw();
    if (prepare && matches!(fault, Fault::StalePrepare))
        || (finish && matches!(fault, Fault::StaleFinish))
    {
        let ty = metadata.types[proof.result.get() as usize];
        ctx.state
            .arena
            .define_virtual(
                proof.result.get(),
                ty.schema,
                ty.key,
                &mut ctx.state.control.work,
            )
            .unwrap();
    }
    if prepare && matches!(fault, Fault::RefusePrepare) {
        super::super::collections::utf8_read::force_dictionary_refusal(true)
    }
    let b = if finish && matches!(fault, Fault::RefuseFinish) {
        1
    } else {
        b
    };
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) };
    if prepare {
        super::super::collections::utf8_read::force_dictionary_refusal(false)
    }
    if status == 0 {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        COUNTS.with(|v| {
            let mut n = v.get();
            if site as u32 == proof.exit && matches!(op, value if value == GlobalHelper::MapDictionaryEnd.raw() || value == GlobalHelper::MapOwnerEnd.raw()) {
                if ctx.state.map_owners.has_dictionary(proof.result.get()) {
                    n[0] += 1
                }
                if ctx
                    .state
                    .arena
                    .copy_current_prepaid(proof.result.get(), &ctx.state.control.work)
                    .unwrap()
                    .is_some()
                {
                    n[1] += 1
                }
            }
            if prepare
                && ctx
                    .state
                    .collections
                    .as_ref()
                    .is_some_and(|s| s.has_frozen_dictionary_loan())
            {
                n[2] += 1
            }
            if (prepare || finish)
                && ctx
                    .state
                    .arena
                    .copy_current_prepaid(proof.result.get(), &ctx.state.control.work)
                    .unwrap()
                    .is_some()
            {
                n[3] += 1
            }
            v.set(n);
        });
    }
    status
}
fn arguments(
    a: &mech_engine::ProgramArtifact,
    m: &ManagedMemoryBudget,
    tags: &[&str],
    queries: &[&str],
) -> Vec<Value> {
    a.inputs()
        .iter()
        .map(|p| {
            let name = mech_engine::decode_source_input_name(&p.name).unwrap();
            let keys = match name.as_str() {
                "tags" => tags,
                "queries" => queries,
                _ => panic!("input {name}"),
            };
            let shape = mech_core::shape_for_resolved_extents(
                a.schemas().get(p.schema).unwrap(),
                &[1, keys.len() as u64],
            )
            .unwrap();
            let v = ValueDraft {
                schema: p.schema,
                shape_values: shape.parameter_values().into(),
                data: Data::Matrix(keys.iter().map(|s| Data::String((*s).into())).collect()),
            }
            .finalize(&SnapshotValidationContext::new(a.schemas()))
            .unwrap();
            let mut r = m
                .reserve_capacity(v.memory_budget_admission_bytes(m).unwrap())
                .unwrap();
            v.into_memory_budget(&mut r).unwrap()
        })
        .collect()
}
pub(in crate::native_artifact::program::runtime) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    m: &ManagedMemoryBudget,
    restored: bool,
    fault: Fault,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    [u64; 4],
) {
    let image = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = image.as_ref().unwrap_or_else(|| jit.descriptor());
    let entry = jit.test_checked_entry();
    FAULT.with(|v| v.set(fault));
    COUNTS.with(|v| v.set([0; 4]));
    let r = execute(
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
    FAULT.with(|v| v.set(Fault::None));
    (r, COUNTS.with(Cell::get))
}
fn data(v: &[Option<Value>]) -> Vec<Option<Data>> {
    v.iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect()
}
#[test]
fn generic_dictionary_runs_native_and_returns_owner_on_prepare_and_loop_refusal() {
    for a in artifacts(&source(false)) {
        let cat = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &cat).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &cat, OptimizationPass::GenericDictionary)
                .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .generic_dictionary
                .regions
                .len(),
            1
        );
        let long_keys: Vec<String> = (0..73)
            .map(|n| format!("{n}:{}", "東京\0é".repeat(20)))
            .collect();
        for restored in [false, true] {
            for keys in [
                vec![],
                vec!["z", "é", "a\0b", "", "東京", "é"],
                long_keys.iter().map(String::as_str).collect(),
            ] {
                let m = NativeProgramLimitMode::NoLimits.memory_budget();
                let args = arguments(&a, &m, &keys, &[]);
                let before = (m.used_bytes(), m.retained_nodes());
                let mut expected_values = vec![None; a.outputs().len()];
                ordinary
                    .evaluate_into(
                        &args,
                        &mut expected_values,
                        NativeProgramLimitMode::NoLimits.execution(),
                        &m,
                    )
                    .unwrap();
                let expected = data(&expected_values);
                drop(expected_values);
                for fault in [Fault::None, Fault::RefusePrepare, Fault::RefuseFinish] {
                    let mut out = vec![None; a.outputs().len()];
                    let (r, n) = run(&native, &args, &mut out, &m, restored, fault);
                    r.unwrap_or_else(|e| panic!("{fault:?}/{restored}: {e:?}"));
                    assert_eq!(data(&out), expected);
                    assert_eq!(n[0], 1);
                    assert_eq!(n[1], 0);
                    if matches!(fault, Fault::RefusePrepare) {
                        assert_eq!(n[2], 0);
                        assert!(n[3] >= 1)
                    } else {
                        assert_eq!(n[2], 1);
                        if matches!(fault, Fault::None) {
                            assert_eq!(n[3], 0)
                        } else {
                            assert!(n[3] >= 1)
                        }
                    }
                    drop(out);
                    assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                }
                drop(args);
                assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
            }
        }
    }
}
#[test]
fn generic_dictionary_preserves_missing_key_errors_stale_generation_outputs_and_retry() {
    for a in artifacts(&source(true)) {
        let cat = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &cat).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &cat, OptimizationPass::GenericDictionary)
                .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .generic_dictionary
                .regions
                .len(),
            1
        );
        for restored in [false, true] {
            let m = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&a, &m, &["b", "a", "é"], &["é", "b"]);
            let aliases = args.clone();
            let mut out = vec![None; a.outputs().len()];
            run(&native, &args, &mut out, &m, restored, Fault::None)
                .0
                .unwrap();
            for fault in [Fault::StalePrepare, Fault::StaleFinish, Fault::OutputCap] {
                let old = out.clone();
                let before = (m.used_bytes(), m.retained_nodes());
                let (r, _) = run(&native, &args, &mut out, &m, restored, fault);
                assert!(r.is_err());
                assert_eq!(data(&out), data(&old));
                assert!(out.iter().zip(&old).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                run(&native, &args, &mut out, &m, restored, Fault::None)
                    .0
                    .unwrap();
            }
            let missing = arguments(&a, &m, &["b", "a", "é"], &["b", "missing", "é"]);
            let old = out.clone();
            let mut expected = old.clone();
            let before = (m.used_bytes(), m.retained_nodes());
            let (x, n) = run(&native, &missing, &mut out, &m, restored, Fault::None);
            let y = ordinary.evaluate_into(
                &missing,
                &mut expected,
                NativeProgramLimitMode::NoLimits.execution(),
                &m,
            );
            assert!(x.is_err() && y.is_err());
            assert_eq!(
                format!("{:?}", x.unwrap_err()),
                format!("{:?}", y.unwrap_err())
            );
            assert_eq!(data(&out), data(&old));
            assert!(n[3] >= 1);
            assert_eq!((m.used_bytes(), m.retained_nodes()), before);
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((old, expected, missing, aliases, args, out));
            assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
        }
    }
}
#[test]
fn generic_dictionary_finite_boundaries_keep_publication_cleanup_and_recovery() {
    resources::finite_boundaries(&source(false), |a, m| {
        arguments(a, m, &["b", "a", "é", "a\0b"], &[])
    });
}
#[test]
fn generic_dictionary_images_rederive_exact_native_sources_and_public_escapes_stay_canonical() {
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    for a in artifacts(&source(false)) {
        let cat = mech_stdlib::source_catalog();
        let object = emit_program_object(&a, &cat).unwrap();
        let expected = &object.descriptor().metadata().generic_dictionary;
        assert_eq!(expected.regions.len(), 1);
        let image = object.native_metadata();
        let independent =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(image, &cat) }.unwrap();
        assert_eq!(&independent.metadata().generic_dictionary, expected);
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                image,
                &cat,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        assert_eq!(&bound.metadata().generic_dictionary, expected);
    }
    let public = source(false).strip_suffix("ids\n").unwrap().to_owned() + "(ids,index)\n";
    for a in artifacts(&public) {
        let native = JitProgramArtifact::compile(&a, &mech_stdlib::source_catalog()).unwrap();
        assert!(
            native
                .descriptor()
                .metadata()
                .generic_dictionary
                .regions
                .is_empty()
        );
    }
}

fn guarded_source(inverted: bool, failing_bypass: bool) -> String {
    let lookup = "[index{tag} | tag <- tags]";
    let bypass = if failing_bypass {
        "[1u64 / divisor<u64> | tag <- tags]"
    } else {
        "[0u64 | tag <- tags]"
    };
    let (yes, no) = if inverted {
        (bypass, lookup)
    } else {
        (lookup, bypass)
    };
    source(false).replace("ids := [index{tag} | tag <- tags]\nids\n",&format!(
        "allowed := (end >= 3u64) && (end <= 64u64)\nids := (allowed? | true => {yes} | false => {no})\nids\n"
    ))
}
#[test]
fn guarded_dictionary_native_prediction_keeps_original_exit_publication_both_paths_and_retry() {
    for inverted in [false, true] {
        for a in artifacts(&guarded_source(inverted, false)) {
            let cat = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&a, &cat).unwrap();
            let ordinary =
                JitProgramArtifact::compile_for_test(&a, &cat, OptimizationPass::GenericDictionary)
                    .unwrap();
            let regions = &native.descriptor().metadata().generic_dictionary.regions;
            assert_eq!(regions.len(), 1);
            assert!(regions[0].guard.is_some());
            for restored in [false, true] {
                let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                for keys in [vec!["z", "a", "é", "z"], vec!["z", "z"], vec![""]] {
                    let selected = (keys
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        >= 3)
                        != inverted;
                    let args = arguments(&a, &memory, &keys, &[]);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut expected_values = vec![None; a.outputs().len()];
                    ordinary
                        .evaluate_into(
                            &args,
                            &mut expected_values,
                            NativeProgramLimitMode::NoLimits.execution(),
                            &memory,
                        )
                        .unwrap();
                    let expected = data(&expected_values);
                    drop(expected_values);
                    for fault in [Fault::None, Fault::RefusePrepare, Fault::RefuseFinish] {
                        let mut output = vec![None; a.outputs().len()];
                        let (result, counts) =
                            run(&native, &args, &mut output, &memory, restored, fault);
                        result.unwrap_or_else(|e| {
                            panic!("{inverted}/{selected}/{restored}/{fault:?}: {e:?}")
                        });
                        assert_eq!(data(&output), expected);
                        assert_eq!(
                            counts[0],
                            u64::from(selected),
                            "a bypass installs no virtual owner"
                        );
                        assert_eq!(
                            counts[1],
                            u64::from(!selected),
                            "canonical bypass publication occurs at original exit"
                        );
                        assert_eq!(
                            counts[2],
                            u64::from(selected && !matches!(fault, Fault::RefusePrepare))
                        );
                        if !selected {
                            assert_eq!(counts[3], 0);
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        drop(output);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    }
                    drop((args, aliases));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
#[test]
fn guarded_dictionary_bypass_checked_error_stays_after_original_canonical_publication() {
    for a in artifacts(&guarded_source(false, true)) {
        let cat = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&a, &cat).unwrap();
        let ordinary =
            JitProgramArtifact::compile_for_test(&a, &cat, OptimizationPass::GenericDictionary)
                .unwrap();
        assert!(
            native.descriptor().metadata().generic_dictionary.regions[0]
                .guard
                .is_some()
        );
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = inputs(
                &a,
                &[
                    ("tags", Data::Matrix(vec![Data::String("z".into())].into())),
                    ("divisor", Data::U64(0)),
                ],
                &memory,
            );
            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut out = vec![None; a.outputs().len()];
            let mut expected = out.clone();
            let (result, counts) = run(&native, &args, &mut out, &memory, restored, Fault::None);
            let expected_error = ordinary
                .evaluate_into(
                    &args,
                    &mut expected,
                    NativeProgramLimitMode::NoLimits.execution(),
                    &memory,
                )
                .unwrap_err();
            assert_eq!(
                format!("{:?}", result.unwrap_err()),
                format!("{expected_error:?}")
            );
            assert_eq!(counts, [0, 1, 0, 0]);
            assert_eq!(data(&out), data(&expected));
            assert!(out.iter().all(Option::is_none));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                held
            );
        }
    }
}
#[test]
fn guarded_dictionary_finite_boundaries_cover_lookup_and_canonical_bypass() {
    let source = guarded_source(false, false);
    for keys in [vec!["b", "a", "é", "b"], vec!["a", "a"]] {
        resources::finite_boundaries(&source, |artifact, memory| {
            arguments(artifact, memory, &keys, &[])
        });
    }
}
#[test]
fn guarded_dictionary_images_rederive_prediction_and_reject_underfunded_exit() {
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    use serde::Deserialize;
    for artifact in artifacts(&guarded_source(false, false)) {
        let catalog = mech_stdlib::source_catalog();
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let descriptor = object.descriptor();
        let expected = &descriptor.metadata().generic_dictionary;
        let proof = &expected.regions[0];
        assert!(proof.guard.is_some());
        let image = object.native_metadata();
        let independent =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(image, &catalog) }.unwrap();
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                image,
                &catalog,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        assert_eq!(&independent.metadata().generic_dictionary, expected);
        assert_eq!(&bound.metadata().generic_dictionary, expected);
        let site = proof.exit as usize;
        let base = crate::native_artifact::iteration_abi::control_visits(
            descriptor.plan().blocks(),
            descriptor.plan().iterations().iter().map(Into::into),
            0,
        )
        .unwrap();
        let required = base[site].work
            + crate::native_artifact::program::prepare::map_owner::extra_work(
                descriptor.metadata().map_owner_words.as_ref().unwrap(),
                site,
            )
            .unwrap()
            + expected.extra_work(site);
        assert!(descriptor.metadata().visits[site].work >= required);
        let mut reader =
            mech_engine::native_artifact::BuildOwnedImageReader::from_image(image).unwrap();
        let mut wire = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        let prices = wire["descriptor"]["visit_work"].as_array_mut().unwrap();
        for (slot, byte) in prices[site * 8..site * 8 + 8]
            .iter_mut()
            .zip((required - 1).to_le_bytes())
        {
            *slot = byte.into();
        }
        let forged = mech_engine::native_artifact::encode_native_metadata_image(
            &serde_json::to_vec(&wire).unwrap(),
        )
        .unwrap();
        let reason =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&forged, &catalog) }
                .err()
                .unwrap();
        assert!(
            reason
                .to_string()
                .contains("guarded dictionary publication template is not funded"),
            "{reason}"
        );
    }
}
