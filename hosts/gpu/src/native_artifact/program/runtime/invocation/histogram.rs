//! Original-call entry/publication around a call-free compiled histogram.
//! Native code owns hashing/counting; this module owns checked loans, exact
//! physical backing and ordinary canonical Map construction at the call site.
use super::{Error, Invocation, invalid};
use crate::native_artifact::NativeValueHandle;
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::runtime::collections::{fixed_box, fixed_slice};
use crate::native_artifact::program::runtime::maps::utf8_histogram::{
    COLD, COMPLETE, FULL, Frame, Slot, bank_limit, maximum_capacity, pass_work,
};
use crate::native_artifact::program::runtime::maps::utf8_read::Cell;
use mech_core::snapshot::{
    MapEntryDraft, SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueDataDraft,
    ValueDraft,
};
use mech_core::{ManagedMemoryReservation, ManagedNodeReservation, ResourceDemand, Value};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError, NativeIterationAction as Action,
};
use std::mem::size_of;

pub(in crate::native_artifact::program::runtime) struct Loan {
    // Owners are destroyed before their complete backing/node reservations.
    source: Value,
    cells: Box<[Cell]>,
    slots: Box<[Slot]>,
    frame: Frame,
    handle: NativeValueHandle,
    site: usize,
    visit: u64,
    compute: u64,
    comparison: u64,
    original_bytes: u64,
    source_bytes: u64,
    maximum_capacity: usize,
    prior: ResourceDemand,
    _reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}
fn add(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}
fn mul(a: u64, b: u64) -> Option<u64> {
    a.checked_mul(b)
}
fn fits(value: u64, bound: Option<u64>) -> bool {
    bound.is_some_and(|b| value <= b)
}

impl Invocation<'_, '_, '_> {
    pub(super) fn histogram_helper(
        &mut self,
        site: usize,
        tag: GlobalHelper,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let Some(call) = metadata.calls.get(site).and_then(Option::as_ref) else {
            return Err(invalid("compiled histogram lacks selected call"));
        };
        let Action::LibraryCall {
            operands,
            output,
            template,
        } = metadata.plan.blocks()[site].action()
        else {
            return Err(invalid("compiled histogram outside original library call"));
        };
        if self.state.consumed
            || self.state.control.completed
            || operands.len() != 1
            || call.template() != template
            || !call.supports_compiled_string_histogram()
        {
            return Err(invalid("compiled histogram contract or visit differs"));
        }
        let origin = metadata.plan.blocks()[site].origin();
        let library_error = |error| Error::Library { origin, error };
        if tag == GlobalHelper::HistogramPrepare {
            if args != [0; 3] || self.histogram.is_some() {
                return Err(invalid("compiled histogram entry payload or replay"));
            }
            if self.worker.is_some()
                || self.has_uninventoried_private_owners()
                || self.borrowed_strings.is_some()
                || self.selected_join.is_some()
            {
                return Ok(Some(0));
            }
            let Some(geometry) = self.retained_caller_geometry(site)? else {
                return Ok(Some(0));
            };
            let prior = ResourceDemand {
                turn_peak_bytes: geometry.live_bytes,
                retained_nodes: geometry.live_nodes,
                ..ResourceDemand::default()
            };
            let memory = self.state.library.memory_budget();
            // Prepay the selected schema/shape validation before obtaining a
            // packed view. The input's already admitted owner bounds every
            // payload byte; this preview never scans or clones its strings.
            let Some(setup) = call
                .binding_work()
                .checked_add(call.binding_metadata_bytes().saturating_mul(8))
                .and_then(|n| n.checked_add(4096))
            else {
                return Ok(Some(0));
            };
            if !fits(setup, self.limits.max_compute_work)
                || self
                    .state
                    .library
                    .unscoped_remaining()
                    .is_none_or(|(c, _)| c < setup)
            {
                return Ok(Some(0));
            }
            self.state
                .library
                .charge(setup, 0)
                .map_err(|error| library_error(NativeCollectionInvocationError::Kernel(error)))?;
            let source = self
                .state
                .arena
                .copy_current(operands[0].get(), &mut self.state.control.work)?;
            let handle = self
                .state
                .arena
                .current_handle(operands[0].get(), &mut self.state.control.work)?;
            let Some(source_bytes) = source.memory_budget_retained_bytes(&memory) else {
                return Ok(Some(0));
            };
            let Some(validation_peak) = prior
                .turn_peak_bytes
                .checked_add(call.binding_metadata_bytes())
            else {
                return Ok(Some(0));
            };
            if !fits(validation_peak, self.limits.max_temporary_bytes) {
                return Ok(Some(0));
            }
            let Ok(validation) = memory.reserve_capacity(call.binding_metadata_bytes()) else {
                return Ok(Some(0));
            };
            let Some(strings) = call.compiled_histogram_strings(&source) else {
                return Ok(Some(0));
            };
            drop(validation);
            let count = strings.len();
            // Start small; only an actual native FULL result can grow this
            // private bank. The largest bank can hold every input key below
            // the selected three-quarter occupancy limit.
            let Some(maximum_capacity) = maximum_capacity(count) else {
                return Ok(Some(0));
            };
            let capacity = maximum_capacity.min(64);
            let Some(physical) = (count as u64)
                .checked_mul(size_of::<Cell>() as u64)
                .and_then(|n| {
                    n.checked_add((capacity as u64).checked_mul(size_of::<Slot>() as u64)?)
                })
                .and_then(|n| n.checked_add(size_of::<Loan>() as u64))
            else {
                return Ok(Some(0));
            };
            let Some(prepare) = physical.checked_mul(8).and_then(|n| n.checked_add(4096)) else {
                return Ok(Some(0));
            };
            let Some((fuel, comparison)) = pass_work(count as u64, source_bytes, capacity as u64)
            else {
                return Ok(Some(0));
            };
            let Some(compute) = setup.checked_add(prepare).and_then(|n| n.checked_add(fuel)) else {
                return Ok(Some(0));
            };
            let Some(peak) = prior.turn_peak_bytes.checked_add(physical) else {
                return Ok(Some(0));
            };
            let Some(nodes) = prior.retained_nodes.checked_add(4) else {
                return Ok(Some(0));
            };
            if !fits(peak, self.limits.max_temporary_bytes)
                || !fits(nodes, self.limits.max_retained_nodes)
                || !fits(compute, self.limits.max_compute_work)
                || !fits(comparison, self.limits.max_comparison_work)
                || self
                    .state
                    .library
                    .unscoped_remaining()
                    .is_none_or(|(c, p)| c < prepare.saturating_add(fuel) || p < comparison)
            {
                return Ok(Some(0));
            }
            let Ok(reservation) = memory.reserve_capacity(physical) else {
                return Ok(Some(0));
            };
            let Ok(nodes) = memory.reserve_nodes(4) else {
                return Ok(Some(0));
            };
            // This same account prepays all allocation initialization, row
            // metadata copies, final bank scan, native hash/probe/compare work and retirement.
            self.state
                .library
                .charge(prepare + fuel, comparison)
                .map_err(|error| library_error(NativeCollectionInvocationError::Kernel(error)))?;
            let Some(mut cells) = fixed_slice(count, Cell::default()) else {
                return Ok(Some(0));
            };
            let Some(mut slots) = fixed_slice(capacity, Slot::default()) else {
                return Ok(Some(0));
            };
            for (cell, text) in cells.iter_mut().zip(strings) {
                *cell = Cell {
                    address: text.as_ptr() as u64,
                    bytes: text.len() as u64,
                };
            }
            let frame = Frame {
                cells: cells.as_ptr() as u64,
                count: count as u64,
                slots: slots.as_mut_ptr() as u64,
                capacity: capacity as u64,
                limit: bank_limit(capacity),
                fuel,
                comparison,
                completed: 0,
                distinct: 0,
                payload: 0,
            };
            let Some(mut loan) = fixed_box(Loan {
                source,
                cells,
                slots,
                frame,
                handle,
                site,
                visit: self.state.control.visits,
                compute,
                comparison,
                original_bytes: physical,
                source_bytes,
                maximum_capacity,
                prior,
                _reservation: reservation,
                _nodes: nodes,
            }) else {
                return Ok(Some(0));
            };
            let pointer = (&mut loan.frame as *mut Frame) as u64;
            self.histogram = Some(loan);
            return Ok(Some(pointer));
        }
        if tag != GlobalHelper::HistogramFinish || args[2] != 0 {
            return Err(invalid("compiled histogram unknown finish or payload"));
        }
        let mut loan = self
            .histogram
            .take()
            .ok_or_else(|| invalid("compiled histogram finish without entry"))?;
        if loan.site != site
            || loan.visit != self.state.control.visits
            || args[0] != (&mut loan.frame as *mut Frame) as u64
            || loan.frame.cells != loan.cells.as_ptr() as u64
            || loan.frame.slots != loan.slots.as_ptr() as u64
            || loan.frame.count != loan.cells.len() as u64
            || loan.frame.capacity != loan.slots.len() as u64
            || loan.frame.limit != bank_limit(loan.slots.len())
        {
            return Err(invalid("compiled histogram loan identity changed"));
        }
        let current = self
            .state
            .arena
            .get(loan.handle, &mut self.state.control.work)?;
        if !current.shares_frozen_storage(&loan.source) {
            return Err(invalid("compiled histogram source generation changed"));
        }
        if args[1] == COLD {
            return Ok(Some(0));
        }
        if args[1] == FULL {
            if loan.frame.distinct != loan.frame.limit || loan.frame.completed >= loan.frame.count {
                return Err(invalid(
                    "compiled histogram FULL lacks an exact native prefix",
                ));
            }
            let Some(capacity) = loan
                .slots
                .len()
                .checked_mul(2)
                .filter(|&n| n <= loan.maximum_capacity)
            else {
                return Err(invalid(
                    "compiled histogram exceeded complete input capacity",
                ));
            };
            let Some(bank_bytes) = (capacity as u64).checked_mul(size_of::<Slot>() as u64) else {
                return Ok(Some(0));
            };
            let Some(coexisting) = loan.original_bytes.checked_add(bank_bytes) else {
                return Ok(Some(0));
            };
            let Some(peak) = loan.prior.turn_peak_bytes.checked_add(coexisting) else {
                return Ok(Some(0));
            };
            let Some(nodes) = loan.prior.retained_nodes.checked_add(5) else {
                return Ok(Some(0));
            };
            let Some(prepare) = bank_bytes.checked_mul(8).and_then(|n| n.checked_add(4096)) else {
                return Ok(Some(0));
            };
            let Some((fuel, comparison)) =
                pass_work(loan.frame.count, loan.source_bytes, capacity as u64)
            else {
                return Ok(Some(0));
            };
            let Some(additional_compute) = prepare.checked_add(fuel) else {
                return Ok(Some(0));
            };
            let Some(compute) = loan.compute.checked_add(additional_compute) else {
                return Ok(Some(0));
            };
            let Some(total_comparison) = loan.comparison.checked_add(comparison) else {
                return Ok(Some(0));
            };
            if !fits(peak, self.limits.max_temporary_bytes)
                || !fits(nodes, self.limits.max_retained_nodes)
                || !fits(compute, self.limits.max_compute_work)
                || !fits(total_comparison, self.limits.max_comparison_work)
                || self
                    .state
                    .library
                    .unscoped_remaining()
                    .is_none_or(|(c, p)| c < additional_compute || p < comparison)
            {
                return Ok(Some(0));
            }
            let memory = self.state.library.memory_budget();
            // Old/new banks coexist during allocation. Neither old bytes nor
            // their node disappear from admission until the old owner drops.
            if loan._reservation.resize_capacity(coexisting).is_err() {
                return Ok(Some(0));
            }
            let Ok(extra_node) = memory.reserve_nodes(1) else {
                return Ok(Some(0));
            };
            self.state
                .library
                .charge(additional_compute, comparison)
                .map_err(|error| library_error(NativeCollectionInvocationError::Kernel(error)))?;
            let Some(slots) = fixed_slice(capacity, Slot::default()) else {
                return Ok(Some(0));
            };
            let old_bytes = (loan.slots.len() * size_of::<Slot>()) as u64;
            let old = std::mem::replace(&mut loan.slots, slots);
            drop(old);
            drop(extra_node);
            loan.original_bytes = coexisting - old_bytes;
            loan._reservation.resize_capacity(loan.original_bytes)?;
            loan.compute = compute;
            loan.comparison = total_comparison;
            // Replay the immutable input in native code into the fresh bank.
            // Previously spent work stays spent; no host rehash/count loop.
            loan.frame.slots = loan.slots.as_mut_ptr() as u64;
            loan.frame.capacity = capacity as u64;
            loan.frame.limit = bank_limit(capacity);
            loan.frame.fuel = fuel;
            loan.frame.comparison = comparison;
            loan.frame.completed = 0;
            loan.frame.distinct = 0;
            loan.frame.payload = 0;
            let pointer = (&mut loan.frame as *mut Frame) as u64;
            self.histogram = Some(loan);
            return Ok(Some(pointer));
        }
        if args[1] != COMPLETE
            || loan.frame.completed != loan.frame.count
            || loan.frame.distinct > loan.frame.count
        {
            return Err(invalid("compiled histogram failed to complete exact input"));
        }
        let distinct = loan.frame.distinct;
        let payload = loan.frame.payload;
        let footprint = call
            .selected_histogram_footprint(distinct, payload)
            .map_err(library_error)?;
        let drafts = call
            .selected_histogram_draft_bytes(distinct)
            .map_err(library_error)?;
        let sorted_finalization = call
            .selected_histogram_finalization_work(distinct, payload)
            .map_err(library_error)?;
        // The ordinary radix family publishes sorted drafts and can use a
        // linear canonicalization allowance. Hash-bank drafts are unordered:
        // every key can compare with every prior key, and insertion can move
        // that whole prefix. Bound all repeated payload/metadata visits and
        // shifts before the first draft is allocated or canonicalized.
        let Some(finalization) = distinct
            .checked_add(1)
            .and_then(|n| sorted_finalization.checked_mul(n))
        else {
            return Ok(Some(0));
        };
        let Some(envelope) = mul(drafts, 2)
            .and_then(|n| add(n, mul(payload, 4)?))
            .and_then(|n| add(n, mul(footprint.retained_bytes, 4)?))
            .and_then(|n| add(n, metadata.scalar_bytes))
            .and_then(|n| add(n, 4096))
        else {
            return Ok(Some(0));
        };
        let Some(finish) = mul(envelope, 16)
            .and_then(|n| add(n, finalization))
            .and_then(|n| add(n, 256))
        else {
            return Ok(Some(0));
        };
        let Some(peak) =
            add(loan.prior.turn_peak_bytes, loan.original_bytes).and_then(|n| add(n, envelope))
        else {
            return Ok(Some(0));
        };
        let Some(nodes) = mul(footprint.node_count, 3)
            .and_then(|n| add(n, loan.prior.retained_nodes))
            .and_then(|n| add(n, 8))
        else {
            return Ok(Some(0));
        };
        if !fits(distinct, self.limits.max_output_elements)
            || !fits(footprint.retained_bytes, self.limits.max_output_bytes)
            || !fits(payload, self.limits.max_cloned_bytes)
            || !fits(peak, self.limits.max_temporary_bytes)
            || !fits(nodes, self.limits.max_retained_nodes)
            || loan
                .compute
                .checked_add(finish)
                .is_none_or(|n| !fits(n, self.limits.max_compute_work))
            || loan
                .comparison
                .checked_add(finalization)
                .is_none_or(|n| !fits(n, self.limits.max_comparison_work))
            || self
                .state
                .library
                .unscoped_remaining()
                .is_none_or(|(c, p)| c < finish || p < finalization)
        {
            return Ok(Some(0));
        }
        let memory = self.state.library.memory_budget();
        let Ok(mut reservation) = memory.reserve_capacity(envelope) else {
            return Ok(Some(0));
        };
        let Ok(node_probe) = memory.reserve_nodes(nodes - loan.prior.retained_nodes) else {
            return Ok(Some(0));
        };
        drop(node_probe);
        self.state
            .library
            .charge(finish, finalization)
            .map_err(|error| library_error(NativeCollectionInvocationError::Kernel(error)))?;
        let strings = call
            .compiled_histogram_strings(&loan.source)
            .ok_or_else(|| invalid("compiled histogram source contract changed"))?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(distinct as usize)
            .map_err(|_| invalid("compiled histogram draft allocation"))?;
        let mut actual_payload = 0u64;
        let mut actual_count = 0u64;
        for slot in loan.slots.iter().filter(|slot| slot.row_plus_one != 0) {
            let index = slot
                .row_plus_one
                .checked_sub(1)
                .filter(|&n| n < loan.frame.count)
                .ok_or_else(|| invalid("compiled histogram result row outside source"))?
                as usize;
            if slot.count == 0 {
                return Err(invalid("compiled histogram published zero count"));
            }
            actual_count = actual_count
                .checked_add(slot.count)
                .ok_or_else(|| invalid("compiled histogram count overflow"))?;
            let key = &strings[index];
            actual_payload = actual_payload
                .checked_add(key.len() as u64)
                .ok_or_else(|| invalid("compiled histogram payload overflow"))?;
            entries.push(MapEntryDraft {
                items: Box::new([
                    ValueDataDraft::String(key.to_string()),
                    ValueDataDraft::U64(slot.count),
                ]),
            });
        }
        if entries.len() as u64 != distinct
            || actual_payload != payload
            || actual_count != loan.frame.count
        {
            return Err(invalid("compiled histogram result inventory differs"));
        }
        let canonical = SnapshotCanonicalizationBudget::new(finalization);
        let result = ValueDraft {
            schema: metadata.plan.values()[output.get() as usize].schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::Map(entries.into_boxed_slice()),
        }
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&metadata.schemas)
                .with_canonicalization_budget(&canonical),
        )
        .map_err(|_| {
            library_error(NativeCollectionInvocationError::Kernel(
                mech_core::ResidentKernelError::InvalidOutput,
            ))
        })?
        .into_memory_budget(&mut reservation)?;
        self.state
            .arena
            .put(output.get(), &result, &mut self.state.control.work)?;
        self.state.consumed = true;
        // The source, banks and reservations retire after canonical publication.
        drop(loan);
        Ok(Some(1))
    }
}
