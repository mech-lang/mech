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
    let status = unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0 {
        let value = unsafe { *out };
        if op == GlobalHelper::NumericMapPrepare.raw() && value != 0 {
            COUNTS.with(|v| {
                let mut counts = v.borrow_mut();
                counts.selected += 1;
                counts.elements.insert(site, unsafe {
                    (*(value as *const NativeNumericMapFrame)).count
                });
            });
        }
        if op == GlobalHelper::NumericMapFinish.raw() && value == 1 {
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
                                assert_eq!((counts.selected, counts.complete), (1, 1));
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
        assert_eq!((counts.selected, counts.complete), (1, 1));
        drop((valid, output));
        drop(args);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
