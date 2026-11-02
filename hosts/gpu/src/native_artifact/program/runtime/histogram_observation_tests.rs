//! INLINE into program_runtime.rs's existing audited FFI boundary only.
//! The checked generated entry and real helper remain the sole execution path.
use crate::native_artifact::program::runtime::collections::{
    HistogramDemand as Demand, HistogramStage as Stage, HistogramVisit as Visit,
};
use crate::native_artifact::program::runtime::test_support::snapshot_outputs as snapshot;
use crate::native_artifact::program::runtime::*;
use mech_core::{MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[derive(Clone, Copy)]
struct Pressure {
    demand: Demand,
    short: bool,
}
#[derive(Clone, Copy, Debug)]
struct Row {
    visit: Visit,
    demand: Option<Demand>,
    after: Option<(u64, u64, u64, u64)>,
    memory_before: u64,
    memory_after: u64,
    work_before: u64,
    work_after: u64,
    global_remaining: Option<u64>,
    status: u64,
}
thread_local! {
    static ROWS:std::cell::RefCell<Vec<Row>>=const{std::cell::RefCell::new(Vec::new())};
    static PRESSURE:std::cell::Cell<Option<Pressure>>=const{std::cell::Cell::new(None)};
}
unsafe extern "C" fn observe(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    crate::native_artifact::program::runtime::collections::State::test_histogram_clear_observation(
    );
    let before = (|| {
        // SAFETY: execute retains the synchronous Invocation; the shared borrow
        // ends before delegating unchanged arguments to the exclusive helper.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let visit = ctx
            .state
            .collections
            .as_ref()?
            .test_histogram_visit(ctx, usize::try_from(site).ok()?)?;
        Some((
            visit,
            ctx.state.library.memory_budget(),
            ctx.state.control.work.used(),
        ))
    })();
    let (mut ballast, mut remaining, mut pressure_failed) = (None, None, false);
    if let Some((visit, memory, _)) = &before {
        if let Some(pressure) = PRESSURE.with(std::cell::Cell::get)
            && *visit == pressure.demand.visit
        {
            let free = pressure
                .demand
                .reservation_bytes
                .saturating_sub(u64::from(pressure.short));
            match memory
                .limit_bytes()
                .checked_sub(memory.used_bytes())
                .and_then(|available| available.checked_sub(free))
            {
                Some(amount) => match memory.reserve_capacity(amount) {
                    Ok(token) => {
                        ballast = Some(token);
                        remaining = Some(free);
                    }
                    Err(_) => pressure_failed = true,
                },
                None => pressure_failed = true,
            }
        }
    }
    let memory_before = before.as_ref().map(|(_, memory, _)| memory.used_bytes());
    // SAFETY: delegate the identical generated call and result pointer. No
    // observer assertion can unwind through this C boundary.
    let status = unsafe {
        crate::native_artifact::program::runtime::hooks::helper(
            context, site, operation, first, second, third, result,
        )
    };
    if let Some((visit, memory, work_before)) = before {
        // SAFETY: the delegated exclusive helper borrow has ended.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let demand = crate::native_artifact::program::runtime::collections::State::test_histogram_take_observation();
        let after = ctx
            .state
            .collections
            .as_ref()
            .and_then(|state| state.test_histogram_storage(visit.collection));
        ROWS.with(|rows| {
            rows.borrow_mut().push(Row {
                visit,
                demand,
                after,
                memory_before: memory_before.unwrap_or(0),
                memory_after: memory.used_bytes(),
                work_before,
                work_after: ctx.state.control.work.used(),
                global_remaining: remaining,
                status: if pressure_failed { u64::MAX } else { status },
            })
        });
    }
    drop(ballast); // Pressure ends before every later lifecycle/publication phase.
    status
}
fn observed(
    jit: &crate::native_artifact::JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    caps: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    pressure: Option<Pressure>,
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
            // SAFETY: the JIT owner is alive; restore immutable helper transport
            // before execute checks the frame. This does not replace computation.
            let previous = unsafe { (*raw).helper };
            unsafe { (*raw).helper = observe };
            let result = unsafe { entry(raw) };
            unsafe { (*raw).helper = previous };
            result
        },
        args,
        outputs,
        caps,
        memory,
    );
    PRESSURE.with(|slot| slot.set(None));
    let rows = ROWS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
    (outcome, rows)
}
fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
    let source = "+> map\nmap/histogram([part | part <- source<[string]>])\n";
    artifacts_source(source)
}
fn artifacts_source(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4849535444454d), Revision(1), source).unwrap(),
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
    library.max_compute_work = Some(134_217_728);
    library.max_comparison_work = Some(4_000_000);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 10_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}
fn args(
    artifact: &mech_engine::ProgramArtifact,
    scalar_bytes: u64,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    // Choose actual vector geometry, not a pricing estimate. The last growth's
    // new backing strictly dominates the earlier source-element envelope, so
    // exact/one-short Temporary caps reach this real preflight, unlike the
    // earlier three-yield TextJoin fixture. Keys are small, sorted and distinct.
    let capacity = (scalar_bytes
        / crate::native_artifact::program::runtime::collections::State::test_histogram_entry_bytes(
        )
        + 128)
        .next_power_of_two()
        .max(1024);
    assert!(
        capacity <= 8192,
        "fixture must stay a small boundary test: {scalar_bytes}"
    );
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let data = ValueDataDraft::Matrix(
                (0..=capacity)
                    .map(|index| ValueDataDraft::String(format!("é\0{index:08x}")))
                    .collect(),
            );
            let shape_values = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(input.schema).unwrap(),
                &[1, capacity + 1],
            )
            .unwrap()
            .parameter_values()
            .to_vec()
            .into_boxed_slice();
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut token = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut token).unwrap()
        })
        .collect()
}
fn captured_binding_peak(jit: &crate::native_artifact::JitProgramArtifact) -> u64 {
    // Fixture geometry uses the actual maintained bindings, including the
    // range's three-port envelope. It does not change any runtime price.
    let plan = jit.descriptor().plan();
    assert_eq!(
        plan.collections()
            .iter()
            .filter(|collection| matches!(
                collection.result(),
                mech_engine::native_artifact::NativeCollectionResult::Histogram(_)
            ))
            .count(),
        1
    );
    assert_eq!(
            plan.iterations()
                .iter()
                .filter(|iteration| matches!(
                    iteration,
                    mech_engine::native_artifact::NativeControlLoop::Collection(generator)
                        if matches!(generator.source(), mech_engine::native_artifact::NativeCollectionSource::U64Range(_))
                ))
                .count(),
            1
        );
    assert!(
        plan.blocks()
            .iter()
            .all(|block| !matches!(block.action(), NativeIterationAction::LibraryCall { .. }))
    );
    let binding = jit
        .descriptor()
        .metadata()
        .calls
        .iter()
        .flatten()
        .map(|call| call.binding_metadata_bytes())
        .max()
        .unwrap();
    binding
        + crate::native_artifact::program::runtime::collections::State::metadata_bytes(
            jit.descriptor(),
        )
        .unwrap()
}
fn captured_roots(bytes: usize, count: u8) -> Vec<String> {
    (0..count)
        .map(|index| format!("{}{}", char::from(b'A' + index), "x".repeat(bytes - 1)))
        .collect()
}
fn captured_args(
    artifact: &mech_engine::ProgramArtifact,
    roots: &[String],
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            assert_eq!(name.len(), 1, "single-letter captured fixture root");
            let index = usize::from(name.as_bytes()[0] - b'a');
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::String(roots[index].clone()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut token = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut token).unwrap()
        })
        .collect()
}
fn row(rows: &[Row], demand: Demand) -> &Row {
    let selected = rows
        .iter()
        .filter(|row| row.visit == demand.visit)
        .collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        1,
        "actual stage must be entered exactly once: {:?}; rows={rows:?}",
        demand.visit
    );
    let row = selected[0];
    assert!(
        row.demand.is_some(),
        "actual search and stage preflight must be reached: {row:?}"
    );
    assert_eq!(
        row.demand.unwrap().stage,
        demand.stage,
        "an earlier boundary inside the helper does not qualify the requested stage"
    );
    row
}
fn unchanged(row: &Row, demand: Demand) {
    assert_eq!(
        row.after,
        Some((
            demand.visit.accepted_before,
            demand.distinct_before,
            demand.payload_before,
            demand.capacity_before
        ))
    );
    assert_eq!(
        row.memory_after, row.memory_before,
        "no future-output capacity credit on denial"
    );
    assert!(
        row.work_after >= row.work_before,
        "spent work is never refunded"
    );
}
fn required(demand: Demand, dimension: Dimension) -> u64 {
    match dimension {
        Dimension::TemporaryBytes => demand.temporary_required,
        Dimension::ComputeWork => demand.compute_required,
        Dimension::ClonedBytes => demand.cloned_required,
        Dimension::OutputBytes => demand.output_required,
        Dimension::OutputElements => demand.elements_required,
        Dimension::RetainedNodes => demand.nodes_required,
        Dimension::ComparisonWork => demand.comparison_required,
        _ => panic!("not an isolated boundary"),
    }
}
fn cap(caps: &mut NativeProgramExecutionLimits, dimension: Dimension, value: u64) {
    match dimension {
        Dimension::TemporaryBytes => caps.library.max_temporary_bytes = Some(value),
        Dimension::ComputeWork => caps.library.max_compute_work = Some(value),
        Dimension::ClonedBytes => caps.library.max_cloned_bytes = Some(value),
        Dimension::OutputBytes => caps.library.max_output_bytes = Some(value),
        Dimension::OutputElements => caps.library.max_output_elements = Some(value),
        Dimension::RetainedNodes => caps.library.max_retained_nodes = Some(value),
        Dimension::ComparisonWork => caps.library.max_comparison_work = Some(value),
        _ => panic!("not an isolated boundary"),
    }
}
#[test]
fn actual_histogram_growth_and_finish_exact_local_caps_precede_mutation_and_recover() {
    // The generated body selects existing managed String roots, not a
    // String element/concatenation factory with an earlier large envelope.
    // Four tiny keys establish capacity four; the last large key forces the
    // actual 4->8 vector growth. Its copied key and untracked captured owner
    // coexist, so 2*large alone exceeds every fixed binding prefix. Keeping
    // the other keys small leaves ample unchanged compute/comparison room.
    let source = "+> map\nmap/histogram([(index? | 1u64 => a<string> | 2u64 => b<string> | 3u64 => c<string> | 4u64 => d<string> | * => e<string>) | index <- 1u64..=5u64])\n";
    for artifact in artifacts_source(source) {
        let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let binding_peak = captured_binding_peak(&jit);
        let large = usize::try_from(binding_peak.div_ceil(2) + 1024).unwrap();
        let mut roots = captured_roots(8, 4);
        roots.push(format!("E{}", "x".repeat(large - 1)));
        let args = captured_args(&artifact, &roots, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshot(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let growth = baseline
            .iter()
            .filter_map(|row| row.demand)
            .filter(|d| d.stage == Stage::Growth)
            .max_by_key(|d| d.visit.accepted_before)
            .unwrap();
        let finish = baseline
            .iter()
            .filter_map(|row| row.demand)
            .find(|d| d.stage == Stage::Finish)
            .unwrap();
        assert!(
            growth.distinct_before == 4
                && growth.capacity_before == 4
                && growth.old_backing_bytes > 0
                && growth.current_yield_bytes > 0
        );
        assert_eq!(growth.current_yield_bytes, growth.untracked_yield_bytes);
        assert!(growth.current_yield_bytes >= large as u64);
        assert!(growth.temporary_required > binding_peak);
        assert_eq!(
                growth.reservation_bytes,
                large as u64 + 8 * crate::native_artifact::program::runtime::collections::State::test_histogram_entry_bytes()
            );
        assert!(
            growth.temporary_required
                >= growth.old_backing_bytes + growth.reservation_bytes + growth.current_yield_bytes
        );
        assert_eq!(growth.visit.accepted_before, growth.distinct_before);
        assert_eq!(
            finish.visit.accepted_before,
            growth.visit.accepted_before + 1
        );
        assert_eq!(finish.output_required, growth.output_required);
        for (demand, dimensions) in [
            (
                growth,
                &[
                    Dimension::TemporaryBytes,
                    Dimension::ComputeWork,
                    Dimension::ClonedBytes,
                    Dimension::OutputBytes,
                    Dimension::OutputElements,
                    Dimension::RetainedNodes,
                ][..],
            ),
            (
                finish,
                &[
                    Dimension::TemporaryBytes,
                    Dimension::ComputeWork,
                    Dimension::ClonedBytes,
                    Dimension::ComparisonWork,
                ][..],
            ),
        ] {
            for &dimension in dimensions {
                let bound = required(demand, dimension);
                assert!(bound > 0);
                let mut exact = caps();
                cap(&mut exact, dimension, bound);
                let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
                assert!(
                    rows.iter().any(|row| row.visit == demand.visit),
                    "exact {dimension:?}={bound} did not reach {:?}: {outcome:?}",
                    demand.visit
                );
                assert_eq!(
                    row(&rows, demand).status,
                    STATUS_OK,
                    "exact {dimension:?}; outcome={outcome:?}"
                );
                if let Err(error) = outcome {
                    assert!(
                        matches!(error,NativeProgramExecutionError::Collection{error:ResidentKernelError::ResourceLimit{dimension:actual,..},..}if actual==dimension),
                        "{error:?}"
                    );
                }
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                let mut low = caps();
                cap(&mut low, dimension, bound - 1);
                let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
                assert!(
                    rows.iter().any(|row| row.visit == demand.visit),
                    "one-short {dimension:?}={bound}-1 did not reach {:?}: {outcome:?}",
                    demand.visit
                );
                let denied = row(&rows, demand);
                assert_ne!(denied.status, STATUS_OK);
                let error = outcome.unwrap_err();
                assert!(
                    matches!(error,NativeProgramExecutionError::Collection{origin,error:ResidentKernelError::ResourceLimit{phase:ResidentResourcePhase::Admission,dimension:actual,required,limit}}if origin==demand.origin&&actual==dimension&&required==bound&&limit==bound-1),
                    "{error:?}"
                );
                unchanged(denied, demand);
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
        }
        // Output/nodes/elements at Finish already bind the last accepted key;
        // a globally one-short cap correctly denies the preceding growth.
        // The separate finite cumulative comparison cap is isolated at Finish,
        // after the real binary search prefix fits unchanged.
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[test]
fn actual_histogram_growth_and_finish_global_coexistence_exact_one_short_and_recovery() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let args = args(&artifact, jit.descriptor().metadata().scalar_bytes, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshot(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let growth = baseline
            .iter()
            .filter_map(|row| row.demand)
            .filter(|d| d.stage == Stage::Growth)
            .max_by_key(|d| d.visit.accepted_before)
            .unwrap();
        let finish = baseline
            .iter()
            .filter_map(|row| row.demand)
            .find(|d| d.stage == Stage::Finish)
            .unwrap();
        for demand in [growth, finish] {
            assert!(demand.reservation_bytes > 0 && demand.compute_debit > 0);
            let (outcome, rows) = observed(
                &jit,
                &args,
                &mut outputs,
                caps(),
                &memory,
                Some(Pressure {
                    demand,
                    short: false,
                }),
            );
            outcome.unwrap();
            let exact = row(&rows, demand);
            assert_eq!(exact.status, STATUS_OK);
            assert_eq!(exact.global_remaining, Some(demand.reservation_bytes));
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            let (outcome, rows) = observed(
                &jit,
                &args,
                &mut outputs,
                caps(),
                &memory,
                Some(Pressure {
                    demand,
                    short: true,
                }),
            );
            let denied = row(&rows, demand);
            assert_ne!(denied.status, STATUS_OK);
            assert_ne!(
                denied.status,
                u64::MAX,
                "pressure installation must succeed"
            );
            assert_eq!(denied.global_remaining, Some(demand.reservation_bytes - 1));
            let error = outcome.unwrap_err();
            assert!(
                matches!(error,NativeProgramExecutionError::Base(NativeExecutionError::Memory{error:mech_core::MemoryRuntimeError::BudgetExceeded{requested,limit,..}})if requested==memory.limit_bytes()+1&&limit==memory.limit_bytes()),
                "{error:?}"
            );
            unchanged(denied, demand);
            assert!(denied.work_after - denied.work_before >= demand.compute_debit);
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                .unwrap();
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
        }
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

// INLINE inside histogram_observation_tests, after its shared helpers.
#[test]
fn actual_histogram_duplicate_body_yield_exact_temporary_peak_and_one_short_recover() {
    // Generated matches copy pre-admitted captured String roots. No per-yield
    // String-element/concatenation admission can hide the actual last yield.
    // Four large keys, four tiny keys and then a large duplicate make that
    // duplicate's untracked owner coexist with strictly more key/vector
    // backing than any earlier new-key allocation. Preserve the existing
    // fixture policy unchanged; only its source/argument geometry changes.
    let source = "+> map\nmap/histogram([(index? | 1u64 => a<string> | 2u64 => b<string> | 3u64 => c<string> | 4u64 => d<string> | 5u64 => e<string> | 6u64 => f<string> | 7u64 => g<string> | 8u64 => h<string> | * => c) | index <- 1u64..=9u64])\n";
    for artifact in artifacts_source(source) {
        let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let mut policy = caps();
        policy.library.max_compute_work = Some(1_000_000_000);
        policy.library.max_comparison_work = Some(1_000_000_000);
        let binding_peak = captured_binding_peak(&jit);
        let large = usize::try_from(binding_peak.div_ceil(5) + 1024).unwrap();
        let mut roots = captured_roots(large, 4);
        roots.extend((0..4).map(|index| format!("s{index}")));
        let args = captured_args(&artifact, &roots, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, policy, &memory)
            .unwrap();
        let prior = snapshot(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, rows) = observed(&jit, &args, &mut outputs, policy, &memory, None);
        outcome.unwrap();
        let duplicate = rows
            .iter()
            .filter_map(|row| row.demand)
            .find(|demand| demand.stage == Stage::Yield && demand.visit.accepted_before == 8)
            .expect("actual last duplicate body yield preflight");
        assert_eq!(duplicate.distinct_before, 8);
        assert_eq!(duplicate.reservation_bytes, 0);
        assert!(
            duplicate.current_yield_bytes >= large as u64
                && duplicate.untracked_yield_bytes == duplicate.current_yield_bytes
        );
        assert!(duplicate.temporary_required > binding_peak);
        assert_eq!(
            duplicate.temporary_required,
            duplicate.live_bytes + duplicate.untracked_yield_bytes
        );
        assert_eq!(
            duplicate.compute_debit, 0,
            "duplicate yield admission invents no append/copy/growth"
        );
        // Every earlier recorded growth must fit strictly below this local cap;
        // row(stage) below also rejects an earlier source/leaf denial loophole.
        assert!(
            rows.iter()
                .filter_map(|row| row.demand)
                .filter(|d| d.stage == Stage::Growth)
                .all(|d| d.temporary_required < duplicate.temporary_required)
        );
        let mut exact = policy;
        exact.library.max_temporary_bytes = Some(duplicate.temporary_required);
        let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
        assert!(
            rows.iter().any(|row| row.visit == duplicate.visit),
            "exact duplicate Temporary={} did not reach {:?}: {outcome:?}",
            duplicate.temporary_required,
            duplicate.visit
        );
        assert_eq!(row(&rows, duplicate).status, STATUS_OK, "{outcome:?}");
        if let Err(error) = outcome {
            assert!(
                matches!(
                    error,
                    NativeProgramExecutionError::Collection {
                        error: ResidentKernelError::ResourceLimit {
                            phase: ResidentResourcePhase::Admission,
                            dimension: Dimension::TemporaryBytes,
                            ..
                        },
                        ..
                    }
                ),
                "{error:?}"
            );
        }
        assert_eq!(snapshot(&artifact, &outputs), prior);
        assert_eq!(memory.used_bytes(), stable);
        let mut low = policy;
        low.library.max_temporary_bytes = Some(duplicate.temporary_required - 1);
        let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
        assert!(
            rows.iter().any(|row| row.visit == duplicate.visit),
            "one-short duplicate Temporary={}-1 did not reach {:?}: {outcome:?}",
            duplicate.temporary_required,
            duplicate.visit
        );
        let denied = row(&rows, duplicate);
        assert_ne!(denied.status, STATUS_OK);
        let error = outcome.unwrap_err();
        assert!(
            matches!(error,NativeProgramExecutionError::Collection{origin,error:ResidentKernelError::ResourceLimit{phase:ResidentResourcePhase::Admission,dimension:Dimension::TemporaryBytes,required,limit}}if origin==duplicate.origin&&required==duplicate.temporary_required&&limit==required-1),
            "{error:?}"
        );
        unchanged(denied, duplicate);
        assert_eq!(snapshot(&artifact, &outputs), prior);
        assert_eq!(memory.used_bytes(), stable);
        jit.evaluate_into(&args, &mut outputs, policy, &memory)
            .unwrap();
        assert_eq!(snapshot(&artifact, &outputs), prior);
        assert_eq!(memory.used_bytes(), stable);
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}
