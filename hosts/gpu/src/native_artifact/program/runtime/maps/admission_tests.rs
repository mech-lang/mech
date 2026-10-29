//! Observe actual emitted callbacks, including refused owner entry and an
//! arithmetic failure inside a prepaid group. No source evaluator is replaced.
use crate::native_artifact::JitProgramArtifact;
use crate::native_artifact::program::runtime::*;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Native,
    Fallback,
    Limited,
    LimitedHelper,
    ScalarLimited,
}
#[derive(Default, Debug)]
struct Observation {
    calls: usize,
    groups: usize,
    native_interiors: usize,
    fallback_interiors: usize,
    probes: usize,
    errors: Vec<&'static str>,
    limited: bool,
    failure: Option<(u64, Option<u32>)>,
    fused: usize,
    helper_probes: usize,
    limited_helper: bool,
    scalar_groups: usize,
    scalar_interiors: usize,
    scalar_fallbacks: usize,
    scalar_limited: bool,
    scalar_probes: usize,
}
thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Native) };
    static OBS: RefCell<Observation> = RefCell::new(Observation::default());
}
fn note(f: impl FnOnce(&mut Observation)) {
    OBS.with(|v| f(&mut v.borrow_mut()));
}

unsafe extern "C" fn observe_admit(raw: *mut c_void, site: u64) -> u64 {
    // SAFETY: this is the exact callback/context supplied by the checked entry.
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    let group = ctx.descriptor.metadata().map_admission.group(site as usize);
    let native = group
        .and_then(|group| group.owner)
        .is_some_and(|owner| ctx.state.map_owners.has_retained_binding(owner));
    let scalar = group.filter(|group| group.owner.is_none());
    let scalar_limited = scalar.is_some_and(|group| {
        group.start() as u64 == site
            && group.sites.iter().any(|&s| {
                matches!(
                    ctx.descriptor.metadata().plan.blocks()[s as usize].action(),
                    mech_engine::native_artifact::NativeIterationAction::Scalar(
                        mech_engine::native_artifact::NativeMatchAction::Primitive {
                            op: mech_engine::native_artifact::NativeScalarOp::U64Div,
                            ..
                        }
                    )
                )
            })
    }) && MODE.with(Cell::get) == Mode::ScalarLimited
        && !OBS.with(|r| r.borrow().scalar_limited);
    if scalar_limited {
        let used = ctx.state.library.used();
        if ctx
            .state
            .library
            .restrict(used.0 + scalar.unwrap().work - 1, used.1)
            .is_err()
        {
            note(|r| {
                r.errors
                    .push("could not establish one-short scalar funding")
            });
        }
        note(|r| r.scalar_limited = true);
    }
    note(|r| {
        r.calls += 1;
        if let Some(group) = group {
            if group.start() as u64 != site {
                if native {
                    r.native_interiors += 1;
                } else {
                    r.fallback_interiors += 1;
                }
            } else if native {
                r.groups += 1;
            }
        }
    });
    let limited =
        native && MODE.with(Cell::get) == Mode::Limited && !OBS.with(|r| r.borrow().limited);
    if limited {
        let used = ctx.state.library.used();
        let work = group.unwrap().work;
        if ctx
            .state
            .library
            .restrict(used.0 + work - 1, used.1)
            .is_err()
        {
            note(|r| {
                r.errors
                    .push("could not establish exact one-short group allowance")
            });
        }
        note(|r| r.limited = true);
    }
    let before = (ctx.state.control.previous, ctx.state.control.work.used());
    // SAFETY: delegate unchanged to the real current-visit admission callback.
    let status = unsafe { crate::native_artifact::program::runtime::hooks::admit(raw, site) };
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    if let Some(group) = scalar {
        let active = ctx.state.scalar_path == Some(group.start());
        if group.start() as u64 != site {
            note(|r| {
                if active {
                    r.scalar_interiors += 1
                } else {
                    r.scalar_fallbacks += 1
                }
            });
        } else if status == STATUS_OK && active {
            note(|r| r.scalar_groups += 1);
            if !OBS.with(|r| r.borrow().scalar_probes != 0) {
                let before = (
                    ctx.state.control.previous,
                    ctx.state.control.work.used(),
                    ctx.state.scalar_path,
                );
                if ctx.visit(site).is_ok()
                    || ctx.visit(u64::from(group.sites[1])).is_ok()
                    || ctx.failure_site(u64::MAX).is_ok()
                {
                    note(|r| {
                        r.errors
                            .push("scalar replay/interior/foreign origin was accepted")
                    });
                }
                if (
                    ctx.state.control.previous,
                    ctx.state.control.work.used(),
                    ctx.state.scalar_path,
                ) != before
                {
                    note(|r| {
                        r.errors
                            .push("rejected scalar probe changed its ledger or entry")
                    });
                }
                note(|r| r.scalar_probes += 3);
            }
        }
        if scalar_limited {
            if active {
                note(|r| {
                    r.errors
                        .push("underfunded scalar group bypassed ordered visits")
                });
            }
        }
    }
    if limited
        && (status == STATUS_OK
            || before != (ctx.state.control.previous, ctx.state.control.work.used()))
    {
        note(|r| {
            r.errors
                .push("denied group advanced its predecessor or spent partial fuel")
        });
    }
    if status == STATUS_OK && native && !OBS.with(|r| r.borrow().probes != 0) {
        let group = group.unwrap();
        let before = (ctx.state.control.previous, ctx.state.control.work.used());
        if before.0 != Some(group.start()) {
            note(|r| r.errors.push("group did not retain its admitted anchor"));
        }
        if ctx.visit(site).is_ok()
            || ctx.visit(u64::from(group.sites[1])).is_ok()
            || ctx.failure_site(u64::MAX).is_ok()
        {
            note(|r| {
                r.errors
                    .push("group replay, interior visit or foreign failure was accepted")
            });
        }
        if before != (ctx.state.control.previous, ctx.state.control.work.used()) {
            note(|r| {
                r.errors
                    .push("rejected group probe changed the ledger or predecessor")
            });
        }
        note(|r| r.probes += 3);
    }
    status
}
unsafe extern "C" fn observe_helper(
    raw: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    let previous = ctx.limits;
    let fused = (operation == crate::native_artifact::program::protocol::abi::MAP_OWNER_CALL
        && ctx.retained_native_call(site as usize))
        || (operation == crate::native_artifact::program::protocol::abi::MAP_OWNER_SPAN
            && ctx.retained_native_span(site as usize));
    let before = (
        ctx.state.control.previous,
        ctx.state.control.work.used(),
        ctx.state.control.visits,
        ctx.state.stats.hash_table_updates,
    );
    let limited = fused
        && MODE.with(Cell::get) == Mode::LimitedHelper
        && !OBS.with(|r| r.borrow().limited_helper);
    if limited {
        let work = ctx
            .descriptor
            .metadata()
            .map_admission
            .group(site as usize)
            .map_or(
                ctx.descriptor.metadata().visits[site as usize].work,
                |group| group.work,
            );
        let used = ctx.state.library.used();
        if ctx
            .state
            .library
            .restrict(used.0 + work - 1, used.1)
            .is_err()
        {
            note(|r| r.errors.push("could not restrict fused helper admission"));
        }
        note(|r| r.limited_helper = true);
    }
    if operation == crate::native_artifact::program::protocol::abi::MAP_OWNER_BEGIN
        && MODE.with(Cell::get) == Mode::Fallback
    {
        ctx.limits.max_temporary_bytes = Some(0);
    }
    // SAFETY: the real helper receives the checked frame's unchanged arguments.
    let status = unsafe {
        crate::native_artifact::program::runtime::hooks::helper(
            raw, site, operation, first, second, third, result,
        )
    };
    let ctx = unsafe { &mut *raw.cast::<Invocation<'_, '_, '_>>() };
    ctx.limits = previous;
    if limited
        && (status == STATUS_OK
            || before
                != (
                    ctx.state.control.previous,
                    ctx.state.control.work.used(),
                    ctx.state.control.visits,
                    ctx.state.stats.hash_table_updates,
                ))
    {
        note(|r| {
            r.errors
                .push("denied fused helper advanced work, storage or control")
        });
    }
    if fused && status == STATUS_OK {
        note(|r| r.fused += 1);
        if ctx.state.control.previous != Some(site as u32)
            || ctx.state.control.visits != before.2 + 1
        {
            note(|r| {
                r.errors
                    .push("helper was not admitted exactly once at its own boundary")
            });
        }
        let before = (
            ctx.state.control.previous,
            ctx.state.control.work.used(),
            ctx.state.control.visits,
            ctx.state.stats.hash_table_updates,
        );
        if ctx.helper(site, operation, [first, second, third]).is_ok()
            || ctx
                .visit(ctx.descriptor.metadata().plan.entry() as u64)
                .is_ok()
        {
            note(|r| {
                r.errors
                    .push("fused helper replay or foreign frontier was accepted")
            });
        }
        if before
            != (
                ctx.state.control.previous,
                ctx.state.control.work.used(),
                ctx.state.control.visits,
                ctx.state.stats.hash_table_updates,
            )
        {
            note(|r| {
                r.errors
                    .push("rejected fused helper probe changed its state")
            });
        }
        note(|r| r.helper_probes += 2);
    }
    status
}
fn run(
    jit: &JitProgramArtifact,
    descriptor: &NativeProgramDescriptor,
    mode: Mode,
    inputs: &[Value],
    outputs: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Observation,
) {
    MODE.with(|v| v.set(mode));
    OBS.with(|v| *v.borrow_mut() = Observation::default());
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |frame| {
            // SAFETY: observe only this pinned JIT owner's synchronous entry, and
            // restore both callbacks before the maintained frame-integrity checks.
            let previous = unsafe { ((*frame).base.admit_next, (*frame).helper) };
            unsafe {
                (*frame).base.admit_next = observe_admit;
                (*frame).helper = observe_helper;
            }
            let status = unsafe { entry(frame) };
            if matches!(status, STATUS_ARITHMETIC | STATUS_REJECT) {
                let ctx = unsafe { &*(*frame).base.context.cast::<Invocation<'_, '_, '_>>() };
                note(|r| {
                    r.failure = Some((
                        unsafe { (*frame).base.error_node },
                        ctx.state.control.previous,
                    ))
                });
            }
            unsafe {
                (*frame).base.admit_next = previous.0;
                (*frame).helper = previous.1;
            }
            status
        },
        inputs,
        outputs,
        crate::native_artifact::program::runtime::tests::limits(),
        memory,
    );
    (result, OBS.with(|v| core::mem::take(&mut *v.borrow_mut())))
}
fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn native_map_groups_reduce_actual_callbacks_preserve_fallback_and_deny_one_short_fuel() {
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/native/map-owner-binding-state.mec"
    )) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: these exact build-owned bytes remain pinned to their JIT.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            assert!(!descriptor.metadata().map_admission.groups.is_empty());
            assert!(
                descriptor
                    .metadata()
                    .map_admission
                    .groups
                    .iter()
                    .any(|group| group.frontier.len() > 1),
                "fixture must exercise native branches with multiple observable successors"
            );
            let mut callbacks = Vec::new();
            for mode in [
                Mode::Native,
                Mode::Fallback,
                Mode::Limited,
                Mode::LimitedHelper,
            ] {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut outputs = vec![None; 1];
                let (result, observed) = run(&jit, descriptor, mode, &[], &mut outputs, &memory);
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!(observed.native_interiors, 0, "{observed:?}");
                if matches!(mode, Mode::Limited | Mode::LimitedHelper) {
                    assert!(
                        result.is_err() && (observed.limited || observed.limited_helper),
                        "{result:?} {observed:?}"
                    );
                    assert!(outputs.iter().all(Option::is_none));
                } else {
                    result.unwrap();
                    let ValueData::Map(map) = outputs[0].as_ref().unwrap().data() else {
                        panic!("map");
                    };
                    assert_eq!(map.entries().len(), 1);
                    assert!(
                        matches!(map.entries()[0].key().data(), ValueData::String(s) if s.as_ref() == "opaque")
                    );
                    assert!(matches!(map.entries()[0].value(), ValueData::U64(3)));
                    if mode == Mode::Native {
                        assert!(observed.groups > 0 && observed.probes == 3);
                        assert!(observed.fused > 0 && observed.helper_probes == observed.fused * 2);
                    } else {
                        assert!(observed.groups == 0 && observed.fallback_interiors > 0);
                    }
                    callbacks.push(observed.calls);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
            assert!(
                callbacks[0] < callbacks[1],
                "native/fallback callbacks: {callbacks:?}"
            );
        }
    }
}

#[test]
fn native_map_group_arithmetic_failure_keeps_the_original_interior_origin() {
    let source = include_str!("../../../../../tests/fixtures/native/map-owner-binding-state.mec")
        .replace(
            "index + 1u64, retain",
            "index + 1u64 + 10u64 / (count - 3u64), retain",
        )
        .replace("#Build(empty, 3u64)", "#Build(empty, total<u64>)");
    for artifact in artifacts(&source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let input = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(3),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut reservation = memory
            .reserve_capacity(input.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let input = input.into_memory_budget(&mut reservation).unwrap();
        drop(reservation);
        let mut outputs = vec![None; 1];
        let (result, observed) = run(
            &jit,
            jit.descriptor(),
            Mode::Native,
            &[input.clone()],
            &mut outputs,
            &memory,
        );
        assert!(observed.errors.is_empty(), "{observed:?}");
        let (failed, previous) = observed.failure.expect("generated arithmetic failure");
        let previous = previous.unwrap();
        let group = jit
            .descriptor()
            .metadata()
            .map_admission
            .group(previous as usize)
            .unwrap();
        assert_eq!(previous, group.start());
        assert_ne!(
            failed,
            u64::from(previous),
            "failure must exercise an interior original instruction"
        );
        assert!(group.sites.contains(&(failed as u32)));
        let Err(NativeProgramExecutionError::Control(NativeIterationExecutionError::Arithmetic {
            origin,
        })) = result
        else {
            panic!("expected the original arithmetic failure: {result:?}");
        };
        assert_eq!(
            origin,
            jit.descriptor().metadata().visits[failed as usize].origin
        );
        assert!(outputs.iter().all(Option::is_none));
        drop((outputs, input));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn ordinary_scalar_groups_keep_interior_origins_and_one_short_funding_falls_back() {
    let body = include_str!("../../../../../tests/fixtures/native/scalar-path-admission.mec")
        .split("store(data")
        .next()
        .unwrap();
    let source = format!("{body}\ncalculate(word<u64>,flag<bool>,permit<bool>,divisor<u64>)\n");
    for artifact in artifacts(&source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: the same checked JIT owner pins this build image and entry.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), image);
        for descriptor in [jit.descriptor(), &restored] {
            assert!(
                descriptor
                    .metadata()
                    .map_admission
                    .groups
                    .iter()
                    .any(|g| g.owner.is_none())
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut outputs = vec![None; artifact.outputs().len()];
            for (word, permit, divisor, mode) in [
                (7, true, 1, Mode::Native),
                (7, true, 0, Mode::Native),
                (7, true, 0, Mode::ScalarLimited),
                (11, true, 2, Mode::Native),
                (u64::MAX, false, 0, Mode::Native),
                (19, true, 3, Mode::Native),
            ] {
                let args = artifact
                    .inputs()
                    .iter()
                    .map(|port| {
                        let name = mech_engine::decode_source_input_name(&port.name).unwrap();
                        let data = match name.as_str() {
                            "word" => ValueDataDraft::U64(word),
                            "flag" => ValueDataDraft::Bool(true),
                            "permit" => ValueDataDraft::Bool(permit),
                            "divisor" => ValueDataDraft::U64(divisor),
                            _ => panic!("{name}"),
                        };
                        let value = ValueDraft {
                            schema: port.schema,
                            shape_values: Box::new([]),
                            data,
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap();
                        let mut claim = memory
                            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                            .unwrap();
                        value.into_memory_budget(&mut claim).unwrap()
                    })
                    .collect::<Vec<_>>();
                let aliases = args.clone();
                let prior = outputs.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (outcome, observed) = run(&jit, descriptor, mode, &args, &mut outputs, &memory);
                assert!(observed.errors.is_empty(), "{observed:?}");
                assert_eq!(
                    observed.scalar_interiors, 0,
                    "funded interiors must have no callbacks: {observed:?}"
                );
                if permit && divisor != 0 {
                    outcome.unwrap();
                    assert!(
                        matches!(outputs[0].as_ref().unwrap().data(),ValueData::U64(n) if *n==((word+1)*3+5)/divisor+17)
                    );
                    assert!(
                        observed.scalar_groups > 0 && observed.scalar_probes == 3,
                        "{observed:?}"
                    );
                } else {
                    let error = outcome.unwrap_err();
                    if !permit {
                        assert!(
                            matches!(
                                error,
                                NativeProgramExecutionError::Control(
                                    NativeIterationExecutionError::Integrity { .. }
                                )
                            ),
                            "{error:?}"
                        );
                    } else {
                        let NativeProgramExecutionError::Control(
                            NativeIterationExecutionError::Arithmetic { origin },
                        ) = error
                        else {
                            panic!(
                                "expected source arithmetic rather than admission denial: {error:?} {observed:?}"
                            )
                        };
                        let (failed, previous) =
                            observed.failure.expect("original arithmetic site");
                        assert_eq!(origin, descriptor.metadata().visits[failed as usize].origin);
                        if mode == Mode::ScalarLimited {
                            assert!(
                                observed.scalar_limited && observed.scalar_fallbacks > 0,
                                "{observed:?}"
                            );
                            assert_eq!(
                                previous,
                                Some(failed as u32),
                                "ordered fallback must reach the original failing visit"
                            );
                        } else {
                            assert_ne!(
                                previous,
                                Some(failed as u32),
                                "funded failure must be interior"
                            );
                        }
                    }
                    assert!(outputs.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
