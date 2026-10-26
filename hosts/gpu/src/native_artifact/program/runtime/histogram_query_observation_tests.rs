//! Header observations around the SAME actual emitted entry and real helper.
use crate::native_artifact::program::runtime::collections::QueryDemand;
use crate::native_artifact::program::runtime::invocation::CollectionRead;
use crate::native_artifact::program::runtime::*;
use mech_core::{
    ManagedMemoryReservation, MemoryBudgetDimension as Dimension, MemoryRuntimeError,
    ResidentKernelError,
};
use mech_engine::native_artifact::{NativeCollectionResult, NativeHistogramQueryStepKind as Step};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const SOURCE: &str = "+> map\nlookup(items<[string]>, query<string>) => (out<u64>) :=\n  words := [part | part <- items]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nlookup(source<[string]>, query<string>)\n";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Target {
    collection: u32,
    step: Option<u32>,
    accepted: u64,
    scalar_bytes: u64,
}
impl Target {
    fn of(d: &QueryDemand) -> Self {
        Self {
            collection: d.collection,
            step: d.step,
            accepted: d.accepted,
            scalar_bytes: d.scalar_bytes,
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct ScalarBoundary {
    target: Target,
    envelope: u64,
    available: u64,
    previous_output_bytes: Option<u64>,
    scalar_owners: (u64, u64),
    memory_before: u64,
    memory_pressured: u64,
    work_before: u64,
    pressure_failed: bool,
}
#[derive(Clone, Copy, Debug)]
struct Row {
    before: QueryDemand,
    after: Option<QueryDemand>,
    status: u64,
    memory_before: u64,
    memory_after: u64,
    work_before: u64,
    work_after: u64,
    live_after: u64,
    scalar_owners_after: (u64, u64),
    scalar: Option<ScalarBoundary>,
}
thread_local! {
    static ROWS: std::cell::RefCell<Vec<Row>> = const { std::cell::RefCell::new(Vec::new()) };
    static PRESSURE: std::cell::Cell<Option<(Target, bool)>> = const { std::cell::Cell::new(None) };
    static SCALAR: std::cell::Cell<Option<ScalarBoundary>> = const { std::cell::Cell::new(None) };
}
/// This safe test-only header hook runs immediately before the unchanged
/// Invocation::scalar reservation E; selected-stage lookup allocates nothing.
/// Ballast is an ordinary reservation on the SAME immutable account; no
/// quota, production price, body, generation or Value owner is changed.
/// It returns the guard so pressure encloses only that real constructor.
/// There are deliberately no assertions that could unwind through C.
pub(in crate::native_artifact::program::runtime) fn before_scalar(
    state: &crate::native_artifact::program::runtime::collections::State,
    ctx: &impl CollectionRead,
    id: u32,
    ordinal: u32,
    previous_output_bytes: Option<u64>,
) -> Option<ManagedMemoryReservation> {
    let mut demand = state.test_query_state(id)?;
    demand.step = Some(ordinal);
    let target = Target::of(&demand);
    let memory = ctx.library().memory_budget();
    let memory_before = memory.used_bytes();
    let envelope = ctx.descriptor().metadata().scalar_bytes;
    let mut ballast = None;
    let mut pressure_failed = false;
    if let Some((requested, short)) = PRESSURE.with(std::cell::Cell::get)
        && requested == target
    {
        let available = envelope.checked_sub(u64::from(short));
        match available.and_then(|available| {
            memory
                .limit_bytes()
                .checked_sub(memory_before)?
                .checked_sub(available)
        }) {
            Some(amount) => match memory.reserve_capacity(amount) {
                Ok(reservation) => ballast = Some(reservation),
                Err(_) => pressure_failed = true,
            },
            None => pressure_failed = true,
        }
    }
    let memory_pressured = memory.used_bytes();
    SCALAR.with(|slot| {
        slot.set(Some(ScalarBoundary {
            target,
            envelope,
            available: memory.limit_bytes().saturating_sub(memory_pressured),
            previous_output_bytes,
            scalar_owners: state.test_query_scalar_owners(),
            memory_before,
            memory_pressured,
            work_before: ctx.ledger().work.used(),
            pressure_failed,
        }))
    });
    ballast
}
// No assertion may unwind through C. The observer neither changes the policy
// nor chooses any replacement work; pressure lasts around only the target.
unsafe extern "C" fn observe(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    SCALAR.with(|slot| slot.set(None));
    let before = (|| {
        // SAFETY: execute retains this initialized Invocation for the actual JIT.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let demand = ctx
            .state
            .collections
            .as_ref()?
            .test_query_demand(ctx, usize::try_from(site).ok()?)?;
        Some((
            demand,
            ctx.state.library.memory_budget(),
            ctx.state.control.work.used(),
        ))
    })();
    let memory_before = before
        .as_ref()
        .map_or(0, |(_, memory, _)| memory.used_bytes());
    // SAFETY: every argument is unchanged from the actual generated call.
    let status = unsafe {
        crate::native_artifact::program::runtime::hooks::helper(
            context, site, operation, first, second, third, result,
        )
    };
    if let Some((demand, memory, work_before)) = before {
        // SAFETY: the real helper's exclusive borrow has ended.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let after = ctx
            .state
            .collections
            .as_ref()
            .and_then(|state| state.test_query_state(demand.collection));
        let live_after = ctx
            .state
            .collections
            .as_ref()
            .map_or(0, |state| state.test_live_bytes());
        let scalar_owners_after = ctx
            .state
            .collections
            .as_ref()
            .map_or((0, 0), |state| state.test_query_scalar_owners());
        ROWS.with(|rows| {
            rows.borrow_mut().push(Row {
                before: demand,
                after,
                status,
                memory_before,
                memory_after: memory.used_bytes(),
                work_before,
                work_after: ctx.state.control.work.used(),
                live_after,
                scalar_owners_after,
                scalar: SCALAR.with(std::cell::Cell::take),
            })
        });
    }
    status
}
fn observed(
    jit: &crate::native_artifact::JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    pressure: Option<(Target, bool)>,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Vec<Row>,
) {
    ROWS.with(|rows| rows.borrow_mut().clear());
    PRESSURE.with(|slot| slot.set(pressure));
    let entry = jit.test_checked_entry();
    let outcome = execute(
        jit.descriptor(),
        |raw| {
            // SAFETY: same actual entry, frame and every argument; restore the hook
            // before the production immutable-frame checks observe the return.
            let original = unsafe { (*raw).helper };
            unsafe { (*raw).helper = observe };
            let status = unsafe { entry(raw) };
            unsafe { (*raw).helper = original };
            status
        },
        args,
        outputs,
        limits,
        memory,
    );
    PRESSURE.with(|slot| slot.set(None));
    let rows = ROWS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
    (outcome, rows)
}
fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
    artifacts_source(SOURCE)
}
fn artifacts_source(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5155455259), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn caps() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    // Explicit finite test policy, not a production default/cap change.
    library.max_compute_work = Some(134_217_728);
    library.max_comparison_work = Some(1_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}
fn args(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    parts: &[String],
    query: &str,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, extents) = match name.as_str() {
                "source" => (
                    ValueDataDraft::Matrix(
                        parts.iter().cloned().map(ValueDataDraft::String).collect(),
                    ),
                    Some([1, parts.len() as u64]),
                ),
                "query" => (ValueDataDraft::String(query.into()), None),
                "word" => (ValueDataDraft::String(query.into()), None),
                "indices" => (
                    ValueDataDraft::Matrix([0, 1, 2, 3].map(ValueDataDraft::U64).into()),
                    Some([1, 4]),
                ),
                "queries" => (
                    ValueDataDraft::Matrix(
                        vec![
                            ValueDataDraft::String(query.into()),
                            ValueDataDraft::String("later".into()),
                        ]
                        .into_boxed_slice(),
                    ),
                    Some([1, 2]),
                ),
                _ => panic!("unexpected {name}"),
            };
            let shape_values = extents
                .map(|extents| {
                    mech_core::shape_for_resolved_extents(
                        artifact.schemas().get(input.schema).unwrap(),
                        &extents,
                    )
                    .unwrap()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice()
                })
                .unwrap_or_default();
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
fn snapshots(artifact: &mech_engine::ProgramArtifact, outputs: &[Option<Value>]) -> Vec<Box<[u8]>> {
    outputs
        .iter()
        .map(|value| {
            value
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap()
        })
        .collect()
}
fn row(rows: &[Row], target: Target) -> &Row {
    let rows = rows
        .iter()
        .filter(|row| Target::of(&row.before) == target)
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        1,
        "actual target must be visited once: {target:?}"
    );
    rows[0]
}
fn unchanged(row: &Row) {
    let after = row
        .after
        .expect("denied helper keeps private state until unwind");
    assert_eq!(
        (after.accepted, after.matched, after.phase),
        (row.before.accepted, row.before.matched, row.before.phase)
    );
    assert_eq!(after.scalar_bytes, row.before.scalar_bytes);
    assert_eq!(after.live_bytes, row.before.live_bytes);
    assert_eq!(row.memory_after, row.memory_before);
    assert!(row.work_after >= row.work_before);
}

#[test]
fn actual_query_last_comparison_and_scalar_local_caps_precede_mutation_and_recover() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|c| {
                if let NativeCollectionResult::GuardedHistogramQuery(s) = c.result() {
                    Some(s)
                } else {
                    None
                }
            })
            .expect("actual demand selected");
        let lookup = sink
            .steps()
            .iter()
            .position(|step| step.kind() == Step::Lookup)
            .unwrap() as u32;
        let args = args(
            &artifact,
            &memory,
            &["é\0x".into(), "é\0x".into(), "later".into(), "é\0x".into()],
            "é\0x",
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshots(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let last = baseline
            .iter()
            .filter(|row| row.before.step.is_none())
            .max_by_key(|row| row.before.accepted)
            .unwrap();
        assert_eq!(last.before.accepted, 3, "no first-hit early exit");
        let scalar = baseline
            .iter()
            .find(|row| row.before.step == Some(lookup))
            .unwrap();
        let scalar_after = scalar.after.unwrap();
        for (target, dimension, required) in [
            (
                Target::of(&last.before),
                Dimension::ComparisonWork,
                last.before.comparison_required,
            ),
            (
                Target::of(&scalar.before),
                Dimension::ComputeWork,
                scalar_after.compute,
            ),
            (
                Target::of(&scalar.before),
                Dimension::ClonedBytes,
                scalar_after.cloned,
            ),
            // Old canonical Bool + captured String + new canonical U64.
            (Target::of(&scalar.before), Dimension::RetainedNodes, 6),
        ] {
            for short in [false, true] {
                let mut limits = caps();
                let bound = required - u64::from(short);
                match dimension {
                    Dimension::ComparisonWork => limits.library.max_comparison_work = Some(bound),
                    Dimension::ComputeWork => limits.library.max_compute_work = Some(bound),
                    Dimension::ClonedBytes => limits.library.max_cloned_bytes = Some(bound),
                    Dimension::TemporaryBytes => limits.library.max_temporary_bytes = Some(bound),
                    Dimension::RetainedNodes => limits.library.max_retained_nodes = Some(bound),
                    _ => unreachable!(),
                }
                let (outcome, rows) = observed(&jit, &args, &mut outputs, limits, &memory, None);
                let actual = row(&rows, target);
                if short {
                    assert_ne!(actual.status, STATUS_OK);
                    unchanged(actual);
                    assert!(
                        matches!(outcome.unwrap_err(), NativeProgramExecutionError::Collection {
                            error: ResidentKernelError::ResourceLimit { dimension: actual, required: amount, limit, .. }, ..
                        } if actual == dimension && amount == required && limit == required - 1)
                    );
                } else {
                    assert_eq!(
                        actual.status, STATUS_OK,
                        "later phases may still need larger caps: {outcome:?}"
                    );
                }
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
            }
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_query_fixed_selected_stage_exact_one_short_keeps_spent_prefix_and_recovers() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let (collection, sink) = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|c| match c.result() {
                NativeCollectionResult::GuardedHistogramQuery(s) => Some((c, s)),
                _ => None,
            })
            .unwrap();
        let ordinal = sink
            .steps()
            .iter()
            .position(|s| s.kind() == Step::Keys)
            .unwrap() as u32;
        let selected = jit.descriptor().metadata().calls
            [sink.steps()[ordinal as usize].site() as usize]
            .as_ref()
            .unwrap();
        assert_eq!(
            selected.template(),
            sink.steps()[ordinal as usize].template().unwrap()
        );
        assert!(
            selected.binding_work() > 128,
            "the real selected binding was not replaced"
        );
        let args = args(
            &artifact,
            &memory,
            &["é\0x".into(), "later".into(), "é\0x".into()],
            "é\0x",
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshots(&artifact, &outputs);
        let stable = memory.used_bytes();
        // A SECOND funded invocation includes exactly the same old-publication
        // retirement prepayment as all exact/short invocations below.
        let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let funded = rows
            .iter()
            .find(|r| r.before.step == Some(ordinal))
            .unwrap();
        let target = Target::of(&funded.before);
        let after = funded.after.unwrap();
        assert_eq!(
            after.compute,
            funded.before.compute.checked_add(128).unwrap()
        );
        assert_eq!(
            funded.before.phase,
            Some(mech_engine::native_artifact::NativeHistogramQueryPhase::HistogramReady)
        );
        assert_eq!(
            after.phase,
            Some(mech_engine::native_artifact::NativeHistogramQueryPhase::KeysReady)
        );
        assert_eq!(
            (after.scalar_bytes, after.live_bytes),
            (funded.before.scalar_bytes, funded.before.live_bytes)
        );
        assert_eq!(funded.before.reservation_bytes, 0);
        assert_eq!(funded.memory_after, funded.memory_before);
        // The saved-generation arena read is honestly spent BEFORE the
        // atomic fixed-stage debit. It is not a free helper or a refund.
        let prefix = funded.work_after.checked_sub(128).unwrap();
        assert!(prefix > funded.work_before && funded.work_before > 0);
        for global in [false, true] {
            let exact = if global {
                funded.work_after
            } else {
                after.compute
            };
            for short in [false, true] {
                let mut limits = caps();
                if global {
                    limits.execution.max_work = exact - u64::from(short);
                } else {
                    limits.library.max_compute_work = Some(exact - u64::from(short));
                }
                let (outcome, rows) = observed(&jit, &args, &mut outputs, limits, &memory, None);
                let actual = row(&rows, target); // Earlier denial is never accepted as this boundary.
                assert_eq!(actual.work_before, funded.work_before);
                if short {
                    assert_ne!(actual.status, STATUS_OK);
                    unchanged(actual);
                    assert_eq!(
                        actual.work_after, prefix,
                        "denial keeps all prefix work without debit/refund"
                    );
                    let error = outcome.unwrap_err();
                    if global {
                        assert!(
                            matches!(error, NativeProgramExecutionError::Base(
                                NativeExecutionError::WorkLimit { phase: NativeExecutionPhase::Setup, required, limit }
                            ) if required == exact && limit == exact - 1),
                            "{error:?}"
                        );
                    } else {
                        assert!(
                            matches!(error, NativeProgramExecutionError::Collection {
                                origin, error: ResidentKernelError::ResourceLimit {
                                    phase: mech_core::ResidentResourcePhase::Admission,
                                    dimension: Dimension::ComputeWork, required, limit,
                                }
                            } if origin == collection.origin() && required == exact && limit == exact - 1),
                            "{error:?}"
                        );
                    }
                } else {
                    assert_eq!(actual.status, STATUS_OK, "{outcome:?}");
                    assert_eq!(actual.work_after, funded.work_after);
                    let admitted = actual.after.unwrap();
                    assert_eq!(
                        (admitted.compute, admitted.phase),
                        (after.compute, after.phase)
                    );
                    if let Err(error) = outcome {
                        assert!(
                            match &error {
                                NativeProgramExecutionError::Control(
                                    NativeIterationExecutionError::Work {
                                        required, limit, ..
                                    },
                                ) if global => *required > *limit && *limit == exact,
                                NativeProgramExecutionError::Collection {
                                    origin,
                                    error:
                                        ResidentKernelError::ResourceLimit {
                                            phase: mech_core::ResidentResourcePhase::Admission,
                                            dimension: Dimension::ComputeWork,
                                            required,
                                            limit,
                                        },
                                } if !global =>
                                    *origin == collection.origin()
                                        && *required > *limit
                                        && *limit == exact,
                                _ => false,
                            },
                            "fixed stage succeeded; only a truthful later work denial is permitted: {error:?}"
                        );
                    }
                }
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_query_lookup_local_temporary_exact_one_short_reaches_selected_stage_and_recovers() {
    // An admitted numeric Matrix yields real scalar words without an
    // earlier Range binder or managed String element-copy reservation.
    // Every visit still yields the genuine captured String. With immutable
    // stage association proved once, Lookup's actual peak is live + E for
    // the unchanged canonical scalar constructor, including the old Bool.
    let source = "+> map\nlookup(word<string>, query<string>, indices<[u64]>) => (out<u64>) :=\n  words := [word | index <- indices]\n  counts := map/histogram(words)\n  keys<{string}> := counts_keys\n  count := (query ∈ keys) ? | true => counts{query} | false => 0u64.\n  out := count.\nlookup(word<string>, query<string>, indices<[u64]>)\n";
    for artifact in artifacts_source(source) {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let (collection, sink) = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| match collection.result() {
                NativeCollectionResult::GuardedHistogramQuery(sink) => Some((collection, sink)),
                _ => None,
            })
            .expect("actual selected query");
        let lookup = sink
            .steps()
            .iter()
            .position(|step| step.kind() == Step::Lookup)
            .unwrap() as u32;
        for &generator in collection.generators() {
            let mech_engine::native_artifact::NativeControlLoop::Collection(generator) =
                &jit.descriptor().plan().iterations()[generator as usize]
            else {
                panic!("actual collection source");
            };
            let source = generator
                .source()
                .materialized_value()
                .expect("fixture must actually use its admitted numeric Matrix");
            assert!(matches!(
                jit.descriptor().plan().values()[source.get() as usize].definition().body(),
                mech_core::SchemaBody::Matrix { element, .. }
                    if element.as_ref() == &mech_core::SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)
            ));
        }
        assert!(
            !jit.descriptor()
                .plan()
                .blocks()
                .iter()
                .any(|block| matches!(
                    block.action(),
                    NativeIterationAction::PrepareU64Range { .. }
                        | NativeIterationAction::LibraryCall { .. }
                )),
            "no unrelated earlier selected binder may mask the actual scalar boundary"
        );
        let envelope = jit.descriptor().metadata().scalar_bytes;
        let args = args(&artifact, &memory, &[], "é\0x");
        let word = args[artifact
            .inputs()
            .iter()
            .position(|input| mech_engine::decode_source_input_name(&input.name).unwrap() == "word")
            .unwrap()]
        .memory_budget_retained_bytes(&memory)
        .unwrap();
        assert!(
            jit.descriptor().metadata().scalar_bytes > word,
            "earlier captured-yield peak must fit"
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::U64(4)
        );
        let prior = snapshots(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        assert_eq!(
            rows.iter().filter(|row| row.before.step.is_none()).count(),
            4,
            "every numeric candidate still yields and compares"
        );
        let scalar = rows
            .iter()
            .find(|row| row.before.step == Some(lookup))
            .expect("actual Lookup scalar stage");
        assert_eq!(scalar.before.accepted, 4);
        assert_eq!(scalar.before.matched, 4);
        assert!(
            scalar.before.scalar_bytes > 0,
            "old Bool owner remains live"
        );
        assert_eq!(scalar.before.reservation_bytes, envelope);
        let target = Target::of(&scalar.before);
        let required = scalar.before.temporary_required;
        assert_eq!(required, scalar.before.live_bytes + envelope);
        let earlier_peak = rows
            .iter()
            .take_while(|row| Target::of(&row.before) != target)
            .map(|row| row.before.temporary_required)
            .max()
            .unwrap();
        assert!(
            earlier_peak < required,
            "Lookup must be a genuine later peak, not an unreachable one-short cap"
        );
        for short in [false, true] {
            let mut limits = caps();
            limits.library.max_temporary_bytes = Some(required - u64::from(short));
            let (outcome, rows) = observed(&jit, &args, &mut outputs, limits, &memory, None);
            let actual = row(&rows, target); // Fails on any earlier setup/body/element denial.
            if short {
                assert_ne!(actual.status, STATUS_OK);
                unchanged(actual);
                assert!(
                    matches!(outcome.unwrap_err(),NativeProgramExecutionError::Collection{error:ResidentKernelError::ResourceLimit{dimension:Dimension::TemporaryBytes,required:amount,limit,..},..}if amount==required&&limit==required-1)
                );
            } else {
                assert_eq!(actual.status, STATUS_OK, "{outcome:?}");
            }
            assert_eq!(snapshots(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            assert_eq!(snapshots(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_query_scalar_global_reservation_exact_and_one_short_has_no_future_credit() {
    let source = SOURCE.replace(
        "lookup(source<[string]>, query<string>)",
        "[lookup(source<[string]>, query) | query <- queries<[string]>]",
    );
    for artifact in artifacts_source(&source) {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let args = args(
            &artifact,
            &memory,
            &["hit".into(), "hit".into(), "later".into()],
            "hit",
        );
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshots(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let sink = jit
            .descriptor()
            .plan()
            .collections()
            .iter()
            .find_map(|collection| {
                if let NativeCollectionResult::GuardedHistogramQuery(sink) = collection.result() {
                    Some(sink)
                } else {
                    None
                }
            })
            .unwrap();
        let cleanup = sink
            .steps()
            .iter()
            .position(|step| step.kind() == Step::Cleanup)
            .unwrap() as u32;
        let cleanups = baseline
            .iter()
            .filter(|row| row.before.step == Some(cleanup))
            .collect::<Vec<_>>();
        assert_eq!(cleanups.len(), 2);
        for row in cleanups {
            assert!(row.after.is_none());
            assert_eq!(
                row.memory_after, row.memory_before,
                "cleanup must not return physical scalar/source owner capacity"
            );
            assert!(row.scalar_owners_after.0 > 0 && row.scalar_owners_after.1 > 0);
            assert_eq!(
                row.live_after,
                row.before.live_bytes - row.before.query_bytes,
                "cleanup ends only the capture pin; scalar roots stay in all following peaks"
            );
            assert!(row.live_after >= row.scalar_owners_after.0);
        }
        for kind in [Step::Membership, Step::Lookup] {
            let ordinal = sink
                .steps()
                .iter()
                .position(|step| step.kind() == kind)
                .unwrap() as u32;
            let visits = baseline
                .iter()
                .filter(|row| row.before.step == Some(ordinal))
                .collect::<Vec<_>>();
            assert_eq!(
                visits.len(),
                2,
                "both fresh outer queries must run the same generic scalar site"
            );
            let previous = visits[0].before.scalar_bytes;
            let target = visits[1];
            assert!(
                target.before.scalar_bytes > previous,
                "second query must retain both old Bool/U64 roots before replacement"
            );
            assert!(target.before.reservation_bytes > 0 && target.before.query_bytes > 0);
            let scalar_bytes = jit.descriptor().metadata().scalar_bytes;
            let baseline_scalar = target.scalar.expect("actual scalar constructor boundary");
            assert_eq!(baseline_scalar.envelope, scalar_bytes);
            assert!(
                baseline_scalar
                    .previous_output_bytes
                    .is_some_and(|bytes| bytes > 0)
            );
            assert!(baseline_scalar.scalar_owners.0 >= target.before.scalar_bytes);
            assert!(baseline_scalar.scalar_owners.1 >= 4);
            assert!(!baseline_scalar.pressure_failed);
            let target = Target::of(&target.before);
            assert_eq!(baseline_scalar.target, target);
            for short in [false, true] {
                let (outcome, rows) = observed(
                    &jit,
                    &args,
                    &mut outputs,
                    caps(),
                    &memory,
                    Some((target, short)),
                );
                let actual = row(&rows, target);
                let boundary = actual.scalar.expect("reached unchanged Invocation::scalar");
                assert_eq!(boundary.target, target);
                assert!(
                    !boundary.pressure_failed,
                    "ballast installation must succeed"
                );
                assert_eq!(boundary.envelope, scalar_bytes);
                assert_eq!(boundary.available, scalar_bytes - u64::from(short));
                assert!(
                    boundary
                        .previous_output_bytes
                        .is_some_and(|bytes| bytes > 0)
                );
                assert_eq!(boundary.scalar_owners, baseline_scalar.scalar_owners);
                assert!(boundary.memory_pressured >= boundary.memory_before);
                assert!(boundary.work_before >= actual.work_before);
                if short {
                    assert_ne!(actual.status, STATUS_OK);
                    unchanged(actual);
                    assert!(matches!(
                        outcome.unwrap_err(),
                        NativeProgramExecutionError::Base(NativeExecutionError::Memory {
                            error: MemoryRuntimeError::BudgetExceeded {
                                operation: None, requested, limit,
                            },
                        }) if limit == memory.limit_bytes() && requested == limit + 1
                    ));
                } else {
                    outcome.unwrap();
                    assert_eq!(actual.status, STATUS_OK);
                }
                assert_eq!(snapshots(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
        }
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_high_diversity_query_retains_no_cardinality_sized_keys_or_payloads() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .plan()
                .collections()
                .iter()
                .any(|c| matches!(c.result(), NativeCollectionResult::GuardedHistogramQuery(_)))
        );
        let parts = (0..256)
            .map(|index| format!("key-{index:014}"))
            .collect::<Vec<_>>();
        let args = args(&artifact, &memory, &parts, &parts[0]);
        let mut outputs = vec![None; artifact.outputs().len()];
        let (outcome, rows) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let pushes = rows
            .iter()
            .filter(|row| row.before.step.is_none())
            .collect::<Vec<_>>();
        assert_eq!(pushes.len(), parts.len());
        assert_eq!(
            pushes.last().unwrap().after.unwrap().accepted,
            parts.len() as u64
        );
        assert!(
            pushes
                .iter()
                .all(|row| row.before.live_bytes == pushes[0].before.live_bytes)
        );
        assert!(
            pushes
                .iter()
                .all(|row| row.before.query_bytes == pushes[0].before.query_bytes)
        );
        assert_eq!(
            outputs
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::U64(1)
        );
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}
