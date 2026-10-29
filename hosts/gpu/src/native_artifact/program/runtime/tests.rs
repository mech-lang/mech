use crate::native_artifact::program::runtime::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifact() -> mech_engine::ProgramArtifact {
    artifact_source("+> sequence\nsequence/length(seed<string>)\n")
}
pub(in crate::native_artifact::program::runtime) fn artifact_source(
    source: &str,
) -> mech_engine::ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x50524f54), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    mech_engine::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
pub(in crate::native_artifact::program::runtime) fn limits() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    library.max_compute_work = Some(134_217_728);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            ..NativeExecutionLimits::default()
        },
        library,
    }
}
pub(in crate::native_artifact::program::runtime) fn input(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::String("abc".into()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}
// Protocol-forgery fixtures below have no closed operations. Simulate only
// their empty entry/checkpoint before injecting a malformed TURN sequence.
fn checkpoint_empty_activation(frame: &mut Frame, descriptor: &NativeProgramDescriptor) -> bool {
    if frame.phase != crate::native_artifact::program::protocol::abi::PHASE_ACTIVATION {
        return false;
    }
    assert!(!descriptor.metadata().plan.has_activation());
    assert_eq!(
        unsafe {
            (frame.base.admit_next)(
                frame.base.context,
                descriptor.metadata().plan.entry() as u64,
            )
        },
        STATUS_OK
    );
    frame.base.error_node = u64::from(descriptor.metadata().plan.activation_boundary().unwrap());
    true
}
fn reach_call(frame: &mut Frame, descriptor: &NativeProgramDescriptor) -> u32 {
    let mut site = descriptor.metadata().plan.activation_boundary().unwrap();
    loop {
        // SAFETY: injected tests use execute's retained callback and frame.
        assert_eq!(
            unsafe { (frame.base.admit_next)(frame.base.context, site as u64) },
            STATUS_OK
        );
        let block = &descriptor.metadata().plan.blocks()[site as usize];
        if matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
            return site;
        }
        let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
            block.terminator()
        else {
            panic!("expected straight-line fixture")
        };
        site = next;
    }
}
#[test]
fn helper_protocol_denies_wrong_site_kind_pointer_duplicate_and_omission_atomically() {
    let artifact = artifact();
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    for mode in 0..9 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [input(&artifact, &memory)];
        let retained = memory.used_bytes();
        let mut outputs = vec![None; artifact.outputs().len()];
        let error = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: execute owns this initialized live frame throughout
                // the synchronous injected test entry; backing stays disjoint.
                let frame = unsafe { &mut *raw };
                if checkpoint_empty_activation(frame, jit.descriptor()) {
                    return crate::native_artifact::program::protocol::abi::STATUS_ACTIVATED;
                }
                let site = reach_call(frame, jit.descriptor());
                let (actual_site, operation, result) = match mode {
                    0 => (site as u64 + 1, LIBRARY, frame.result),
                    1 => (site as u64, MOVE, frame.result),
                    2 => (site as u64, LIBRARY, frame.result.wrapping_add(1)),
                    _ => (site as u64, LIBRARY, frame.result),
                };
                if mode == 4 {
                    let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                        jit.descriptor().metadata().plan.blocks()[site as usize].terminator()
                    else {
                        panic!("fixture terminator")
                    };
                    // SAFETY: same retained callback/context, deliberately
                    // skipping the required leaf to test protocol denial.
                    return unsafe { (frame.base.admit_next)(frame.base.context, next as u64) };
                }
                let first = if mode == 5 {
                    frame.arguments.wrapping_add(1) as u64
                } else {
                    frame.arguments as u64
                };
                let arity = match jit.descriptor().metadata().plan.blocks()[site as usize].action()
                {
                    NativeIterationAction::LibraryCall { operands, .. } => operands.len() as u64,
                    _ => unreachable!(),
                };
                let second = if mode == 6 { arity + 1 } else { arity };
                let third = u64::from(mode == 7);
                if mode == 8 {
                    // SAFETY: this is the owned, initialized first operand
                    // word of text/lines; managed operands must be zero.
                    unsafe {
                        *frame.arguments = 1;
                    }
                }
                // SAFETY: arguments remain within the retained ABI16 boundary;
                // wrong metadata/pointer is rejected before dereference.
                let status = unsafe {
                    (frame.helper)(
                        frame.base.context,
                        actual_site,
                        operation,
                        first,
                        second,
                        third,
                        result,
                    )
                };
                if mode == 3 {
                    assert_eq!(status, STATUS_OK);
                    // SAFETY: same live context/result lane, deliberate repeat.
                    unsafe {
                        (frame.helper)(
                            frame.base.context,
                            actual_site,
                            operation,
                            first,
                            second,
                            0,
                            result,
                        )
                    }
                } else {
                    status
                }
            },
            &args,
            &mut outputs,
            limits(),
            &memory,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
            ),
            "mode {mode}: {error:?}"
        );
        assert!(outputs.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), retained);
    }
}
#[test]
fn phase_protocol_rejects_forged_early_checkpoint_completion_and_frame_phase() {
    let artifact = artifact();
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    for mode in 0..4 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [input(&artifact, &memory)];
        let before = memory.used_bytes();
        let mut output = vec![None; artifact.outputs().len()];
        let result = execute(
            jit.descriptor(),
            |raw| {
                // SAFETY: the test receives this synchronous owned frame only.
                let frame = unsafe { &mut *raw };
                let boundary = jit
                    .descriptor()
                    .metadata()
                    .plan
                    .activation_boundary()
                    .unwrap();
                if mode == 0 {
                    frame.phase = crate::native_artifact::program::protocol::abi::PHASE_TURN;
                } else if mode == 1 {
                    return STATUS_OK;
                } else if mode == 2 {
                    // No admitted activation entry, despite a forged origin.
                    frame.base.error_node = u64::from(boundary);
                } else {
                    return unsafe {
                        (frame.base.admit_next)(frame.base.context, u64::from(boundary))
                    };
                }
                crate::native_artifact::program::protocol::abi::STATUS_ACTIVATED
            },
            &args,
            &mut output,
            limits(),
            &memory,
        );
        assert!(result.is_err(), "forged phase mode {mode}");
        assert!(output.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), before);
    }
}

#[test]
fn setup_denies_before_entry_and_before_foreign_payload_type_scan() {
    let artifact = artifact();
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let foreign = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = [input(&artifact, &foreign)];
    for zero_work in [false, true] {
        let memory = ManagedMemoryBudget::new(0);
        let called = std::cell::Cell::new(false);
        let mut outputs = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        if zero_work {
            policy.execution.max_work = 0;
        }
        let error = execute(
            jit.descriptor(),
            |_| {
                called.set(true);
                STATUS_ABI
            },
            &args,
            &mut outputs,
            policy,
            &memory,
        )
        .unwrap_err();
        assert!(!called.get());
        if zero_work {
            assert!(matches!(
                error,
                NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit { .. })
            ));
        } else {
            assert!(matches!(
                error,
                NativeProgramExecutionError::Base(NativeExecutionError::Memory { .. })
            ));
        }
        assert!(outputs.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[test]
fn independent_program_entry_cannot_refill_an_active_library_account() {
    let artifact = artifact();
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let args = [input(&artifact, &memory)];
    let mut work = NativeWorkMeter::new(1000).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    let before = (library.used(), memory.used_bytes());
    library
        .with_scope(|| {
            let mut outputs = vec![None; artifact.outputs().len()];
            assert!(matches!(
                jit.evaluate_into(&args, &mut outputs, limits(), &memory),
                Err(NativeProgramExecutionError::Base(
                    NativeExecutionError::Descriptor {
                        reason: "independent native work ledger cannot replace an active invocation"
                    }
                ))
            ));
            assert!(outputs.iter().all(Option::is_none));
        })
        .unwrap();
    assert_eq!((library.used(), memory.used_bytes()), before);
}
#[test]
fn collection_protocol_denies_forged_omitted_repeated_and_early_visits_atomically() {
    let artifact = artifact_source("[letter | letter <- seed<string>]\n");
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    let collection = &descriptor.metadata().plan.collections()[0];
    let mech_engine::native_artifact::NativeControlLoop::Collection(generator) =
        &descriptor.metadata().plan.iterations()[collection.generators()[0] as usize]
    else {
        panic!("collection generator")
    };
    for mode in 0..9 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = [input(&artifact, &memory)];
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs[0]
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap();
        let retained = memory.used_bytes();
        let error = execute(
            descriptor,
            |raw| {
                // SAFETY: execute retains this initialized exclusive frame,
                // callback context and disjoint result lane until return.
                let frame = unsafe { &mut *raw };
                if checkpoint_empty_activation(frame, descriptor) {
                    return crate::native_artifact::program::protocol::abi::STATUS_ACTIVATED;
                }
                let admit = |site| {
                    // SAFETY: the same live private callback/context; only
                    // visit metadata is varied, never pointer ownership.
                    unsafe { (frame.base.admit_next)(frame.base.context, site as u64) }
                };
                let invoke = |site, operation, result| {
                    // SAFETY: retained private helper/context; a deliberately
                    // wrong result pointer is checked before dereference.
                    unsafe {
                        (frame.helper)(frame.base.context, site as u64, operation, 0, 0, 0, result)
                    }
                };
                let next = |site: u32| {
                    let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                        descriptor.metadata().plan.blocks()[site as usize].terminator()
                    else {
                        panic!("straight-line collection setup")
                    };
                    next
                };
                let mut site = descriptor.metadata().plan.activation_boundary().unwrap();
                loop {
                    assert_eq!(admit(site), STATUS_OK);
                    if site == collection.begin() {
                        break;
                    }
                    site = next(site);
                }
                match mode {
                    0 => return invoke(site + 1, COLLECTION, frame.result),
                    1 => return invoke(site, MOVE, frame.result),
                    2 => return invoke(site, COLLECTION, frame.result.wrapping_add(1)),
                    4 => return admit(next(site)),
                    _ => {}
                }
                assert_eq!(invoke(site, COLLECTION, frame.result), STATUS_OK);
                if mode == 3 {
                    return invoke(site, COLLECTION, frame.result);
                }
                loop {
                    site = next(site);
                    assert_eq!(admit(site), STATUS_OK);
                    if site == generator.initialize() {
                        break;
                    }
                }
                if mode == 5 {
                    return admit(generator.header());
                }
                assert_eq!(invoke(site, COLLECTION, frame.result), STATUS_OK);
                if mode == 6 {
                    return invoke(site, COLLECTION, frame.result);
                }
                assert_eq!(admit(generator.header()), STATUS_OK);
                if mode == 7 {
                    return admit(generator.exit());
                }
                assert_eq!(admit(generator.body()), STATUS_OK);
                admit(next(generator.body()))
            },
            &args,
            &mut outputs,
            limits(),
            &memory,
        )
        .unwrap_err();
        if mode == 7 {
            assert!(
                matches!(error, NativeProgramExecutionError::Control(
                        NativeIterationExecutionError::Visit { previous: Some(previous), actual }
                    ) if previous == generator.header() && actual == u64::from(generator.exit())),
                "mode {mode}: {error:?}"
            );
        } else {
            assert!(
                matches!(
                    error,
                    NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
                ),
                "mode {mode}: {error:?}"
            );
        }
        assert_eq!(memory.used_bytes(), retained, "mode {mode}");
        assert_eq!(
            outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior,
            "mode {mode}"
        );
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
    }
}
#[test]
fn range_cursor_protocol_rejects_wrong_skipped_duplicate_and_malformed_preparation_atomically() {
    use mech_engine::native_artifact::{NativeCollectionSource, NativeControlLoop};
    let artifact = artifact_source("[i | i <- 0u64..count<u64>]\n");
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    let prepare = descriptor
        .metadata()
        .plan
        .iterations()
        .iter()
        .find_map(|data| match data {
            NativeControlLoop::Collection(data) => match data.source() {
                NativeCollectionSource::U64Range(cursor) => Some(cursor.prepare()),
                _ => None,
            },
            _ => None,
        })
        .expect("fixture really selects the source cursor");
    for mode in 0..6 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let value = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(4),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut admission = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let args = [value.into_memory_budget(&mut admission).unwrap()];
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs[0]
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap();
        let before = memory.used_bytes();
        let error = execute(
            descriptor,
            |raw| {
                // SAFETY: execute owns and retains this private frame and all
                // callbacks; only the deliberately invalid sequence varies.
                let frame = unsafe { &mut *raw };
                if checkpoint_empty_activation(frame, descriptor) {
                    return crate::native_artifact::program::protocol::abi::STATUS_ACTIVATED;
                }
                let mut site = descriptor.metadata().plan.activation_boundary().unwrap();
                let next = |site: u32| {
                    let NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) =
                        descriptor.metadata().plan.blocks()[site as usize].terminator()
                    else {
                        panic!("straight range setup")
                    };
                    next
                };
                loop {
                    assert_eq!(
                        unsafe { (frame.base.admit_next)(frame.base.context, u64::from(site)) },
                        STATUS_OK
                    );
                    if site == prepare {
                        break;
                    }
                    match descriptor.metadata().plan.blocks()[site as usize].action() {
                        NativeIterationAction::CollectionBegin { .. } => {
                            // Inline ranges are prepared inside a live
                            // builder. Consume its real required helper
                            // before testing a malformed preparation.
                            assert_eq!(
                                unsafe {
                                    (frame.helper)(
                                        frame.base.context,
                                        u64::from(site),
                                        COLLECTION,
                                        0,
                                        0,
                                        0,
                                        frame.result,
                                    )
                                },
                                STATUS_OK
                            );
                        }
                        NativeIterationAction::Scalar(NativeMatchAction::None) => {}
                        other => panic!("unexpected range protocol setup action: {other:?}"),
                    }
                    site = next(site);
                }
                if mode == 3 {
                    return unsafe {
                        (frame.base.admit_next)(frame.base.context, u64::from(next(site)))
                    };
                }
                let called_site = if mode == 0 { site + 1 } else { site };
                let operation = if mode == 1 { MOVE } else { COLLECTION };
                let result = if mode == 2 {
                    frame.result.wrapping_add(1)
                } else {
                    frame.result
                };
                let third = u64::from(mode == 4);
                let status = unsafe {
                    (frame.helper)(
                        frame.base.context,
                        u64::from(called_site),
                        operation,
                        0,
                        4,
                        third,
                        result,
                    )
                };
                if mode != 5 {
                    return status;
                }
                assert_eq!(status, STATUS_OK);
                unsafe {
                    (frame.helper)(
                        frame.base.context,
                        u64::from(site),
                        COLLECTION,
                        0,
                        4,
                        0,
                        frame.result,
                    )
                }
            },
            &args,
            &mut outputs,
            limits(),
            &memory,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                NativeProgramExecutionError::Base(NativeExecutionError::Descriptor { .. })
            ),
            "mode {mode}: {error:?}"
        );
        assert_eq!(memory.used_bytes(), before, "mode {mode}");
        assert_eq!(
            outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
        jit.evaluate_into(&args, &mut outputs, limits(), &memory)
            .unwrap();
        assert_eq!(
            outputs[0]
                .as_ref()
                .unwrap()
                .canonical_snapshot_bytes(artifact.schemas())
                .unwrap(),
            prior
        );
        drop((outputs, args, admission));
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[derive(Clone, Copy, Debug)]
struct RangeScratchObservation {
    origin: NativeMatchOrigin,
    live_bytes: u64,
    scratch_bytes: u64,
    binding_work: u64,
    memory_before: u64,
    memory_after: u64,
    work_before: u64,
    work_after: u64,
    status: u64,
}
thread_local! {
    static RANGE_SCRATCH_OBSERVATIONS: std::cell::RefCell<Vec<RangeScratchObservation>> = const { std::cell::RefCell::new(Vec::new()) };
}
// Transparent test observation only: every call delegates unchanged to
// the production helper. No generator, body, branch or value executes here.
unsafe extern "C" fn observe_range_scratch(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let before = (|| {
        // SAFETY: the actual generated entry forwards execute's retained
        // private context. This shared borrow ends before helper mutates it.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let site = usize::try_from(site).ok()?;
        let block = ctx.descriptor.metadata().plan.blocks().get(site)?;
        if !matches!(
            block.action(),
            NativeIterationAction::PrepareU64Range { .. }
        ) {
            return None;
        }
        let selected = ctx.descriptor.metadata().calls.get(site)?.as_ref()?;
        Some(RangeScratchObservation {
            origin: block.origin(),
            live_bytes: ctx.state.collections.as_ref()?.test_live_bytes(),
            scratch_bytes: selected.binding_metadata_bytes(),
            binding_work: selected.binding_work(),
            memory_before: ctx.state.library.memory_budget().used_bytes(),
            memory_after: 0,
            work_before: ctx.state.control.work.used(),
            work_after: 0,
            status: 0,
        })
    })();
    // SAFETY: identical checked context, site, operation, words and result
    // pointer; the production helper remains the only execution authority.
    let status = unsafe {
        crate::native_artifact::program::runtime::hooks::helper(
            context, site, operation, first, second, third, result,
        )
    };
    if let Some(mut observation) = before {
        // SAFETY: helper has returned and its exclusive borrow ended.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        observation.memory_after = ctx.state.library.memory_budget().used_bytes();
        observation.work_after = ctx.state.control.work.used();
        observation.status = status;
        RANGE_SCRATCH_OBSERVATIONS.with(|rows| rows.borrow_mut().push(observation));
    }
    status
}
fn execute_observed_range_scratch(
    jit: &crate::native_artifact::JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    caps: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Vec<RangeScratchObservation>,
) {
    RANGE_SCRATCH_OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
    let entry = jit.test_checked_entry();
    let outcome = execute(
        jit.descriptor(),
        |raw| {
            // SAFETY: this test retains the exact checked JIT owner. Replace
            // only the transport observer for this synchronous entry and
            // restore the immutable wire before execute verifies the frame.
            let original = unsafe { (*raw).helper };
            unsafe {
                (*raw).helper = observe_range_scratch;
            }
            let status = unsafe { entry(raw) };
            unsafe {
                (*raw).helper = original;
            }
            status
        },
        args,
        outputs,
        caps,
        memory,
    );
    let observations =
        RANGE_SCRATCH_OBSERVATIONS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
    (outcome, observations)
}
#[test]
fn nested_range_scratch_exact_caps_precede_traversal_and_preserve_coexisting_builder_backing() {
    use mech_core::{
        MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
    };
    let artifact = artifact_source("[i + j | i <- 1u64..=outer<u64>, j <- 1u64..=i]\n");
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let value = ValueDraft {
        schema: artifact.inputs()[0].schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(3),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut admission = memory
        .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
        .unwrap();
    let args = [value.into_memory_budget(&mut admission).unwrap()];
    let mut outputs = vec![None; artifact.outputs().len()];
    jit.evaluate_into(&args, &mut outputs, limits(), &memory)
        .unwrap();
    let prior = outputs
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_snapshot_bytes(artifact.schemas())
        .unwrap();
    let stable = memory.used_bytes();
    let (baseline, rows) =
        execute_observed_range_scratch(&jit, &args, &mut outputs, limits(), &memory);
    baseline.unwrap();
    assert_eq!(
        rows.len(),
        4,
        "one outer source plus a fresh inner source for each of three visits"
    );
    let fixed_metadata =
        crate::native_artifact::program::runtime::collections::State::metadata_bytes(
            jit.descriptor(),
        )
        .unwrap();
    assert!(
        rows.iter().any(|row| row.live_bytes > fixed_metadata),
        "inner preparation must coexist with already-retained builder drafts"
    );
    for row in &rows {
        assert_eq!(row.status, STATUS_OK);
        assert_eq!(
            row.memory_after, row.memory_before,
            "temporary scratch retires before the helper returns"
        );
        assert_eq!(
            row.work_after - row.work_before,
            row.binding_work,
            "the unchanged full debit precedes the traversal/allocation"
        );
    }
    let temporary = rows
        .iter()
        .map(|row| row.live_bytes.checked_add(row.scratch_bytes).unwrap())
        .max()
        .unwrap();
    let compute = rows.iter().map(|row| row.binding_work).max().unwrap();
    let global_peak = rows
        .iter()
        .map(|row| row.memory_before.checked_add(row.scratch_bytes).unwrap())
        .max()
        .unwrap();
    let mut exact = limits();
    exact.library.max_temporary_bytes = Some(temporary);
    exact.library.max_compute_work = Some(compute);
    let (accepted, exact_rows) =
        execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
    accepted.unwrap();
    assert_eq!(exact_rows.len(), rows.len());
    assert_eq!(memory.used_bytes(), stable);
    let mut low = exact;
    low.library.max_temporary_bytes = Some(temporary - 1);
    let (denied, denied_rows) =
        execute_observed_range_scratch(&jit, &args, &mut outputs, low, &memory);
    let last = denied_rows
        .last()
        .expect("the denial must occur at actual range preparation");
    let error = denied.unwrap_err();
    assert!(
        matches!(error, NativeProgramExecutionError::Collection { origin,
            error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                dimension: Dimension::TemporaryBytes, required, limit }
        } if origin == last.origin && required == temporary && limit == temporary - 1),
        "{error:?}"
    );
    assert_eq!(
        last.work_after, last.work_before,
        "a denied cap does not start schema traversal"
    );
    assert_eq!(last.memory_after, last.memory_before);
    assert_eq!(memory.used_bytes(), stable);
    assert_eq!(
        outputs
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap(),
        prior
    );
    low = exact;
    low.library.max_compute_work = Some(compute - 1);
    let (denied, denied_rows) =
        execute_observed_range_scratch(&jit, &args, &mut outputs, low, &memory);
    let last = denied_rows.last().unwrap();
    let error = denied.unwrap_err();
    assert!(
        matches!(error, NativeProgramExecutionError::Collection { origin,
            error: ResidentKernelError::ResourceLimit { phase: ResidentResourcePhase::Admission,
                dimension: Dimension::ComputeWork, required, limit }
        } if origin == last.origin && required == compute && limit == compute - 1),
        "{error:?}"
    );
    assert_eq!(last.work_after, last.work_before);
    assert_eq!(last.memory_after, last.memory_before);
    assert_eq!(memory.used_bytes(), stable);
    // Account-wide coexistence is independently enforced, not replaced by
    // the local TemporaryBytes cap. Leave exactly the measured real peak.
    let mut ballast = memory
        .reserve_capacity(memory.limit_bytes() - global_peak)
        .unwrap();
    let (accepted, _) = execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
    accepted.unwrap();
    ballast
        .resize_capacity(ballast.capacity_bytes() + 1)
        .unwrap();
    let (denied, denied_rows) =
        execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
    let last = denied_rows
        .last()
        .expect("one byte less must deny the actual scratch reservation");
    let error = denied.unwrap_err();
    assert!(
        matches!(error, NativeProgramExecutionError::Base(NativeExecutionError::Memory {
            error: mech_core::MemoryRuntimeError::BudgetExceeded { requested, limit, .. }
        }) if requested == memory.limit_bytes() + 1 && limit == memory.limit_bytes()),
        "{error:?}"
    );
    assert_eq!(
        last.work_after - last.work_before,
        last.binding_work,
        "failed reservation never refunds the preceding work debit"
    );
    assert_eq!(
        last.memory_after, last.memory_before,
        "failed reservation changes no owner or Ready slot backing"
    );
    assert_eq!(
        outputs
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap(),
        prior
    );
    drop(ballast);
    assert_eq!(memory.used_bytes(), stable);
    let (recovered, _) = execute_observed_range_scratch(&jit, &args, &mut outputs, exact, &memory);
    recovered.unwrap();
    assert_eq!(
        outputs
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_snapshot_bytes(artifact.schemas())
            .unwrap(),
        prior
    );
    drop((args, outputs, admission));
    assert_eq!(memory.used_bytes(), 0);
}
#[test]
fn managed_link_metadata_rederives_semantics_and_rejects_mutations_without_an_entry() {
    let source = "+> text\n+> sequence\n+> number\nwords := text/lines(seed<string>)\ntext/join([number/format(sequence/length(word), 0u64) | word <- words], \",\")\n";
    let original = artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    let different = artifact_source(&source.replace("0u64", "1u64"));
    let catalog = mech_stdlib::source_catalog();
    let empty = mech_core::FunctionCatalogBuilder::new().build().unwrap();
    let prepare = crate::native_artifact::program::linked::prepare_descriptor;
    for artifact in [original, decoded] {
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let identity = *object.identity().bytes();
        let descriptor = prepare(&artifact, &catalog, &identity).unwrap();
        assert_eq!(
            descriptor.typed_ir_sha256(),
            object.descriptor().typed_ir_sha256()
        );
        for offset in 0..identity.len() {
            // These two nonzero resource claims require build-owned Object
            // provenance, tested by the real linked pin-mutation qualifier.
            // They are not falsely treated as runtime-rederived semantics.
            if (112..128).contains(&offset) {
                continue;
            }
            let mut mutated = identity;
            mutated[offset] ^= 1;
            assert!(
                prepare(&artifact, &catalog, &mutated).is_err(),
                "identity byte {offset}"
            );
        }
        for (offset, old) in [
            (16, 1u64),
            (24, 4u64),
            (24, 5u64),
            (24, 6u64),
            (24, 7u64),
            (16, 4u64),
            (32, 7u64),
            (32, 8u64),
            (32, 9u64),
            (32, 10u64),
            (32, 1u64),
            (32, 2u64),
            (32, 3u64),
            (32, 4u64),
            (16, 24u64),
            (16, 25u64),
            (24, 11u64),
            (32, 34u64),
            (112, 0u64),
            (120, 0u64),
        ] {
            let mut mutated = identity;
            mutated[offset..offset + 8].copy_from_slice(&old.to_le_bytes());
            assert!(
                prepare(&artifact, &catalog, &mutated).is_err(),
                "identity field {offset}"
            );
        }
        assert!(prepare(&different, &catalog, &identity).is_err());
        assert!(prepare(&artifact, &empty, &identity).is_err());
    }
}
#[test]
fn worker_abi_keeps_the_frozen_scalar_prefix_and_disjoint_helper_lanes() {
    assert_eq!(VERSION, 34);
    assert_eq!(
        crate::native_artifact::program::protocol::abi::PHASE_CAPTURE_TAIL,
        3
    );
    assert_eq!(
        crate::native_artifact::program::protocol::abi::STATUS_INPUT_SCANNED,
        7
    );
    assert_eq!(core::mem::offset_of!(Frame, base), 0);
    assert_eq!(
        core::mem::offset_of!(Frame, helper),
        core::mem::size_of::<NativeEntryFrame>()
    );
    assert_eq!(
        core::mem::offset_of!(Frame, result),
        core::mem::size_of::<NativeEntryFrame>() + 8
    );
    assert_eq!(
        core::mem::offset_of!(Frame, arguments),
        core::mem::size_of::<NativeEntryFrame>() + 16
    );
    assert_eq!(
        core::mem::offset_of!(Frame, argument_capacity),
        core::mem::size_of::<NativeEntryFrame>() + 24
    );
    assert_eq!(
        core::mem::offset_of!(Frame, phase),
        core::mem::size_of::<NativeEntryFrame>() + 32
    );
    assert_eq!(
        core::mem::offset_of!(Frame, resume_site),
        core::mem::size_of::<NativeEntryFrame>() + 40
    );
    assert_eq!(
        core::mem::offset_of!(Frame, available_lines),
        core::mem::size_of::<NativeEntryFrame>() + 48
    );
    assert_eq!(
        core::mem::offset_of!(Frame, input_eof),
        core::mem::size_of::<NativeEntryFrame>() + 56
    );
    assert_eq!(
        core::mem::size_of::<Frame>(),
        core::mem::size_of::<NativeEntryFrame>() + 64
    );
}

#[path = "numeric_fuel_tests.rs"]
mod numeric_fuel;

thread_local! {
    static NUMERIC_FUEL_OBSERVATIONS: std::cell::RefCell<Vec<Vec<u64>>> = const { std::cell::RefCell::new(Vec::new()) };
    static NUMERIC_NORMAL_EXIT_WORK: std::cell::RefCell<Vec<u64>> = const { std::cell::RefCell::new(Vec::new()) };
}
// Observe the real cold reconciliation without changing fuel, source execution,
// arguments, or the account. Unsafe access stays in this existing audited owner.
unsafe extern "C" fn observe_numeric_fuel(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let closing = operation == crate::native_artifact::program::protocol::abi::CLOSED_TAIL_END
        && matches!(third, 0 | 1);
    // SAFETY: live execute-owned context; this read ends before helper borrows it.
    let before = closing.then(|| {
        unsafe { &*context.cast::<Invocation<'_, '_, '_>>() }
            .state
            .library
            .used()
            .0
    });
    // SAFETY: identical synchronous context and words from the generated entry.
    let status =
        unsafe { super::hooks::helper(context, site, operation, first, second, third, result) };
    if let Some(before) = before
        && status == STATUS_OK
    {
        // SAFETY: the production helper returned; its exclusive borrow ended.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        if third == 0 {
            NUMERIC_NORMAL_EXIT_WORK
                .with(|rows| rows.borrow_mut().push(ctx.state.library.used().0));
            return status;
        }
        let metadata = ctx.descriptor.metadata();
        if let Some(id) = metadata
            .closed_tails
            .sites
            .get(site as usize)
            .copied()
            .flatten()
            && let Some(region) = metadata.closed_tails.regions.get(id)
            && region.numeric.is_some()
            && let Some(fields) = metadata.buffer_frames.fields(region.tail.carrier())
        {
            let mut bits = vec![
                ctx.state.library.used().0 - before,
                region.compute,
                u64::from(ctx.closed_tail.is_none()),
            ];
            bits.extend(fields.iter().enumerate().filter_map(|(ordinal, _)| {
                ctx.state.buffer_frames.test_scalar_bits(
                    &metadata.buffer_frames,
                    region.tail.carrier(),
                    ordinal,
                )
            }));
            if let Some(proof) = &region.numeric
                && proof.writes()
            {
                for array in &proof.arrays {
                    let owner = ctx
                        .state
                        .buffer_frames
                        .managed_owner(&metadata.buffer_frames, region.tail.carrier(), array.field)
                        .unwrap();
                    let ValueData::Matrix(matrix) = owner.data() else {
                        panic!("numeric fuel owner is not a matrix")
                    };
                    let mech_core::snapshot::SequenceView::U32(words) = matrix.elements() else {
                        panic!("numeric write fuel fixture is U32")
                    };
                    bits.push(words.len() as u64);
                    bits.extend(words.iter().copied().map(u64::from));
                }
            }
            NUMERIC_FUEL_OBSERVATIONS.with(|rows| rows.borrow_mut().push(bits));
        }
    }
    status
}
fn execute_observed_numeric_fuel(
    jit: &crate::native_artifact::JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    caps: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Vec<Vec<u64>>,
) {
    NUMERIC_FUEL_OBSERVATIONS.with(|rows| rows.borrow_mut().clear());
    NUMERIC_NORMAL_EXIT_WORK.with(|rows| rows.borrow_mut().clear());
    let entry = jit.test_checked_entry();
    let outcome = execute(
        jit.descriptor(),
        |raw| {
            // SAFETY: exact retained JIT entry and live execute-owned frame. The
            // observer forwards every helper unchanged, then restores the wire.
            let original = unsafe { (*raw).helper };
            unsafe {
                (*raw).helper = observe_numeric_fuel;
            }
            let status = unsafe { entry(raw) };
            unsafe {
                (*raw).helper = original;
            }
            status
        },
        args,
        outputs,
        caps,
        memory,
    );
    let rows = NUMERIC_FUEL_OBSERVATIONS.with(|rows| core::mem::take(&mut *rows.borrow_mut()));
    (outcome, rows)
}
