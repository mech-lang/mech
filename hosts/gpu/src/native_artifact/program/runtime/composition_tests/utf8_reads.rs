//! Whole-loop String-key work: real collection producers, atomic fallback,
//! current generations, finite caps and independently reconstructed images.
use super::*;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use mech_core::{
    SchemaBody,
    snapshot::{MapEntryDraft, NamedValueDraft},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::c_void,
};

const DIRECT: &str = r#"+> map
index := dictionary<{string:u64}>
tags := keys<[string]>
result := [index{tag} | tag <- tags]
result
"#;
const RECORD: &str = r#"+> map
<bag> := <{index<{string:u64}> salt<u64>}>
words := dictionary<bag>
tags := keys<[string]>
result := [words.index{tag} | tag <- tags]
result
"#;
#[derive(Clone, Copy, Default)]
struct Flags {
    ordinary: bool,
    miss_at_finish: bool,
    replace: u8,
    stale_classes: bool,
    decline_histogram: bool,
}
#[derive(Default, Debug)]
struct Counts {
    selected: u64,
    complete: u64,
    misses: u64,
    replacements: u64,
    classified_queries: Vec<(u64, u64)>,
    helpers: BTreeMap<(u64, u64), u64>,
}
thread_local! {
    static ORIGINAL: Cell<Option<Helper>> = const { Cell::new(None) };
    static FLAGS: Cell<Flags> = Cell::new(Flags::default());
    static COUNTS: RefCell<Counts> = RefCell::new(Counts::default());
}
unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    op: u64,
    a: u64,
    mut b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    COUNTS.with(|v| *v.borrow_mut().helpers.entry((site, op)).or_default() += 1);
    let flags = FLAGS.with(Cell::get);
    if op == GlobalHelper::HistogramPrepare.raw() && flags.decline_histogram {
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::Utf8ReadPrepare.raw() && flags.stale_classes {
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        let source = invocation
            .descriptor
            .metadata()
            .utf8_reads
            .at_header(site as usize)
            .unwrap()
            .source;
        let old = invocation
            .state
            .arena
            .copy_current(source, &mut invocation.state.control.work)
            .unwrap();
        invocation
            .state
            .arena
            .put(source, &old, &mut invocation.state.control.work)
            .unwrap();
    }
    if op == GlobalHelper::Utf8ReadPrepare.raw() && flags.ordinary {
        // SAFETY: return this optional helper's documented zero/declined result
        // without invoking a producer, reserving metadata or altering a Value.
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::Utf8ReadFinish.raw() {
        if flags.miss_at_finish {
            b = 2;
        }
        if flags.replace != 0 {
            // Replace ONLY the current generation of the same canonical root.
            // Payload/schema/account pointer equality cannot validate old handles.
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let proof = invocation
                .descriptor
                .metadata()
                .utf8_reads
                .regions
                .iter()
                .find(|region| u64::from(region.header) == site)
                .unwrap();
            let slot = if flags.replace == 1 {
                proof.source
            } else {
                proof.map_root
            };
            let before = invocation
                .state
                .arena
                .current_handle(slot, &mut invocation.state.control.work)
                .unwrap();
            let value = invocation
                .state
                .arena
                .copy_current(slot, &mut invocation.state.control.work)
                .unwrap();
            let after = invocation
                .state
                .arena
                .put(slot, &value, &mut invocation.state.control.work)
                .unwrap();
            assert_ne!(before, after);
            COUNTS.with(|v| v.borrow_mut().replacements += 1);
        }
        if b == 2 {
            COUNTS.with(|v| v.borrow_mut().misses += 1);
        }
    }
    // SAFETY: unchanged live callback/context, delegated once, with the one
    // explicit test-only missing-status injection described above.
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0 {
        let word = unsafe { *out };
        if op == GlobalHelper::Utf8ReadPrepare.raw() && word != 0 {
            COUNTS.with(|v| v.borrow_mut().selected += 1);
            let frame = unsafe {
                &*(word as *const crate::native_artifact::program::runtime::maps::utf8_read::Frame)
            };
            if frame.representatives != 0 {
                COUNTS.with(|v| {
                    v.borrow_mut()
                        .classified_queries
                        .push((frame.count, frame.output_count))
                });
            }
        }
        if op == GlobalHelper::Utf8ReadFinish.raw() && word == 1 {
            COUNTS.with(|v| v.borrow_mut().complete += 1);
        }
    }
    status
}
fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
    flags: Flags,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    let decoded = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = decoded.as_ref().unwrap_or_else(|| jit.descriptor());
    FLAGS.with(|v| v.set(flags));
    COUNTS.with(|v| *v.borrow_mut() = Counts::default());
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| {
            // SAFETY: execute retains this exact initialized frame and JIT owner.
            unsafe {
                let original = (*raw).helper;
                ORIGINAL.with(|v| v.set(Some(original)));
                (*raw).helper = helper;
                let status = entry(raw);
                (*raw).helper = original;
                ORIGINAL.with(|v| v.set(None));
                status
            }
        },
        args,
        output,
        policy,
        memory,
    );
    FLAGS.with(|v| v.set(Flags::default()));
    (
        result,
        COUNTS.with(|v| std::mem::take(&mut *v.borrow_mut())),
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    entries: &[(&str, u64)],
    keys: &[&str],
    rows: u64,
    columns: u64,
) -> Vec<Value> {
    assert_eq!(rows.checked_mul(columns), Some(keys.len() as u64));
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let schema = artifact.schemas().get(input.schema).unwrap();
            let (data, shape_values) = if name == "keys" {
                let shape =
                    mech_core::shape_for_resolved_extents(schema, &[rows, columns]).unwrap();
                (
                    Data::Matrix(keys.iter().map(|key| Data::String((*key).into())).collect()),
                    shape.parameter_values().into(),
                )
            } else {
                assert_eq!(name, "dictionary");
                let map = Data::Map(
                    entries
                        .iter()
                        .map(|(key, value)| MapEntryDraft {
                            items: [Data::String((*key).into()), Data::U64(*value)].into(),
                        })
                        .collect(),
                );
                let data = if matches!(schema.body(), SchemaBody::Record(_)) {
                    Data::Record(
                        [
                            NamedValueDraft {
                                name: "index".into(),
                                value: map,
                            },
                            NamedValueDraft {
                                name: "salt".into(),
                                value: Data::U64(91),
                            },
                        ]
                        .into(),
                    )
                } else {
                    map
                };
                (data, Box::new([]) as Box<[u64]>)
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
fn drafts(output: &[Option<Value>]) -> Vec<Data> {
    output
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn assert_prior(output: &[Option<Value>], old: &[Option<Value>]) {
    assert_eq!(drafts(output), drafts(old));
    assert!(output.iter().zip(old).all(|(a, b)| {
        a.as_ref()
            .unwrap()
            .shares_frozen_storage(b.as_ref().unwrap())
    }));
}
fn selected(
    jit: &JitProgramArtifact,
) -> &crate::native_artifact::program::prepare::utf8_reads::Region {
    let metadata = jit.descriptor().metadata();
    let independent = crate::native_artifact::program::prepare::utf8_reads::Layout::derive(
        jit.descriptor().plan(),
    )
    .unwrap();
    assert_eq!(independent, metadata.utf8_reads);
    assert_eq!(
        metadata.utf8_reads.regions.len(),
        1,
        "real fixture must select native work"
    );
    &metadata.utf8_reads.regions[0]
}

#[test]
fn utf8_collection_native_work_preserves_physical_order_unicode_duplicates_and_current_inputs() {
    let long = "雪\0é👩‍🔬".repeat(7);
    let names = ["", "a\0b", "é", "e\u{301}", "plain", long.as_str()];
    let entries = names
        .iter()
        .enumerate()
        .map(|(i, key)| (*key, 100 + i as u64 * 17))
        .collect::<Vec<_>>();
    for source in [DIRECT, RECORD] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let proof = selected(&jit);
            assert_eq!(proof.project_site.is_some(), source == RECORD);
            for restored in [false, true] {
                let memory = mode.memory_budget();
                for (keys, rows, columns) in [
                    (names.to_vec(), 2, 3),
                    (vec![long.as_str(), "é", long.as_str(), ""], 1, 4),
                    (vec![], 1, 0),
                ] {
                    let args = arguments(&artifact, &memory, &entries, &keys, rows, columns);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let expected = Data::Matrix(
                        keys.iter()
                            .map(|key| Data::U64(entries.iter().find(|(k, _)| k == key).unwrap().1))
                            .collect(),
                    );
                    let mut native = vec![None; artifact.outputs().len()];
                    let (result, counts) = run(
                        &jit,
                        &args,
                        &mut native,
                        mode.execution(),
                        &memory,
                        restored,
                        Flags::default(),
                    );
                    result.unwrap();
                    assert_eq!(drafts(&native).last(), Some(&expected));
                    assert_eq!((counts.selected, counts.complete), (1, 1), "{counts:?}");
                    for site in [
                        Some(proof.element_site),
                        proof.project_site,
                        Some(proof.read_site),
                        Some(proof.push_site),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        assert!(
                            counts.helpers.keys().all(|(at, _)| *at != u64::from(site)),
                            "body callback survived native work: {counts:?}"
                        );
                    }
                    for flags in [
                        Flags {
                            ordinary: true,
                            ..Flags::default()
                        },
                        Flags {
                            miss_at_finish: true,
                            ..Flags::default()
                        },
                    ] {
                        let mut ordinary = vec![None; artifact.outputs().len()];
                        let (result, counts) = run(
                            &jit,
                            &args,
                            &mut ordinary,
                            mode.execution(),
                            &memory,
                            restored,
                            flags,
                        );
                        result.unwrap();
                        assert_eq!(drafts(&ordinary), drafts(&native));
                        assert_eq!(counts.complete, 0);
                        if flags.miss_at_finish {
                            assert_eq!(counts.misses, 1);
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    drop((args, aliases, native));
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
fn utf8_collection_missing_prefix_discards_private_output_and_keeps_exact_error_and_retry() {
    for artifact in artifacts(RECORD) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let proof = selected(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let good = arguments(&artifact, &memory, &[("old", 7), ("é", 11)], &["old"], 1, 1);
            let mut output = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &good,
                &mut output,
                mode.execution(),
                &memory,
                restored,
                Flags::default(),
            )
            .0
            .unwrap();
            let prior = output.clone();
            let missing = arguments(
                &artifact,
                &memory,
                &[("old", 7), ("é", 11)],
                &["old", "absent", "é"],
                1,
                3,
            );
            let aliases = missing.clone();
            let before = contents(&missing);
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut ordinary_error = None;
            for ordinary in [true, false] {
                let (result, counts) = run(
                    &jit,
                    &missing,
                    &mut output,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags {
                        ordinary,
                        ..Flags::default()
                    },
                );
                let error = format!("{:?}", result.unwrap_err());
                assert!(error.contains("Library"), "{error}");
                if ordinary {
                    ordinary_error = Some(error);
                } else {
                    assert_eq!(Some(error), ordinary_error);
                    assert_eq!(
                        (counts.selected, counts.complete, counts.misses),
                        (1, 0, 1),
                        "{counts:?}"
                    );
                    assert_eq!(
                        counts.helpers.get(&(
                            u64::from(proof.element_site),
                            GlobalHelper::Collection.raw()
                        )),
                        Some(&2),
                        "ordinary producer must restart at0 and stop onsecond missing key: {counts:?}"
                    );
                }
                assert_prior(&output, &prior);
                assert_eq!(contents(&missing), before);
                assert_eq!(contents(&aliases), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            run(
                &jit,
                &good,
                &mut output,
                mode.execution(),
                &memory,
                restored,
                Flags::default(),
            )
            .0
            .unwrap();
            assert_eq!(drafts(&output), drafts(&prior));
            drop((good, missing, aliases, prior, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn utf8_collection_stale_source_or_map_generation_denies_before_publication_and_retries() {
    for artifact in artifacts(DIRECT) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        selected(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, &[("old", 7)], &["old"], 1, 1);
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &args,
                &mut output,
                mode.execution(),
                &memory,
                restored,
                Flags::default(),
            )
            .0
            .unwrap();
            for replace in [1, 2] {
                // A successful retry may publish a fresh equal root. Each
                // failure must preserve the roots current immediately before it.
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags {
                        replace,
                        ..Flags::default()
                    },
                );
                assert!(format!("{:?}", result.unwrap_err()).contains("Stale"));
                assert_eq!(
                    (counts.selected, counts.complete, counts.replacements),
                    (1, 0, 1)
                );
                assert_prior(&output, &prior);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                run(
                    &jit,
                    &args,
                    &mut output,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                assert_eq!(drafts(&output), drafts(&prior));
            }
            drop((args, aliases, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn utf8_collection_finite_caps_preserve_exact_boundary_atomic_denial_and_retry() {
    // One existing key keeps strict work floors deterministic; Unicode/growth,
    // duplicates and distinct2Dkeys are independently exercised above.
    resources::finite_boundaries(DIRECT, |artifact, memory| {
        arguments(artifact, memory, &[("old", 7)], &["old"], 1, 1)
    });
    for artifact in artifacts(DIRECT) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        selected(&jit);
        let args = arguments(&artifact, &memory, &[("old", 7)], &["old"], 1, 1);
        let before = contents(&args);
        let mut output = vec![None; artifact.outputs().len()];
        let (result, counts) = run(
            &jit,
            &args,
            &mut output,
            mode.execution(),
            &memory,
            false,
            Flags::default(),
        );
        result.unwrap();
        assert_eq!((counts.selected, counts.complete), (1, 1));
        let prior = output.clone();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for dimension in [
            "output-bytes",
            "output-elements",
            "cloned",
            "logical-temporary",
            "logical-nodes",
        ] {
            for restored in [false, true] {
                let trial = |allowance| {
                    let mut policy = mode.execution();
                    match dimension {
                        "output-bytes" => policy.library.max_output_bytes = Some(allowance),
                        "output-elements" => policy.library.max_output_elements = Some(allowance),
                        "cloned" => policy.library.max_cloned_bytes = Some(allowance),
                        "logical-temporary" => policy.library.max_temporary_bytes = Some(allowance),
                        "logical-nodes" => policy.library.max_retained_nodes = Some(allowance),
                        _ => unreachable!(),
                    }
                    let mut target = prior.clone();
                    let (result, _) = run(
                        &jit,
                        &args,
                        &mut target,
                        policy,
                        &memory,
                        restored,
                        Flags::default(),
                    );
                    if result.is_err() {
                        assert_prior(&target, &prior);
                    } else {
                        assert_eq!(drafts(&target), drafts(&prior));
                    }
                    drop(target);
                    assert_eq!(contents(&args), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    result.is_ok()
                };
                let mut lo = 0;
                let mut hi = 1_000_000_000u64;
                assert!(trial(hi), "{dimension}");
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if trial(mid) { hi = mid } else { lo = mid + 1 }
                }
                assert!(trial(lo), "{dimension} exact");
                if lo > 0 {
                    assert!(!trial(lo - 1), "{dimension} one-short");
                }
                assert!(trial(1_000_000_000));
            }
        }
        drop((args, output, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn utf8_collection_images_reject_old_identity_forged_uses_and_unfunded_header() {
    use crate::native_artifact::{NativeProgramDescriptor, NativeProgramObjectIdentity};
    for artifact in artifacts(DIRECT) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let proof = selected(&jit);
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&154u64.to_le_bytes());
        old[32..40].copy_from_slice(&105u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
        let original: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        for field in [
            "source",
            "map_root",
            "element_site",
            "read_site",
            "push_site",
            "header",
            "output",
        ] {
            let mut forged = original.clone();
            let at = &mut forged["descriptor"]["utf8_reads"]["regions"][0][field];
            *at = (at.as_u64().unwrap() + 1).into();
            // SAFETY: malformed metadata diagnostics only, never executed.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&forged).unwrap(),
                        &catalog,
                    )
                }
                .is_err(),
                "forged{field}"
            );
        }
        for variant in 0..3 {
            let mut forged = original.clone();
            match variant {
                0 => {
                    forged["descriptor"]["utf8_reads"]["headers"][proof.header as usize] =
                        serde_json::Value::Null
                }
                1 => forged["descriptor"]["utf8_reads"]["regions"] = serde_json::json!([]),
                2 => forged["descriptor"]["visits"][proof.header as usize]["work"] = 0.into(),
                _ => unreachable!(),
            }
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&forged).unwrap(),
                        &catalog,
                    )
                }
                .is_err(),
                "forgedlayout{variant}"
            );
        }
    }
}

#[test]
fn utf8_word_chunks_keep_original_decoded_restored_outputs_reports_and_atomic_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let names = (0..=65)
        .map(|n| "x".repeat(n))
        .chain(["é東京\0👩‍🔬".into()])
        .collect::<Vec<_>>();
    let entries = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i as u64))
        .collect::<Vec<_>>();
    let keys = names.iter().map(String::as_str).collect::<Vec<_>>();
    for source in [DIRECT, RECORD] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let legacy = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::Utf8WordChunks,
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
                &artifact,
                &catalog,
                OptimizationPass::Utf8WordChunks,
            )
            .unwrap();
            assert_eq!(
                object.metadata()["compiled_utf8_word_chunk_sites"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert!(
                old.metadata()["compiled_utf8_word_chunk_sites"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            for restored in [false, true] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let mut left = vec![None; artifact.outputs().len()];
                let mut right = vec![None; artifact.outputs().len()];
                for missing in [false, true, false] {
                    let mut query = keys.clone();
                    if missing {
                        query.push("absent-tail");
                    }
                    let args =
                        arguments(&artifact, &memory, &entries, &query, 1, query.len() as u64);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = right.clone();
                    let held = (
                        memory.used_bytes(),
                        memory.retained_nodes(),
                        memory.retained_snapshot_nodes(),
                    );
                    let (a, ac) = run(
                        &native,
                        &args,
                        &mut left,
                        mode.execution(),
                        &memory,
                        restored,
                        Flags::default(),
                    );
                    let (b, bc) = run(
                        &legacy,
                        &args,
                        &mut right,
                        mode.execution(),
                        &memory,
                        restored,
                        Flags::default(),
                    );
                    if missing {
                        assert_eq!(
                            format!("{:?}", a.unwrap_err()),
                            format!("{:?}", b.unwrap_err())
                        );
                        assert_prior(&right, &prior);
                        assert_eq!(
                            (
                                memory.used_bytes(),
                                memory.retained_nodes(),
                                memory.retained_snapshot_nodes()
                            ),
                            held
                        );
                    } else {
                        assert_eq!(format!("{:?}", a.unwrap()), format!("{:?}", b.unwrap()));
                        assert_eq!(drafts(&left), drafts(&right));
                        assert_eq!(
                            (ac.selected, ac.complete, bc.selected, bc.complete),
                            (1, 1, 1, 1)
                        );
                    }
                    assert_eq!(contents(&aliases), before);
                }
                drop((left, right));
                assert_eq!(
                    (
                        memory.used_bytes(),
                        memory.retained_nodes(),
                        memory.retained_snapshot_nodes()
                    ),
                    (0, 0, 0)
                );
            }
        }
    }
}

#[test]
fn native_string_classes_reuse_survives_images_arbitrary_maps_stale_handles_and_fallback() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for source in [
        DIRECT.to_string(),
        RECORD.to_string(),
        DIRECT.replace(
            "result :=",
            "unrelated := map/histogram([\"other\"])\nresult :=",
        ),
    ] {
        let source = source.replace("result :=", "classes := map/histogram(tags)\nresult :=");
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let legacy = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ReuseStringClasses,
            )
            .unwrap();
            assert!(selected(&native).classification_site.is_some());
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert_eq!(
                object.metadata()["compiled_utf8_histogram"]["classified"]["sites"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            let names = (0..97)
                .map(|i| format!("{}é\0東京", "x".repeat(i)))
                .collect::<Vec<_>>();
            let entries = names
                .iter()
                .enumerate()
                .map(|(i, key)| (key.as_str(), 1000 + i as u64 * 13))
                .collect::<Vec<_>>();
            for restored in [false, true] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                for keys in [
                    vec![],
                    vec![names[60].as_str(); 257],
                    names.iter().map(String::as_str).cycle().take(300).collect(),
                ] {
                    let args = arguments(&artifact, &memory, &entries, &keys, 1, keys.len() as u64);
                    let mut expected = vec![None; artifact.outputs().len()];
                    run(
                        &legacy,
                        &args,
                        &mut expected,
                        mode.execution(),
                        &memory,
                        restored,
                        Flags::default(),
                    )
                    .0
                    .unwrap();
                    for flags in [
                        Flags::default(),
                        Flags {
                            ordinary: true,
                            ..Flags::default()
                        },
                        Flags {
                            stale_classes: true,
                            ..Flags::default()
                        },
                        Flags {
                            decline_histogram: true,
                            ..Flags::default()
                        },
                        Flags {
                            miss_at_finish: true,
                            ..Flags::default()
                        },
                    ] {
                        let mut out = vec![None; artifact.outputs().len()];
                        let (result, counts) = run(
                            &native,
                            &args,
                            &mut out,
                            mode.execution(),
                            &memory,
                            restored,
                            flags,
                        );
                        result.unwrap();
                        assert_eq!(drafts(&out), drafts(&expected));
                        if !flags.ordinary && !flags.stale_classes && !flags.decline_histogram {
                            let distinct = keys
                                .iter()
                                .copied()
                                .collect::<std::collections::BTreeSet<_>>()
                                .len();
                            assert_eq!(
                                counts.classified_queries,
                                [(distinct as u64, keys.len() as u64)],
                                "{counts:?}"
                            );
                        } else {
                            assert!(counts.classified_queries.is_empty());
                        }
                        drop(out);
                    }
                    drop((args, expected));
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                }
                // Multiple absent classes can appear in a different hash-bank
                // order. Native fallback must preserve the original first error
                // and the previous published output, including an exact retry.
                let good = arguments(&artifact, &memory, &entries, &[names[3].as_str()], 1, 1);
                let mut left = vec![None; artifact.outputs().len()];
                let mut right = left.clone();
                run(
                    &native,
                    &good,
                    &mut left,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                run(
                    &legacy,
                    &good,
                    &mut right,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                let before = left.clone();
                let bad = arguments(
                    &artifact,
                    &memory,
                    &entries,
                    &[names[3].as_str(), "missing-z", "missing-a", "missing-z"],
                    1,
                    4,
                );
                let a = run(
                    &native,
                    &bad,
                    &mut left,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap_err();
                let b = run(
                    &legacy,
                    &bad,
                    &mut right,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap_err();
                assert_eq!(format!("{a:?}"), format!("{b:?}"));
                assert_prior(&left, &before);
                run(
                    &native,
                    &good,
                    &mut left,
                    mode.execution(),
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                assert_eq!(drafts(&left), drafts(&right));
                drop((good, bad, left, right, before));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            // Serialized hints are never independent proof authority.
            let mut image: serde_json::Value =
                serde_json::from_slice(&native.descriptor().encode_build_owned_image().unwrap())
                    .unwrap();
            image["descriptor"]["utf8_reads"]["regions"][0]["classification_site"] =
                serde_json::json!(u32::MAX);
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&image).unwrap(),
                        &catalog,
                    )
                }
                .is_err()
            );
        }
    }
}

#[test]
fn native_string_classes_finite_caps_admit_private_backing_and_preserve_atomic_retry() {
    let source = DIRECT.replace("result :=", "classes := map/histogram(tags)\nresult :=");
    resources::finite_boundaries(&source, |artifact, memory| {
        arguments(
            artifact,
            memory,
            &[("old", 7)],
            &["old", "old", "old"],
            1,
            3,
        )
    });
}
