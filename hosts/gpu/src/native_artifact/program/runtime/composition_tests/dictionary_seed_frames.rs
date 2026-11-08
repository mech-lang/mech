//! Real selected seed ownership, native import, and original-factory refusal.
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
ids := [index{tag} | tag <- tags]
"#;
fn source(alias: bool) -> String {
    format!(
        "{DECLARATIONS}{}\n",
        if alias { "(histogram,ids)" } else { "ids" }
    )
}
#[derive(Clone, Copy, Default, Debug)]
pub(in crate::native_artifact::program::runtime) enum Fault {
    #[default]
    None,
    Nested,
    RefuseOwner,
    StaleSeed,
    ManagedBits,
    CloneCap,
}
thread_local! {
    static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
    static FAULT:Cell<Fault>=const{Cell::new(Fault::None)};
    // owned seeds, canonical seeds at producer, private imports, late fallback seeds
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
    let proof = ctx.descriptor.metadata().dictionary_seed_frames.regions[0].clone();
    let producer = site == proof.producer as u64 && op == GlobalHelper::MapSeedFrame.raw();
    let initialize = site == proof.initialize as u64 && op == GlobalHelper::MapOwnerBegin.raw();
    let before = initialize && ctx.state.map_owners.has_seed(proof.seed.get());
    let fault = FAULT.with(Cell::get);
    if initialize && matches!(fault, Fault::StaleSeed) {
        let ty = ctx.descriptor.metadata().types[proof.seed.get() as usize];
        ctx.state
            .arena
            .define_virtual(
                proof.seed.get(),
                ty.schema,
                ty.key,
                &mut ctx.state.control.work,
            )
            .unwrap();
    }
    if producer && matches!(fault, Fault::ManagedBits) {
        unsafe {
            *(a as *mut u64).add(proof.map_ordinal) = 1;
        }
    }
    if producer && matches!(fault, Fault::CloneCap) {
        ctx.limits.max_cloned_bytes = Some(0);
    }
    if initialize && matches!(fault, Fault::RefuseOwner) {
        super::super::maps::owner::set_borrowed_entry_refusal(true);
    }
    let work = ctx.state.library.clone();
    let status = if producer && matches!(fault, Fault::Nested) {
        work.with_call_limits(u64::MAX, u64::MAX, || unsafe {
            ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out)
        })
        .unwrap()
    } else {
        unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) }
    };
    if initialize {
        super::super::maps::owner::set_borrowed_entry_refusal(false);
    }
    if status == 0 {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        let canonical = (producer || initialize)
            && ctx
                .state
                .arena
                .copy_current_prepaid(proof.seed.get(), &ctx.state.control.work)
                .unwrap()
                .is_some();
        COUNTS.with(|v| {
            let mut n = v.get();
            if producer {
                n[0] += u64::from(ctx.state.map_owners.has_seed(proof.seed.get()));
                n[1] += u64::from(canonical);
            }
            if initialize && before {
                n[2] += u64::from(ctx.state.map_owners.has_retained_binding(proof.owner));
                n[3] += u64::from(canonical);
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
    FAULT.with(|v| v.set(Fault::None));
    (result, COUNTS.with(Cell::get))
}
fn data(v: &[Option<Value>]) -> Vec<Option<Data>> {
    v.iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect()
}
#[test]
fn native_dictionary_seed_runs_without_aggregate_and_materializes_on_real_owner_refusal() {
    for alias in [false, true] {
        for a in artifacts(&source(alias)) {
            let cat = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&a, &cat).unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &a,
                &cat,
                OptimizationPass::DictionarySeedFrames,
            )
            .unwrap();
            assert_eq!(
                jit.descriptor()
                    .metadata()
                    .dictionary_seed_frames
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
                    let args = arguments(&a, &m, &keys, &[]);
                    let before = (m.used_bytes(), m.retained_nodes());
                    let mut expected = vec![None; a.outputs().len()];
                    ordinary
                        .evaluate_into(
                            &args,
                            &mut expected,
                            NativeProgramLimitMode::NoLimits.execution(),
                            &m,
                        )
                        .unwrap();
                    let expected_data = data(&expected);
                    drop(expected);
                    for fault in [Fault::None, Fault::Nested, Fault::RefuseOwner] {
                        let mut out = vec![None; a.outputs().len()];
                        let (r, n) = run(&jit, &args, &mut out, &m, restored, fault);
                        r.unwrap_or_else(|e| panic!("{fault:?}/{restored}: {e:?}"));
                        assert_eq!(data(&out), expected_data);
                        match fault {
                            Fault::None => assert_eq!(n, [1, 0, 1, 0]),
                            Fault::Nested => assert_eq!(n, [0, 1, 0, 0]),
                            Fault::RefuseOwner => assert_eq!(n, [1, 0, 0, 1]),
                            _ => unreachable!(),
                        };
                        drop(out);
                        assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                    }
                    drop(args);
                    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
                }
            }
        }
    }
}
#[test]
fn native_dictionary_seed_stale_generation_malformed_words_and_clone_denial_are_atomic() {
    for a in artifacts(&source(true)) {
        let jit = JitProgramArtifact::compile(&a, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let m = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&a, &m, &["b", "a", "é"], &[]);
            let aliases = args.clone();
            let mut out = vec![None; a.outputs().len()];
            run(&jit, &args, &mut out, &m, restored, Fault::None)
                .0
                .unwrap();
            for fault in [Fault::StaleSeed, Fault::ManagedBits, Fault::CloneCap] {
                let old = out.clone();
                let before = (m.used_bytes(), m.retained_nodes());
                let (r, _) = run(&jit, &args, &mut out, &m, restored, fault);
                assert!(r.is_err(), "{fault:?}");
                assert_eq!(data(&out), data(&old));
                assert!(out.iter().zip(&old).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!((m.used_bytes(), m.retained_nodes()), before);
                run(&jit, &args, &mut out, &m, restored, Fault::None)
                    .0
                    .unwrap();
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((out, args, aliases));
            assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
        }
    }
}
#[test]
fn native_dictionary_seed_finite_boundaries_preserve_source_checks_publication_and_retry() {
    resources::finite_boundaries(&source(false), |a, m| {
        arguments(a, m, &["b", "é", "a\0b", ""], &[])
    });
}
#[test]
fn native_dictionary_seed_images_rederive_exact_adjacent_factory_and_parent_ownership() {
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    for a in artifacts(&source(false)) {
        let c = mech_stdlib::source_catalog();
        let object = emit_program_object(&a, &c).unwrap();
        let expected = &object.descriptor().metadata().dictionary_seed_frames;
        assert_eq!(expected.regions.len(), 1);
        let image = object.native_metadata();
        let independent =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(image, &c) }.unwrap();
        assert_eq!(&independent.metadata().dictionary_seed_frames, expected);
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                image,
                &c,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        assert_eq!(&bound.metadata().dictionary_seed_frames, expected);
    }
}
