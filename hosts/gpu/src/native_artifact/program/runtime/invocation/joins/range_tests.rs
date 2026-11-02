//! Observe the actual emitted visits and inject private protocol/limit faults.
//! No replacement executor, fake selector Value or alternate geometry leaf.
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program::runtime::invocation::joins::*;
use crate::native_artifact::program::runtime::tests::{artifact_source, limits};
use mech_core::{ManagedMemoryBudget, MemoryBudgetLimits};
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug)]
struct Observation {
    range: bool,
    compact: bool,
    selector_empty: bool,
    selected_empty: bool,
    parent: Option<u32>,
    persistent_metadata: u64,
}
#[derive(Clone, Copy, Debug)]
struct Demand {
    temporary: u64,
    nodes: u64,
    compute: u64,
    peak_metadata: u64,
}
#[derive(Clone, Copy, Debug)]
enum Fault {
    BadRangeWords,
    ReplayRange,
    MissingRange,
    WrongRegion,
    WrongParent,
    JoinBeforeSelect,
    MissingSelection,
    ConsumeTwice,
}
thread_local! {
    static OBSERVATIONS: RefCell<Vec<Observation>> = const { RefCell::new(Vec::new()) };
    static DEMAND: Cell<Option<Demand>> = const { Cell::new(None) };
    static FAULT: Cell<Option<Fault>> = const { Cell::new(None) };
    static FIRED: Cell<bool> = const { Cell::new(false) };
    static CAP: Cell<Option<(Dimension, u64)>> = const { Cell::new(None) };
    static ORIGINAL_LIMITS: Cell<Option<MemoryBudgetLimits>> = const { Cell::new(None) };
}

fn observe(ctx: &Invocation<'_, '_, '_>, index: usize, range: bool, compact: bool) {
    let region = &ctx.descriptor.metadata().selected_joins.regions[index];
    if region.range().is_none() {
        return;
    }
    OBSERVATIONS.with(|items| {
        items.borrow_mut().push(Observation {
            range,
            compact,
            selector_empty: ctx
                .state
                .arena
                .test_current_value(region.selector().get())
                .is_none(),
            selected_empty: ctx
                .state
                .arena
                .test_current_value(region.selected().get())
                .is_none(),
            parent: ctx.state.control.current,
            persistent_metadata: ctx
                .selected_join
                .as_ref()
                .unwrap()
                .reservation
                .capacity_bytes(),
        })
    });
}
pub(in crate::native_artifact::program::runtime::invocation::joins) fn after_range(
    ctx: &mut Invocation<'_, '_, '_>,
    index: usize,
) {
    observe(ctx, index, true, true);
    if let Some(limits) = ORIGINAL_LIMITS.with(Cell::take) {
        ctx.limits = limits;
    }
}
pub(in crate::native_artifact::program::runtime::invocation::joins) fn after_selection(
    ctx: &Invocation<'_, '_, '_>,
    index: usize,
    compact: bool,
) {
    observe(ctx, index, false, compact);
}
pub(in crate::native_artifact::program::runtime::invocation::joins) fn before_range_admission(
    ctx: &mut Invocation<'_, '_, '_>,
    _site: usize,
    prior: ResourceDemand,
    metadata: u64,
    price: u64,
) {
    DEMAND.with(|slot| {
        slot.set(Some(Demand {
            temporary: prior.turn_peak_bytes,
            nodes: prior.retained_nodes,
            compute: price,
            peak_metadata: metadata,
        }))
    });
    // This tests the actual original Range's local admission boundary, not a
    // claim that this small cap admits the earlier text/lines producer too.
    if let Some((dimension, value)) = CAP.with(Cell::get) {
        ORIGINAL_LIMITS.with(|slot| slot.set(Some(ctx.limits)));
        match dimension {
            Dimension::TemporaryBytes => ctx.limits.max_temporary_bytes = Some(value),
            Dimension::RetainedNodes => ctx.limits.max_retained_nodes = Some(value),
            Dimension::ComputeWork => ctx.limits.max_compute_work = Some(value),
            _ => panic!("unsupported range boundary"),
        }
    }
}
pub(in crate::native_artifact::program::runtime::invocation::joins) fn before_call(
    ctx: &mut Invocation<'_, '_, '_>,
    site: usize,
    role: Site,
    words: &[u64],
) -> Result<(), Error> {
    let Some(fault) = FAULT.with(Cell::get) else {
        return Ok(());
    };
    if FIRED.with(Cell::get) {
        return Ok(());
    }
    let target = match fault {
        Fault::BadRangeWords => matches!(role, Site::Range(_)),
        Fault::MissingSelection | Fault::ConsumeTwice => matches!(role, Site::Join(_)),
        _ => matches!(role, Site::Select(_)),
    };
    if !target {
        return Ok(());
    }
    let index = match role {
        Site::Range(i) | Site::Select(i) | Site::Join(i) => i,
    };
    let region = &ctx.descriptor.metadata().selected_joins.regions[index];
    let Some(range) = region.range() else {
        return Ok(());
    };
    let producer = range.producer() as usize;
    let consumer = region.consumer() as usize;
    FIRED.with(|slot| slot.set(true));
    match fault {
        Fault::BadRangeWords => {
            ctx.selected_join_call(site, &words[..1])?;
        }
        Fault::ReplayRange => {
            ctx.selected_join_call(producer, &[1, 2])?;
        }
        Fault::MissingRange | Fault::MissingSelection => ctx.selected_join = None,
        Fault::WrongRegion => {
            ctx.selected_join.as_mut().unwrap().region = usize::MAX;
        }
        Fault::WrongParent => {
            ctx.selected_join.as_mut().unwrap().parent = Some(u32::MAX);
        }
        Fault::JoinBeforeSelect => {
            ctx.selected_join_call(consumer, &[0, 0])?;
        }
        Fault::ConsumeTwice => {
            ctx.selected_join_call(site, &[0, 0])?;
        }
    }
    Ok(())
}

fn variants(lexical: bool, inclusive: bool) -> [mech_engine::ProgramArtifact; 2] {
    let operator = if inclusive { "..=" } else { ".." };
    let inner = format!("text/join(lines[first{operator}last], \"|\")");
    let body = if lexical {
        format!("text/join([{inner} | attempt <- 1u64..=2u64], \"/\")")
    } else {
        inner
    };
    let source = format!(
        "+> text\nlines := text/lines(input<string>)\nfirst := begin<u64>\nlast := stop<u64>\n{body}\n"
    );
    let original = artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn inputs(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    text: &str,
    first: u64,
    last: u64,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "input" => ValueDataDraft::String(text.into()),
                "begin" => ValueDataDraft::U64(first),
                "stop" => ValueDataDraft::U64(last),
                _ => panic!("unexpected input {name}"),
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
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
fn observations() -> Vec<Observation> {
    OBSERVATIONS.with(|items| std::mem::take(&mut *items.borrow_mut()))
}
fn assert_preserved(outputs: &[Option<Value>], prior: &[Option<Value>]) {
    for (actual, old) in outputs.iter().zip(prior) {
        assert!(
            actual
                .as_ref()
                .unwrap()
                .shares_frozen_storage(old.as_ref().unwrap())
        );
    }
}

#[test]
fn actual_compact_range_root_lexical_and_ordinary_unicode_fallback_keep_empty_private_slots() {
    for lexical in [false, true] {
        for inclusive in [false, true] {
            for artifact in variants(lexical, inclusive) {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                assert!(
                    jit.descriptor()
                        .metadata()
                        .selected_joins
                        .regions
                        .iter()
                        .any(|r| r.range().is_some())
                );
                assert_eq!(jit.descriptor().metadata().line_matrices.regions.len(), 1);
                let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
                for text in ["a\nb\nc\nd", "a\nb\nc\né"] {
                    let args = inputs(&artifact, &memory, text, 2, if inclusive { 3 } else { 4 });
                    let mut outputs = vec![None; artifact.outputs().len()];
                    observations();
                    jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                        .unwrap();
                    let expected = if lexical { "b|c/b|c" } else { "b|c" };
                    assert!(matches!(outputs.last().unwrap().as_ref().unwrap().data(),
                        ValueData::String(actual) if actual.as_ref() == expected));
                    let seen = observations();
                    let visits = if lexical { 2 } else { 1 };
                    assert_eq!(
                        seen.iter().filter(|r| r.range).count(),
                        if text.is_ascii() { visits } else { 0 }
                    );
                    assert_eq!(seen.iter().filter(|r| !r.range).count(), visits);
                    for entry in seen {
                        assert_eq!(entry.compact, text.is_ascii(), "{entry:?}");
                        assert_eq!(entry.selector_empty, text.is_ascii(), "{entry:?}");
                        assert!(entry.selected_empty, "{entry:?}");
                        assert_eq!(entry.parent.is_some(), lexical, "{entry:?}");
                        assert_eq!(
                            entry.persistent_metadata,
                            size_of::<State>() as u64,
                            "only live State metadata survives each original call: {entry:?}"
                        );
                    }
                    drop((args, outputs));
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}

#[test]
fn actual_compact_range_errors_keep_original_range_or_selection_origin_and_recover() {
    for inclusive in [false, true] {
        for artifact in variants(false, inclusive) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let region = jit
                .descriptor()
                .metadata()
                .selected_joins
                .regions
                .iter()
                .find(|r| r.range().is_some())
                .unwrap();
            let range_origin = jit.descriptor().metadata().plan.blocks()
                [region.range().unwrap().producer() as usize]
                .origin();
            let selection_origin =
                jit.descriptor().metadata().plan.blocks()[region.selection() as usize].origin();
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let good = inputs(&artifact, &memory, "a\nb\nc\nd", 1, 3);
            let mut outputs = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&good, &mut outputs, limits(), &memory)
                .unwrap();
            for (first, last, range_error) in [(3, 2, true), (0, 2, false), (1, 9, false)] {
                let mut errors = Vec::new();
                for text in ["a\nb\nc\nd", "a\nb\nc\né"] {
                    let args = inputs(&artifact, &memory, text, first, last);
                    let prior = outputs.clone();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut outputs, limits(), &memory)
                        .unwrap_err();
                    assert!(
                        matches!(&error, Error::Library { origin, .. }
                        if *origin == if range_error { range_origin } else { selection_origin }),
                        "{error:?}"
                    );
                    errors.push(format!("{error:?}"));
                    assert_preserved(&outputs, &prior);
                    assert_eq!(memory.used_bytes(), before);
                }
                assert_eq!(
                    errors[0], errors[1],
                    "compact and genuine canonical fallback"
                );
                jit.evaluate_into(&good, &mut outputs, limits(), &memory)
                    .unwrap();
            }
            observations();
            drop((good, outputs));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn actual_compact_range_local_metadata_limits_are_exact_and_cumulative_account_recovers() {
    for artifact in variants(false, true) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let range = jit
            .descriptor()
            .metadata()
            .selected_joins
            .regions
            .iter()
            .find_map(|r| r.range())
            .unwrap();
        let origin = jit.descriptor().metadata().plan.blocks()[range.producer() as usize].origin();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let args = inputs(&artifact, &memory, "a\nb\nc\nd", 1, 3);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let demand = DEMAND.with(Cell::get).unwrap();
        assert!(
            demand.peak_metadata > size_of::<State>() as u64,
            "Range peak includes its non-retained shape witness"
        );
        for (dimension, required) in [
            (Dimension::TemporaryBytes, demand.temporary),
            (Dimension::RetainedNodes, demand.nodes),
            (Dimension::ComputeWork, demand.compute),
        ] {
            CAP.with(|slot| slot.set(Some((dimension, required))));
            let result = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            CAP.with(|slot| slot.set(None));
            ORIGINAL_LIMITS.with(|slot| slot.set(None));
            result.unwrap();
            let prior = outputs.clone();
            let before = memory.used_bytes();
            CAP.with(|slot| slot.set(Some((dimension, required - 1))));
            let result = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            CAP.with(|slot| slot.set(None));
            ORIGINAL_LIMITS.with(|slot| slot.set(None));
            let error = result.unwrap_err();
            assert!(
                matches!(error, Error::Library { origin: actual, .. } if actual == origin),
                "{error:?}"
            );
            assert_preserved(&outputs, &prior);
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
        }
        observations();
        drop((args, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_compact_range_missing_replayed_and_wrong_owner_phases_fail_closed_atomically() {
    for artifact in variants(false, true) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let args = inputs(&artifact, &memory, "a\nb\nc\nd", 1, 3);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        for fault in [
            Fault::BadRangeWords,
            Fault::ReplayRange,
            Fault::MissingRange,
            Fault::WrongRegion,
            Fault::WrongParent,
            Fault::JoinBeforeSelect,
            Fault::MissingSelection,
            Fault::ConsumeTwice,
        ] {
            let prior = outputs.clone();
            let before = memory.used_bytes();
            FAULT.with(|slot| slot.set(Some(fault)));
            FIRED.with(|slot| slot.set(false));
            let result = jit.evaluate_into(&args, &mut outputs, limits(), &memory);
            FAULT.with(|slot| slot.set(None));
            assert!(FIRED.with(Cell::get), "actual target visit: {fault:?}");
            let error = result.unwrap_err();
            assert!(
                matches!(
                    error,
                    Error::Base(crate::native_artifact::NativeExecutionError::Descriptor { .. })
                ),
                "{fault:?}: {error:?}"
            );
            assert_preserved(&outputs, &prior);
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
        }
        observations();
        FIRED.with(|slot| slot.set(false));
        drop((args, outputs));
        assert_eq!(memory.used_bytes(), 0);
    }
}
