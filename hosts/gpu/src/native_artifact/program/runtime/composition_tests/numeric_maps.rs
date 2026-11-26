//! Native numeric transforms, exact checked errors, finite decline and current loans.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use crate::native_artifact::program::runtime::NativeNumericMapFrame;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::c_void,
};
#[derive(Clone, Copy, Default)]
pub(super) struct Flags {
    pub(super) ordinary: bool,
    pub(super) numeric_only: bool,
    pub(super) corrupt: u8,
    pub(super) interleave: bool,
}
#[derive(Default, Debug)]
pub(super) struct Counts {
    pub(super) selected: u64,
    pub(super) complete: u64,
    pub(super) word_selected: u64,
    pub(super) word_complete: u64,
    pub(super) transpose_complete: u64,
    pub(super) interleaved_refused: u64,
    pub(super) elements: BTreeMap<u64, u64>,
    pub(super) helpers: BTreeMap<(u64, u64), u64>,
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
    mut a: u64,
    mut b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    COUNTS.with(|v| *v.borrow_mut().helpers.entry((site, op)).or_default() += 1);
    let flags = FLAGS.with(Cell::get);
    // Corruption controls target the numeric-map frame specifically. Decline
    // the independently proved total-word alternative before it borrows a loan.
    if op == GlobalHelper::WordCollectionPrepare.raw()
        && (flags.ordinary || flags.numeric_only || flags.corrupt != 0 || flags.interleave)
    {
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::NumericTransposeFinish.raw() && flags.ordinary {
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::NumericTransposeFinish.raw() {
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        if flags.interleave {
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
        if flags.corrupt == 22 {
            a = 1;
        }
        if flags.corrupt == 23 {
            invocation.state.consumed = true;
        }
        if flags.corrupt == 24 {
            invocation.state.control.previous = None;
        }
    }
    if op == GlobalHelper::NumericMapPrepare.raw() && flags.ordinary {
        unsafe { *out = 0 };
        return 0;
    }
    if op == GlobalHelper::NumericMapFinish.raw() {
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
            let frame = unsafe { &mut *(a as *mut NativeNumericMapFrame) };
            match flags.corrupt {
                1 => frame.epoch += 1,
                2 => frame.issued += 1,
                3 => frame.count += 1,
                4 => frame.capacity_bytes += 1,
                5 => frame.output = 0,
                6 => frame.input ^= 1,
                7 => frame.input_width += 1,
                8 => frame.next = 0,
                9 => b = 2,
                _ => unreachable!(),
            }
        }
        if (13..=15).contains(&flags.corrupt) {
            let frame = unsafe { &mut *(a as *mut NativeNumericMapFrame) };
            match flags.corrupt {
                13 => frame.range_start ^= 1,
                14 => frame.matrices[0].input ^= 1,
                15 => frame.matrices[0].columns += 1,
                _ => unreachable!(),
            }
        }
        if flags.corrupt == 18 {
            let frame = unsafe { &mut *(a as *mut NativeNumericMapFrame) };
            frame.output_count += 1;
        }
        if flags.corrupt == 17 {
            let frame = unsafe { &mut *(a as *mut NativeNumericMapFrame) };
            frame.source_count += 1;
        }
        if flags.corrupt == 16 {
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            let source = invocation
                .descriptor
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .find(|r| u64::from(r.header) == site)
                .unwrap()
                .matrices[0]
                .value;
            let owner = invocation
                .state
                .arena
                .copy_current(source, &mut invocation.state.control.work)
                .unwrap();
            invocation
                .state
                .arena
                .put(source, &owner, &mut invocation.state.control.work)
                .unwrap();
        }
        if flags.corrupt == 10 || flags.corrupt == 11 {
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            if flags.corrupt == 10 {
                let iteration = invocation
                    .descriptor
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .find(|p| u64::from(p.header) == site)
                    .unwrap()
                    .iteration;
                invocation.progress[iteration as usize * 2] -= 1;
            } else {
                invocation.state.control.visits += 1;
            }
        }
    }
    if op == GlobalHelper::NumericMapFinish.raw() && flags.corrupt == 12 {
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        let source = invocation
            .descriptor
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .find(|r| u64::from(r.header) == site)
            .unwrap()
            .source;
        let before = invocation
            .state
            .arena
            .current_handle(source, &mut invocation.state.control.work)
            .unwrap();
        let value = invocation
            .state
            .arena
            .copy_current(source, &mut invocation.state.control.work)
            .unwrap();
        let after = invocation
            .state
            .arena
            .put(source, &value, &mut invocation.state.control.work)
            .unwrap();
        assert_ne!(before, after);
    }
    if op == GlobalHelper::NumericReductionFinish.raw() {
        let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        if flags.corrupt == 21 {
            invocation.state.control.visits += 1;
        }
        if flags.interleave {
            let before = invocation.state.control.visits;
            assert!(invocation.visit(site).is_err());
            assert!(
                invocation
                    .helper(site, GlobalHelper::Move.raw(), [0; 3])
                    .is_err()
            );
            assert!(
                invocation
                    .helper(site, GlobalHelper::NumericReductionFinish.raw(), [1, 0, 0])
                    .is_err()
            );
            assert_eq!(invocation.state.control.visits, before);
            COUNTS.with(|v| v.borrow_mut().interleaved_refused += 3);
        }
    }
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0 {
        let value = unsafe { *out };
        if op == GlobalHelper::WordCollectionPrepare.raw() && value == 1 {
            COUNTS.with(|counts| counts.borrow_mut().word_selected += 1);
        }
        if op == GlobalHelper::WordCollectionFinish.raw() {
            COUNTS.with(|counts| counts.borrow_mut().word_complete += 1);
        }
        if op == GlobalHelper::NumericMapPrepare.raw() && value != 0 {
            COUNTS.with(|v| {
                let mut counts = v.borrow_mut();
                counts.selected += 1;
                if flags.corrupt == 19 || flags.corrupt == 20 {
                    let frame = unsafe { &mut *(value as *mut NativeNumericMapFrame) };
                    if flags.corrupt == 19 {
                        frame.fold = 42;
                    } else {
                        frame.output_count += 1;
                    }
                }
                counts.elements.insert(site, unsafe {
                    (*(value as *const NativeNumericMapFrame)).count
                });
            });
        }
        if op == GlobalHelper::NumericTransposeFinish.raw() && value == 1 {
            COUNTS.with(|v| v.borrow_mut().transpose_complete += 1);
        }
        if op == GlobalHelper::NumericMapFinish.raw() && value == 1 {
            COUNTS.with(|v| v.borrow_mut().complete += 1);
        }
    }
    status
}
pub(super) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    output: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    policy: NativeProgramExecutionLimits,
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
    width: u8,
    words: &[u64],
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let schema = artifact.schemas().get(input.schema).unwrap();
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, shape_values) = if name == "words" {
                let shape = mech_core::shape_for_resolved_extents(schema, &[1, words.len() as u64])
                    .unwrap();
                let elements = words
                    .iter()
                    .map(|&x| match width {
                        1 => Data::U8(u8::try_from(x).unwrap()),
                        2 => Data::U16(u16::try_from(x).unwrap()),
                        4 => Data::U32(u32::try_from(x).unwrap()),
                        8 => Data::U64(x),
                        _ => unreachable!(),
                    })
                    .collect();
                (Data::Matrix(elements), shape.parameter_values().into())
            } else {
                assert_eq!(name, "factor");
                (Data::U32(3), Box::new([]) as Box<[u64]>)
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
fn last(output: &[Option<Value>]) -> Data {
    output
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn scalar(width: u8, x: u64) -> Data {
    match width {
        1 => Data::U8(x as u8),
        2 => Data::U16(x as u16),
        4 => Data::U32(x as u32),
        8 => Data::U64(x),
        _ => unreachable!(),
    }
}
fn simple(from: u8, to: u8) -> String {
    format!(
        "convert-word(x<u{}>) => (out<u{}>) :=\n  narrowed<u{}> := x\n  out := narrowed.\n[convert-word(x) | x <- words<[u{}]>]\n",
        from * 8,
        to * 8,
        to * 8,
        from * 8
    )
}
#[test]
fn numeric_maps_all_unsigned_widths_original_decoded_have_no_element_callbacks_and_release_loans() {
    for from in [1, 2, 4, 8] {
        for to in [1, 2, 4, 8] {
            let source = simple(from, to);
            for artifact in artifacts(&source) {
                let mode = NativeProgramLimitMode::NoLimits;
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &mech_stdlib::source_catalog(),
                    mode.compilation(),
                )
                .unwrap();
                assert_eq!(
                    jit.descriptor().metadata().numeric_maps.regions.len(),
                    1,
                    "{source}"
                );
                let proof = &jit.descriptor().metadata().numeric_maps.regions[0];
                let element_sites = jit
                    .descriptor()
                    .plan()
                    .blocks()
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| {
                        b.iteration() == Some(proof.iteration)
                            && matches!(
                                b.action(),
                                NativeIterationAction::CollectionElement { .. }
                                    | NativeIterationAction::CollectionPush { .. }
                            )
                    })
                    .map(|(i, _)| i as u64)
                    .collect::<Vec<_>>();
                for length in [0, 1, 2, 7, 16, 33] {
                    let data = (0..length)
                        .map(|i| (i * 7 % 256) as u64)
                        .collect::<Vec<_>>();
                    let memory = mode.memory_budget();
                    let args = arguments(&artifact, &memory, from, &data);
                    let retained = memory.used_bytes();
                    for restored in [false, true] {
                        for ordinary in [false, true] {
                            let mut output = vec![None; artifact.outputs().len()];
                            let (result, counts) = run(
                                &jit,
                                &args,
                                &mut output,
                                &memory,
                                mode.execution(),
                                restored,
                                Flags {
                                    ordinary,
                                    ..Default::default()
                                },
                            );
                            result.unwrap();
                            assert_eq!(
                                last(&output),
                                Data::Matrix(data.iter().map(|&x| scalar(to, x)).collect())
                            );
                            if !ordinary {
                                assert_eq!(
                                    (
                                        counts.selected + counts.word_selected,
                                        counts.complete + counts.word_complete
                                    ),
                                    (1, 1)
                                );
                                for &site in &element_sites {
                                    assert_eq!(
                                        counts
                                            .helpers
                                            .get(&(site, GlobalHelper::Collection.raw()))
                                            .copied()
                                            .unwrap_or(0),
                                        0
                                    )
                                }
                            }
                            drop(output);
                            assert_eq!(memory.used_bytes(), retained);
                        }
                    }
                    drop(args);
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
fn numeric_maps_checked_pipeline_captures_materialized_sources_errors_limits_and_recovery() {
    let source = "+> sequence\nscale(x<u64>, factor<u32>) => (out<u8>) :=\n  small<u32> := x\n  value := ((small * factor) + 1u32) / 2u32\n  byte<u8> := value\n  out := byte.\nsource := [x | x <- words<[u64]>]\nanswer := [scale(x, factor<u32>) | x <- source]\n(answer, source)\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 2);
        for restored in [false, true] {
            for words in [
                vec![0, 1, 2, 3, 7, 170],
                vec![0, 1, 171, 2],
                vec![0, u32::MAX as u64, 1],
                vec![0, u64::MAX, 1],
            ] {
                let memory = mode.memory_budget();
                let args = arguments(&artifact, &memory, 8, &words);
                let mut expected = vec![None; artifact.outputs().len()];
                let (cold, _) = run(
                    &jit,
                    &args,
                    &mut expected,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                let mut actual = vec![None; artifact.outputs().len()];
                let (fast, counts) = run(
                    &jit,
                    &args,
                    &mut actual,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                assert!(counts.selected > 0);
                match (cold, fast) {
                    (Ok(_), Ok(_)) => assert_eq!(last(&actual), last(&expected)),
                    (Err(a), Err(b)) => {
                        assert_eq!(format!("{a:?}"), format!("{b:?}"));
                        assert!(actual.iter().all(Option::is_none));
                    }
                    x => panic!("different results: {x:?}"),
                }
                drop((args, expected, actual));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
        let memory = mode.memory_budget();
        let words = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let args = arguments(&artifact, &memory, 8, &words);
        for cap in [0, 1, 7] {
            let mut policy = mode.execution();
            policy.library.max_output_elements = Some(cap);
            let mut old = vec![None; artifact.outputs().len()];
            let mut new = old.clone();
            let (a, _) = run(
                &jit,
                &args,
                &mut old,
                &memory,
                policy,
                false,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            );
            let (b, _) = run(
                &jit,
                &args,
                &mut new,
                &memory,
                policy,
                false,
                Flags::default(),
            );
            assert!(a.is_err() && b.is_err());
            assert_eq!(
                format!("{:?}", a.unwrap_err()),
                format!("{:?}", b.unwrap_err())
            );
            assert!(new.iter().all(Option::is_none));
        }
        let mut output = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &args,
            &mut output,
            &memory,
            mode.execution(),
            false,
            Flags::default(),
        )
        .0
        .unwrap();
        let ablated = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::NumericCollections,
        )
        .unwrap();
        let mut ordinary = vec![None; artifact.outputs().len()];
        let (result, counts) = run(
            &ablated,
            &args,
            &mut ordinary,
            &memory,
            mode.execution(),
            false,
            Flags::default(),
        );
        result.unwrap();
        assert_eq!(counts.selected, 0);
        assert_eq!(last(&ordinary), last(&output));
        drop((args, output, ordinary));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_map_loan_mutations_and_interleaved_visits_never_publish_and_recover() {
    for artifact in artifacts(&simple(8, 1)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            for corrupt in 1..=12 {
                let memory = mode.memory_budget();
                let args = arguments(&artifact, &memory, 8, &[1, 2, 3]);
                let mut output = vec![None; artifact.outputs().len()];
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert_eq!(counts.selected, 1);
                assert!(result.is_err(), "mutation {corrupt}");
                assert!(output.iter().all(Option::is_none));
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        interleave: true,
                        ..Default::default()
                    },
                );
                result.unwrap();
                assert_eq!(
                    (counts.selected, counts.complete, counts.interleaved_refused),
                    (1, 1, 2)
                );
                assert_eq!(
                    last(&output),
                    Data::Matrix(vec![Data::U8(1), Data::U8(2), Data::U8(3)].into())
                );
                drop((args, output));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
#[test]
fn numeric_map_fallible_probe_keeps_arithmetic_errors_under_tight_native_work_caps() {
    for artifact in artifacts(&simple(8, 1)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 8, &[1, 255, 256, 2]);
        let retained = memory.used_bytes();
        let mut checked = 0;
        for cap in (1000..=2_000_000).step_by(1000) {
            let mut policy = mode.execution();
            policy.execution.max_work = cap;
            let mut cold = vec![None; artifact.outputs().len()];
            let mut fast = cold.clone();
            let (a, _) = run(
                &jit,
                &args,
                &mut cold,
                &memory,
                policy,
                false,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            );
            let (b, _) = run(
                &jit,
                &args,
                &mut fast,
                &memory,
                policy,
                false,
                Flags::default(),
            );
            let a = format!("{:?}", a.unwrap_err());
            let b = format!("{:?}", b.unwrap_err());
            if a.contains("Arithmetic") {
                checked += 1;
                assert_eq!(
                    a, b,
                    "optimization must not displace the arithmetic error at cap {cap}"
                );
            }
            assert!(cold.iter().all(Option::is_none) && fast.iter().all(Option::is_none));
            drop((cold, fast));
            assert_eq!(memory.used_bytes(), retained);
        }
        assert!(checked > 0);
        let mut finite = mode.execution();
        finite.execution.max_work = 1_000_000_000;
        finite.library.max_compute_work = Some(1_000_000_000);
        finite.library.max_output_elements = Some(1024);
        finite.library.max_output_bytes = Some(1_048_576);
        finite.library.max_temporary_bytes = Some(1_048_576);
        finite.library.max_cloned_bytes = Some(1_048_576);
        finite.library.max_retained_nodes = Some(1_048_576);
        let mut output = vec![None; artifact.outputs().len()];
        let (result, counts) = run(
            &jit,
            &args,
            &mut output,
            &memory,
            finite,
            false,
            Flags::default(),
        );
        assert!(format!("{:?}", result.unwrap_err()).contains("Arithmetic"));
        assert_eq!((counts.selected, counts.complete), (1, 0));
        assert!(output.iter().all(Option::is_none));
        let valid = arguments(&artifact, &memory, 8, &[1, 2, 3]);
        let (result, counts) = run(
            &jit,
            &valid,
            &mut output,
            &memory,
            finite,
            false,
            Flags::default(),
        );
        result.unwrap();
        assert_eq!(
            (
                counts.selected + counts.word_selected,
                counts.complete + counts.word_complete
            ),
            (1, 1)
        );
        drop((valid, output));
        drop(args);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

pub(super) fn typed_arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    named: &[(&str, Data, &[u64])],
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (_, data, extents) = named.iter().find(|(n, _, _)| *n == name).unwrap();
            let shape_values = if extents.is_empty() {
                Box::new([]) as Box<[u64]>
            } else {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    extents,
                )
                .unwrap()
                .parameter_values()
                .into()
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data: data.clone(),
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

#[test]
fn numeric_maps_exact_scalar_kinds_keep_bits_and_never_call_per_element_helpers() {
    use mech_core::snapshot::F64Bits;
    let cases = [
        ("u8", vec![Data::U8(0), Data::U8(1), Data::U8(u8::MAX)]),
        ("u16", vec![Data::U16(0), Data::U16(1), Data::U16(u16::MAX)]),
        ("u32", vec![Data::U32(0), Data::U32(1), Data::U32(u32::MAX)]),
        ("u64", vec![Data::U64(0), Data::U64(1), Data::U64(u64::MAX)]),
        (
            "bool",
            vec![Data::Bool(true), Data::Bool(false), Data::Bool(true)],
        ),
        (
            "i64",
            vec![Data::I64(i64::MIN), Data::I64(-1), Data::I64(i64::MAX)],
        ),
        (
            "f64",
            [0x8000000000000000, 0x7ff8000000000037, 0x3ff8000000000000]
                .map(|bits| Data::F64(F64Bits::from_bits(bits)))
                .to_vec(),
        ),
    ];
    for (kind, cells) in cases {
        for by_range in [false, true] {
            let source = if by_range {
                format!("data := words<[{kind}]>\n[data[i] | i <- 1u64..=3u64]\n")
            } else {
                format!("[x | x <- words<[{kind}]>]\n")
            };
            for artifact in artifacts(&source) {
                let mode = NativeProgramLimitMode::NoLimits;
                let catalog = mech_stdlib::source_catalog();
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &catalog,
                    mode.compilation(),
                )
                .unwrap();
                assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
                let object =
                    crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                assert_eq!(
                    object.metadata()["compiled_numeric_map_regions"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                for values in [vec![], cells.clone()] {
                    if by_range && values.is_empty() {
                        continue;
                    }
                    let memory = mode.memory_budget();
                    let args = typed_arguments(
                        &artifact,
                        &memory,
                        &[(
                            "words",
                            Data::Matrix(values.clone().into()),
                            &[1, values.len() as u64],
                        )],
                    );
                    let aliases = args.clone();
                    let retained = memory.used_bytes();
                    for restored in [false, true] {
                        for ordinary in [false, true] {
                            let mut output = vec![None; artifact.outputs().len()];
                            let (result, counts) = run(
                                &jit,
                                &args,
                                &mut output,
                                &memory,
                                mode.execution(),
                                restored,
                                Flags {
                                    ordinary,
                                    ..Default::default()
                                },
                            );
                            result.unwrap();
                            assert_eq!(last(&output), Data::Matrix(values.clone().into()));
                            if !ordinary {
                                assert_eq!(
                                    (
                                        counts.selected + counts.word_selected,
                                        counts.complete + counts.word_complete
                                    ),
                                    (1, 1)
                                );
                            }
                            for region in &jit.descriptor().metadata().numeric_maps.regions {
                                if !ordinary {
                                    for site in &region.steps {
                                        assert!(
                                            !counts
                                                .helpers
                                                .keys()
                                                .any(|(at, _)| *at == u64::from(*site))
                                        );
                                    }
                                }
                            }
                            drop(output);
                            assert_eq!(memory.used_bytes(), retained);
                        }
                    }
                    assert_eq!(
                        aliases[0].canonical_data_draft().unwrap(),
                        Data::Matrix(values.into())
                    );
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
fn numeric_maps_range_matrix_reads_keep_axes_column_order_errors_and_finite_declines() {
    use mech_core::snapshot::F64Bits;
    let cells = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0]
        .map(|v| Data::F64(F64Bits::from_f64(v)))
        .to_vec();
    for two_axes in [false, true] {
        let selection = if two_axes {
            "data[selected-row,i]"
        } else {
            "data[i]"
        };
        let source = format!(
            "data := words<[f64]:2,3>\nfirst := begin<u64>\nlast := end<u64>\nselected-row := row<u64>\nshift := bias<f64>\n[{selection} + shift | i <- first..=last]\n"
        );
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let proof = &jit.descriptor().metadata().numeric_maps.regions;
            assert_eq!(proof.len(), 1, "{source}");
            assert!(proof[0].range);
            assert_eq!((proof[0].matrices.len(), proof[0].reads.len()), (1, 1));
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            for restored in [false, true] {
                for (begin, end, row) in [
                    (1, 3, 1),
                    (1, 3, 2),
                    (1, 4, 2),
                    (0, 1, 1),
                    (1, 1, 0),
                    (1, 1, 3),
                    (4, 3, 2),
                ] {
                    let memory = mode.memory_budget();
                    let args = typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("words", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("begin", Data::U64(begin), &[]),
                            ("end", Data::U64(end), &[]),
                            ("row", Data::U64(row), &[]),
                            ("bias", Data::F64(F64Bits::from_f64(0.5)), &[]),
                        ],
                    );
                    let retained = memory.used_bytes();
                    let mut cold = vec![None; artifact.outputs().len()];
                    let (a, _) = run(
                        &jit,
                        &args,
                        &mut cold,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    let mut fast = vec![None; artifact.outputs().len()];
                    let (b, counts) = run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags::default(),
                    );
                    match (a, b) {
                        (Ok(_), Ok(_)) => {
                            assert_eq!(last(&cold), last(&fast));
                            assert_eq!((counts.selected, counts.complete), (1, 1));
                            if begin == 1 && end == 3 {
                                let expected: Vec<f64> = if two_axes {
                                    if row == 1 {
                                        vec![10.5, 20.5, 30.5]
                                    } else {
                                        vec![40.5, 50.5, 60.5]
                                    }
                                } else {
                                    vec![10.5, 40.5, 20.5]
                                };
                                assert_eq!(
                                    last(&fast),
                                    Data::Matrix(
                                        expected
                                            .into_iter()
                                            .map(|v| Data::F64(F64Bits::from_f64(v)))
                                            .collect()
                                    )
                                );
                            }
                            for site in &proof[0].steps {
                                assert!(
                                    !counts.helpers.keys().any(|(at, _)| *at == u64::from(*site))
                                );
                            }
                        }
                        (Err(a), Err(b)) => {
                            assert_eq!(format!("{a:?}"), format!("{b:?}"));
                            assert!(fast.iter().all(Option::is_none));
                        }
                        pair => panic!("different read outcomes: {pair:?}"),
                    }
                    drop((cold, fast));
                    assert_eq!(memory.used_bytes(), retained);
                    if begin == 1 && end == 3 && row == 1 {
                        for dimension in 0..8 {
                            for cap in [0, 1, 16, 255, 4095, 65535] {
                                let mut policy = mode.execution();
                                match dimension {
                                    0 => policy.library.max_output_elements = Some(cap),
                                    1 => policy.library.max_output_bytes = Some(cap),
                                    2 => policy.library.max_temporary_bytes = Some(cap),
                                    3 => policy.library.max_cloned_bytes = Some(cap),
                                    4 => policy.library.max_retained_nodes = Some(cap),
                                    5 => policy.library.max_compute_work = Some(cap),
                                    6 => policy.library.max_comparison_work = Some(cap),
                                    _ => policy.execution.max_work = cap,
                                }
                                let mut cold = vec![None; artifact.outputs().len()];
                                let mut fast = cold.clone();
                                let (a, _) = run(
                                    &jit,
                                    &args,
                                    &mut cold,
                                    &memory,
                                    policy,
                                    restored,
                                    Flags {
                                        ordinary: true,
                                        ..Default::default()
                                    },
                                );
                                let (b, _) = run(
                                    &jit,
                                    &args,
                                    &mut fast,
                                    &memory,
                                    policy,
                                    restored,
                                    Flags::default(),
                                );
                                match (a, b) {
                                    (Ok(_), Ok(_)) => assert_eq!(last(&cold), last(&fast)),
                                    (Err(a), Err(b)) => {
                                        assert_eq!(
                                            format!("{a:?}"),
                                            format!("{b:?}"),
                                            "dimension {dimension} cap {cap}"
                                        );
                                        assert!(fast.iter().all(Option::is_none));
                                    }
                                    pair => panic!(
                                        "different finite outcomes dimension {dimension} cap {cap}: {pair:?}"
                                    ),
                                }
                                drop((cold, fast));
                                assert_eq!(memory.used_bytes(), retained);
                            }
                        }
                    }
                    drop(args);
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
fn numeric_maps_range_read_loans_reject_changed_geometry_and_stale_matrix_owners() {
    use mech_core::snapshot::F64Bits;
    let source = "data := words<[f64]:2,3>\n[data[i] | i <- 1u64..=6u64]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = typed_arguments(
                &artifact,
                &memory,
                &[(
                    "words",
                    Data::Matrix(
                        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
                            .into_iter()
                            .map(|v| Data::F64(F64Bits::from_f64(v)))
                            .collect(),
                    ),
                    &[2, 3],
                )],
            );
            let retained = memory.used_bytes();
            for corrupt in 13..=16 {
                let mut output = vec![None; artifact.outputs().len()];
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert_eq!(counts.selected, 1);
                assert!(result.is_err(), "corruption {corrupt}");
                assert!(output.iter().all(Option::is_none));
                assert_eq!(memory.used_bytes(), retained);
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut output,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        interleave: true,
                        ..Default::default()
                    },
                );
                result.unwrap();
                assert_eq!(
                    (counts.selected, counts.complete, counts.interleaved_refused),
                    (1, 1, 2)
                );
                drop(output);
                assert_eq!(memory.used_bytes(), retained);
            }
            drop(args);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_maps_real_numerical_comprehensions_select_and_match_complete_c_liveouts() {
    use mech_core::snapshot::F64Bits;
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/polybench");
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixtures.join("cases.json")).unwrap())
            .unwrap();
    fn flatten(data: Data, out: &mut Vec<f64>) {
        match data {
            Data::F64(v) => out.push(v.to_f64()),
            Data::Matrix(v) | Data::Tuple(v) => {
                for item in v {
                    flatten(item, out)
                }
            }
            _ => panic!("unexpected numerical output"),
        }
    }
    for name in ["gramschmidt", "cholesky", "deriche", "durbin"] {
        let case = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap();
        let named: Vec<_> = case["inputs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| {
                let (data, extents) = if let Some(cells) = value["values"].as_array() {
                    (
                        Data::Matrix(
                            cells
                                .iter()
                                .map(|v| Data::F64(F64Bits::from_f64(v.as_f64().unwrap())))
                                .collect(),
                        ),
                        value["shape"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_u64().unwrap())
                            .collect::<Vec<_>>(),
                    )
                } else if let Some(n) = value["u64"].as_u64() {
                    (Data::U64(n), vec![])
                } else {
                    (
                        Data::F64(F64Bits::from_f64(value.as_f64().unwrap())),
                        vec![],
                    )
                };
                (name.as_str(), data, extents)
            })
            .collect();
        let source = std::fs::read_to_string(fixtures.join(format!("{name}.mec"))).unwrap();
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert!(
                !jit.descriptor().metadata().numeric_maps.regions.is_empty(),
                "{name}"
            );
            let memory = mode.memory_budget();
            let borrowed = named
                .iter()
                .map(|(n, d, e)| (*n, d.clone(), &e[..]))
                .collect::<Vec<_>>();
            let args = typed_arguments(&artifact, &memory, &borrowed);
            let retained = memory.used_bytes();
            for restored in [false, true] {
                let mut fast = vec![None; artifact.outputs().len()];
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                result.unwrap();
                assert!(
                    counts.selected > 0 && counts.selected == counts.complete,
                    "{name}: {counts:?}"
                );
                let folded: Vec<_> = jit
                    .descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .filter(|region| {
                        region.reduction.is_some()
                            && counts.elements.contains_key(&u64::from(region.header))
                    })
                    .collect();
                if name != "deriche" {
                    assert!(
                        !folded.is_empty(),
                        "actual numerical source selects scalar folds: {name}"
                    );
                }
                if name != "cholesky" {
                    assert!(
                        counts.transpose_complete > 0,
                        "actual numerical source directly publishes transposed maps: {name} {counts:?}"
                    );
                }
                for region in folded {
                    let reduction = region.reduction.as_ref().unwrap();
                    assert!(
                        counts
                            .helpers
                            .get(&(
                                u64::from(reduction.finish),
                                GlobalHelper::NumericReductionFinish.raw()
                            ))
                            .copied()
                            .unwrap_or(0)
                            > 0
                    );
                    for site in [reduction.sum, reduction.select] {
                        assert_eq!(
                            counts
                                .helpers
                                .get(&(u64::from(site), GlobalHelper::Library.raw()))
                                .copied()
                                .unwrap_or(0),
                            0,
                            "native source reduction {name} {site}"
                        );
                    }
                }
                if name == "gramschmidt" {
                    let conditional = jit.descriptor().metadata().numeric_maps.regions.iter().filter(|region|
                        region.body.iter().any(|&site| matches!(jit.descriptor().plan().blocks()[site as usize].terminator(),
                            mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                                mech_engine::native_artifact::NativeMatchTerminator::Branch { .. }
                                | mech_engine::native_artifact::NativeMatchTerminator::LiteralBranch { .. })))).collect::<Vec<_>>();
                    assert!(
                        !conditional.is_empty(),
                        "qualify actual conditional matrix updates"
                    );
                    assert!(
                        conditional.iter().all(|region| {
                            counts.elements.contains_key(&u64::from(region.header))
                                || jit.descriptor().metadata().numeric_maps.regions.iter().any(
                                    |parent| {
                                        parent.rectangle.as_ref().is_some_and(|rectangle| {
                                            rectangle.inner_iteration == region.iteration
                                        }) && parent.body == region.body
                                            && parent.steps == region.steps
                                            && parent.reads == region.reads
                                            && counts
                                                .elements
                                                .contains_key(&u64::from(parent.header))
                                    },
                                )
                        }),
                        "every conditional body must execute directly or inside its exact completed native rectangle"
                    );
                }

                let mut values = Vec::new();
                flatten(last(&fast), &mut values);
                let expected = case["expected"].as_array().unwrap();
                assert_eq!(values.len(), expected.len());
                for (actual, expected) in values.iter().zip(expected) {
                    let expected = expected.as_f64().unwrap();
                    assert!(
                        actual.is_finite()
                            && (actual - expected).abs() <= 1e-9 + 1e-10 * expected.abs()
                    );
                }
                drop(fast);
                assert_eq!(memory.used_bytes(), retained);
            }
            drop(args);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

fn assert_private_body(
    proof: &crate::native_artifact::program::prepare::numeric_maps::Region,
    counts: &Counts,
) {
    assert_eq!((counts.selected, counts.complete), (1, 1));
    for &site in &proof.body {
        assert!(
            !counts.helpers.keys().any(|(at, _)| *at == u64::from(site)),
            "per-element helper at {site}"
        );
    }
}

#[test]
fn conditional_numeric_maps_keep_phi_words_literal_float_equality_and_all_exact_kinds() {
    use mech_core::snapshot::F64Bits;
    let cases = [
        ("u8", vec![Data::U8(0), Data::U8(1), Data::U8(u8::MAX)]),
        ("u16", vec![Data::U16(0), Data::U16(1), Data::U16(u16::MAX)]),
        ("u32", vec![Data::U32(0), Data::U32(1), Data::U32(u32::MAX)]),
        ("u64", vec![Data::U64(0), Data::U64(1), Data::U64(u64::MAX)]),
        (
            "i64",
            vec![Data::I64(i64::MIN), Data::I64(-1), Data::I64(i64::MAX)],
        ),
        (
            "bool",
            vec![Data::Bool(true), Data::Bool(false), Data::Bool(true)],
        ),
        (
            "f64",
            [
                0x8000_0000_0000_0000,
                0x7ff8_0000_0000_0037,
                0x3ff8_0000_0000_0000,
            ]
            .map(|bits| Data::F64(F64Bits::from_bits(bits)))
            .to_vec(),
        ),
    ];
    for (kind, cells) in cases {
        let source = format!(
            "enabled := gate<bool>\nother := alternative<{kind}>\n[(enabled? | true => value | false => other) | value <- words<[{kind}]>]\n"
        );
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert_eq!(
                jit.descriptor().metadata().numeric_maps.regions.len(),
                1,
                "{source}"
            );
            let proof = &jit.descriptor().metadata().numeric_maps.regions[0];
            assert!(proof.body.iter().any(|&site| matches!(
                jit.descriptor().plan().blocks()[site as usize].terminator(),
                mech_engine::native_artifact::NativeIterationTerminator::Scalar(
                    mech_engine::native_artifact::NativeMatchTerminator::LiteralBranch { .. }
                        | mech_engine::native_artifact::NativeMatchTerminator::Branch { .. }
                )
            )));
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
            for restored in [false, true] {
                for gate in [true, false] {
                    let memory = mode.memory_budget();
                    let args = typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("gate", Data::Bool(gate), &[]),
                            ("alternative", cells[1].clone(), &[]),
                            ("words", Data::Matrix(cells.clone().into()), &[1, 3]),
                        ],
                    );
                    let retained = memory.used_bytes();
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, counts) = run(
                        &jit,
                        &args,
                        &mut output,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            numeric_only: true,
                            ..Default::default()
                        },
                    );
                    result.unwrap();
                    assert_private_body(proof, &counts);
                    let expected = if gate {
                        cells.clone()
                    } else {
                        vec![cells[1].clone(); 3]
                    };
                    assert_eq!(last(&output), Data::Matrix(expected.into()));
                    drop(output);
                    assert_eq!(memory.used_bytes(), retained);
                    drop(args);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
    let source = "[(value? | 0.0 => 7.0 | found => found) | value <- words<[f64]>]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
        let cells = [
            0x8000_0000_0000_0000,
            0,
            0x7ff8_0000_0000_0037,
            0x7ff0_0000_0000_0000,
            0x3ff0_0000_0000_0000,
        ]
        .map(|bits| Data::F64(F64Bits::from_bits(bits)));
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = typed_arguments(
                &artifact,
                &memory,
                &[("words", Data::Matrix(cells.to_vec().into()), &[1, 5])],
            );
            let mut output = vec![None; artifact.outputs().len()];
            let (result, counts) = run(
                &jit,
                &args,
                &mut output,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            );
            result.unwrap();
            assert_private_body(
                &jit.descriptor().metadata().numeric_maps.regions[0],
                &counts,
            );
            let mut expected = cells.to_vec();
            expected[..2].fill(Data::F64(F64Bits::from_f64(7.0)));
            assert_eq!(last(&output), Data::Matrix(expected.into()));
            drop((args, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn conditional_numeric_maps_do_not_speculate_inactive_reads_and_keep_finite_error_order() {
    use mech_core::snapshot::F64Bits;
    let source = "data := words<[f64]:2,3>\nenabled := gate<bool>\nbad := row<u64>\n[((enabled? | true => data[bad,i] | false => data[1u64,i])) | i <- 1u64..=3u64]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
        let proof = &jit.descriptor().metadata().numeric_maps.regions[0];
        for restored in [false, true] {
            for (gate, row) in [(false, 0), (false, 3), (true, 0), (true, 2), (true, 3)] {
                let memory = mode.memory_budget();
                let args = typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "words",
                            Data::Matrix(
                                [1., 2., 3., 4., 5., 6.]
                                    .into_iter()
                                    .map(|v| Data::F64(F64Bits::from_f64(v)))
                                    .collect(),
                            ),
                            &[2, 3],
                        ),
                        ("gate", Data::Bool(gate), &[]),
                        ("row", Data::U64(row), &[]),
                    ],
                );
                let retained = memory.used_bytes();
                let mut cold = vec![None; artifact.outputs().len()];
                let mut fast = cold.clone();
                let (a, _) = run(
                    &jit,
                    &args,
                    &mut cold,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                let (b, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                match (a, b) {
                    (Ok(_), Ok(_)) => {
                        assert_private_body(proof, &counts);
                        assert_eq!(last(&cold), last(&fast));
                    }
                    (Err(a), Err(b)) => {
                        assert_eq!(counts.selected, 1);
                        assert_eq!(format!("{a:?}"), format!("{b:?}"));
                        assert!(fast.iter().all(Option::is_none));
                    }
                    pair => panic!("different conditional read outcomes: {pair:?}"),
                }
                drop((cold, fast));
                assert_eq!(memory.used_bytes(), retained);
                if !gate && row == 0 {
                    for dim in 0..8 {
                        for cap in [0, 1, 16, 255, 4095, 65535] {
                            let mut policy = mode.execution();
                            match dim {
                                0 => policy.library.max_output_elements = Some(cap),
                                1 => policy.library.max_output_bytes = Some(cap),
                                2 => policy.library.max_temporary_bytes = Some(cap),
                                3 => policy.library.max_cloned_bytes = Some(cap),
                                4 => policy.library.max_retained_nodes = Some(cap),
                                5 => policy.library.max_compute_work = Some(cap),
                                6 => policy.library.max_comparison_work = Some(cap),
                                _ => policy.execution.max_work = cap,
                            }
                            let mut cold = vec![None; artifact.outputs().len()];
                            let mut fast = cold.clone();
                            let (a, _) = run(
                                &jit,
                                &args,
                                &mut cold,
                                &memory,
                                policy,
                                restored,
                                Flags {
                                    ordinary: true,
                                    ..Default::default()
                                },
                            );
                            let (b, _) = run(
                                &jit,
                                &args,
                                &mut fast,
                                &memory,
                                policy,
                                restored,
                                Flags::default(),
                            );
                            match (a, b) {
                                (Ok(_), Ok(_)) => assert_eq!(last(&cold), last(&fast)),
                                (Err(a), Err(b)) => {
                                    assert_eq!(
                                        format!("{a:?}"),
                                        format!("{b:?}"),
                                        "dimension {dim} cap {cap}"
                                    );
                                    assert!(fast.iter().all(Option::is_none));
                                }
                                pair => {
                                    panic!("different finite branch outcomes {dim} {cap}: {pair:?}")
                                }
                            }
                            drop((cold, fast));
                            assert_eq!(memory.used_bytes(), retained);
                        }
                    }
                }
                drop(args);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn conditional_numeric_maps_discard_partial_outputs_and_replay_original_arithmetic_errors() {
    let source = "denom := divisor<u64>\n[((value == 0u64)? | true => 9u64 | false => ((value == 1u64)? | true => 100u64 / denom | false => value + 1u64)) | value <- words<[u64]>]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
        for restored in [false, true] {
            for (words, divisor) in [
                (vec![0, 2, 1, 4], 0),
                (vec![0, 2, 1, 4], 2),
                (vec![u64::MAX, 1], 0),
                (vec![2, 0, 4, 0, 2], 0),
            ] {
                let memory = mode.memory_budget();
                let args = typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "words",
                            Data::Matrix(words.iter().copied().map(Data::U64).collect()),
                            &[1, words.len() as u64],
                        ),
                        ("divisor", Data::U64(divisor), &[]),
                    ],
                );
                let mut cold = vec![None; artifact.outputs().len()];
                let mut fast = cold.clone();
                let (a, _) = run(
                    &jit,
                    &args,
                    &mut cold,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                let (b, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        numeric_only: true,
                        ..Default::default()
                    },
                );
                assert_eq!(counts.selected, 1);
                match (a, b) {
                    (Ok(_), Ok(_)) => {
                        assert_private_body(
                            &jit.descriptor().metadata().numeric_maps.regions[0],
                            &counts,
                        );
                        assert_eq!(last(&cold), last(&fast));
                    }
                    (Err(a), Err(b)) => {
                        assert_eq!(format!("{a:?}"), format!("{b:?}"));
                        assert!(fast.iter().all(Option::is_none));
                    }
                    pair => panic!("different branch arithmetic outcomes: {pair:?}"),
                }
                drop((args, cold, fast));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn conditional_numeric_maps_snapshot_current_scalar_and_matrix_phi_captures_once() {
    use mech_core::snapshot::F64Bits;
    let source = "first := left<[f64]:2,3>\nsecond := right<[f64]:2,3>\npicked := (which<bool>? | true => first | false => second)\nextra := (gate<bool>? | true => 1.0 | false => 2.0)\n[picked[1u64,i] + extra | i <- 1u64..=3u64]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(jit.descriptor().metadata().numeric_maps.regions.len(), 1);
        for restored in [false, true] {
            for which in [false, true] {
                for gate in [false, true] {
                    let memory = mode.memory_budget();
                    let cells = |base| {
                        Data::Matrix(
                            (1..=6)
                                .map(|v| Data::F64(F64Bits::from_f64(f64::from(v) + base)))
                                .collect(),
                        )
                    };
                    let args = typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("left", cells(0.), &[2, 3]),
                            ("right", cells(10.), &[2, 3]),
                            ("which", Data::Bool(which), &[]),
                            ("gate", Data::Bool(gate), &[]),
                        ],
                    );
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, counts) = run(
                        &jit,
                        &args,
                        &mut output,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags::default(),
                    );
                    result.unwrap();
                    assert_private_body(
                        &jit.descriptor().metadata().numeric_maps.regions[0],
                        &counts,
                    );
                    let base = if which { 0. } else { 10. };
                    let extra = if gate { 1. } else { 2. };
                    assert_eq!(
                        last(&output),
                        Data::Matrix(
                            (1..=3)
                                .map(|n| Data::F64(F64Bits::from_f64(f64::from(n) + base + extra)))
                                .collect()
                        )
                    );
                    drop((args, output));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
