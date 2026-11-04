//! Real uniform comprehensions, paid bank ownership, exact cold paths and retry.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use crate::native_artifact::program::runtime::NativeUniformFillFrame;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::c_void,
};
#[derive(Clone, Copy, Default)]
struct Flags {
    ordinary: bool,
    corrupt: u8,
    interleave: bool,
}
#[derive(Default, Debug)]
struct Counts {
    selected: u64,
    complete: u64,
    interleaved_refused: u64,
    elements: BTreeMap<u64, u64>,
    helpers: BTreeMap<(u64, u64), u64>,
}
thread_local! {
    static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
    static FLAGS:Cell<Flags>=Cell::new(Flags::default());
    static COUNTS:RefCell<Counts>=RefCell::new(Counts::default());
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
    if op == GlobalHelper::UniformFillPrepare.raw() && flags.ordinary {
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::UniformFillFinish.raw() {
        if flags.interleave {
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let before = invocation.state.control.visits;
            assert!(invocation.visit(site).is_err());
            assert!(
                invocation
                    .helper(site, GlobalHelper::Move.raw(), [0; 3])
                    .is_err()
            );
            assert_eq!(invocation.state.control.visits, before);
            COUNTS.with(|v| v.borrow_mut().interleaved_refused += 2);
        }
        if (1..=9).contains(&flags.corrupt) {
            // Only the current build-owned frame returned by this invocation.
            // No pointer is retained across callback completion or a later call.
            let frame = unsafe { &mut *(a as *mut NativeUniformFillFrame) };
            match flags.corrupt {
                1 => frame.epoch += 1,
                2 => frame.issued += 1,
                3 => frame.count += 1,
                4 => frame.capacity_bytes += 1,
                5 => frame.output = 0,
                6 => frame.word ^= 1,
                7 => frame.width += 1,
                8 => frame.next = 0,
                9 => b = 1,
                _ => unreachable!(),
            }
        }
        if flags.corrupt == 10 || flags.corrupt == 11 {
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            if flags.corrupt == 10 {
                let iteration = invocation
                    .descriptor
                    .metadata()
                    .uniform_word_collections
                    .iter()
                    .find(|p| u64::from(p.header()) == site)
                    .unwrap()
                    .iteration();
                invocation.progress[iteration as usize * 2] -= 1;
            } else {
                invocation.state.control.visits += 1;
            }
        }
    }
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0 {
        let value = unsafe { *out };
        if op == GlobalHelper::UniformFillPrepare.raw() && value != 0 {
            COUNTS.with(|v| {
                let mut counts = v.borrow_mut();
                counts.selected += 1;
                counts.elements.insert(site, b);
            });
        }
        if op == GlobalHelper::UniformFillFinish.raw() && value == 1 {
            COUNTS.with(|v| v.borrow_mut().complete += 1);
        }
    }
    status
}
fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    restored: bool,
    flags: Flags,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    let restored = restored.then(|| super::super::tests::restore_admitted_helper_descriptor(jit));
    let descriptor = restored.as_ref().unwrap_or_else(|| jit.descriptor());
    FLAGS.with(|v| v.set(flags));
    COUNTS.with(|v| *v.borrow_mut() = Counts::default());
    let entry = jit.test_checked_entry();
    let mode = NativeProgramLimitMode::NoLimits;
    let result = execute(
        descriptor,
        |raw: *mut Frame| unsafe {
            let original = (*raw).helper;
            ORIGINAL.with(|v| v.set(Some(original)));
            (*raw).helper = helper;
            let status = entry(raw);
            (*raw).helper = original;
            ORIGINAL.with(|v| v.set(None));
            status
        },
        args,
        output,
        mode.execution(),
        memory,
    );
    FLAGS.with(|v| v.set(Flags::default()));
    (
        result,
        COUNTS.with(|v| std::mem::take(&mut *v.borrow_mut())),
    )
}
fn source(kind: &str, shape: u8) -> String {
    let range = match shape {
        0 => "first..=last",
        1 => "first..last",
        2 => "0u64..=last, i < last",
        _ => unreachable!(),
    };
    format!(
        "first := start<u64>\nlast := end<u64>\nvalue := word<{kind}>\nresult := [value | i <- {range}]\n(result, value)\n"
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    start: u64,
    end: u64,
    word: Data,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("start", Data::U64(start)),
            ("end", Data::U64(end)),
            ("word", word),
        ],
        memory,
    )
}
fn last(output: &[Option<Value>]) -> Data {
    output
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn word(width: u8, zero: bool) -> Data {
    match (width, zero) {
        (1, true) => Data::U8(0),
        (2, true) => Data::U16(0),
        (4, true) => Data::U32(0),
        (8, true) => Data::U64(0),
        (1, false) => Data::U8(0xef),
        (2, false) => Data::U16(0xdcba),
        (4, false) => Data::U32(0xfedc_ba98),
        (8, false) => Data::U64(0xfedc_ba98_7654_3210),
        _ => unreachable!(),
    }
}
#[test]
fn uniform_fill_complete_sources_match_ordinary_all_widths_and_shapes() {
    for (kind, width) in [("u8", 1), ("u16", 2), ("u32", 4), ("u64", 8)] {
        for shape in [0, 1, 2] {
            let text = source(kind, shape);
            for artifact in artifacts(&text) {
                let mode = NativeProgramLimitMode::NoLimits;
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &mech_stdlib::source_catalog(),
                    mode.compilation(),
                )
                .unwrap();
                let metadata = jit.descriptor().metadata();
                assert_eq!(metadata.uniform_word_collections.len(), 1);
                let proof = &metadata.uniform_word_collections[0];
                let iteration = proof.iteration();
                let body_sites: Vec<_> = metadata
                    .plan
                    .blocks()
                    .iter()
                    .enumerate()
                    .filter(|(site, block)| {
                        block.iteration() == Some(iteration) && *site != proof.header() as usize
                    })
                    .map(|(site, _)| site as u64)
                    .collect();
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let cases = match shape {
                        0 => vec![(0, 0, 1), (5, 5, 1), (7, 39, 33)],
                        1 => vec![(0, 1, 1), (5, 6, 1), (7, 40, 33)],
                        2 => vec![(0, 0, 0), (0, 5, 5), (0, 33, 33)],
                        _ => unreachable!(),
                    };
                    for (start, end, count) in cases {
                        for zero in [false, true] {
                            let word = word(width, zero);
                            let args = arguments(&artifact, &memory, start, end, word.clone());
                            let aliases = args.clone();
                            let before = contents(&args);
                            let expected = Data::Tuple(
                                vec![Data::Matrix(vec![word.clone(); count].into()), word].into(),
                            );
                            for ordinary in [false, true] {
                                let mut output = vec![None; artifact.outputs().len()];
                                let (result, counts) = run(
                                    &jit,
                                    &args,
                                    &mut output,
                                    &memory,
                                    restored,
                                    Flags {
                                        ordinary,
                                        ..Default::default()
                                    },
                                );
                                result.unwrap();
                                assert_eq!(last(&output), expected);
                                if !ordinary {
                                    assert_eq!(
                                        (counts.selected, counts.complete),
                                        (1, 1),
                                        "{counts:?}"
                                    );
                                    assert!(
                                        counts
                                            .helpers
                                            .keys()
                                            .all(|(site, _)| !body_sites.contains(site)),
                                        "{counts:?}"
                                    );
                                }
                            }
                            assert_eq!(contents(&args), before);
                            assert_eq!(contents(&aliases), before);
                        }
                    }
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
fn uniform_fill_complete_bank_blocks_interleaving_and_rejects_tampered_completion_atomically() {
    for artifact in artifacts(&source("u64", 0)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 7, 23, Data::U64(519));
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = run(
                &jit,
                &args,
                &mut output,
                &memory,
                restored,
                Flags {
                    interleave: true,
                    ..Default::default()
                },
            );
            result.unwrap();
            assert_eq!(counts.interleaved_refused, 2);
            let expected = last(&output);
            for corrupt in 1..=11 {
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert!(result.is_err(), "corrupt{corrupt}: {counts:?}");
                assert_eq!(counts.selected, 1);
                assert!(output.iter().zip(&prior).all(|(now, old)| {
                    now.as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    restored,
                    Flags::default(),
                )
                .0
                .unwrap();
                assert_eq!(last(&output), expected);
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
fn uniform_fill_finite_caps_keep_publication_contents_aliases_and_retry() {
    resources::finite_boundaries(&source("u64", 0), |artifact, memory| {
        arguments(artifact, memory, 7, 23, Data::U64(519))
    });
}

fn merged_source() -> &'static str {
    "first := start<u64>\nlast0 := end<u64>\nvalue0 := word<u64>\n\
     last := ((first < last0)? | true => last0 | false => first)\n\
     value := ((first < last0)? | true => value0 | false => value0 + 1u64)\n\
     result := [value | i <- first..=last]\n(result, value)\n"
}

#[test]
fn uniform_fill_conditional_extent_and_word_use_native_stores_on_both_arms() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(merged_source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::UniformWords,
        )
        .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .uniform_word_collections
                .len(),
            1
        );
        let proof = &native.descriptor().metadata().uniform_word_collections[0];
        let sites: Vec<_> = native
            .descriptor()
            .plan()
            .blocks()
            .iter()
            .enumerate()
            .filter(|(site, block)| {
                block.iteration() == Some(proof.iteration()) && *site != proof.header() as usize
            })
            .map(|(site, _)| site as u64)
            .collect();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for (start, end, count, value) in [(7, 23, 17, 519), (23, 7, 1, 520), (5, 5, 1, 520)] {
                let args = arguments(&artifact, &memory, start, end, Data::U64(519));
                let aliases = args.clone();
                let before = contents(&args);
                let expected = Data::Tuple(
                    vec![
                        Data::Matrix(vec![Data::U64(value); count].into()),
                        Data::U64(value),
                    ]
                    .into(),
                );
                for (jit, selected) in [(&native, true), (&ordinary, false)] {
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, counts) =
                        run(jit, &args, &mut output, &memory, restored, Flags::default());
                    result.unwrap();
                    assert_eq!(last(&output), expected);
                    if selected {
                        assert_eq!((counts.selected, counts.complete), (1, 1));
                        assert!(
                            counts.helpers.keys().all(|(site, _)| !sites.contains(site)),
                            "{counts:?}"
                        );
                    }
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn uniform_fill_conditional_extent_preserves_all_finite_boundaries() {
    resources::finite_boundaries(merged_source(), |artifact, memory| {
        arguments(artifact, memory, 7, 23, Data::U64(519))
    });
}
#[test]
fn uniform_fill_keeps_selected_range_errors_before_construction() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&source("u64", 0)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::UniformWords,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for (start, end) in [(1, 0), (0, u64::MAX), (1, u64::MAX)] {
                let args = arguments(&artifact, &memory, start, end, Data::U64(91));
                let before = contents(&args);
                let mut errors = Vec::new();
                for jit in [&native, &ordinary] {
                    let mut output = vec![None; artifact.outputs().len()];
                    let error = run(jit, &args, &mut output, &memory, restored, Flags::default())
                        .0
                        .unwrap_err();
                    errors.push(format!("{error:?}"));
                    assert!(output.iter().all(Option::is_none));
                }
                assert_eq!(errors[0], errors[1]);
                assert_eq!(contents(&args), before);
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn uniform_fill_proof_declines_dependent_yields_nonprefix_filters_and_other_kinds() {
    for text in [
        "[i | i <- 1u64..=count<u64>]\n",
        "value := word<u64>\n[value | i <- 1u64..=count<u64>, i % 2u64 == 0u64]\n",
        "value := word<u64>\n[value + i | i <- 1u64..=count<u64>]\n",
        "value := word<bool>\n[value | i <- 1u64..=count<u64>]\n",
    ] {
        for artifact in artifacts(text) {
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
                    .uniform_word_collections
                    .is_empty()
            );
        }
    }
}
#[test]
fn uniform_fill_images_reject_old_identity_and_underfunded_source_header() {
    use crate::native_artifact::{NativeProgramDescriptor, NativeProgramObjectIdentity};
    for artifact in artifacts(&source("u64", 0)) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let identity = NativeProgramObjectIdentity::from_descriptor(jit.descriptor()).unwrap();
        let mut old = *identity.bytes();
        old[16..24].copy_from_slice(&155u64.to_le_bytes());
        old[32..40].copy_from_slice(&106u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
        let mut image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let header = jit.descriptor().metadata().uniform_word_collections[0].header() as usize;
        image["descriptor"]["visits"][header]["work"] = 0.into();
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
