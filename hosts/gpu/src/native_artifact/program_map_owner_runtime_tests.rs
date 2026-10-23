//! Validate invocation-bound storage alongside the actual canonical generated
//! program. Native opaque-word selection remains a separate integration gate.
use super::State;
use super::super::*;
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq)]
enum Mode { Mirror, ForeignDescriptor, ForeignLedger, RefuseEntry }
#[derive(Default, Debug)]
struct Report {
    entries: u64, reads: u64, writes: u64, exits: u64,
    rejected: u64, declined: u64, mismatches: u64, errors: Vec<String>,
}
thread_local! {
    static OWNER: Cell<*mut c_void> = const { Cell::new(core::ptr::null_mut()) };
    static TOKENS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    static MODE: Cell<Mode> = const { Cell::new(Mode::Mirror) };
    static REPORT: RefCell<Report> = RefCell::new(Report::default());
}
fn record(update: impl FnOnce(&mut Report)) { REPORT.with(|report| update(&mut report.borrow_mut())); }

fn map_pairs(value: &Value) -> Option<Vec<(String, u64)>> {
    let ValueData::Map(map) = value.data() else { return None; };
    map.entries().iter().map(|entry| {
        let (ValueData::String(key), ValueData::U64(value)) = (entry.key().data(), entry.value())
            else { return None; };
        Some((key.to_string(), *value))
    }).collect()
}

fn shadow(
    state: &mut State<'_>, ctx: &mut Context<'_, '_>, site: usize, operation: u64,
) -> Result<Option<u64>, NativeProgramExecutionError> {
    let descriptor = ctx.control.descriptor;
    let Some(layout) = descriptor.map_owner_words.as_ref() else { return Ok(None); };
    if let Some(index) = layout.initializer(site) {
        if matches!(MODE.with(Cell::get), Mode::ForeignDescriptor | Mode::ForeignLedger) {
            if state.begin(ctx, site).is_err() { record(|r| r.rejected += 1); }
            else { record(|r| r.mismatches += 1); }
            return Ok(None);
        }
        let memory = ctx.library.memory_budget();
        let blocker = if MODE.with(Cell::get) == Mode::RefuseEntry {
            Some(memory.reserve_capacity(memory.limit_bytes() - memory.used_bytes())?)
        } else { None };
        let initial = state.begin(ctx, site)?;
        drop(blocker);
        if let Some(initial) = initial {
            // This fixture's state is :Next(index, map): tag/index/map. The
            // independent word-emission suite qualifies general layouts.
            let words = initial.words();
            if words.len() != 3 { return Err(invalid("unexpected shadow fixture layout")); }
            TOKENS.with(|tokens| tokens.borrow_mut()[index] = words[2]);
            record(|r| r.entries += 1);
        } else { record(|r| r.declined += 1); }
        if state.begin(ctx, site).is_err() { record(|r| r.rejected += 1); }
        else { record(|r| r.mismatches += 1); }
        return Ok(None);
    }
    if let Some(index) = layout.exit(site) {
        let token = TOKENS.with(|tokens| tokens.borrow()[index]);
        if token == 0 { return Ok(None); }
        let returned = layout.regions()[index].tail().returned();
        let expected = ctx.arena.copy_current(returned.get(), &mut ctx.control.work)?;
        let actual = state.finish(ctx, site, token)?;
        // Both algorithms consume the same source-produced values; canonical
        // immutable maps and retained standard-library tables are independent.
        if map_pairs(&actual) != map_pairs(&expected) { record(|r| r.mismatches += 1); }
        if state.finish(ctx, site, token).is_err() { record(|r| r.rejected += 1); }
        else { record(|r| r.mismatches += 1); }
        TOKENS.with(|tokens| tokens.borrow_mut()[index] = 0);
        record(|r| r.exits += 1);
        return Ok(None);
    }
    if operation != LIBRARY { return Ok(None); }
    let Some((index, region)) = layout.regions().iter().enumerate().find(|(_, region)| {
        region.reads().contains(&(site as u32)) || region.writes().contains(&(site as u32))
    }) else { return Ok(None); };
    let token = TOKENS.with(|tokens| tokens.borrow()[index]);
    if token == 0 { return Ok(None); }
    let NativeIterationAction::LibraryCall { operands, .. } = descriptor.plan.blocks()[site].action()
        else { return Err(invalid("shadow call kind")); };
    if operands.len() > 3 || operands.len() > ctx.argument_capacity {
        return Err(invalid("shadow operand capacity"));
    }
    let mut words = [0u64; 3];
    // SAFETY: this is the actual admitted argument lane; generated code is
    // suspended in the same helper and no reference survives delegation.
    words[..operands.len()].copy_from_slice(unsafe {
        core::slice::from_raw_parts(ctx.arguments, operands.len())
    });
    words[0] = token.wrapping_add(1);
    if state.call(ctx, site, &words[..operands.len()]).is_err() { record(|r| r.rejected += 1); }
    else { record(|r| r.mismatches += 1); }
    words[0] = token;
    let result = state.call(ctx, site, &words[..operands.len()])?;
    if region.writes().contains(&(site as u32)) {
        TOKENS.with(|tokens| tokens.borrow_mut()[index] = result);
        words[0] = result;
        record(|r| r.writes += 1);
    } else { record(|r| r.reads += 1); }
    if state.call(ctx, site, &words[..operands.len()]).is_err() { record(|r| r.rejected += 1); }
    else { record(|r| r.mismatches += 1); }
    Ok(region.reads().contains(&(site as u32)).then_some(result))
}

unsafe extern "C" fn observe(
    context: *mut c_void, site: u64, operation: u64,
    first: u64, second: u64, third: u64, result: *mut u64,
) -> u64 {
    let mut expected = None;
    let owner = OWNER.with(Cell::get);
    if !owner.is_null() {
        // SAFETY: run retains the descriptor, lane and real Context through
        // each synchronous entry. Both exclusive borrows end before helper.
        let state = unsafe { &mut *owner.cast::<State<'_>>() };
        let ctx = unsafe { &mut *context.cast::<Context<'_, '_>>() };
        match shadow(state, ctx, site as usize, operation) {
            Ok(value) => expected = value,
            Err(error) => record(|r| r.errors.push(format!("site {site}: {error:?}"))),
        }
    }
    // SAFETY: call the real canonical helper with every original operand.
    let status = unsafe { helper(context, site, operation, first, second, third, result) };
    if status == STATUS_OK && let Some(expected) = expected {
        // SAFETY: the actual caller owns this same live result word.
        if unsafe { *result } != expected { record(|r| r.mismatches += 1); }
    }
    status
}

fn run(
    jit: &crate::native_artifact::JitProgramArtifact,
    lane_descriptor: &NativeProgramDescriptor,
    mode: Mode,
) -> Report {
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let mut outputs = vec![None; jit.descriptor().plan.transport_output_count()];
    let mut state = None;
    let mut setup_error = None;
    MODE.with(|slot| slot.set(mode));
    REPORT.with(|report| *report.borrow_mut() = Report::default());
    TOKENS.with(|tokens| *tokens.borrow_mut() = vec![0;
        lane_descriptor.map_owner_words.as_ref().unwrap().regions().len()]);
    let entry = jit.test_checked_entry();
    let result = execute(jit.descriptor(), |raw| {
        if state.is_none() && setup_error.is_none() {
            // SAFETY: the owning session supplies this exact Context and pins
            // it through the actual generated entry, as in production.
            let ctx = unsafe { &mut *(*raw).base.context.cast::<Context<'_, '_>>() };
            let prepared = if mode == Mode::ForeignLedger {
                let mut foreign = NativeWorkMeter::new(1_000_000_000).unwrap();
                foreign.bind_memory_budget(&memory).unwrap();
                State::new(lane_descriptor, &memory, &mut foreign,
                    ctx.limits.max_temporary_bytes.unwrap())
            } else {
                State::new(lane_descriptor, &memory, &mut ctx.control.work,
                    ctx.limits.max_temporary_bytes.unwrap())
            };
            match prepared {
                Ok(lane) => state = Some(lane),
                Err(error) => setup_error = Some(error),
            }
        }
        if let Some(state) = &mut state {
            OWNER.with(|slot| slot.set((state as *mut State<'_>).cast()));
        }
        // SAFETY: install the observation hook around the actual immutable
        // checked code, then restore the original frame before return checks.
        let previous = unsafe { (*raw).helper };
        unsafe { (*raw).helper = observe };
        let status = unsafe { entry(raw) };
        unsafe { (*raw).helper = previous };
        OWNER.with(|slot| slot.set(core::ptr::null_mut()));
        status
    }, &[], &mut outputs, tests::limits(), &memory);
    assert!(setup_error.is_none(), "{setup_error:?}");
    result.unwrap();
    drop((outputs, state));
    assert_eq!(memory.retained_snapshot_nodes(), 0);
    assert_eq!(memory.used_bytes(), 0);
    REPORT.with(|report| core::mem::take(&mut *report.borrow_mut()))
}

fn variants() -> [mech_engine::ProgramArtifact; 2] {
    let original = tests::artifact_source(include_str!("../../tests/fixtures/native/map-owner-binding-state.mec"));
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    [original, decoded]
}

#[test]
fn actual_generated_visits_bind_tables_validate_calls_and_publish_equal_results() {
    for artifact in variants() {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact, &mech_stdlib::source_catalog()).unwrap();
        let report = run(&jit, jit.descriptor(), Mode::Mirror);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!(report.mismatches, 0, "{report:?}");
        assert_eq!(report.entries, 1);
        assert_eq!(report.exits, 1);
        assert_eq!(report.writes, 3);
        assert!(report.reads >= 3 && report.rejected >= 8, "{report:?}");
    }
}

#[test]
fn actual_generated_visits_reject_foreign_descriptors_and_preserve_refused_entry() {
    for artifact in variants() {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact, &mech_stdlib::source_catalog()).unwrap();
        let other = crate::native_artifact::JitProgramArtifact::compile(
            &artifact, &mech_stdlib::source_catalog()).unwrap();
        let foreign = run(&jit, other.descriptor(), Mode::ForeignDescriptor);
        assert!(foreign.errors.is_empty(), "{foreign:?}");
        assert_eq!(foreign.mismatches, 0);
        assert!(foreign.rejected > 0);
        assert_eq!(foreign.entries + foreign.writes + foreign.exits, 0);
        let foreign_work = run(&jit, jit.descriptor(), Mode::ForeignLedger);
        assert!(foreign_work.errors.is_empty(), "{foreign_work:?}");
        assert_eq!(foreign_work.mismatches, 0);
        assert!(foreign_work.rejected > 0);
        assert_eq!(foreign_work.entries + foreign_work.writes + foreign_work.exits, 0);
        let refused = run(&jit, jit.descriptor(), Mode::RefuseEntry);
        assert!(refused.errors.is_empty(), "{refused:?}");
        assert_eq!(refused.mismatches, 0);
        assert_eq!(refused.declined, 1);
        assert_eq!(refused.entries + refused.writes + refused.exits, 0);
        assert!(refused.rejected > 0);
    }
}
