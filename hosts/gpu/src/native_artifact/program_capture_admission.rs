//! Strategy eligibility before acquisition, never another work account.
//!
//! The selected producer and speculative ASCII scanner must jointly fit the
//! existing account. A failed bound selects ordinary snapshot execution. Work
//! is still charged only by the actual source/entry operations as they happen.
use super::{NativeProgramDescriptor, NativeProgramExecutionLimits};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeLibraryWorkBudget, NativeMatchAction as Scalar,
};

fn sum(parts: impl IntoIterator<Item = u64>) -> Option<u64> {
    parts.into_iter().try_fold(0u64, u64::checked_add)
}

/// The caller issues at most one window/entry/retention per nonempty capture
/// fragment, visiting each newly completed row at most once. A non-ASCII
/// window stops speculation until EOF; maintained Unicode calls then execute
/// only after the final producer and length admissions. An error also stops
/// speculation, but its original source continues through capture/admission.
///
/// This function only observes immutable metadata. Its plan traversal belongs
/// to the owning session's ordinary pre-capture setup charge. It reserves no
/// fuel, changes no limits, and cannot turn ineligibility into a program error.
pub(super) fn preflight(
    descriptor: &NativeProgramDescriptor,
    limits: NativeProgramExecutionLimits,
    used: (u64, u64),
    input_limit: u64,
    chunk_bytes: usize,
    memory_limit: u64,
) -> Option<()> {
    if chunk_bytes == 0 || chunk_bytes > u32::MAX as usize || input_limit >= usize::MAX as u64 {
        return None;
    }
    let region = descriptor.prefix.as_ref()?;
    let scan_states = region.frontier.scans_per_ordinal()?;
    let source = region.frontier.read().count().lines();
    if source.lengths().len() != 1 {
        return None;
    }
    let producer = descriptor.calls[source.producer() as usize].as_ref()?;
    let producer_compute = producer.text_line_capture_work_bound(input_limit)?;
    let library_compute = limits.library.max_compute_work?;
    let library_comparison = limits.library.max_comparison_work?;
    if producer_compute > library_compute {
        return None;
    }
    let closed = descriptor.closed_tails.sites[region.tail.header() as usize]?;
    let closed = &descriptor.closed_tails.regions[closed];
    if closed.borrowed != Some(region.borrowed) || closed.iteration != region.frontier.iteration() {
        return None;
    }

    // This prelude runs before the first source acquisition. Only infallible
    // scalar copies/packing and the deferred original source/length visits may
    // precede its first zero-available checkpoint. Even closed arithmetic here
    // could trap before a capture error that originally preceded it.
    let mut site = descriptor.plan.activation_boundary()?;
    let mut prelude = 0u64;
    let mut reached = false;
    let mut source_visits = 0u8;
    for _ in 0..descriptor.plan.blocks().len() {
        let block = descriptor.plan.blocks().get(site as usize)?;
        let visit = descriptor.visits.get(site as usize)?;
        prelude = sum([prelude, visit.work, 256])?;
        if site == region.tail.initialize() {
            reached = true;
            break;
        }
        if site == source.producer() {
            if source_visits != 0 {
                return None;
            }
            source_visits = 1;
        } else if site == source.lengths()[0] {
            if source_visits != 1 {
                return None;
            }
            source_visits = 2;
        }
        if block.iteration().is_some() {
            return None;
        }
        let allowed = descriptor.scalar_aggregates.action(site as usize).is_some()
            || match block.action() {
                Action::Scalar(Scalar::None) => true,
                Action::Scalar(Scalar::Copy { output, .. }) => descriptor.plan.values()
                    [output.get() as usize]
                    .scalar_kind()
                    .is_some(),
                Action::LibraryCall { .. } => {
                    site == source.producer() || source.lengths().contains(&site)
                }
                _ => false,
            };
        if !allowed {
            return None;
        }
        let successors = visit.successors();
        site = match successors {
            [Some(next), None] | [None, Some(next)] => next,
            [Some(a), Some(b)] if a == b => a,
            _ => return None,
        };
    }
    if !reached || source_visits != 2 {
        return None;
    }

    // One-byte reads are permitted by the provider: requested chunk size is
    // never mistaken for a lower bound on delivered fragment size. The extra
    // boundary covers initial/EOF bookkeeping without assuming nonempty input.
    let boundaries = input_limit.checked_add(1)?;
    let traversals = boundaries.checked_mul(scan_states)?;
    let borrowed = &descriptor.borrowed_strings.regions[region.borrowed];
    let captures = (borrowed.proof.matrices().len() + borrowed.proof.literals().len()) as u64;
    let iterations = descriptor.plan.iterations().len() as u64;
    let arguments = descriptor.max_library_inputs.max(1) as u64;
    let checkpoint_words = sum([(borrowed.end - borrowed.ready) as u64, 3, arguments])?;
    let transition = sum([
        iterations.checked_mul(32)?,
        (descriptor.plan.collections().len() as u64).checked_mul(32)?,
        checkpoint_words.checked_mul(16)?,
        region.header_work,
        768,
    ])?;
    let guard = sum([iterations.checked_mul(64)?, 1024])?;
    let loan = sum([
        captures.checked_mul(256)?,
        (borrowed.proof.matrices().len() as u64).checked_mul(128)?,
        (borrowed.proof.literals().len() as u64).checked_mul(32)?,
        8192,
    ])?;
    let boundary = sum([
        transition,
        guard.checked_mul(3)?,
        loan,
        (descriptor.closed_tails.regions.len() as u64).checked_mul(128)?,
        arguments.checked_mul(16)?,
        region.header_work,
        2048,
    ])?;
    let length = descriptor.calls[source.lengths()[0] as usize].as_ref()?;
    // A generated String bounds failure may reconstruct one selector and then
    // call the original failing leaf. Both keep their finite maintained caps;
    // the scanner stops on this first failure. Whole-source search/copy and
    // scalar/arena housekeeping are included separately, once, not per row.
    let error_reconstruction = sum([
        library_compute.checked_mul(2)?,
        input_limit.checked_mul(512)?,
        descriptor.scalar_bytes.checked_mul(256)?,
        (descriptor.plan.values().len() as u64).checked_mul(256)?,
        (borrowed.proof.actions().len() as u64).checked_mul(256)?,
        65536,
    ])?;
    // Caller-owned old outputs are supplied only at EOF. The ordinary output
    // validator visits each root separately, including repeated aliases. Every
    // accepted root belongs to this memory account and therefore retains at
    // most its entire ceiling. Protect those actual retirement debits as well
    // as producer/length admission from speculative scanner work.
    let output_retirement = memory_limit
        .checked_mul(4)?
        .checked_add(32)?
        .checked_mul(descriptor.plan.transport_output_count() as u64)?;
    let compute = sum([
        producer_compute,
        prelude,
        length.binding_work().checked_mul(2)?,
        traversals.checked_mul(closed.compute)?,
        boundaries.checked_mul(boundary)?,
        error_reconstruction,
        output_retirement,
    ])?;
    if used.0.checked_add(compute)? > limits.execution.max_work || used.1 > library_comparison {
        return None;
    }
    Some(())
}

/// A separate admission boundary for the next actual window, checked before
/// borrowing its rows. Insufficient comparison fuel suspends speculation until
/// EOF, when producer admission no longer needs protection. This prevents a
/// short native fuel envelope from switching to more expensive maintained
/// calls during capture, while avoiding a whole-input comparison reservation.
pub(super) fn window_comparison_fits(
    descriptor: &NativeProgramDescriptor,
    work: &NativeLibraryWorkBudget,
    new_rows: u64,
) -> bool {
    let required = (|| {
        let region = descriptor.prefix.as_ref()?;
        let states = region.frontier.scans_per_ordinal()?;
        let closed = descriptor.closed_tails.sites[region.tail.header() as usize]?;
        let comparison = descriptor.closed_tails.regions.get(closed)?.comparison;
        new_rows
            .checked_add(1)?
            .checked_mul(states)?
            .checked_mul(comparison)
    })();
    required
        .zip(work.unscoped_remaining())
        .is_some_and(|(required, remaining)| required <= remaining.1)
}
