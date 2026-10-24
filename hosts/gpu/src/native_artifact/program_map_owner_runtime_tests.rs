//! Exercise selected owner helpers from the actual generated CFG. No shadow
//! table or Rust implementation of the source counting body participates.
use super::State;
use super::super::*;
use crate::native_artifact::program_abi::{MAP_OWNER_BEGIN, MAP_OWNER_CALL, MAP_OWNER_END, MAP_OWNER_SPAN};
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq)]
enum Mode { Normal, ForeignDescriptor, ForeignLedger, RefuseEntry, Abort, Corrupt }
#[derive(Default, Debug)]
struct Report {
    entries: u64, reads: u64, writes: u64, exits: u64, rejected: u64,
    declined: u64, canonical_map_calls: u64, errors: Vec<String>,
    projections: u64, canonical_key_calls: u64,
    cached_geometry: u64, full_geometry: u64,
}
thread_local! {
    static MODE: Cell<Mode> = const { Cell::new(Mode::Normal) };
    static FOREIGN: Cell<*const NativeProgramDescriptor> = const { Cell::new(core::ptr::null()) };
    static REPORT: RefCell<Report> = RefCell::new(Report::default());
}
fn record(update: impl FnOnce(&mut Report)) { REPORT.with(|report| update(&mut report.borrow_mut())); }
pub(super) fn observe_geometry(cached: bool) {
    record(|report| if cached { report.cached_geometry += 1; } else { report.full_geometry += 1; });
}
fn rejected(result: Result<impl Sized, NativeProgramExecutionError>) {
    if result.is_err() { record(|r| r.rejected += 1); }
    else { record(|r| r.errors.push("malformed/replayed operation accepted".into())); }
}
fn map_pairs(value: &Value) -> Option<Vec<(String, u64)>> {
    let ValueData::Map(map) = value.data() else { return None; };
    map.entries().iter().map(|entry| {
        let (ValueData::String(key), ValueData::U64(value)) = (entry.key().data(), entry.value())
            else { return None; };
        Some((key.to_string(), *value))
    }).collect()
}

unsafe extern "C" fn observe(
    context: *mut c_void, site: u64, operation: u64,
    first: u64, second: u64, third: u64, result: *mut u64,
) -> u64 {
    // SAFETY: execute owns this exact context through the synchronous entry;
    // every reference ends before delegation or return to generated code.
    let ctx = unsafe { &mut *context.cast::<Context<'_, '_>>() };
    let mode = MODE.with(Cell::get);
    let index = site as usize;
    let args = [first, second, third];
    let descriptor = ctx.control.descriptor;
    let selected_call = descriptor.map_owner_words.as_ref()
        .and_then(|layout| crate::native_artifact::program_map_owner_layout::call_region(layout, index));
    if operation == LIBRARY && selected_call.is_some() {
        record(|r| r.canonical_map_calls += 1);
    }
    if operation == LIBRARY && descriptor.map_owner_words.as_ref().is_some_and(|layout| layout.span_site(index).is_some()) {
        record(|r| r.canonical_key_calls += 1);
    }
    if operation == MAP_OWNER_SPAN {
        rejected(ctx.retained_map_helper(index, operation, [first, second, 1]));
        rejected(ctx.helper(site, LIBRARY, args));
    }
    if operation == MAP_OWNER_BEGIN {
        rejected(ctx.retained_map_helper(index, operation, [1, 0, 0]));
        if matches!(mode, Mode::ForeignDescriptor | Mode::ForeignLedger) {
            let memory = ctx.library.memory_budget();
            let foreign = FOREIGN.with(Cell::get);
            // SAFETY: run pins both JIT descriptors until execute returns.
            let lane_descriptor = if mode == Mode::ForeignDescriptor { unsafe { &*foreign } } else { descriptor };
            let lane = if mode == Mode::ForeignLedger {
                let mut work = NativeWorkMeter::new(1_000_000_000).unwrap();
                work.bind_memory_budget(&memory).unwrap();
                State::new(lane_descriptor, &memory, &mut work, ctx.limits.max_temporary_bytes.unwrap())
            } else {
                State::new(lane_descriptor, &memory, &mut ctx.control.work, ctx.limits.max_temporary_bytes.unwrap())
            };
            match lane {
                Ok(mut lane) => rejected(lane.begin(ctx, index)),
                Err(error) => record(|r| r.errors.push(format!("foreign setup: {error:?}"))),
            }
        }
    }
    if operation == MAP_OWNER_CALL {
        rejected(ctx.retained_map_helper(index, operation, [first.wrapping_add(8), second, third]));
        rejected(ctx.helper(site, LIBRARY, args));
        let count = second as usize;
        if count <= 3 && first == ctx.arguments as u64 {
            let mut words = [0; 3];
            // SAFETY: the exact admitted lane is initialized by this actual
            // call; the generic helper validates its pointer and arity again.
            words[..count].copy_from_slice(unsafe { core::slice::from_raw_parts(ctx.arguments, count) });
            words[0] = words[0].wrapping_add(1);
            if mode == Mode::Corrupt {
                // SAFETY: intentionally corrupt our own admitted token word.
                unsafe { *ctx.arguments = words[0] };
            } else {
                rejected(ctx.call_retained_map(index, &words[..count]));
            }
        }
    }
    let old_limits = ctx.limits;
    // This suite exercises the guarded per-operation transport deliberately.
    // The closed-loop transport has separate actual-entry/fuel tests.
    if operation == crate::native_artifact::program_abi::CLOSED_TAIL_BEGIN {
        ctx.limits.max_temporary_bytes = Some(0);
    }
    if operation == MAP_OWNER_BEGIN && mode == Mode::RefuseEntry {
        // Refuse fixed lane metadata without starving the canonical MOVE.
        ctx.limits.max_temporary_bytes = Some(0);
    }
    // SAFETY: the original helper receives exactly this live admitted frame.
    let status = unsafe { helper(context, site, operation, first, second, third, result) };
    let ctx = unsafe { &mut *context.cast::<Context<'_, '_>>() };
    ctx.limits = old_limits;
    if status == STATUS_OK {
        if operation == MAP_OWNER_BEGIN {
            if unsafe { *result } == 0 { record(|r| r.declined += 1); }
            else { record(|r| r.entries += 1); }
        } else if operation == MAP_OWNER_CALL {
            let layout = descriptor.map_owner_words.as_ref().unwrap();
            let region = &layout.regions()[selected_call.unwrap()];
            if region.writes().contains(&(site as u32)) { record(|r| r.writes += 1); }
            else { record(|r| r.reads += 1); }
        } else if operation == MAP_OWNER_END { record(|r| r.exits += 1); }
        else if operation == MAP_OWNER_SPAN { record(|r| r.projections += 1); }
        if matches!(operation, MAP_OWNER_BEGIN | MAP_OWNER_CALL | MAP_OWNER_END | MAP_OWNER_SPAN) {
            rejected(ctx.retained_map_helper(index, operation, args));
        }
        if mode == Mode::Abort && REPORT.with(|r| r.borrow().writes == 1) {
            ctx.error = Some(invalid("injected retained map abort"));
            return STATUS_ADMISSION;
        }
    }
    status
}

fn run_expected(
    jit: &crate::native_artifact::JitProgramArtifact,
    foreign: &NativeProgramDescriptor, mode: Mode, expected: &[(&str, u64)],
) -> Report {
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let mut outputs = vec![None; jit.descriptor().plan.transport_output_count()];
    MODE.with(|slot| slot.set(mode));
    FOREIGN.with(|slot| slot.set(foreign));
    REPORT.with(|report| *report.borrow_mut() = Report::default());
    let entry = jit.test_checked_entry();
    let result = execute(jit.descriptor(), |raw| {
        // SAFETY: install observation only around the immutable checked entry,
        // restoring the real helper before the session's frame checks.
        let previous = unsafe { (*raw).helper };
        unsafe { (*raw).helper = observe };
        let status = unsafe { entry(raw) };
        unsafe { (*raw).helper = previous };
        status
    }, &[], &mut outputs, tests::limits(), &memory);
    if mode == Mode::Abort {
        assert!(format!("{:?}", result.unwrap_err()).contains("injected retained map abort"));
    } else if mode == Mode::Corrupt {
        assert!(result.is_err(), "corrupted native map token was accepted");
    } else {
        result.unwrap();
        assert_eq!(map_pairs(outputs[0].as_ref().unwrap()), Some(expected.iter().map(|&(key, value)| (key.to_owned(), value)).collect()));
    }
    drop(outputs);
    FOREIGN.with(|slot| slot.set(core::ptr::null()));
    assert_eq!(memory.retained_snapshot_nodes(), 0);
    assert_eq!(memory.used_bytes(), 0);
    REPORT.with(|report| core::mem::take(&mut *report.borrow_mut()))
}
fn run(jit: &crate::native_artifact::JitProgramArtifact, foreign: &NativeProgramDescriptor, mode: Mode) -> Report {
    run_expected(jit, foreign, mode, &[("opaque", 3)])
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
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let report = run(&jit, jit.descriptor(), Mode::Normal);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!((report.entries, report.exits, report.writes), (1, 1, 3));
        assert!(report.reads >= 3 && report.rejected >= 8, "{report:?}");
        assert_eq!(report.canonical_map_calls, 0);
        assert!(report.cached_geometry > 0, "{report:?}");
    }
}

#[test]
fn caller_census_declines_maps_with_managed_values_created_in_the_loop() {
    let source = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec")
        .replace("+> map", "+> map\n+> number")
        .replace("\"opaque\"", "number/format(n, 0u64)");
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let report = run_expected(&jit, jit.descriptor(), Mode::Normal, &[("0", 0), ("1", 1), ("2", 2)]);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!((report.entries, report.exits, report.writes), (1, 1, 3));
        assert_eq!(report.cached_geometry, 0, "{report:?}");
        assert!(report.full_geometry > 0, "{report:?}");
    }
}

#[test]
fn caller_census_declines_nested_control_even_when_its_result_is_scalar() {
    let source = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec")
        .replace("retain(h<", "#Advance(limit<u64>) => <u64>\n  | :Next(value<u64>)\n  | :Done(value<u64>).\n#Advance(limit) -> :Next(0u64)\n  :Next(value)\n    | value >= limit -> :Done(value)\n    | * -> :Next(value + 1u64)\n  :Done(value) => value.\nretain(h<")
        .replace("  previous := ", "  advanced := #Advance(n)\n  previous := ")
        .replace("previous + n", "previous + advanced");
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let layout = jit.descriptor().map_owner_words.as_ref().unwrap();
        assert_eq!(layout.regions().len(), 1);
        let owner = layout.regions()[0].iteration();
        assert!(jit.descriptor().plan.iterations().iter().any(|child| child.as_ref().parent() == Some(owner)));
        let report = run(&jit, jit.descriptor(), Mode::Normal);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!((report.entries, report.exits, report.writes), (1, 1, 3));
        assert_eq!(report.cached_geometry, 0, "{report:?}");
        assert!(report.full_geometry > 0, "{report:?}");
    }
}
#[test]
fn actual_generated_visits_reject_foreign_descriptors_and_preserve_refused_entry() {
    for artifact in variants() {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let other = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for mode in [Mode::ForeignDescriptor, Mode::ForeignLedger, Mode::RefuseEntry] {
            let report = run(&jit, other.descriptor(), mode);
            assert!(report.errors.is_empty(), "{report:?}");
            assert!(report.rejected > 0, "{report:?}");
            if mode == Mode::RefuseEntry {
                assert_eq!((report.declined, report.entries, report.writes, report.exits), (1, 0, 0, 0));
                assert!(report.canonical_map_calls >= 3);
            } else { assert_eq!((report.entries, report.exits, report.writes), (1, 1, 3)); }
        }
    }
}
#[test]
fn owning_session_lazily_admits_maps_and_retires_tables_without_external_owner() {
    let base = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec");
    for count in [0u64, 3] {
        let source = base.replace("index<u64>, counts<", "index<u64>, stamp<u64>, counts<")
            .replace(":Next(0u64, initial)", ":Next(0u64, 7u64, initial)")
            .replace(":Next(index, counts)", ":Next(index, stamp, counts)")
            .replace(":Next(index + 1u64, retain(counts, index))", ":Next(index + 1u64, stamp + 2u64, retain(counts, index + stamp))")
            .replace("#Build(empty, 3u64)", &format!("seeded := map/insert(map/insert(empty, \"opaque\", 5u64), \"seed\", 99u64)\n#Build(seeded, {count}u64)"));
        let original = tests::artifact_source(&source);
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
        for artifact in [original, decoded] {
            let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let report = run_expected(&jit, jit.descriptor(), Mode::Normal,
                &[("opaque", if count == 0 { 5 } else { 35 }), ("seed", 99)]);
            assert!(report.errors.is_empty(), "{report:?}");
            assert_eq!((report.entries, report.exits, report.writes), (1, 1, count));
            assert_eq!(report.canonical_map_calls, 0);
        }
    }
}
#[test]
fn owning_session_failure_retires_an_unpublished_live_table() {
    for artifact in variants() {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let report = run(&jit, jit.descriptor(), Mode::Abort);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!((report.entries, report.writes, report.exits), (1, 1, 0));
    }
}
#[test]
fn generated_corrupted_owner_words_fail_and_release_the_invocation() {
    for artifact in variants() {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let report = run(&jit, jit.descriptor(), Mode::Corrupt);
        assert!(report.errors.is_empty(), "{report:?}");
        assert_eq!((report.entries, report.writes, report.exits), (1, 0, 0));
    }
}

#[test]
fn projected_map_keys_preserve_refused_owner_entry_and_failure_cleanup() {
    let source = format!("{}\n#Count(\"aba\", 1u64, 3u64, empty)\n",
        super::super::structural_projection_tests::COUNT);
    let original = tests::artifact_source(&source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap()).unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(jit.descriptor().map_owner_words.as_ref().unwrap().spans().len(), 1);
        for mode in [Mode::Normal, Mode::RefuseEntry, Mode::Abort, Mode::Corrupt] {
            let report = run_expected(&jit, jit.descriptor(), mode, &[("a", 2), ("b", 1)]);
            assert!(report.errors.is_empty(), "{report:?}");
            if mode == Mode::RefuseEntry {
                assert_eq!((report.entries, report.projections, report.declined), (0, 0, 1));
                assert_eq!(report.canonical_key_calls, 6);
            } else {
                assert!(report.projections >= 2, "{report:?}");
                assert_eq!(report.canonical_key_calls, 0);
            }
        }
    }
}

/// Observe the actual source-written counting procedure, not only the small
/// ownership fixture. The caller checks every canonical key/value separately.
pub(in crate::native_artifact::program_runtime) fn evaluate_count_case(
    jit: &crate::native_artifact::JitProgramArtifact, inputs: &[Value],
    outputs: &mut [Option<Value>], limits: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget, updates: u64,
) -> Result<NativeProgramExecutionReport, NativeProgramExecutionError> {
    MODE.with(|slot| slot.set(Mode::Normal));
    REPORT.with(|report| *report.borrow_mut() = Report::default());
    let entry = jit.test_checked_entry();
    let result = execute(jit.descriptor(), |raw| {
        // SAFETY: this synchronous observation wrapper preserves the checked
        // entry and restores its exact helper before immutable frame checks.
        let previous = unsafe { (*raw).helper };
        unsafe { (*raw).helper = observe };
        let status = unsafe { entry(raw) };
        unsafe { (*raw).helper = previous };
        status
    }, inputs, outputs, limits, memory)?;
    let report = REPORT.with(|report| core::mem::take(&mut *report.borrow_mut()));
    assert!(report.errors.is_empty(), "{report:?}");
    assert_eq!((report.entries, report.exits, report.writes), (1, 1, updates), "{report:?}");
    assert_eq!(report.canonical_map_calls, 0, "{report:?}");
    assert_eq!(report.canonical_key_calls, 0, "{report:?}");
    assert_eq!(report.projections, updates * 2, "{report:?}");
    if updates > 0 { assert!(report.cached_geometry > 0, "{report:?}"); }
    Ok(result)
}
