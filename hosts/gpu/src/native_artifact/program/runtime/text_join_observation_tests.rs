//! Observe actual emitted append/finalize calls without replacing their work.
//! The observer never changes a policy, payload, visit, operation, or argument.
use crate::native_artifact::program::runtime::collections::{TextJoinDemand, TextJoinStage};
use crate::native_artifact::program::runtime::*;
use mech_core::{MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Target {
    collection: u32,
    stage: TextJoinStage,
    accepted_before: u64,
}
impl Target {
    fn of(demand: &TextJoinDemand) -> Self {
        Self {
            collection: demand.collection,
            stage: demand.stage,
            accepted_before: demand.accepted_before,
        }
    }
}
#[derive(Clone, Copy)]
struct Pressure {
    target: Target,
    short: bool,
}
#[derive(Clone, Copy, Debug)]
struct Observation {
    before: TextJoinDemand,
    after_storage: Option<(u64, u64, u64)>,
    memory_before: u64,
    memory_after: u64,
    work_before: u64,
    work_after: u64,
    global_remaining: Option<u64>,
    status: u64,
}
thread_local! {
    static OBSERVATIONS: std::cell::RefCell<Vec<Observation>> = const { std::cell::RefCell::new(Vec::new()) };
    static PRESSURE: std::cell::Cell<Option<Pressure>> = const { std::cell::Cell::new(None) };
}

// Same checked callback, with a shared header read before/after its exclusive
// borrow. In particular no assertion may unwind through this C callback.
unsafe extern "C" fn observe(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let before = (|| {
        // SAFETY: execute retains this exact context for the synchronous JIT
        // entry. This shared borrow ends before calling the real helper.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let state = ctx.state.collections.as_ref()?;
        let demand = state
            .test_text_join_demand(ctx, usize::try_from(site).ok()?)
            .ok()??;
        Some((
            demand,
            ctx.state.library.memory_budget(),
            ctx.state.control.work.used(),
        ))
    })();
    let mut pressure_failure = false;
    let mut ballast = None;
    let mut global_remaining = None;
    if let Some((demand, memory, _)) = &before {
        if let Some(pressure) = PRESSURE.with(std::cell::Cell::get)
            && pressure.target == Target::of(demand)
        {
            let remaining = demand
                .reservation_bytes
                .saturating_sub(u64::from(pressure.short));
            let amount = memory
                .limit_bytes()
                .checked_sub(memory.used_bytes())
                .and_then(|available| available.checked_sub(remaining));
            match amount {
                Some(amount) => match memory.reserve_capacity(amount) {
                    Ok(reservation) => {
                        ballast = Some(reservation);
                        global_remaining = Some(remaining);
                    }
                    Err(_) => pressure_failure = true,
                },
                None => pressure_failure = true,
            }
        }
    }
    // Pressure installation is test infrastructure only. A failure is recorded
    // after the same real callback; it never chooses a replacement computation.
    let memory_before = before.as_ref().map(|(_, memory, _)| memory.used_bytes());
    // SAFETY: context, site, operation, all words and result are identical to
    // the generated call; the production helper is the only authority.
    let status = unsafe {
        crate::native_artifact::program::runtime::hooks::helper(
            context, site, operation, first, second, third, result,
        )
    };
    if let Some((demand, memory, work_before)) = before {
        // SAFETY: helper returned and its exclusive borrow has ended.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let after_storage = ctx
            .state
            .collections
            .as_ref()
            .and_then(|state| state.test_text_join_storage(demand.collection));
        let row = Observation {
            before: demand,
            after_storage,
            memory_before: memory_before.unwrap_or(0),
            memory_after: memory.used_bytes(),
            work_before,
            work_after: ctx.state.control.work.used(),
            global_remaining,
            status: if pressure_failure { u64::MAX } else { status },
        };
        OBSERVATIONS.with(|rows| rows.borrow_mut().push(row));
    }
    // Exact global pressure exists ONLY around the selected real stage. Later
    // arena/publication phases get the original unchanged account capacity.
    drop(ballast);
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
    Vec<Observation>,
) {
    OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
    PRESSURE.with(|slot| slot.set(pressure));
    let entry = jit.test_checked_entry();
    let outcome = execute(
        jit.descriptor(),
        |raw| {
            // SAFETY: the JIT owner remains alive. Replace only the transport
            // observer, invoke the same actual entry, and restore before the
            // production immutable-frame check.
            let original = unsafe { (*raw).helper };
            unsafe { (*raw).helper = observe };
            let status = unsafe { entry(raw) };
            unsafe { (*raw).helper = original };
            status
        },
        args,
        outputs,
        caps,
        memory,
    );
    PRESSURE.with(|slot| slot.set(None));
    let rows = OBSERVATIONS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
    (outcome, rows)
}

fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
    let source = "+> text\ntext/join([part | part <- source<[string]>], separator<string>)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4a4f494e44454d), Revision(1), source).unwrap(),
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
fn args(artifact: &mech_engine::ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
    let text = "a".repeat(96 * 1024);
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let data = match name.as_str() {
                "source" => ValueDataDraft::Matrix(
                    // Four nonempty yields force the last geometric growth
                    // above the preceding element's construction envelope.
                    // With three, an exact growth cap can deny that earlier
                    // helper and never exercise the intended boundary.
                    (0..4)
                        .map(|_| ValueDataDraft::String(text.clone()))
                        .collect(),
                ),
                "separator" => ValueDataDraft::String("|é\0".repeat(32)),
                _ => panic!("unexpected input {name}"),
            };
            let shape_values = match &data {
                ValueDataDraft::Matrix(values) => mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[1, values.len() as u64],
                )
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
                _ => Box::new([]),
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
fn snapshot(artifact: &mech_engine::ProgramArtifact, outputs: &[Option<Value>]) -> Vec<Box<[u8]>> {
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
fn row(rows: &[Observation], target: Target) -> &Observation {
    let selected = rows
        .iter()
        .filter(|row| Target::of(&row.before) == target)
        .collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        1,
        "selected actual stage must be reached exactly once: {target:?}"
    );
    selected[0]
}
fn unchanged_payload(row: &Observation) {
    let after = row
        .after_storage
        .expect("failed stage keeps its unpublished builder until unwind");
    assert_eq!(after.0, row.before.accepted_before);
    assert_eq!(after.1, row.before.payload_before);
    assert_eq!(after.2, row.before.capacity_before);
    assert_eq!(
        row.memory_after, row.memory_before,
        "denied real reservation grants no future-output credit"
    );
    assert!(
        row.work_after >= row.work_before,
        "spent work is never refunded"
    );
}
fn required(demand: &TextJoinDemand, dimension: Dimension) -> u64 {
    match dimension {
        Dimension::TemporaryBytes => demand.temporary_required,
        Dimension::ComputeWork => demand.compute_required,
        Dimension::ClonedBytes => demand.cloned_required,
        Dimension::OutputBytes => demand.output_required,
        _ => panic!("unsupported isolated boundary"),
    }
}
fn set_cap(caps: &mut NativeProgramExecutionLimits, dimension: Dimension, amount: u64) {
    match dimension {
        Dimension::TemporaryBytes => caps.library.max_temporary_bytes = Some(amount),
        Dimension::ComputeWork => caps.library.max_compute_work = Some(amount),
        Dimension::ClonedBytes => caps.library.max_cloned_bytes = Some(amount),
        Dimension::OutputBytes => caps.library.max_output_bytes = Some(amount),
        _ => panic!("unsupported isolated boundary"),
    }
}

#[test]
fn actual_join_growth_and_finish_exact_local_caps_precede_mutation_and_recover() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let args = args(&artifact, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshot(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (accepted, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        accepted.unwrap();
        let growth = baseline
            .iter()
            .filter(|row| row.before.stage == TextJoinStage::Growth)
            .max_by_key(|row| row.before.accepted_before)
            .expect("actual growth")
            .before;
        let finish = baseline
            .iter()
            .find(|row| row.before.stage == TextJoinStage::Finish)
            .expect("actual consumer finalization")
            .before;
        assert_eq!(growth.accepted_before, 3);
        assert!(
            growth.old_backing_bytes > 0
                && growth.current_yield_bytes > 0
                && growth.separator_bytes > 0
        );
        assert!(
            growth.temporary_required
                >= growth.old_backing_bytes
                    + growth.reservation_bytes
                    + growth.current_yield_bytes
                    + growth.separator_bytes
        );
        assert_eq!(finish.accepted_before, 4);
        assert_eq!(finish.output_required, growth.output_required);
        for (demand, dimensions) in [
            (
                growth,
                &[
                    Dimension::TemporaryBytes,
                    Dimension::ComputeWork,
                    Dimension::ClonedBytes,
                    Dimension::OutputBytes,
                ][..],
            ),
            (
                finish,
                &[
                    Dimension::TemporaryBytes,
                    Dimension::ComputeWork,
                    Dimension::ClonedBytes,
                ][..],
            ),
        ] {
            let target = Target::of(&demand);
            for &dimension in dimensions {
                let bound = required(&demand, dimension);
                assert!(bound > 0);
                let mut exact = caps();
                set_cap(&mut exact, dimension, bound);
                let (outcome, rows) = observed(&jit, &args, &mut outputs, exact, &memory, None);
                assert!(
                    rows.iter().any(|row| Target::of(&row.before) == target),
                    "exact {dimension:?} cap {bound} must reach {target:?}; outcome={outcome:?}; rows={rows:?}"
                );
                assert_eq!(
                    row(&rows, target).status,
                    STATUS_OK,
                    "{dimension:?} exact stage must succeed"
                );
                // A later phase may need a higher bound than this targeted
                // growth. Never report whole-invocation acceptance from that.
                if let Err(error) = outcome {
                    assert!(
                        matches!(error, NativeProgramExecutionError::Collection {
                            error: ResidentKernelError::ResourceLimit { dimension: actual, .. }, ..
                        } if actual == dimension),
                        "{error:?}"
                    );
                }
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                let mut low = caps();
                set_cap(&mut low, dimension, bound - 1);
                let (outcome, rows) = observed(&jit, &args, &mut outputs, low, &memory, None);
                assert!(
                    rows.iter().any(|row| Target::of(&row.before) == target),
                    "one-short {dimension:?} cap {} must reach {target:?}; outcome={outcome:?}; rows={rows:?}",
                    bound - 1
                );
                let denied = row(&rows, target);
                assert_ne!(denied.status, STATUS_OK);
                let error = outcome.unwrap_err();
                let origin = if matches!(dimension, Dimension::ComputeWork | Dimension::ClonedBytes)
                {
                    demand.account_origin
                } else {
                    demand.origin
                };
                assert!(
                    matches!(error, NativeProgramExecutionError::Collection {
                        origin: actual_origin,
                        error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                            dimension: actual, required, limit }, ..
                    } if actual_origin == origin && actual == dimension && required == bound && limit == bound - 1),
                    "{error:?}"
                );
                unchanged_payload(denied);
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
                jit.evaluate_into(&args, &mut outputs, caps(), &memory)
                    .unwrap();
                assert_eq!(snapshot(&artifact, &outputs), prior);
                assert_eq!(memory.used_bytes(), stable);
            }
        }
        // Finish OutputBytes is already required by the preceding last append;
        // a one-short policy would correctly deny there, not at Finish. Nodes
        // are likewise needed at Begin. Do not mutate caps inside the observer
        // or pretend those earlier denials qualify a later-stage boundary.
        drop((outputs, args));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn actual_join_growth_and_finish_global_coexistence_boundary_has_no_credit_or_refund() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let args = args(&artifact, &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, caps(), &memory)
            .unwrap();
        let prior = snapshot(&artifact, &outputs);
        let stable = memory.used_bytes();
        let (outcome, baseline) = observed(&jit, &args, &mut outputs, caps(), &memory, None);
        outcome.unwrap();
        let growth = baseline
            .iter()
            .filter(|row| row.before.stage == TextJoinStage::Growth)
            .max_by_key(|row| row.before.accepted_before)
            .unwrap()
            .before;
        let finish = baseline
            .iter()
            .find(|row| row.before.stage == TextJoinStage::Finish)
            .unwrap()
            .before;
        for demand in [growth, finish] {
            let target = Target::of(&demand);
            assert!(demand.reservation_bytes > 0 && demand.compute_debit > 0);
            let (exact, rows) = observed(
                &jit,
                &args,
                &mut outputs,
                caps(),
                &memory,
                Some(Pressure {
                    target,
                    short: false,
                }),
            );
            exact.unwrap();
            let exact = row(&rows, target);
            assert_eq!(exact.status, STATUS_OK);
            assert_eq!(exact.global_remaining, Some(demand.reservation_bytes));
            assert_eq!(snapshot(&artifact, &outputs), prior);
            assert_eq!(memory.used_bytes(), stable);
            let (denied, rows) = observed(
                &jit,
                &args,
                &mut outputs,
                caps(),
                &memory,
                Some(Pressure {
                    target,
                    short: true,
                }),
            );
            let denied_row = row(&rows, target);
            assert_ne!(denied_row.status, STATUS_OK);
            assert_ne!(
                denied_row.status,
                u64::MAX,
                "test pressure installation must succeed"
            );
            assert_eq!(
                denied_row.global_remaining,
                Some(demand.reservation_bytes - 1)
            );
            let error = denied.unwrap_err();
            assert!(
                matches!(error, NativeProgramExecutionError::Base(NativeExecutionError::Memory {
                    error: mech_core::MemoryRuntimeError::BudgetExceeded { requested, limit, .. }
                }) if requested == memory.limit_bytes() + 1 && limit == memory.limit_bytes()),
                "{error:?}"
            );
            unchanged_payload(denied_row);
            assert!(
                denied_row.work_after - denied_row.work_before >= demand.compute_debit,
                "actual reservation denial follows its full own compute debit and never refunds it"
            );
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
