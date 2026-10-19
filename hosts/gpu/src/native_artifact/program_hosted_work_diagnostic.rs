//! Explicit ignored whole-source hosted diagnostic, with no expected failure.
use super::{Callback, Context, NativeProgramExecutionError};
use crate::native_artifact::{
    JitProgramArtifact, NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase,
    NativeProgramExecutionLimits, NativeProgramExecutionReport,
};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{NativeIterationAction, NativeMatchAction, NativeMatchOrigin};
use mech_runtime::CanonicalNativeResourceRunError;
use std::cell::Cell;

#[path = "../../tests/support/canonical_hosted_fixture.rs"]
mod fixture;

const FULL: &str = include_str!("../../tests/fixtures/native/knucleotide-idiomatic.mec");
#[path = "program_hosted_work_totals.rs"]
mod totals;
pub(in super::super) use totals::{begin_callback, checkpoint, end_callback};

#[derive(Clone, Copy, Debug)]
enum ActionTag {
    None,
    Copy,
    Primitive,
    Structural,
    Library,
    RangeSource,
    SpanSelector,
    SpanProjection,
    StringMapProjection,
    CollectionBegin,
    CollectionElement,
    CollectionPush,
    CollectionFinish,
    TextJoinFinish,
    HistogramQuery,
    HistogramFinish,
    Initialize,
    Latch,
}

fn action_tag(action: &NativeIterationAction) -> ActionTag {
    match action {
        NativeIterationAction::Scalar(NativeMatchAction::None) => ActionTag::None,
        NativeIterationAction::Scalar(NativeMatchAction::Copy { .. }) => ActionTag::Copy,
        NativeIterationAction::Scalar(NativeMatchAction::Primitive { .. }) => ActionTag::Primitive,
        NativeIterationAction::Structural { .. } => ActionTag::Structural,
        NativeIterationAction::LibraryCall { .. } => ActionTag::Library,
        NativeIterationAction::PrepareU64Range { .. } => ActionTag::RangeSource,
        NativeIterationAction::PrepareStringSpanSelector { .. } => ActionTag::SpanSelector,
        NativeIterationAction::ProjectStringSpan { .. } => ActionTag::SpanProjection,
        NativeIterationAction::ProjectStringMapEntry { .. } => ActionTag::StringMapProjection,
        NativeIterationAction::CollectionBegin { .. } => ActionTag::CollectionBegin,
        NativeIterationAction::CollectionElement { .. } => ActionTag::CollectionElement,
        NativeIterationAction::CollectionPush { .. } => ActionTag::CollectionPush,
        NativeIterationAction::CollectionFinish { .. } => ActionTag::CollectionFinish,
        NativeIterationAction::TextJoinFinish { .. } => ActionTag::TextJoinFinish,
        NativeIterationAction::HistogramQueryStep { .. } => ActionTag::HistogramQuery,
        NativeIterationAction::HistogramFinish { .. } => ActionTag::HistogramFinish,
        NativeIterationAction::Initialize { .. } => ActionTag::Initialize,
        NativeIterationAction::Latch { .. } => ActionTag::Latch,
    }
}

#[derive(Clone, Copy, Debug)]
enum ErrorTag {
    BaseWork,
    BaseOther,
    Control,
    Value,
    Library,
    Collection,
    Snapshot,
}

#[derive(Clone, Copy)]
struct Failure {
    callback: Callback,
    site: u64,
    origin: Option<NativeMatchOrigin>,
    action: Option<ActionTag>,
    error: ErrorTag,
    work_phase: Option<NativeExecutionPhase>,
    native_used_after: u64,
    required: Option<u64>,
    limit: Option<u64>,
    exact_debit: Option<u64>,
    selected_binding_work: Option<u64>,
    selected_selector_work: Option<u64>,
    scalar_bytes: u64,
}

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static FAILURE: Cell<Option<Failure>> = const { Cell::new(None) };
}

// Called only in the production callback's existing Err branch. All reads
// inspect fixed immutable headers/scalars after the real helper returned.
// No allocation, owner clone, formatting, assertion or unwind crosses C.
pub(in super::super) fn record(
    context: &Context<'_, '_>,
    site: u64,
    callback: Callback,
    error: &NativeProgramExecutionError,
) {
    if !ENABLED.with(Cell::get) || FAILURE.with(Cell::get).is_some() {
        return;
    }
    let descriptor = context.control.descriptor;
    let index = usize::try_from(site).ok();
    let block = index.and_then(|index| descriptor.plan.blocks().get(index));
    let selected = index
        .and_then(|index| descriptor.calls.get(index))
        .and_then(Option::as_ref);
    let used = context.control.work.used();
    let (tag, phase, required, limit) = match error {
        NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit {
            phase,
            required,
            limit,
        }) => (
            ErrorTag::BaseWork,
            Some(*phase),
            Some(*required),
            Some(*limit),
        ),
        NativeProgramExecutionError::Base(_) => (ErrorTag::BaseOther, None, None, None),
        NativeProgramExecutionError::Control(_) => (ErrorTag::Control, None, None, None),
        NativeProgramExecutionError::Value(_) => (ErrorTag::Value, None, None, None),
        NativeProgramExecutionError::Library { .. } => (ErrorTag::Library, None, None, None),
        NativeProgramExecutionError::Collection { .. } => (ErrorTag::Collection, None, None, None),
        NativeProgramExecutionError::Snapshot(_) => (ErrorTag::Snapshot, None, None, None),
    };
    let exact_debit = required
        .filter(|required| *required != u64::MAX)
        .and_then(|required| required.checked_sub(used));
    FAILURE.with(|slot| {
        slot.set(Some(Failure {
            callback,
            site,
            origin: block.map(|block| block.origin()),
            action: block.map(|block| action_tag(block.action())),
            error: tag,
            work_phase: phase,
            native_used_after: used,
            required,
            limit,
            exact_debit,
            selected_binding_work: selected.map(|call| call.binding_work()),
            selected_selector_work: selected
                .map(|call| call.selected_u64_string_span_selector_work()),
            scalar_bytes: descriptor.scalar_bytes,
        }))
    });
}

struct Enabled;
impl Enabled {
    fn begin() -> Self {
        FAILURE.with(|slot| slot.set(None));
        totals::reset();
        ENABLED.with(|slot| slot.set(true));
        Self
    }
}
impl Drop for Enabled {
    fn drop(&mut self) {
        ENABLED.with(|slot| slot.set(false));
    }
}

// This function runs only AFTER the ordinary hosted invocation has returned.
fn print_failure(variant: usize, bases: usize, failure: Option<Failure>) {
    match failure {
        Some(row) => eprintln!(
            "HOSTED_SETUP_FAILURE variant={variant} bases={bases} callback={:?} site={} origin={:?} action={:?} error={:?} work_phase={:?} native_used_after={} required={:?} limit={:?} exact_debit={:?} selected_binding_work={:?} selected_selector_work={:?} scalar_bytes={}",
            row.callback,
            row.site,
            row.origin,
            row.action,
            row.error,
            row.work_phase,
            row.native_used_after,
            row.required,
            row.limit,
            row.exact_debit,
            row.selected_binding_work,
            row.selected_selector_work,
            row.scalar_bytes,
        ),
        None => eprintln!(
            "HOSTED_SETUP_FAILURE variant={variant} bases={bases} callback=none (success or failure outside admitted callbacks; see outcome)"
        ),
    }
}

#[test]
#[ignore = "explicit actual hosted original/decoded whole-source work diagnostic; no timings or expected failure"]
fn complete_secured_knucleotide_original_and_decoded_setup_debit_diagnostic() {
    let (variants, catalog) = fixture::artifacts(FULL);
    for (variant, artifact) in variants.into_iter().enumerate() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .filter(|collection| collection.string_span_projection().is_some())
                .count(),
            2
        );
        assert_eq!(jit.descriptor().plan().resource_requirements().len(), 2);
        for repeats in [25usize, 100] {
            // Exactly the existing unchanged100/400gate input and oracle.
            let input = format!(
                ">ONE\nAAAA\n>THREE timing\n{}\n>FOUR\nCC\n",
                "GGTA".repeat(repeats)
            );
            let windows = 4 * repeats - 1;
            let common = 100.0 * repeats as f64 / windows as f64;
            let other = format!("AG {:.3}\n", 100.0 * (repeats - 1) as f64 / windows as f64);
            let expected = format!(
                "G 50.000\nA 25.000\nT 25.000\n\nGG {common:.3}\nGT {common:.3}\nTA {common:.3}\n{other}\n{repeats}\tGGT\n{repeats}\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n"
            );
            let (mut runtime, trace) = fixture::runtime(input.as_bytes(), true);
            fixture::no_resident(&runtime);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let enabled = Enabled::begin();
            let outcome =
                fixture::execute(&mut runtime, &artifact, &jit, fixture::limits(), &memory);
            let failure = FAILURE.with(Cell::take);
            drop(enabled);
            print_failure(variant, repeats * 4, failure);
            let flags = totals::report(
                jit.descriptor(),
                variant,
                repeats * 4,
                outcome.as_ref().ok().map(|report| report.work),
            );
            let snapshot = trace.lock().unwrap().clone();
            match outcome {
                Ok(report) => {
                    eprintln!(
                        "HOSTED_SETUP_SUCCESS variant={variant} bases={} work={} visits={} comparison_work={}",
                        repeats * 4,
                        report.work,
                        report.visits,
                        report.comparison_work
                    );
                    assert!(failure.is_none());
                    assert!(report.work <= fixture::limits().execution.max_work);
                    assert_eq!(snapshot.output.as_bytes(), expected.as_bytes());
                    assert_eq!(
                        (
                            snapshot.reads,
                            snapshot.acquisitions,
                            snapshot.prepares,
                            snapshot.deliveries,
                            snapshot.flushes
                        ),
                        (1, 1, 1, 1, 1)
                    );
                }
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase,
                    error,
                    cleanup,
                }) => {
                    eprintln!(
                        "HOSTED_SETUP_DENIAL variant={variant} bases={} phase={phase} error={error:?} cleanup={cleanup:?}",
                        repeats * 4
                    );
                    assert!(cleanup.is_none(), "normal transaction abort must succeed");
                    assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
                    let captured = match phase {
                        "native activation" => 0,
                        "native execution" => 1,
                        other => panic!("unexpected fixture failure phase: {other}"),
                    };
                    assert_eq!(
                        (snapshot.reads, snapshot.acquisitions),
                        (captured, captured)
                    );
                    assert_eq!(
                        (snapshot.prepares, snapshot.deliveries, snapshot.flushes),
                        (0, 0, 0)
                    );
                    assert!(snapshot.output.is_empty());
                }
                Err(error) => panic!("unexpected committed/provider diagnostic failure: {error:?}"),
            }
            fixture::no_resident(&runtime);
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(
                flags, 0,
                "actual cumulative-work observation must be complete"
            );
        }
    }
}
