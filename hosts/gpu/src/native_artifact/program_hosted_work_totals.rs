//! Enabled-test-only fixed counters; no allocation, assertions or formatting in C.
use super::{Callback, Context, NativeProgramExecutionError};
use crate::native_artifact::{NativeExecutionError, NativeProgramDescriptor, NativeValueError};
use mech_core::{MemoryBudgetDimension, ResidentKernelError};
use mech_engine::native_artifact::NativeCollectionInvocationError;
use std::cell::Cell;

const SITES: usize = 4096;
const BOUNDS: u64 = 1;
const OVERFLOW: u64 = 2;
const UNDERFLOW: u64 = 4;
const CALLBACK_ORDER: u64 = 8;
const MISSING_FINAL: u64 = 16;
const RECONCILIATION: u64 = 32;
const UNKNOWN_DENIAL: u64 = 64;

#[derive(Clone, Copy, Default)]
struct Counts {
    calls: u64,
    accepted: u64,
    max_accepted: u64,
    failed: u64,
    candidate_denied_attempts: u64,
    max_candidate_denied_attempt: u64,
    unknown_denials: u64,
    last_error_required: Option<u64>,
    last_error_limit: Option<u64>,
}
impl Counts {
    const ZERO: Self = Self {
        calls: 0,
        accepted: 0,
        max_accepted: 0,
        failed: 0,
        candidate_denied_attempts: 0,
        max_candidate_denied_attempt: 0,
        unknown_denials: 0,
        last_error_required: None,
        last_error_limit: None,
    };
}
#[derive(Clone, Copy)]
struct Stream {
    first_before: Option<u64>,
    last_after: Option<u64>,
    gaps: u64,
    accepted: u64,
    calls: u64,
    pending: bool,
}
impl Stream {
    const ZERO: Self = Self {
        first_before: None,
        last_after: None,
        gaps: 0,
        accepted: 0,
        calls: 0,
        pending: false,
    };
}
thread_local! {
    static COUNTS: [Cell<Counts>; SITES * 2] = const { [const { Cell::new(Counts::ZERO) }; SITES * 2] };
    static STREAM: Cell<Stream> = const { Cell::new(Stream::ZERO) };
    static FLAGS: Cell<u64> = const { Cell::new(0) };
    static CHECKPOINT: Cell<Option<u64>> = const { Cell::new(None) };
}
fn enabled() -> bool {
    super::ENABLED.with(Cell::get)
}
fn flag(value: u64) {
    FLAGS.with(|flags| flags.set(flags.get() | value));
}
fn add(left: u64, right: u64) -> u64 {
    left.checked_add(right).unwrap_or_else(|| {
        flag(OVERFLOW);
        u64::MAX
    })
}
fn subtract(after: u64, before: u64) -> u64 {
    after.checked_sub(before).unwrap_or_else(|| {
        flag(UNDERFLOW);
        0
    })
}
fn index(site: u64, callback: Callback) -> Option<usize> {
    let kind = match callback {
        Callback::Admit => 0,
        Callback::Helper => 1,
    };
    usize::try_from(site)
        .ok()?
        .checked_mul(2)?
        .checked_add(kind)
}
pub(super) fn reset() {
    // This loop materializes/initializes all fixed TLS storage OUTSIDE C and
    // before enablement. No callback allocates or copies an array.
    COUNTS.with(|counts| {
        for cell in counts {
            cell.set(Counts::ZERO);
        }
    });
    STREAM.with(|stream| stream.set(Stream::ZERO));
    FLAGS.with(|flags| flags.set(0));
    CHECKPOINT.with(|checkpoint| checkpoint.set(None));
}
pub(in crate::native_artifact::program_runtime) fn begin_callback(
    ctx: &Context<'_, '_>,
    site: u64,
    callback: Callback,
) -> Option<u64> {
    if !enabled() {
        return None;
    }
    if index(site, callback).is_none_or(|index| index >= SITES * 2) {
        flag(BOUNDS);
    }
    let before = ctx.control.work.used();
    STREAM.with(|cell| {
        let mut stream = cell.get();
        if stream.pending {
            flag(CALLBACK_ORDER);
        }
        stream.pending = true;
        if let Some(last) = stream.last_after {
            stream.gaps = add(stream.gaps, subtract(before, last));
        } else {
            stream.first_before = Some(before);
        }
        cell.set(stream);
    });
    Some(before)
}
fn work_error(error: &NativeProgramExecutionError) -> Option<(u64, u64)> {
    use super::super::super::NativeIterationExecutionError;
    let (required, actual_limit) = match error {
        NativeProgramExecutionError::Base(NativeExecutionError::WorkLimit {
            required,
            limit,
            ..
        })
        | NativeProgramExecutionError::Control(NativeIterationExecutionError::Work {
            required,
            limit,
            ..
        })
        | NativeProgramExecutionError::Control(NativeIterationExecutionError::Base(
            NativeExecutionError::WorkLimit {
                required, limit, ..
            },
        ))
        | NativeProgramExecutionError::Value(NativeValueError::Base(
            NativeExecutionError::WorkLimit {
                required, limit, ..
            },
        )) => (*required, *limit),
        NativeProgramExecutionError::Library {
            error:
                NativeCollectionInvocationError::Kernel(ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    required,
                    limit,
                    ..
                }),
            ..
        }
        | NativeProgramExecutionError::Collection {
            error:
                ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    required,
                    limit,
                    ..
                },
            ..
        } => (*required, *limit),
        _ => return None,
    };
    Some((required, actual_limit))
}
fn candidate_denied(error: &NativeProgramExecutionError, used: u64, limit: u64) -> Option<u64> {
    let (required, actual_limit) = work_error(error)?;
    // Numeric limit equality is NOT a charge-source witness: a local scope
    // could have the same cap. This is only a CANDIDATE subtraction; accepted
    // deltas and actual final-ledger reconciliation are authoritative.
    (actual_limit == limit && required != u64::MAX && required > limit)
        .then(|| required.checked_sub(used))
        .flatten()
        .filter(|debit| *debit > 0)
}
pub(in crate::native_artifact::program_runtime) fn end_callback(
    ctx: &Context<'_, '_>,
    site: u64,
    callback: Callback,
    before: Option<u64>,
    error: Option<&NativeProgramExecutionError>,
) {
    if !enabled() {
        return;
    }
    let Some(before) = before else {
        flag(CALLBACK_ORDER);
        return;
    };
    let after = ctx.control.work.used();
    let accepted = subtract(after, before);
    STREAM.with(|cell| {
        let mut stream = cell.get();
        if !stream.pending {
            flag(CALLBACK_ORDER);
        }
        stream.pending = false;
        stream.last_after = Some(after);
        stream.accepted = add(stream.accepted, accepted);
        stream.calls = add(stream.calls, 1);
        cell.set(stream);
    });
    let attempted =
        error.and_then(|error| candidate_denied(error, after, ctx.control.work.limit()));
    let numbers = error.and_then(work_error);
    COUNTS.with(|counts| {
        let Some(cell) = index(site, callback).and_then(|index| counts.get(index)) else {
            flag(BOUNDS);
            return;
        };
        let mut value = cell.get();
        value.calls = add(value.calls, 1);
        value.accepted = add(value.accepted, accepted);
        value.max_accepted = value.max_accepted.max(accepted);
        if error.is_some() {
            value.failed = add(value.failed, 1);
            value.last_error_required = numbers.map(|numbers| numbers.0);
            value.last_error_limit = numbers.map(|numbers| numbers.1);
            if let Some(attempted) = attempted {
                value.candidate_denied_attempts = add(value.candidate_denied_attempts, attempted);
                value.max_candidate_denied_attempt =
                    value.max_candidate_denied_attempt.max(attempted);
            } else {
                value.unknown_denials = add(value.unknown_denials, 1);
                flag(UNKNOWN_DENIAL);
            }
        }
        cell.set(value);
    });
}
pub(in crate::native_artifact::program_runtime) fn checkpoint(used: u64) {
    if !enabled() {
        return;
    }
    CHECKPOINT.with(|cell| {
        if cell.get().is_some_and(|old| used < old) {
            flag(UNDERFLOW);
        }
        cell.set(Some(used));
    });
}

/// Only called after ordinary capture/resume/commit-or-abort has returned.
/// Allocation, sorting, metadata traversal, assertions and printing are here,
/// never in the generated C callbacks or the selected factory/leaf.
pub(super) fn report(
    descriptor: &NativeProgramDescriptor,
    variant: usize,
    bases: usize,
    reported: Option<u64>,
) -> u64 {
    let stream = STREAM.with(Cell::get);
    let checkpoint = CHECKPOINT.with(Cell::get);
    let final_used = reported.or(checkpoint);
    if reported.is_some() && reported != checkpoint {
        flag(RECONCILIATION);
    }
    if stream.pending {
        flag(CALLBACK_ORDER);
    }
    let mut rows = Vec::new();
    let mut sum = 0;
    let mut calls = 0;
    let mut failures = 0;
    let mut candidate_denied_attempts = 0;
    let mut max_candidate_denied_attempt = 0;
    COUNTS.with(|counts| {
        for (index, cell) in counts.iter().enumerate() {
            let row = cell.get();
            if row.calls != 0 {
                sum = add(sum, row.accepted);
                calls = add(calls, row.calls);
                failures = add(failures, row.failed);
                candidate_denied_attempts =
                    add(candidate_denied_attempts, row.candidate_denied_attempts);
                max_candidate_denied_attempt =
                    max_candidate_denied_attempt.max(row.max_candidate_denied_attempt);
                rows.push((index / 2, index % 2, row));
            }
        }
    });
    let (prefix, tail, reconciled) = if let Some(final_used) = final_used {
        let prefix = stream.first_before.unwrap_or(final_used);
        let tail = stream
            .last_after
            .map_or(0, |last| subtract(final_used, last));
        let reconciled = add(add(prefix, stream.gaps), add(sum, tail));
        if sum != stream.accepted || calls != stream.calls || reconciled != final_used {
            flag(RECONCILIATION);
        }
        (Some(prefix), Some(tail), Some(reconciled))
    } else {
        flag(MISSING_FINAL);
        (None, None, None)
    };
    eprintln!(
        "HOSTED_WORK_TOTAL variant={variant} bases={bases} final_used={final_used:?} final_source={} callbacks={} callback_accepted={} failed_callbacks={failures} candidate_denied_attempts={candidate_denied_attempts} max_candidate_denied_attempt={max_candidate_denied_attempt} entry_prefix={prefix:?} intercallback_gaps={} tail={tail:?} reconciled={reconciled:?} flags={}",
        if reported.is_some() {
            "success_report_and_actual_checkpoint"
        } else {
            "actual_checkpoint_or_unavailable"
        },
        stream.calls,
        sum,
        stream.gaps,
        FLAGS.with(Cell::get),
    );
    // Aggregate by actual immutable action and callback kind OUTSIDE C.
    let mut actions = std::collections::BTreeMap::<(String, usize), (u64, u64)>::new();
    for &(site, kind, row) in &rows {
        let action = descriptor
            .plan
            .blocks()
            .get(site)
            .map(|block| super::action_tag(block.action()));
        let total = actions.entry((format!("{action:?}"), kind)).or_default();
        total.0 = add(total.0, row.calls);
        total.1 = add(total.1, row.accepted);
    }
    for ((action, kind), (calls, accepted)) in actions {
        eprintln!(
            "HOSTED_WORK_ACTION variant={variant} bases={bases} callback={} action={action} calls={calls} accepted={accepted}",
            if kind == 0 { "Admit" } else { "Helper" }
        );
    }
    rows.sort_by_key(|(_, _, row)| core::cmp::Reverse(row.accepted));
    for (rank, (site, kind, row)) in rows.into_iter().enumerate() {
        if rank >= 40 && row.failed == 0 {
            continue;
        }
        let block = descriptor.plan.blocks().get(site);
        let selected = descriptor.calls.get(site).and_then(Option::as_ref);
        eprintln!(
            "HOSTED_WORK_SITE variant={variant} bases={bases} callback={} site={site} origin={:?} action={:?} primitive={:?} calls={} accepted={} max_accepted={} failed={} candidate_denied_attempts={} max_candidate_denied_attempt={} unknown_denials={} error_required={:?} error_limit={:?} candidate_scope=UNVERIFIED selected_binding_work={:?}",
            if kind == 0 { "Admit" } else { "Helper" },
            block.map(|block| block.origin()),
            block.map(|block| super::action_tag(block.action())),
            selected.map(|call| call.template().primitive()),
            row.calls,
            row.accepted,
            row.max_accepted,
            row.failed,
            row.candidate_denied_attempts,
            row.max_candidate_denied_attempt,
            row.unknown_denials,
            row.last_error_required,
            row.last_error_limit,
            selected.map(|call| call.binding_work()),
        );
    }
    let flags = FLAGS.with(Cell::get);
    eprintln!("HOSTED_WORK_FLAGS variant={variant} bases={bases} flags={flags}");
    flags
}
