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
    pub admission_callbacks: BTreeMap<u64, u64>,
    pub helpers: BTreeMap<(u64, u64), u64>,
    pub selected: u64,
    pub fused_numeric_entries: u64,
    pub poisoned_exit_words: usize,
    pub poisoned_private_homes: usize,
    pub checked_private_homes: usize,
    pub exit_modes: BTreeMap<u64, u64>,
    pub selected_sites: BTreeMap<u64, u64>,
}
thread_local! {
    static COUNTS: RefCell<Counts> = RefCell::new(Counts::default());
    static POISON_UNUSED_EXIT: Cell<bool> = const { Cell::new(false) };
    static LOCALS: Cell<(*mut u64, usize)> = const { Cell::new((std::ptr::null_mut(), 0)) };
    static ADMIT: Cell<Option<Admit>> = const { Cell::new(None) };
    static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
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
    COUNTS.with(|v| {
        let mut counts = v.borrow_mut();
        *counts.admits.entry(site).or_default() += 1;
        *counts.admission_callbacks.entry(site).or_default() += 1;
    });
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
        GlobalHelper::AdmitMove
            | GlobalHelper::AdmitStructuralObservation
            | GlobalHelper::AdmitNumericSegmentBegin
    ) {
        COUNTS.with(|v| *v.borrow_mut().admits.entry(site).or_default() += 1);
    }
    let fused_numeric = op == GlobalHelper::AdmitNumericSegmentBegin.raw();
    if fused_numeric {
        COUNTS.with(|v| v.borrow_mut().fused_numeric_entries += 1);
    }
    let logical_op = if fused_numeric {
        GlobalHelper::NumericSegmentBegin.raw()
    } else {
        op
    };
    COUNTS.with(|v| {
        *v.borrow_mut()
            .helpers
            .entry((site, logical_op))
            .or_default() += 1
    });
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
    // SAFETY: unchanged arguments and result pointer from the checked entry.
    let status = unsafe { HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if logical_op == GlobalHelper::NumericSegmentBegin.raw() && status == 0 {
        // SAFETY: the original helper has successfully filled its live result.
        let selected = unsafe { *out } >> u32::from(fused_numeric);
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
