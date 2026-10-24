//! Terminal origins of source failures and a forged state tag in packed SSA.
//! Every case must enter packed SSA; an ordinary fallback cannot qualify it.
use super::*;
use mech_engine::native_artifact::NativeCollectionPrimitive as P;

#[derive(Clone, Copy, Debug)]
enum Failure { Add, Subtract, Divide, Reject, Selection, MissingKey, MalformedHelper, Refill, ReadCap, WriteCap, InsertAfterGrowth }

type Bound = unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, u64, *mut u64) -> u64;

thread_local! {
    static TAG_FAULT: Cell<Option<(*mut Frame, usize)>> = const { Cell::new(None) };
    static HELPER_FAULT: Cell<bool> = const { Cell::new(false) };
    static ORIGINAL_BOUND: Cell<Option<Bound>> = const { Cell::new(None) };
    static FALSE_SUCCESS: Cell<u64> = const { Cell::new(0) };
}
unsafe extern "C" fn incorrectly_succeed(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    if op == 8 {
        // Deliberately violate the private failure-helper contract. Generated
        // code must refuse to resume the unchecked missing-key memory access.
        FALSE_SUCCESS.with(|calls| calls.set(calls.get() + 1));
        return STATUS_OK;
    }
    let original = ORIGINAL_BOUND.with(Cell::get).expect("owned test bound function");
    // SAFETY: the test saved this exact function/pointer/ABI pair at entry.
    unsafe { original(raw, site, op, first, second, third, result) }
}
unsafe extern "C" fn observe_origin(raw: *mut c_void, site: u64, op: u64,
    first: u64, second: u64, third: u64, result: *mut u64) -> u64
{
    // SAFETY: delegate the exact observer call, without changing its operands.
    let status = unsafe { observe(raw, site, op, first, second, third, result) };
    if op == CLOSED_TAIL_BEGIN && status == STATUS_OK && unsafe { *result } == 1 {
        if let Some((frame, word)) = TAG_FAULT.with(|fault| fault.take()) {
            // SAFETY: this test pins its frame throughout entry, and the slot
            // came from the selected owner's carrier layout. Corrupt only the
            // enum tag after initialization, before its first SSA load.
            let frame = unsafe { &mut *frame };
            if word < frame.base.local_count as usize {
                unsafe { *frame.base.locals.add(word) = u64::MAX; }
            } else { note(|r| r.errors.push("test carrier slot outside frame")); }
        }
        if HELPER_FAULT.with(|fault| fault.replace(false)) {
            // SAFETY: the maintained begin just returned four private words
            // to this pinned session; word 2 is its exact bound callback.
            let ctx = unsafe { &mut *raw.cast::<Context<'_, '_>>() };
            if ctx.argument_capacity >= 4 && !ctx.arguments.is_null() {
                let original = unsafe { *ctx.arguments.add(2) };
                // SAFETY: this is the original maintained callback word, not
                // arbitrary source data; only the same checked test can use it.
                ORIGINAL_BOUND.with(|function| function.set(Some(unsafe { core::mem::transmute::<usize, Bound>(original as usize) })));
                unsafe { *ctx.arguments.add(2) = incorrectly_succeed as *const () as u64; }
            } else { note(|r| r.errors.push("test bound function lane outside frame")); }
        }
    }
    status
}

#[test]
fn packed_ssa_cold_errors_keep_source_status_origin_and_cleanup() {
    for failure in [Failure::Add, Failure::Subtract, Failure::Divide, Failure::Reject,
        Failure::Selection, Failure::MissingKey, Failure::MalformedHelper, Failure::Refill, Failure::ReadCap,
        Failure::WriteCap, Failure::InsertAfterGrowth]
    {
        let mut body = structural_projection_tests::COUNT.to_owned();
        match failure {
            Failure::Add => body = body.replace("previous + 1u64", "previous + 18446744073709551615u64"),
            Failure::Subtract => body = body.replace("previous + 1u64", "previous - 1u64"),
            Failure::Divide => body = body.replace("previous + 1u64", "1u64 / previous"),
            Failure::MissingKey | Failure::MalformedHelper => body = body.replace("(key ∈ h)? | true => h{key} | false => 0u64.", "h{key}"),
            _ => {}
        }
        let call = if matches!(failure, Failure::Selection) {
            "#Count(\"abc\", 2u64, 4u64, empty)"
        } else if matches!(failure, Failure::InsertAfterGrowth) {
            "#Count(\"abcdefghijklmnopq\", 1u64, 17u64, empty)"
        } else { "#Count(\"aa\", 1u64, 2u64, empty)" };
        let original = tests::artifact_source(&format!("{body}\n{call}\n"));
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let catalog = mech_stdlib::source_catalog();
            use crate::native_artifact::program_compiler::packed_loop_codegen::{take_edge_fact_emissions,
                take_tag_thread_emissions, take_frame_effect_emissions, without_frame_effects, take_loop_bounds_emissions};
            let mut executions = Vec::new();
            for disabled in [false, true] {
                take_edge_fact_emissions();
                take_tag_thread_emissions();
                take_frame_effect_emissions();
                take_loop_bounds_emissions();
                let compile = || crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog);
                let jit = if disabled { without_frame_effects(compile) } else { compile() }.unwrap();
                let emissions = take_edge_fact_emissions();
                assert!(emissions.0 >= 8, "{failure:?}: error fixture must use the edge-fact substitutions: {emissions:?}");
                let threading = take_tag_thread_emissions();
                assert!(threading.0 > 0 && threading.1 >= 4 && threading.2 >= 2,
                    "{failure:?}: error fixture must use tag threading: {threading:?}");
                assert!(take_loop_bounds_emissions().0 > 0,
                    "{failure:?}: guarded graph must be present even for unknown tags and failed runtime selectors");
                let effects = take_frame_effect_emissions();
                if disabled { assert_eq!(effects, (0, 0, 0)); }
                else { assert!(effects.0 > 0 && effects.1 < 14 * effects.0 && effects.2 < 14 * effects.0); }
                let image = jit.descriptor().encode_build_owned_image().unwrap();
                // SAFETY: the image is from this exact still-owned generated entry.
                let restored = unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
                for descriptor in [jit.descriptor(), &restored] {
                    let region = descriptor.closed_tails.regions.iter().find(|r| r.packed.is_some())
                        .expect("fixture must retain its packed proof");
                    let owner = descriptor.map_owner_words.as_ref().unwrap();
                    let span = owner.spans().iter().find(|s| Some(s.region()) == region.map).unwrap();
                    let tag_word = owner.aggregates().slot(region.tail.carrier()).unwrap().0;
                    let call_site = |primitive| descriptor.plan.blocks().iter().enumerate().find_map(|(site, block)| {
                        match block.action() {
                            NativeIterationAction::LibraryCall { template, .. } if template.primitive() == primitive => Some(site),
                            _ => None,
                        }
                    }).expect("fixture source call");
                    let replacement_site = || {
                        let NativeIterationAction::LibraryCall { operands, .. } =
                            descriptor.plan.blocks()[call_site(P::MapInsert)].action() else { unreachable!() };
                        descriptor.plan.blocks().iter().position(|block| matches!(block.action(),
                            NativeIterationAction::Scalar(NativeMatchAction::Primitive { output, .. })
                                if *output == operands[2])).expect("source replacement arithmetic")
                    };
                    let expected_site = match failure {
                        Failure::Add | Failure::Subtract | Failure::Divide => Some(replacement_site()),
                        Failure::Selection => Some(span.range_site() as usize),
                        Failure::MissingKey | Failure::MalformedHelper => Some(call_site(P::MapKeyAccess)),
                        Failure::Refill => Some(region.tail.header() as usize),
                        Failure::ReadCap => Some(call_site(P::MapContainsKey)),
                        Failure::WriteCap | Failure::InsertAfterGrowth => Some(call_site(P::MapInsert)),
                        Failure::Reject => None,
                    };
                    MODE.with(|mode| mode.set(match failure {
                        Failure::Refill => Mode::OneShort,
                        Failure::ReadCap => Mode::TightCall,
                        Failure::WriteCap => Mode::TightWrite,
                        Failure::InsertAfterGrowth => Mode::LimitedInsert,
                        _ => Mode::Native,
                    }));
                    OBS.with(|v| *v.borrow_mut() = Observation::default());
                    FALSE_SUCCESS.with(|calls| calls.set(0));
                    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                    let mut output = vec![None];
                    let terminal = Cell::new(None);
                    let entry = jit.test_checked_entry();
                    let outcome = execute(descriptor, |frame| {
                        // SAFETY: observer replacement and terminal frame read are
                        // confined to this synchronous, owned checked invocation.
                        let old = unsafe { (*frame).helper };
                        if matches!(failure, Failure::Reject) { TAG_FAULT.with(|fault| fault.set(Some((frame, tag_word)))); }
                        HELPER_FAULT.with(|fault| fault.set(matches!(failure, Failure::MalformedHelper)));
                        unsafe { (*frame).helper = observe_origin; }
                        let status = unsafe { entry(frame) };
                        unsafe { (*frame).helper = old; }
                        TAG_FAULT.with(|fault| fault.set(None));
                        HELPER_FAULT.with(|fault| fault.set(false));
                        ORIGINAL_BOUND.with(|function| function.set(None));
                        if matches!(status, STATUS_ARITHMETIC | STATUS_REJECT | STATUS_ADMISSION | STATUS_ABI) {
                            let ctx = unsafe { &*(*frame).base.context.cast::<Context<'_, '_>>() };
                            terminal.set(Some((status, unsafe { (*frame).base.error_node },
                                ctx.control.work.used(), ctx.library.used())));
                        }
                        status
                    }, &[], &mut output, tests::limits(), &memory);
                    let observed = OBS.with(|v| core::mem::take(&mut *v.borrow_mut()));
                    assert!(observed.errors.is_empty(), "{failure:?}: {observed:?}");
                    assert_eq!((observed.entries, observed.packed, observed.guarded, observed.bound[13]),
                        (1, 1, 0, 1), "{failure:?}: actual packed SSA selection");
                    assert_eq!(observed.bound[14], u64::from(!matches!(failure, Failure::Selection)),
                        "{failure:?}: actual guarded selector, including invalid tag and first-header denial");
                    let (status, site, work, library_work) = terminal.get().expect("generated failure must leave its original site");
                    let site = usize::try_from(site).unwrap();
                    assert!(descriptor.closed_tails.sites[site].is_some(), "{failure:?}");
                    if let Some(expected) = expected_site { assert_eq!(site, expected, "{failure:?}"); }
                    let source_origin = descriptor.plan.blocks()[site].origin();
                    let error = outcome.unwrap_err();
                    match failure {
                        Failure::Add | Failure::Subtract | Failure::Divide => {
                            assert_eq!(status, STATUS_ARITHMETIC, "{failure:?}");
                            assert!(matches!(&error, NativeProgramExecutionError::Control(
                                NativeIterationExecutionError::Arithmetic { origin }) if *origin == source_origin), "{error:?}");
                        }
                        Failure::Reject => {
                            assert_eq!(status, STATUS_REJECT);
                            assert!(matches!(descriptor.plan.blocks()[site].terminator(),
                                NativeIterationTerminator::Scalar(NativeMatchTerminator::Reject)));
                            assert!(matches!(&error, NativeProgramExecutionError::Control(
                                NativeIterationExecutionError::Unmatched { origin }) if *origin == source_origin), "{error:?}");
                            assert_eq!(observed.bound[7], 0, "forged state cannot reach an update");
                        }
                        Failure::Selection => {
                            assert_eq!(status, STATUS_ADMISSION);
                            assert_eq!(observed.bound[6], 1);
                            // Range+selection are fused, but the maintained library
                            // failure must identify the selection, not the range.
                            let selection = descriptor.plan.blocks()[span.selection_site() as usize].origin();
                            assert!(matches!(&error, NativeProgramExecutionError::Library {
                                origin, error: NativeCollectionInvocationError::Kernel(mech_core::ResidentKernelError::InvalidInput),
                            } if *origin == selection), "{error:?}");
                        }
                        Failure::MissingKey => {
                            assert_eq!(status, STATUS_ADMISSION);
                            assert_eq!((observed.bound[8], observed.bound[7]), (1, 0));
                            assert!(format!("{error:?}").contains("missing map key"));
                        }
                        Failure::MalformedHelper => {
                            assert_eq!(status, STATUS_ABI);
                            assert_eq!(FALSE_SUCCESS.with(Cell::get), 1);
                            assert_eq!(observed.bound[7], 0, "invalid helper success cannot insert");
                            assert!(matches!(&error, NativeProgramExecutionError::Base(
                                NativeExecutionError::InvalidStatus { status: STATUS_ABI })), "{error:?}");
                        }
                        Failure::Refill => {
                            assert_eq!(status, STATUS_ADMISSION);
                            assert_eq!((observed.bound[0], observed.bound[7]), (1, 0));
                        }
                        Failure::ReadCap | Failure::WriteCap => {
                            assert_eq!(status, STATUS_ADMISSION);
                            let op = if matches!(failure, Failure::ReadCap) { 11 } else { 12 };
                            assert_eq!((observed.bound[op], observed.bound[7]), (1, 0));
                        }
                        Failure::InsertAfterGrowth => {
                            assert_eq!(status, STATUS_ADMISSION);
                            assert_eq!(observed.bound[7], 15, "first growth succeeds; second growth is denied atomically");
                            assert_eq!(observed.bound[12], 0, "failure is the actual insertion's growth cap");
                        }
                    }
                    executions.push((status, site, work, library_work, observed.bound, format!("{error:?}")));
                    assert!(output.iter().all(Option::is_none), "{failure:?}");
                    drop(output);
                    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0), "{failure:?}");
                }
            }
            assert_eq!(executions.len(), 4);
            for execution in &executions[1..] {
                assert_eq!(execution, &executions[0],
                    "{failure:?}: full/narrow synchronization and restored image retain first failure, exact spend and helper counts");
            }
        }
    }
}
