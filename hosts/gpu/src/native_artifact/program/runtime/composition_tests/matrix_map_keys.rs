//! Original/decoded and restored execution of the selected borrowed-key bridge.
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

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact::program::runtime) enum Fault {
    #[default]
    None,
    SourceStale,
    Token,
    ProducerCloned,
    ConsumerOutput,
    Headroom,
}
thread_local! {
    static ORIGINAL: Cell<Option<Helper>> = const { Cell::new(None) };
    // producer callbacks, borrowed cells, canonical cells, insertion callbacks
    static COUNTS: Cell<(u64,u64,u64,u64)> = const { Cell::new((0,0,0,0)) };
    static FAULT: Cell<Fault> = const { Cell::new(Fault::None) };
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
    let proof = ctx.descriptor.metadata().matrix_map_keys.regions[0].clone();
    if site as usize == proof.producer
        && op == GlobalHelper::MapMatrixKey.raw()
        && matches!(FAULT.with(Cell::get), Fault::ProducerCloned)
    {
        ctx.limits.max_cloned_bytes = Some(0);
    }
    if site as usize == proof.consumer && op == GlobalHelper::MapOwnerCall.raw() {
        match FAULT.with(Cell::get) {
            Fault::SourceStale => {
                let source = ctx
                    .state
                    .arena
                    .copy_current(proof.source.get(), &mut ctx.state.control.work)
                    .unwrap();
                ctx.state
                    .arena
                    .put(proof.source.get(), &source, &mut ctx.state.control.work)
                    .unwrap();
            }
            Fault::Token => unsafe {
                *(a as *mut u64) = (*(a as *mut u64)).wrapping_add(1);
            },
            Fault::ConsumerOutput => ctx.limits.max_output_bytes = Some(0),
            _ => {}
        }
    }
    if site as usize == proof.producer
        && op == GlobalHelper::MapMatrixKey.raw()
        && matches!(FAULT.with(Cell::get), Fault::Headroom)
    {
        ctx.limits.max_temporary_bytes =
            Some(ctx.state.library.memory_budget().used_bytes() + 24576);
    }
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(raw, site, op, a, b, c, out) };
    if status == 0 {
        let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
        if matches!(FAULT.with(Cell::get), Fault::Headroom) {
            ctx.limits.max_temporary_bytes = Some(u64::MAX);
        }
        COUNTS.with(|counts| {
            let (mut producers, mut borrowed, mut canonical, mut consumers) = counts.get();
            if site as usize == proof.producer
                && (op == GlobalHelper::MapMatrixKey.raw() || op == GlobalHelper::Library.raw())
            {
                producers += 1;
                if ctx
                    .state
                    .arena
                    .copy_current_prepaid(proof.output.get(), &ctx.state.control.work)
                    .unwrap()
                    .is_none()
                {
                    borrowed += 1;
                } else {
                    canonical += 1;
                }
            }
            if site as usize == proof.consumer && op == GlobalHelper::MapOwnerCall.raw() {
                consumers += 1;
            }
            counts.set((producers, borrowed, canonical, consumers));
        });
        if op == GlobalHelper::MapMatrixKey.raw() {
            let spent = (ctx.state.control.work.used(), ctx.state.library.used());
            assert!(
                ctx.helper(site, op, [a, b, c]).is_err(),
                "producer replay must be refused"
            );
            assert_eq!(
                spent,
                (ctx.state.control.work.used(), ctx.state.library.used())
            );
        }
    }
    status
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    keys: &[&str],
    first: u64,
    last: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, shape_values) = match name.as_str() {
                "tags" => {
                    let schema = artifact.schemas().get(input.schema).unwrap();
                    let shape =
                        mech_core::shape_for_resolved_extents(schema, &[1, keys.len() as u64])
                            .unwrap();
                    (
                        Data::Matrix(keys.iter().map(|key| Data::String((*key).into())).collect()),
                        shape.parameter_values().into(),
                    )
                }
                "first" => (Data::U64(first), Box::new([]) as Box<[u64]>),
                "last" => (Data::U64(last), Box::new([]) as Box<[u64]>),
                _ => panic!("unexpected input {name}"),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
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
    (u64, u64, u64, u64),
) {
    let image = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = image.as_ref().unwrap_or_else(|| jit.descriptor());
    FAULT.with(|f| f.set(fault));
    COUNTS.with(|c| c.set((0, 0, 0, 0)));
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| unsafe {
            let old = (*raw).helper;
            ORIGINAL.with(|v| v.set(Some(old)));
            (*raw).helper = observed;
            let status = entry(raw);
            (*raw).helper = old;
            ORIGINAL.with(|v| v.set(None));
            status
        },
        args,
        output,
        NativeProgramLimitMode::NoLimits.execution(),
        memory,
    );
    FAULT.with(|f| f.set(Fault::None));
    (result, COUNTS.with(Cell::get))
}
fn drafts(output: &[Option<Value>]) -> Vec<Option<Data>> {
    output
        .iter()
        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
        .collect()
}
#[test]
fn matrix_map_keys_execute_without_string_values_for_unicode_partial_and_fresh_insertions() {
    for source in [
        SOURCE.to_string(),
        SOURCE.replace(
            "renumber(keys, first, last, histogram)",
            "renumber(tags, first, last, histogram)",
        ),
        SOURCE
            .replace(
                "  histogram :=",
                "  empty<{string:u64}> := map/new()\n  histogram :=",
            )
            .replace(
                "renumber(keys, first, last, histogram)",
                "renumber(tags, first, last, empty)",
            ),
    ] {
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::BorrowedMatrixMapKeys,
            )
            .unwrap();
            assert_eq!(
                native.descriptor().metadata().matrix_map_keys.regions.len(),
                1
            );
            for restored in [false, true] {
                let long = "🦀東京é".repeat(40);
                let many: Vec<String> = (0..65).map(|i| format!("{i:03}--{long}")).collect();
                let mut cases = vec![
                    (vec![], 0, 0),
                    (vec!["z", "é", "a\0b", "", "東京", "é"], 0, 5),
                    (vec!["z", "é", "a\0b", "", "東京", "é"], 1, 4),
                    (vec!["z", "é", "a\0b", "", "東京", "é"], 9, 3),
                    (vec!["z", "é", "a\0b", "", "東京", "é"], 0, 7),
                ];
                cases.push((many.iter().map(String::as_str).collect(), 0, 65));
                for (keys, first, end) in cases {
                    let memory = NativeProgramLimitMode::NoLimits.memory_budget();
                    let args = arguments(&artifact, &memory, &keys, first, end);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut a = vec![None; artifact.outputs().len()];
                    let mut b = a.clone();
                    let (x, counts) = run(&native, &args, &mut a, &memory, restored, Fault::None);
                    let (y, plain) = run(&ordinary, &args, &mut b, &memory, restored, Fault::None);
                    assert_eq!(x.is_ok(), y.is_ok(), "borrowed={x:?} canonical={y:?}");
                    if let (Err(x), Err(y)) = (&x, &y) {
                        assert_eq!(format!("{x:?}"), format!("{y:?}"));
                    }
                    assert_eq!(drafts(&a), drafts(&b));
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        counts.0, counts.1,
                        "all selected reads must avoid a canonical String: {counts:?}"
                    );
                    assert_eq!(counts.2, 0);
                    assert_eq!(plain.1, 0);
                    if x.is_ok() {
                        assert_eq!(counts.1, counts.3);
                    }
                    if first < end && !keys.is_empty() {
                        assert!(counts.1 > 0, "bridge not exercised");
                    }
                    drop((a, b, args, aliases));
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
fn matrix_map_keys_reject_stale_source_token_and_per_call_caps_and_retry_atomically() {
    for artifact in artifacts(SOURCE) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &["b", "a", "b"], 0, 2);
            let mut output = vec![None; artifact.outputs().len()];
            run(&jit, &args, &mut output, &memory, restored, Fault::None)
                .0
                .unwrap();
            for fault in [
                Fault::SourceStale,
                Fault::Token,
                Fault::ProducerCloned,
                Fault::ConsumerOutput,
            ] {
                let old = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                assert!(
                    run(&jit, &args, &mut output, &memory, restored, fault)
                        .0
                        .is_err(),
                    "{fault:?}"
                );
                assert_eq!(drafts(&output), drafts(&old));
                assert!(output.iter().zip(&old).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                run(&jit, &args, &mut output, &memory, restored, Fault::None)
                    .0
                    .unwrap();
            }
            drop((args, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn matrix_map_keys_finite_global_boundaries_preserve_publication_and_cleanup() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &["b", "a", "é", "a\0b"], 0, 4)
    });
}

#[test]
fn matrix_map_keys_decline_metadata_headroom_and_keep_canonical_fallback() {
    for artifact in artifacts(SOURCE) {
        let native =
            JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::BorrowedMatrixMapKeys,
        )
        .unwrap();
        for restored in [false, true] {
            let keys: Vec<String> = (0..512).map(|i| format!("key-{i:03}")).collect();
            let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &keys, 0, 512);
            let mut a = vec![None; artifact.outputs().len()];
            let mut b = a.clone();
            super::super::maps::owner::tests::take_geometry_counts();
            let (result, counts) = run(&native, &args, &mut a, &memory, restored, Fault::Headroom);
            result.unwrap();
            assert_eq!(
                super::super::maps::owner::tests::take_geometry_counts().0,
                0,
                "refused loans must retain the full caller census"
            );
            run(&ordinary, &args, &mut b, &memory, restored, Fault::None)
                .0
                .unwrap();
            assert_eq!(drafts(&a), drafts(&b));
            assert_eq!(counts, (512, 0, 512, 512));
            drop((args, a, b));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn matrix_map_keys_checked_loading_keeps_independent_validation_and_fresh_identical_proofs() {
    use crate::native_artifact::program::prepare::matrix_map_keys::take_plan_validations;
    use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        take_plan_validations();
        let object = emit_program_object(&artifact, &catalog).unwrap();
        assert!(
            take_plan_validations() > 0,
            "checked source preparation validates the plan"
        );
        let expected = &object.descriptor().metadata().matrix_map_keys;
        assert_eq!(expected.regions.len(), 1);
        let image = object.native_metadata();
        // SAFETY: exact checked image, deliberately use independent loading.
        let independent =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(image, &catalog) }.unwrap();
        assert!(
            take_plan_validations() > 0,
            "independent bytes retain global validation"
        );
        assert_eq!(&independent.metadata().matrix_map_keys, expected);
        // SAFETY: genuine immutable metadata/identity/binding from this held
        // compiler Object. No fabricated digest or foreign entry is accepted.
        let bound = unsafe {
            NativeProgramDescriptor::decode_object_bound_image(
                image,
                &catalog,
                object.identity().bytes(),
                object.native_metadata_binding().bytes(),
            )
        }
        .unwrap();
        assert_eq!(
            take_plan_validations(),
            0,
            "same-Object provenance avoids repeated validation"
        );
        assert_eq!(&bound.metadata().matrix_map_keys, expected);
        assert_eq!(
            bound.typed_ir_sha256(),
            object.descriptor().typed_ir_sha256()
        );
        let mut changed = image.to_vec();
        let last = changed.last_mut().unwrap();
        *last ^= 1;
        // SAFETY: negative integrity-only check; never attach or execute.
        assert!(
            unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    &changed,
                    &catalog,
                    object.identity().bytes(),
                    object.native_metadata_binding().bytes(),
                )
            }
            .is_err()
        );
        assert_eq!(
            take_plan_validations(),
            0,
            "changed bytes fail binding before derivation"
        );
    }
}

#[test]
fn matrix_map_keys_ready_loans_enable_current_census_and_canonical_ablation_does_not() {
    use super::super::maps::owner::tests::take_geometry_counts;
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::BorrowedMatrixMapKeys,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, &["a", "z", "é", "a\0b"], 0, 4);
            let mut left = vec![None; artifact.outputs().len()];
            let mut right = left.clone();
            take_geometry_counts();
            let (a, counts) = run(&native, &args, &mut left, &memory, restored, Fault::None);
            a.unwrap();
            let (cached, full) = take_geometry_counts();
            assert_eq!(counts.1, 4);
            assert!(
                cached >= 4,
                "must exercise cached census after loan setup: {cached}/{full}"
            );
            run(&ordinary, &args, &mut right, &memory, restored, Fault::None)
                .0
                .unwrap();
            let (uncached, full) = take_geometry_counts();
            assert_eq!(uncached, 0);
            assert!(full > 0);
            assert_eq!(drafts(&left), drafts(&right));
            drop((args, left, right));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
