//! Observe the actual checked callbacks, delegating to the unchanged runtime.
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
    pub admits: BTreeMap<u64, u64>,
    pub helpers: BTreeMap<(u64, u64), u64>,
    pub selected: u64,
}
thread_local! {
    static COUNTS: RefCell<Counts> = RefCell::new(Counts::default());
    static ADMIT: Cell<Option<Admit>> = const { Cell::new(None) };
    static HELPER: Cell<Option<Helper>> = const { Cell::new(None) };
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
    COUNTS.with(|v| *v.borrow_mut().helpers.entry((site, op)).or_default() += 1);
    // SAFETY: unchanged arguments and result pointer from the checked entry.
    let status = unsafe { HELPER.with(Cell::get).unwrap()(context, site, op, a, b, c, out) };
    if op == crate::native_artifact::program::protocol::GlobalHelper::NumericSegmentBegin.raw()
        && status == 0
    {
        // SAFETY: the original helper has successfully filled its live result.
        let selected = unsafe { *out };
        COUNTS.with(|v| v.borrow_mut().selected += u64::from(selected == 1));
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
    COUNTS.with(|v| *v.borrow_mut() = Counts::default());
    let entry = jit.test_checked_entry();
    let result = execute(
        jit.descriptor(),
        |raw: *mut Frame| {
            // SAFETY: execute retains the initialized frame throughout this call;
            // only the callback fields change and are restored before returning.
            unsafe {
                let previous = ((*raw).base.admit_next, (*raw).helper);
                ADMIT.with(|v| v.set(Some(previous.0)));
                HELPER.with(|v| v.set(Some(previous.1)));
                (*raw).base.admit_next = admit;
                (*raw).helper = helper;
                let status = entry(raw);
                (*raw).base.admit_next = previous.0;
                (*raw).helper = previous.1;
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
