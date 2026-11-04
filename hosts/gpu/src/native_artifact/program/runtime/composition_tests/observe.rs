//! Observe logical admissions and physical helpers, delegating to the runtime.
use super::*;
use crate::native_artifact::program::protocol::abi::{Frame, Helper};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::c_void,
};
type Admit = unsafe extern "C" fn(*mut c_void, u64) -> u64;
#[derive(Default, Debug)]
pub(super) struct Counts {
    // Admission attempts include visits composed inside zero-word helpers.
    pub admits: BTreeMap<u64, u64>,
    pub helpers: BTreeMap<(u64, u64), u64>,
    pub selected: u64,
    pub phases: u64,
    pub phase_sites: BTreeMap<u64, u64>,
    pub phase_initial_fuel: Vec<u64>,
    pub phase_write_fuel: Vec<u64>,
    pub first_change_backings: Vec<(usize, usize)>,
    pub replacement_backings: Vec<(u64, usize, usize)>,
    pub terminal_backings: Vec<(usize, usize)>,
    pub prepaid_segment_charges: Vec<(u64, u64)>,
    prepaid_segment_entry_work: Option<u64>,
    pub phase_settlements: Vec<(u64, u64, u64, u64)>,
    pub cold_private_arrays: Vec<usize>,
    pub poisoned_exit_words: usize,
    pub poisoned_private_homes: usize,
    pub checked_private_homes: usize,
    pub exit_modes: BTreeMap<u64, u64>,
    pub child_fuel_exits: Vec<(u64, bool)>,
    pub prepaid_source_visits: BTreeMap<usize, u64>,
    pub selected_sites: BTreeMap<u64, u64>,
    pub segment_retirements: BTreeMap<(u32, u32), u64>,
    pub virtual_retirements: BTreeMap<u32, u64>,
}
pub(in crate::native_artifact::program::runtime) fn prepaid_source_visit(site: usize) {
    COUNTS.with(|counts| {
        *counts
            .borrow_mut()
            .prepaid_source_visits
            .entry(site)
            .or_default() += 1;
    });
}
pub(in crate::native_artifact::program::runtime) fn segment_retirement(site: u32, value: u32) {
    COUNTS.with(|counts| {
        *counts
            .borrow_mut()
            .segment_retirements
            .entry((site, value))
            .or_default() += 1
    });
}
pub(in crate::native_artifact::program::runtime) fn virtual_retirement(site: u32) {
    COUNTS.with(|counts| {
        *counts
            .borrow_mut()
            .virtual_retirements
            .entry(site)
            .or_default() += 1
    });
}
thread_local! {
    static COUNTS: RefCell<Counts> = RefCell::new(Counts::default());
    static POISON_UNUSED_EXIT: Cell<bool> = const { Cell::new(false) };
    static LOCALS: Cell<(*mut u64, usize)> = const { Cell::new((std::ptr::null_mut(), 0)) };
    static ADMIT: Cell<Option<Admit>> = const { Cell::new(None) };
    static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
    static FIRST_CHANGE_COPY_LIMIT: Cell<Option<u64>> = const { Cell::new(None) };
}
pub(in crate::native_artifact::program::runtime) fn prepaid_segment_entry_work(entry: u64) {
    COUNTS.with(|v| v.borrow_mut().prepaid_segment_entry_work = Some(entry));
}
fn private_homes(
    metadata: &crate::native_artifact::program::prepare::ProgramMetadata,
    words: usize,
) -> *mut u64 {
    let start = metadata.local_words().unwrap() - metadata.compositions.words + 1;
    let (locals, capacity) = LOCALS.with(Cell::get);
    assert!(!locals.is_null() && start + words <= capacity);
    // SAFETY: run_descriptor registers this checked frame's exact local bank.
    // The private composition range is disjoint from all source/owner lanes.
    unsafe { locals.add(start) }
}
unsafe extern "C" fn admit(context: *mut c_void, site: u64) -> u64 {
    COUNTS.with(|v| *v.borrow_mut().admits.entry(site).or_default() += 1);
    // SAFETY: run retains and restores this same synchronous callback/context.
    unsafe { ADMIT.with(Cell::get).unwrap()(context, site) }
}
unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    op: u64,
    a: u64,
    b: u64,
    c: u64,
    out: *mut u64,
) -> u64 {
    if matches!(
        GlobalHelper::decode(op),
        GlobalHelper::AdmitMove | GlobalHelper::AdmitStructuralObservation
    ) {
        COUNTS.with(|v| *v.borrow_mut().admits.entry(site).or_default() += 1);
    }
    COUNTS.with(|v| *v.borrow_mut().helpers.entry((site, op)).or_default() += 1);
    if op == GlobalHelper::ClosedTailEnd.raw() && c == 7 {
        // SAFETY: inspect only the current immutable recipe, then delegate
        // this exact synchronous callback without retaining any native view.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        let id = metadata.closed_tails.sites[site as usize].unwrap();
        let exit = metadata.closed_tails.regions[id]
            .numeric
            .as_ref()
            .unwrap()
            .exits
            .iter()
            .find(|exit| exit.to == site as usize)
            .unwrap();
        let header = matches!(
            exit.kind,
            crate::native_artifact::program::prepare::numeric_plan::ExitKind::ChildHeader { .. }
        );
        COUNTS.with(|v| v.borrow_mut().child_fuel_exits.push((site, header)));
    }
    if op == GlobalHelper::NumericSegmentEnd.raw() && POISON_UNUSED_EXIT.with(Cell::get) {
        // SAFETY: the synchronous checked entry retains this same Invocation;
        // no reference crosses the delegated helper or generated continuation.
        let ctx = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        let segment = metadata
            .compositions
            .parents
            .iter()
            .flat_map(|p| &p.segments)
            .find(|segment| {
                metadata
                    .compositions
                    .entry(segment.entry as usize)
                    .is_some()
                    && segment.sites.contains(&(site as u32))
            })
            .unwrap();
        let proof = segment.numeric.as_ref().unwrap();
        let homes = private_homes(metadata, proof.words);
        for word in 0..proof.words {
            // SAFETY: this is only a private word home in the suspended checked
            // entry. Neither a canonical owner nor a source operand is touched.
            assert_eq!(unsafe { *homes.add(word) }, u64::MAX - word as u64);
        }
        let used = proof
            .exit_argument_words(segment.frame_plan.as_ref().unwrap())
            .unwrap();
        let omitted = (0..proof.words)
            .filter(|word| used.binary_search(word).is_err())
            .collect::<Vec<_>>();
        for &word in &omitted {
            ctx.write_borrowed_word(word, u64::MAX - word as u64)
                .unwrap();
        }
        COUNTS.with(|v| {
            let mut counts = v.borrow_mut();
            counts.poisoned_exit_words += omitted.len();
            counts.checked_private_homes += proof.words;
            *counts.exit_modes.entry(a).or_default() += 1;
        });
    }
    if matches!(
        GlobalHelper::decode(op),
        GlobalHelper::ClosedTailEnd | GlobalHelper::NumericWritePrepare
    ) {
        // Read the admitted generated counters before settlement consumes them.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        if let Some(id) = metadata.closed_tails.sites[site as usize]
            .or(metadata.closed_tails.exits[site as usize])
            && let Some(phase) = metadata.closed_tails.regions[id]
                .numeric
                .as_ref()
                .and_then(|p| p.phase.as_ref())
        {
            let root = ctx.read_borrowed_word(2 + phase.loops[0].counter).unwrap();
            let children = phase
                .loops
                .iter()
                .skip(1)
                .map(|l| ctx.read_borrowed_word(2 + l.counter).unwrap())
                .sum::<u64>();
            let mode = if op == GlobalHelper::NumericWritePrepare.raw() {
                5
            } else {
                c
            };
            COUNTS.with(|v| {
                v.borrow_mut()
                    .phase_settlements
                    .push((mode, root, children, a))
            });
            if mode == 6 {
                let count = metadata.closed_tails.regions[id]
                    .numeric
                    .as_ref()
                    .unwrap()
                    .arrays
                    .iter()
                    .enumerate()
                    .filter(|(array, _)| {
                        let proof = metadata.closed_tails.regions[id].numeric.as_ref().unwrap();
                        if !proof.writes() && proof.result.direct_array().is_none() {
                            return false;
                        }
                        // Observe only this current synchronous private owner.
                        let ctx = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
                        ctx.state
                            .buffer_frames
                            .private_array_mut(id, *array)
                            .unwrap()
                            .is_some()
                    })
                    .count();
                COUNTS.with(|v| v.borrow_mut().cold_private_arrays.push(count));
            }
        }
    }
    let prepaid_segment = if op == GlobalHelper::NumericSegmentBegin.raw() {
        // SAFETY: only current counter numbers and immutable prepared prices
        // cross this synchronous callback; no owner or account is retained.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let layout = &ctx.descriptor.metadata().compositions;
        layout.entry(site as usize).and_then(|id| {
            let (_, segment) = layout.region(id)?;
            (ctx.numeric_prepaid.is_some()
                && ctx.numeric_prepaid_region() == segment.prepaid_region)
                .then(|| {
                    let proof = segment.numeric.as_ref().unwrap();
                    assert!(proof.source_work > 0);
                    (
                        ctx.state.library.used().0,
                        proof.compute - proof.source_work,
                    )
                })
        })
    } else {
        None
    };
    if prepaid_segment.is_some() {
        COUNTS.with(|v| v.borrow_mut().prepaid_segment_entry_work = None);
    }
    let terminal = if op == GlobalHelper::ClosedTailEnd.raw() && c == 0 {
        // SAFETY: only pointer numbers from the current private carrier are
        // retained across its synchronous terminal synchronization callback.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        metadata.closed_tails.exits[site as usize].and_then(|id| {
            let region = &metadata.closed_tails.regions[id];
            let proof = region.numeric.as_ref()?;
            let frame = proof.result.frame()?;
            let fields = &proof.frames.iter().find(|f| f.value == frame)?.arrays;
            let pointers = fields
                .iter()
                .enumerate()
                .filter_map(|(ordinal, array)| {
                    let array = &proof.arrays[(*array)?];
                    let owner = ctx.state.buffer_frames.managed_owner(
                        &metadata.buffer_frames,
                        region.tail.carrier(),
                        array.field,
                    )?;
                    let mech_core::ValueData::Matrix(matrix) = owner.data() else {
                        return None;
                    };
                    Some((ordinal, address(matrix.elements())))
                })
                .collect::<Vec<_>>();
            Some((id, pointers))
        })
    } else {
        None
    };
    let replacement = if op == GlobalHelper::Library.raw() {
        // SAFETY: the same checked helper is suspended; only pointer numbers
        // are retained, with no new Value pin that could change exclusivity.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        metadata
            .calls
            .get(site as usize)
            .and_then(Option::as_ref)
            .filter(|call| call.supports_unsigned_matrix_replace())
            .and_then(|_| {
                let mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                    operands,
                    output,
                    ..
                } = metadata.plan.blocks()[site as usize].action()
                else {
                    return None;
                };
                let value = ctx.state.arena.test_current_value(operands[0].get())?;
                let mech_core::ValueData::Matrix(matrix) = value.data() else {
                    return None;
                };
                matches!(matrix.elements(), mech_core::snapshot::SequenceView::U32(_))
                    .then(|| (output.get(), address(matrix.elements())))
            })
    } else {
        None
    };
    let first_change = if op == GlobalHelper::NumericWritePrepare.raw() {
        // SAFETY: the generated entry is suspended at this same synchronous
        // helper. Only an address is retained across the original callback.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        let id = metadata.closed_tails.sites[site as usize].unwrap();
        let region = &metadata.closed_tails.regions[id];
        let proof = region.numeric.as_ref().unwrap();
        use crate::native_artifact::program::prepare::readonly_numeric::Operation;
        let (Operation::Write { array, .. } | Operation::Append { array, .. }) =
            proof.action(site as usize).unwrap()
        else {
            panic!("first change requires a numeric store");
        };
        let owner = ctx
            .state
            .buffer_frames
            .managed_owner(
                &metadata.buffer_frames,
                region.tail.carrier(),
                proof.arrays[*array].field,
            )
            .unwrap();
        let mech_core::ValueData::Matrix(matrix) = owner.data() else {
            panic!("numeric store requires packed words");
        };
        Some((id, *array, address(matrix.elements())))
    } else {
        None
    };
    let old_copy_limit = if first_change.is_some()
        && let Some(limit) = FIRST_CHANGE_COPY_LIMIT.with(|v| v.replace(None))
    {
        // SAFETY: the owning helper is suspended. Deny only its original copy
        // admission; earlier source setup and generated fuel remain intact.
        let ctx = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        Some(ctx.limits.max_cloned_bytes.replace(limit))
    } else {
        None
    };
    // SAFETY: unchanged arguments and result pointer from the checked entry.
    let status = unsafe { HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if status == 0
        && let Some((id, pointers)) = terminal
    {
        // SAFETY: the callback has finished; its returned frame still owns the
        // managed fields. No Value clone or new ownership pin is introduced.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        for (ordinal, before) in pointers {
            let owner = ctx
                .state
                .buffer_frames
                .managed_owner(&metadata.buffer_frames, region.tail.returned(), ordinal)
                .unwrap();
            let mech_core::ValueData::Matrix(matrix) = owner.data() else {
                panic!("terminal managed array")
            };
            let after = address(matrix.elements());
            COUNTS.with(|v| v.borrow_mut().terminal_backings.push((before, after)));
        }
    }
    if status == 0
        && let Some((output, before)) = replacement
    {
        // SAFETY: the original callback has returned. Its normal output arena
        // retains this current immutable result through the generated resume.
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        if let Some(value) = ctx.state.arena.test_current_value(output)
            && let mech_core::ValueData::Matrix(matrix) = value.data()
        {
            let after = address(matrix.elements());
            COUNTS.with(|v| {
                v.borrow_mut()
                    .replacement_backings
                    .push((site, before, after))
            });
        }
    }
    if let Some(previous) = old_copy_limit {
        // SAFETY: the original callback returned and its borrow has ended.
        unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() }
            .limits
            .max_cloned_bytes = previous;
    }
    if status == 0
        && let Some((id, array, before)) = first_change
    {
        // SAFETY: the delegated callback has ended; its private owner is now
        // retained by this invocation until the generated continuation resumes.
        let ctx = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
        let private = ctx
            .state
            .buffer_frames
            .private_array_mut(id, array)
            .unwrap()
            .as_ref()
            .unwrap();
        let after = address(private.elements());
        COUNTS.with(|v| v.borrow_mut().first_change_backings.push((before, after)));
    }
    if op == GlobalHelper::ClosedTailBegin.raw() && status == 0 {
        // SAFETY: the checked helper completed and retained its result cell.
        let selected = unsafe { *out };
        if selected == 1 {
            let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
            let metadata = ctx.descriptor.metadata();
            let id = metadata.closed_tails.initializers[site as usize]
                .or(metadata.closed_tails.sites[site as usize])
                .unwrap();
            if metadata.closed_tails.regions[id]
                .numeric
                .as_ref()
                .is_some_and(|p| p.phase.is_some())
            {
                COUNTS.with(|v| {
                    let mut counts = v.borrow_mut();
                    counts.phases += 1;
                    counts
                        .phase_initial_fuel
                        .push(ctx.read_borrowed_word(0).unwrap());
                    *counts.phase_sites.entry(site).or_default() += 1;
                });
            }
        }
    }
    if op == GlobalHelper::NumericWritePrepare.raw() && status == 0 {
        let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
        let metadata = ctx.descriptor.metadata();
        if metadata.closed_tails.sites[site as usize].is_some_and(|id| {
            metadata.closed_tails.regions[id]
                .numeric
                .as_ref()
                .is_some_and(|p| p.phase.is_some())
        }) {
            COUNTS.with(|v| {
                v.borrow_mut()
                    .phase_write_fuel
                    .push(ctx.read_borrowed_word(0).unwrap())
            });
        }
    }
    if op == crate::native_artifact::program::protocol::GlobalHelper::NumericSegmentBegin.raw()
        && status == 0
    {
        // SAFETY: the original helper has successfully filled its live result.
        let selected = unsafe { *out };
        if selected == 1
            && let Some((before, expected)) = prepaid_segment
        {
            // SAFETY: the callback returned, and this same Invocation retains
            // its current account. Observe the actual debit without changing it.
            let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
            let charged = ctx.state.library.used().0 - before;
            COUNTS.with(|v| {
                let mut counts = v.borrow_mut();
                // The account also observes the separately charged current
                // entry work. Credit removes only original source prices.
                let entry = counts.prepaid_segment_entry_work.take().unwrap();
                counts
                    .prepaid_segment_charges
                    .push((charged, expected + entry));
            });
        }
        if selected == 1 && POISON_UNUSED_EXIT.with(Cell::get) {
            // Poison the unused physical homes before selected SSA entry. The
            // body must initialize from arguments, and no exit may spill here.
            let ctx = unsafe { &*context.cast::<Invocation<'_, '_, '_>>() };
            let metadata = ctx.descriptor.metadata();
            let (_, segment) = metadata
                .compositions
                .region(metadata.compositions.entry(site as usize).unwrap())
                .unwrap();
            let words = segment.numeric.as_ref().unwrap().words;
            let homes = private_homes(metadata, words);
            for word in 0..words {
                // SAFETY: same checked private range as private_homes above.
                unsafe { *homes.add(word) = u64::MAX - word as u64 };
            }
            COUNTS.with(|v| v.borrow_mut().poisoned_private_homes += words);
        }
        COUNTS.with(|v| {
            let mut counts = v.borrow_mut();
            counts.selected += u64::from(selected == 1);
            if selected == 1 {
                *counts.selected_sites.entry(site).or_default() += 1;
            }
        });
    }
    status
}
fn address(view: mech_core::snapshot::SequenceView<'_>) -> usize {
    use mech_core::snapshot::SequenceView as S;
    match view {
        S::U8(v) => v.as_ptr() as usize,
        S::U16(v) => v.as_ptr() as usize,
        S::U32(v) => v.as_ptr() as usize,
        S::U64(v) => v.as_ptr() as usize,
        _ => panic!("first-change observation requires unsigned packed words"),
    }
}
pub(super) fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    run_descriptor(jit, jit.descriptor(), args, outputs, policy, memory)
}

pub(super) fn run_restored(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    let descriptor = super::super::tests::restore_admitted_helper_descriptor(jit);
    run_descriptor(jit, &descriptor, args, outputs, policy, memory)
}

pub(super) fn run_with_first_change_copy_limit(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
    limit: u64,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    FIRST_CHANGE_COPY_LIMIT.with(|v| v.set(Some(limit)));
    let result = if restored {
        run_restored(jit, args, outputs, policy, memory)
    } else {
        run(jit, args, outputs, policy, memory)
    };
    assert!(FIRST_CHANGE_COPY_LIMIT.with(|v| v.get()).is_none());
    result
}

pub(super) fn run_descriptor(
    jit: &JitProgramArtifact,
    descriptor: &NativeProgramDescriptor,
    args: &[Value],
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    COUNTS.with(|v| *v.borrow_mut() = Counts::default());
    let entry = jit.test_checked_entry();
    let result = execute(
        descriptor,
        |raw: *mut Frame| {
            // SAFETY: execute retains the initialized frame throughout this call;
            // only the callback fields change and are restored before returning.
            unsafe {
                let previous = ((*raw).base.admit_next, (*raw).helper);
                let previous_locals = LOCALS
                    .with(|v| v.replace(((*raw).base.locals, (*raw).base.local_count as usize)));
                ADMIT.with(|v| v.set(Some(previous.0)));
                HELPER.with(|v| v.set(Some(previous.1)));
                (*raw).base.admit_next = admit;
                (*raw).helper = helper;
                let status = entry(raw);
                (*raw).base.admit_next = previous.0;
                (*raw).helper = previous.1;
                LOCALS.with(|v| v.set(previous_locals));
                ADMIT.with(|v| v.set(None));
                HELPER.with(|v| v.set(None));
                status
            }
        },
        args,
        outputs,
        policy,
        memory,
    );
    (
        result,
        COUNTS.with(|v| std::mem::take(&mut *v.borrow_mut())),
    )
}

pub(super) fn run_with_unused_exit_poison(
    jit: &JitProgramArtifact,
    args: &[Value],
    outputs: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    Counts,
) {
    POISON_UNUSED_EXIT.with(|v| v.set(true));
    let result = if restored {
        run_restored(jit, args, outputs, policy, memory)
    } else {
        run(jit, args, outputs, policy, memory)
    };
    POISON_UNUSED_EXIT.with(|v| v.set(false));
    result
}
